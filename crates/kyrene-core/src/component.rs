use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use downcast_rs::{impl_downcast, DowncastSync};
use itertools::Either;

use crate::{
    bundle::Bundle,
    entity::{Entity, EntityMap, EntitySet},
    loan::{Loan, LoanMut, LoanStorage},
    util::{TypeIdMap, TypeInfo},
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
    pub(crate) inner: Loan<Box<dyn Component>>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Component> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.downcast_ref().unwrap()
    }
}

pub struct Mut<T: Component> {
    pub(crate) inner: LoanMut<Box<dyn Component>>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Component> Deref for Mut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.downcast_ref().unwrap()
    }
}

impl<T: Component> DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.downcast_mut().unwrap()
    }
}

#[derive(Default)]
pub struct Components {
    entity_map: EntityMap<TypeIdMap<ComponentEntry>>,
    component_map: TypeIdMap<EntitySet>,
}

impl Components {
    pub async fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        let component_type_id = TypeInfo::of::<T>();

        self.component_map
            .entry(component_type_id)
            .or_default()
            .insert(entity);

        let old = self
            .entity_map
            .entry(entity)
            .or_default()
            .insert(component_type_id, ComponentEntry::new(component))?;

        let old = old.loan.await_owned().await;
        let old: T = *old.downcast().unwrap_or_else(|_| unreachable!());
        Some(old)
    }

    pub fn insert_discard<T: Component>(&mut self, entity: Entity, component: T) {
        let component_type_id = TypeInfo::of::<T>();

        self.entity_map
            .entry(entity)
            .or_default()
            .insert(component_type_id, ComponentEntry::new(component));

        self.component_map
            .entry(component_type_id)
            .or_default()
            .insert(entity);
    }

    pub fn insert_bundle<T: Bundle>(&mut self, entity: Entity, bundle: T) {
        for (component_type_id, component) in bundle.into_dyn_components() {
            self.entity_map.entry(entity).or_default().insert(
                component_type_id,
                ComponentEntry {
                    loan: LoanStorage::new(component),
                    type_id: component_type_id.type_id,
                },
            );

            self.component_map
                .entry(component_type_id)
                .or_default()
                .insert(entity);
        }
    }

    pub async fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let component_type_id = TypeInfo::of::<T>();
        let components = self.entity_map.get_mut(&entity)?;
        let component = components.remove(&component_type_id)?;

        self.component_map
            .get_mut(&component_type_id)
            .unwrap()
            .remove(&entity);

        let component = component.loan.await_owned().await;
        let component = *component.downcast::<T>().unwrap_or_else(|_| unreachable!());
        Some(component)
    }

    pub async fn get<T: Component>(&mut self, entity: Entity) -> Option<Ref<T>> {
        let component_type_id = TypeInfo::of::<T>();
        let components = self.entity_map.get_mut(&entity)?;
        let component = components.get_mut(&component_type_id)?;
        let inner = component.loan.await_loan().await;
        Some(Ref {
            inner,
            _marker: PhantomData,
        })
    }

    pub async fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<Mut<T>> {
        let component_type_id = TypeInfo::of::<T>();
        let components = self.entity_map.get_mut(&entity)?;
        let component = components.get_mut(&component_type_id)?;
        let inner = component.loan.await_loan_mut().await;
        Some(Mut {
            inner,
            _marker: PhantomData,
        })
    }

    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        if let Some(components) = self.entity_map.get(&entity) {
            components.contains_key(&TypeInfo::of::<T>())
        } else {
            false
        }
    }

    pub fn entities_with<T: Component>(&self) -> impl Iterator<Item = Entity> + use<'_, T> {
        if let Some(entities) = self.component_map.get(&TypeInfo::of::<T>()) {
            Either::Left(entities.iter().copied())
        } else {
            Either::Right(std::iter::empty())
        }
    }

    pub fn entity_iter(&self) -> impl Iterator<Item = Entity> + use<'_> {
        self.entity_map.keys().copied()
    }
}
