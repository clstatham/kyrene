use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{lock::Mutex, prelude::Component};

pub struct Loan<T>(Arc<T>);

impl<T> Loan<T> {
    pub fn strong_count(this: &Self) -> usize {
        Arc::strong_count(&this.0)
    }
}

impl<T> Clone for Loan<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Loan<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct LoanMut<T: Component> {
    inner: Option<T>,
    outer: Arc<Mutex<Option<T>>>,
}

impl<T: Component> Deref for LoanMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T: Component> DerefMut for LoanMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl<T: Component> Drop for LoanMut<T> {
    fn drop(&mut self) {
        let inner = self.inner.take();
        let outer = self.outer.clone();
        tokio::spawn(async move {
            *outer.lock().await = inner;
        });
    }
}

pub enum LoanStorage<T: Component> {
    Vacant,
    Owned(T),
    Loan(Arc<T>),
    LoanMut(Arc<Mutex<Option<T>>>),
}

impl<T: Component> Default for LoanStorage<T> {
    fn default() -> Self {
        Self::Vacant
    }
}

impl<T: Component> LoanStorage<T> {
    pub fn new(value: T) -> Self {
        Self::Owned(value)
    }

    pub async fn into_owned(self) -> Result<T, Self> {
        match self {
            Self::Vacant => Err(Self::Vacant),
            Self::Owned(value) => Ok(value),
            Self::Loan(value) => match Arc::try_unwrap(value) {
                Ok(value) => Ok(value),
                Err(value) => Err(Self::Loan(value)),
            },
            Self::LoanMut(value) => {
                let mut guard = value.lock().await;
                let maybe = guard.take();
                drop(guard);
                maybe.ok_or_else(|| Self::LoanMut(value))
            }
        }
    }

    pub async fn into_loaned(self) -> Result<Loan<T>, Self> {
        match self {
            Self::Vacant => Err(Self::Vacant),
            Self::Owned(value) => Ok(Loan(Arc::new(value))),
            Self::Loan(value) => Ok(Loan(value)),
            Self::LoanMut(value) => {
                let mut guard = value.lock().await;
                let maybe = guard.take();
                drop(guard);
                maybe
                    .map(|value| Loan(Arc::new(value)))
                    .ok_or_else(|| Self::LoanMut(value))
            }
        }
    }

    pub fn as_owned_ref(&self) -> Option<&T> {
        match self {
            Self::Owned(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_owned_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Owned(value) => Some(value),
            _ => None,
        }
    }

    pub async fn loan(&mut self) -> Option<Loan<T>> {
        let this = std::mem::replace(self, Self::Vacant);
        match this.into_loaned().await {
            Ok(loan) => {
                *self = Self::Loan(loan.0.clone());
                Some(loan)
            }
            Err(this) => {
                *self = this;
                None
            }
        }
    }

    pub async fn loan_mut(&mut self) -> Option<LoanMut<T>> {
        let this = std::mem::replace(self, Self::Vacant);
        match this.into_owned().await {
            Ok(value) => {
                let outer = Arc::new(Mutex::new(None));
                *self = Self::LoanMut(outer.clone());
                Some(LoanMut {
                    inner: Some(value),
                    outer,
                })
            }
            Err(this) => {
                *self = this;
                None
            }
        }
    }

    pub async fn await_owned(mut self) -> T {
        loop {
            match self.into_owned().await {
                Ok(t) => return t,
                Err(this) => self = this,
            }

            tokio::task::yield_now().await;
        }
    }

    pub async fn await_loan(&mut self) -> Loan<T> {
        loop {
            if let Some(loan) = self.loan().await {
                return loan;
            }

            tokio::task::yield_now().await;
        }
    }

    pub async fn await_loan_mut(&mut self) -> LoanMut<T> {
        loop {
            if let Some(loan) = self.loan_mut().await {
                return loan;
            }

            tokio::task::yield_now().await;
        }
    }
}

impl<T: Component> From<T> for LoanStorage<T> {
    fn from(value: T) -> Self {
        Self::Owned(value)
    }
}

impl<T: Component> From<Loan<T>> for LoanStorage<T> {
    fn from(value: Loan<T>) -> Self {
        Self::Loan(value.0)
    }
}
