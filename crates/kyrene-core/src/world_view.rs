use std::sync::Arc;

use async_fn_traits::AsyncFnMut2;
use futures::StreamExt;

use crate::{
    bundle::Bundle,
    component::{Component, Mut, Ref},
    entity::{Entity, EntitySet},
    event::Event,
    lock::RwLock,
    query::{Query, Queryable},
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

    pub async fn all_entities(&self) -> EntitySet {
        self.world.read().await.entity_iter().collect()
    }

    pub async fn insert<T: Component>(&self, entity: Entity, component: T) -> Option<T> {
        self.world.write().await.insert(entity, component).await
    }

    pub async fn insert_bundle<T: Bundle>(&self, entity: Entity, bundle: T) {
        self.world.write().await.insert_bundle(entity, bundle);
    }

    pub async fn spawn<T: Bundle>(&self, bundle: T) -> Entity {
        self.world.write().await.spawn(bundle)
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

    pub async fn has<T: Component>(&self, entity: Entity) -> bool {
        self.world.read().await.has::<T>(entity)
    }

    pub async fn entities_with<T: Component>(&self) -> EntitySet {
        self.world.read().await.entities_with::<T>().collect()
    }

    pub async fn query<Q: Queryable>(&self) -> Query<Q> {
        Query::new(self.clone()).await
    }

    pub async fn query_iter<Q>(&self, mut f: impl AsyncFnMut2<Self, Q::Item>)
    where
        Q: Queryable,
    {
        let q = self.query::<Q>().await;
        let mut iter = Box::pin(q.iter());
        while let Some(item) = iter.next().await {
            f(self.clone(), item).await;
        }
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

    pub async fn await_resource<T: Component>(&self) -> Ref<T> {
        self.world.write().await.await_resource::<T>().await
    }

    pub async fn await_resource_mut<T: Component>(&self) -> Mut<T> {
        self.world.write().await.await_resource_mut::<T>().await
    }

    pub async fn add_event<T: Component>(&self) -> Event<T> {
        self.world.write().await.add_event::<T>()
    }

    pub async fn get_event<T: Component>(&self) -> Option<Event<T>> {
        self.world.read().await.get_event::<T>()
    }

    pub async fn has_event<T: Component>(&self) -> bool {
        self.world.read().await.has_event::<T>()
    }

    pub async fn fire_event<T: Component>(&self, payload: T, await_all_handlers: bool) -> usize {
        let event = { self.world.read().await.get_event::<T>().unwrap() };
        event.fire(self.clone(), payload, await_all_handlers).await
    }
}
