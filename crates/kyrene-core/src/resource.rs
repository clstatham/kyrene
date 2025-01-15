use std::marker::PhantomData;

use crate::{
    component::Mut,
    loan::LoanStorage,
    prelude::{Component, Ref},
    util::{TypeIdMap, TypeInfo},
};

#[derive(Default)]
pub struct Resources {
    map: TypeIdMap<LoanStorage<Box<dyn Component>>>,
}

impl Resources {
    pub async fn insert<T: Component>(&mut self, resource: T) -> Option<T> {
        let component_type_id = TypeInfo::of::<T>();

        let old = self
            .map
            .insert(component_type_id, LoanStorage::new(Box::new(resource)))?;

        let old = old.await_owned().await;
        let old: T = *old.downcast().unwrap_or_else(|_| unreachable!());
        Some(old)
    }

    pub async fn remove<T: Component>(&mut self) -> Option<T> {
        let component_type_id = TypeInfo::of::<T>();

        let component = self.map.remove(&component_type_id)?;

        let component = component.await_owned().await;
        let component: T = *component.downcast().unwrap_or_else(|_| unreachable!());
        Some(component)
    }

    pub fn contains<T: Component>(&self) -> bool {
        let component_type_id = TypeInfo::of::<T>();
        self.map.contains_key(&component_type_id)
    }

    pub async fn get<T: Component>(&mut self) -> Option<Ref<T>> {
        let component_type_id = TypeInfo::of::<T>();

        let component = self.map.get_mut(&component_type_id)?;
        let inner = component.await_loan().await;

        Some(Ref {
            inner,
            _marker: PhantomData,
        })
    }

    pub async fn get_mut<T: Component>(&mut self) -> Option<Mut<T>> {
        let component_type_id = TypeInfo::of::<T>();

        let component = self.map.get_mut(&component_type_id)?;
        let inner = component.await_loan_mut().await;

        Some(Mut {
            inner,
            _marker: PhantomData,
        })
    }
}
