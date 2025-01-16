use std::{marker::PhantomData, sync::Arc};

use crate::{
    component::{DynComponent, Mut},
    lock::RwLock,
    prelude::{Component, Ref},
    util::{TypeIdMap, TypeInfo},
};

#[derive(Default)]
pub struct Resources {
    map: TypeIdMap<Arc<RwLock<Option<DynComponent>>>>,
}

impl Resources {
    pub async fn insert<T: Component>(&mut self, resource: T) -> Option<T> {
        let component_type_id = TypeInfo::of::<T>();

        let old = self.map.insert(
            component_type_id,
            Arc::new(RwLock::new(Some(DynComponent::new(resource)))),
        )?;

        let old = old.write().await.take().unwrap();
        let old: T = *old.component.downcast().unwrap_or_else(|_| unreachable!());
        Some(old)
    }

    pub async fn remove<T: Component>(&mut self) -> Option<T> {
        let component_type_id = TypeInfo::of::<T>();

        let component = self.map.remove(&component_type_id)?;

        let component = component.write().await.take().unwrap();
        let component: T = *component
            .component
            .downcast()
            .unwrap_or_else(|_| unreachable!());
        Some(component)
    }

    pub fn contains<T: Component>(&self) -> bool {
        let component_type_id = TypeInfo::of::<T>();
        self.map.contains_key(&component_type_id)
    }

    pub(crate) fn contains_dyn(&self, resource_type_id: TypeInfo) -> bool {
        self.map.contains_key(&resource_type_id)
    }

    pub async fn get<T: Component>(&self) -> Option<Ref<T>> {
        let component_type_id = TypeInfo::of::<T>();

        let component = self.map.get(&component_type_id)?;
        let inner = component.clone().read_owned().await;

        Some(Ref {
            inner,
            _marker: PhantomData,
        })
    }

    pub async fn get_mut<T: Component>(&self) -> Option<Mut<T>> {
        let component_type_id = TypeInfo::of::<T>();

        let component = self.map.get(&component_type_id)?;
        let inner = component.clone().write_owned().await;

        Some(Mut {
            inner,
            _marker: PhantomData,
        })
    }

    pub async fn wait_for<T: Component>(&mut self) -> Ref<T> {
        let mut start = tokio::time::Instant::now();

        loop {
            if let Some(res) = self.get::<T>().await {
                return res;
            }

            tokio::task::yield_now().await;

            if start.elapsed() >= tokio::time::Duration::from_secs(5) {
                tracing::warn!(
                    "Waiting a long time for resource ref {}...",
                    std::any::type_name::<T>()
                );
                start = tokio::time::Instant::now();
            }
        }
    }

    pub async fn wait_for_mut<T: Component>(&mut self) -> Mut<T> {
        let mut start = tokio::time::Instant::now();

        loop {
            if let Some(res) = self.get_mut::<T>().await {
                return res;
            }

            tokio::task::yield_now().await;

            if start.elapsed() >= tokio::time::Duration::from_secs(5) {
                tracing::warn!(
                    "Waiting a long time for resource mut {}...",
                    std::any::type_name::<T>()
                );
                start = tokio::time::Instant::now();
            }
        }
    }
}
