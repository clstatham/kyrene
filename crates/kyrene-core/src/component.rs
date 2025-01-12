use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use downcast_rs::{impl_downcast, DowncastSync};
use kyrene_util::{FxHashMap, TypeIdMap};

use crate::{
    entity::Entity,
    loan::{LoanMut, LoanStorage},
};

pub trait Component: DowncastSync {}
impl_downcast!(sync Component);
impl<T: DowncastSync> Component for T {}

pub struct ComponentEntry {
    type_id: TypeId,
    loan: LoanStorage<Box<dyn Component>>,
}

impl ComponentEntry {
    pub fn new<T: Component>(component: T) -> Self {
        ComponentEntry {
            type_id: TypeId::of::<T>(),
            loan: LoanStorage::new(Box::new(component)),
        }
    }

    pub fn is<T: Component>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }
}

pub struct Ref<T: Component> {
    inner: LoanMut<Box<dyn Component>>,
    _marker: PhantomData<T>,
}

impl<T: Component> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.downcast_ref().unwrap()
    }
}

impl<T: Component> DerefMut for Ref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.downcast_mut().unwrap()
    }
}

#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct EntityComponents(TypeIdMap<ComponentEntry>);

impl Deref for EntityComponents {
    type Target = TypeIdMap<ComponentEntry>;

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
    pub fn insert_discard<T: Component>(&mut self, entity: Entity, component: T) {
        let component_type_id = TypeId::of::<T>();

        self.map
            .entry(entity)
            .or_default()
            .insert(component_type_id, ComponentEntry::new(component));
    }

    pub async fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let component_type_id = TypeId::of::<T>();

        let components = self.map.get_mut(&entity)?;

        let component = components.remove(&component_type_id)?;

        let component = component.loan.await_owned().await;

        let component = *component.downcast::<T>().unwrap_or_else(|_| unreachable!());
        Some(component)
    }

    pub async fn get<T: Component>(&mut self, entity: Entity) -> Option<Ref<T>> {
        let component_type_id = TypeId::of::<T>();

        let components = self.map.get_mut(&entity)?;

        let component = components.get_mut(&component_type_id)?;

        let inner = component.loan.await_loan_mut().await;

        Some(Ref {
            inner,
            _marker: PhantomData,
        })
    }
}
