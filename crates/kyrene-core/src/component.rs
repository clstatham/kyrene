use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use downcast_rs::{impl_downcast, DowncastSync};
use itertools::Either;

use crate::{
    bundle::Bundle,
    entity::{Entity, EntityMap, EntitySet},
    lock::{Read, RwLock, Write},
    util::{TypeIdMap, TypeInfo},
};

pub trait Component: DowncastSync {}
impl_downcast!(sync Component);
impl<T: DowncastSync> Component for T {}

pub struct DynComponent {
    pub(crate) type_id: TypeInfo,
    pub(crate) component: Box<dyn Component>,
}

impl DynComponent {
    pub fn new<T: Component>(component: T) -> Self {
        Self {
            type_id: TypeInfo::of::<T>(),
            component: Box::new(component),
        }
    }
}

impl Debug for DynComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id.fmt(f)
    }
}

impl Deref for DynComponent {
    type Target = dyn Component;

    fn deref(&self) -> &Self::Target {
        &*self.component
    }
}

impl DerefMut for DynComponent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.component
    }
}

pub struct ComponentStorage {
    type_id: TypeInfo,
    loan: Arc<RwLock<Option<DynComponent>>>,
}

impl ComponentStorage {
    pub fn new<T: Component>(component: T) -> Self {
        ComponentStorage {
            type_id: TypeInfo::of::<T>(),
            loan: Arc::new(RwLock::new(Some(DynComponent::new(component)))),
        }
    }

    pub fn is<T: Component>(&self) -> bool {
        self.type_id == TypeInfo::of::<T>()
    }
}

pub struct Ref<T: Component> {
    pub(crate) inner: Read<Option<DynComponent>>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Component> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap().downcast_ref().unwrap()
    }
}

impl<T: Component + Debug> Debug for Ref<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner
            .as_ref()
            .unwrap()
            .downcast_ref::<T>()
            .unwrap()
            .fmt(f)
    }
}

pub struct Mut<T: Component> {
    pub(crate) inner: Write<Option<DynComponent>>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Component> Deref for Mut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap().downcast_ref().unwrap()
    }
}

impl<T: Component> DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap().downcast_mut().unwrap()
    }
}

impl<T: Component + Debug> Debug for Mut<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner
            .as_ref()
            .unwrap()
            .downcast_ref::<T>()
            .unwrap()
            .fmt(f)
    }
}

#[derive(Default)]
pub struct Components {
    entity_map: EntityMap<TypeIdMap<ComponentStorage>>,
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
            .insert(component_type_id, ComponentStorage::new(component))?;

        let old = old.loan.write().await.take().unwrap();
        let old: T = *old.component.downcast().unwrap_or_else(|_| unreachable!());
        Some(old)
    }

    pub fn insert_discard<T: Component>(&mut self, entity: Entity, component: T) {
        let component_type_id = TypeInfo::of::<T>();

        self.entity_map
            .entry(entity)
            .or_default()
            .insert(component_type_id, ComponentStorage::new(component));

        self.component_map
            .entry(component_type_id)
            .or_default()
            .insert(entity);
    }

    pub fn insert_bundle<T: Bundle>(&mut self, entity: Entity, bundle: T) {
        for (component_type_id, component) in bundle.into_dyn_components() {
            self.entity_map.entry(entity).or_default().insert(
                component_type_id,
                ComponentStorage {
                    loan: Arc::new(RwLock::new(Some(DynComponent {
                        type_id: component_type_id,
                        component,
                    }))),
                    type_id: component_type_id,
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

        let component = component.loan.write().await.take().unwrap();
        let component = *component
            .component
            .downcast::<T>()
            .unwrap_or_else(|_| unreachable!());
        Some(component)
    }

    pub async fn get<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        let component_type_id = TypeInfo::of::<T>();
        let components = self.entity_map.get(&entity)?;
        let component = components.get(&component_type_id)?;
        let inner = component.loan.clone().read_owned().await;
        Some(Ref {
            inner,
            _marker: PhantomData,
        })
    }

    pub async fn get_mut<T: Component>(&self, entity: Entity) -> Option<Mut<T>> {
        let component_type_id = TypeInfo::of::<T>();
        let components = self.entity_map.get(&entity)?;
        let component = components.get(&component_type_id)?;
        let inner = component.loan.clone().write_owned().await;
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
