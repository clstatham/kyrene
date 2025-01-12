use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use downcast_rs::{impl_downcast, DowncastSync};
use kyrene_util::{FxHashMap, TypeIdMap};

use crate::{entity::Entity, lock::Mutex};

pub trait Component: DowncastSync {}
impl_downcast!(sync Component);
impl<T: DowncastSync> Component for T {}

pub struct Ref<T: Component> {
    inner: Option<T>,
    loan: Arc<Mutex<Option<Box<dyn Component>>>>,
}

impl<T: Component> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T: Component> DerefMut for Ref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl<T: Component> Drop for Ref<T> {
    fn drop(&mut self) {
        let Ref { inner, loan } = self;
        let mut lock = loan.try_lock().unwrap();
        let inner = inner.take().unwrap();
        let inner: Box<dyn Component> = Box::new(inner);
        *lock = Some(inner);
    }
}

#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct EntityComponents(TypeIdMap<Arc<Mutex<Option<Box<dyn Component>>>>>);

impl Deref for EntityComponents {
    type Target = TypeIdMap<Arc<Mutex<Option<Box<dyn Component>>>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EntityComponents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
pub struct Components {
    map: FxHashMap<Entity, EntityComponents>,
}

impl Components {
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        let component_type_id = TypeId::of::<T>();

        let old = self.map.entry(entity).or_default().insert(
            component_type_id,
            Arc::new(Mutex::new(Some(Box::new(component)))),
        );

        if let Some(old) = old {
            if let Ok(old) = Arc::try_unwrap(old) {
                let old = old.into_inner();

                let Some(old) = old else {
                    panic!("Internal error: Old component still borrowed");
                };

                if let Ok(old) = old.downcast::<T>() {
                    // todo: fire EntityEvent
                    Some(*old)
                } else {
                    panic!("Internal error: component downcast failed")
                }
            } else {
                log::debug!("Couldn't acquire unique access to replaced component");
                None
            }
        } else {
            // todo: fire EntityEvent
            None
        }
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let component_type_id = TypeId::of::<T>();

        let components = self.map.get_mut(&entity)?;

        let component = components.remove(&component_type_id)?;

        let Ok(component) = Arc::try_unwrap(component) else {
            log::debug!("Couldn't acquire unique access to removed component");
            return None;
        };

        let component = component.into_inner();
        let Some(component) = component else {
            panic!("Internal error: Component still borrowed");
        };

        if let Ok(component) = component.downcast::<T>() {
            // todo: fire EntityEvent
            Some(*component)
        } else {
            panic!("Internal error: component downcast failed")
        }
    }

    pub async fn get_async<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        let component_type_id = TypeId::of::<T>();

        let components = self.map.get(&entity)?;

        let component = components.get(&component_type_id)?;

        let mut locked = component.lock().await;

        let inner = locked.take()?;

        let inner = *inner.downcast().unwrap_or_else(|_| unreachable!());

        Some(Ref {
            inner: Some(inner),
            loan: component.clone(),
        })
    }
}
