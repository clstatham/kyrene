use std::{any::TypeId, marker::PhantomData};

use crate::{
    loan::LoanStorage,
    prelude::{Component, Ref},
    util::TypeIdMap,
};

#[derive(Default)]
pub struct Resources {
    map: TypeIdMap<LoanStorage<Box<dyn Component>>>,
}

impl Resources {
    pub async fn insert<T: Component>(&mut self, resource: T) -> Option<T> {
        let component_type_id = TypeId::of::<T>();

        let old = self
            .map
            .insert(component_type_id, LoanStorage::new(Box::new(resource)))?;

        let old = old.await_owned().await;
        let old: T = *old.downcast().unwrap_or_else(|_| unreachable!());
        Some(old)
    }

    pub async fn remove<T: Component>(&mut self) -> Option<T> {
        let component_type_id = TypeId::of::<T>();

        let component = self.map.remove(&component_type_id)?;

        let component = component.await_owned().await;
        let component: T = *component.downcast().unwrap_or_else(|_| unreachable!());
        Some(component)
    }

    pub async fn get<T: Component>(&mut self) -> Option<Ref<T>> {
        let component_type_id = TypeId::of::<T>();

        let component = self.map.get_mut(&component_type_id)?;
        let inner = component.await_loan_mut().await;

        Some(Ref {
            inner,
            _marker: PhantomData,
        })
    }
}
