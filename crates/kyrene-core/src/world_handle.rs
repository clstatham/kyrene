use std::{future::Future, sync::Arc};

use async_fn_traits::AsyncFnMut2;
use futures::StreamExt;

use crate::{
    bundle::Bundle,
    component::{Component, Mut, Ref},
    entity::{Entity, EntitySet},
    event::EventDispatcher,
    handler::{EventHandlerMeta, HandlerParam},
    lock::RwLock,
    query::{Query, Queryable},
    util::TypeInfo,
    world::World,
};

#[derive(Clone)]
pub struct WorldHandle {
    pub(crate) world: Arc<RwLock<World>>,
}

impl WorldHandle {
    pub fn from_inner(world: Arc<RwLock<World>>) -> Self {
        Self { world }
    }
}

impl WorldHandle {
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
        self.world.read().await.get::<T>(entity).await
    }

    pub async fn get_mut<T: Component>(&self, entity: Entity) -> Option<Mut<T>> {
        self.world.read().await.get_mut::<T>(entity).await
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

    pub(crate) async fn has_resource_dyn(&self, resource_type_id: TypeInfo) -> bool {
        self.world.read().await.has_resource_dyn(resource_type_id)
    }

    pub async fn get_resource<T: Component>(&self) -> Option<Ref<T>> {
        self.world.read().await.get_resource::<T>().await
    }

    pub async fn get_resource_mut<T: Component>(&self) -> Option<Mut<T>> {
        self.world.read().await.get_resource_mut::<T>().await
    }

    pub async fn add_event<T: Component>(&self) -> EventDispatcher<T> {
        self.world.write().await.add_event::<T>()
    }

    pub async fn get_event<T: Component>(&self) -> Option<EventDispatcher<T>> {
        self.world.read().await.get_event::<T>()
    }

    pub async fn has_event<T: Component>(&self) -> bool {
        self.world.read().await.has_event::<T>()
    }

    pub async fn fire_event<T: Component>(&self, event: T, await_all_handlers: bool) -> usize {
        let dis = { self.world.read().await.get_event::<T>().unwrap() };
        dis.fire(self.clone(), event, await_all_handlers).await
    }
}

impl HandlerParam for WorldHandle {
    type Item = WorldHandle;
    type State = ();

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default()
    }

    async fn init_state(_world: WorldHandle) -> Self::State {}

    async fn fetch(world: WorldHandle, _: &mut ()) -> Self::Item {
        world.clone()
    }

    async fn can_run(_world: WorldHandle, _: &()) -> bool {
        true
    }
}

pub trait FromWorldHandle: Sized {
    fn from_world_handle(world: &WorldHandle) -> impl Future<Output = Self> + Send;
}

impl<T: Default> FromWorldHandle for T {
    async fn from_world_handle(_world: &WorldHandle) -> Self {
        Self::default()
    }
}
