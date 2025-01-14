use std::sync::Arc;

use crate::{
    component::{Component, Mut, Ref},
    entity::Entity,
    event::Event,
    lock::RwLock,
    world::World,
};

#[derive(Clone)]
pub struct WorldView {
    pub(crate) world: Arc<RwLock<World>>,
}

impl WorldView {
    pub fn from_inner(world: Arc<RwLock<World>>) -> Self {
        Self { world }
    }
}

impl WorldView {
    pub async fn entity(&self) -> Entity {
        self.world.write().await.entity()
    }

    pub async fn insert<T: Component>(&self, entity: Entity, component: T) -> Option<T> {
        self.world.write().await.insert(entity, component).await
    }

    pub async fn remove<T: Component>(&self, entity: Entity) -> Option<T> {
        self.world.write().await.remove::<T>(entity).await
    }

    pub async fn get<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        self.world.write().await.get::<T>(entity).await
    }

    pub async fn get_mut<T: Component>(&self, entity: Entity) -> Option<Mut<T>> {
        self.world.write().await.get_mut::<T>(entity).await
    }

    pub async fn insert_resource<T: Component>(&self, resource: T) -> Option<T> {
        self.world.write().await.insert_resource(resource).await
    }

    pub async fn remove_resource<T: Component>(&self) -> Option<T> {
        self.world.write().await.remove_resource::<T>().await
    }

    pub async fn has_resource<T: Component>(&self) -> bool {
        self.world.read().await.has_resource::<T>()
    }

    pub async fn get_resource<T: Component>(&self) -> Option<Ref<T>> {
        self.world.write().await.get_resource::<T>().await
    }

    pub async fn get_resource_mut<T: Component>(&self) -> Option<Mut<T>> {
        self.world.write().await.get_resource_mut::<T>().await
    }

    pub async fn add_event<T: Component>(&self) -> Event<T> {
        self.world.write().await.add_event::<T>()
    }

    pub async fn get_event<T: Component>(&self) -> Option<Event<T>> {
        self.world.read().await.get_event::<T>()
    }

    pub async fn fire_event<T: Component>(&self, payload: T, await_all_handlers: bool) -> usize {
        let event = { self.world.read().await.get_event::<T>().unwrap() };
        event.fire(self.clone(), payload, await_all_handlers).await
    }
}
