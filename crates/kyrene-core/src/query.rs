use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, Stream, StreamExt};

use crate::{
    component::Mut,
    entity::{Entity, EntitySet},
    handler::{EventHandlerMeta, HandlerParam},
    prelude::{Component, Ref, WorldHandle},
};

pub struct QueryFilterState {
    entities_matching: EntitySet,
}

pub trait Queryable: Send + Sync {
    type Item: Send + Sync;

    fn filter_state(
        world: &WorldHandle,
        state: &mut QueryFilterState,
    ) -> impl Future<Output = ()> + Send;

    fn get(
        world: &WorldHandle,
        state: &QueryFilterState,
        entity: Entity,
    ) -> impl Future<Output = Option<Self::Item>> + Send;

    fn iter(world: &WorldHandle, state: &QueryFilterState)
        -> impl Stream<Item = Self::Item> + Send;
}

impl Queryable for Entity {
    type Item = Entity;

    async fn filter_state(_world: &WorldHandle, _state: &mut QueryFilterState) {}

    async fn get(
        _world: &WorldHandle,
        _state: &QueryFilterState,
        entity: Entity,
    ) -> Option<Self::Item> {
        Some(entity)
    }

    fn iter(
        _world: &WorldHandle,
        state: &QueryFilterState,
    ) -> impl Stream<Item = Self::Item> + Send {
        futures::stream::iter(state.entities_matching.iter().copied()).fuse()
    }
}

impl<T: Component> Queryable for &T {
    type Item = Ref<T>;

    async fn filter_state(world: &WorldHandle, state: &mut QueryFilterState) {
        let old_entities = state.entities_matching.clone();
        for entity in old_entities {
            if !world.has::<T>(entity).await {
                state.entities_matching.remove(&entity);
            }
        }
    }

    async fn get(
        world: &WorldHandle,
        state: &QueryFilterState,
        entity: Entity,
    ) -> Option<Self::Item> {
        if state.entities_matching.contains(&entity) {
            world.get::<T>(entity).await
        } else {
            None
        }
    }

    fn iter(
        world: &WorldHandle,
        state: &QueryFilterState,
    ) -> impl Stream<Item = Self::Item> + Send {
        let futs = FuturesUnordered::new();
        for entity in state.entities_matching.iter() {
            futs.push(async move { world.get::<T>(*entity).await.unwrap() });
        }
        futs.fuse()
    }
}

impl<T: Component> Queryable for &mut T {
    type Item = Mut<T>;

    async fn filter_state(world: &WorldHandle, state: &mut QueryFilterState) {
        let entities_with_component = world.entities_with::<T>().await;
        state
            .entities_matching
            .retain(|e| entities_with_component.contains(e));
    }

    async fn get(
        world: &WorldHandle,
        state: &QueryFilterState,
        entity: Entity,
    ) -> Option<Self::Item> {
        if state.entities_matching.contains(&entity) {
            world.get_mut::<T>(entity).await
        } else {
            None
        }
    }

    fn iter(
        world: &WorldHandle,
        state: &QueryFilterState,
    ) -> impl Stream<Item = Self::Item> + Send {
        let futs = FuturesUnordered::new();
        for entity in state.entities_matching.iter() {
            futs.push(async move { world.get_mut::<T>(*entity).await.unwrap() });
        }
        futs.fuse()
    }
}

pub struct ZipStream<T> {
    zip: T,
}

macro_rules! impl_zip_stream_tuple {
    ($($name:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($name: Stream + Unpin),*> Stream for ZipStream<($($name,)*)> {
            type Item = ($($name::Item,)*);

            fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let ($(ref mut $name,)*) = self.zip;

                $(
                    let $name = match $name.poll_next_unpin(cx) {
                        Poll::Ready(Some(elt)) => elt,
                        Poll::Ready(None) => return Poll::Ready(None),
                        Poll::Pending => return Poll::Pending,
                    };
                )*

                Poll::Ready(Some(($($name,)*)))
            }
        }
    };
}
impl_zip_stream_tuple!(A);
impl_zip_stream_tuple!(A, B);
impl_zip_stream_tuple!(A, B, C);
impl_zip_stream_tuple!(A, B, C, D);
impl_zip_stream_tuple!(A, B, C, D, E);
impl_zip_stream_tuple!(A, B, C, D, E, F);
impl_zip_stream_tuple!(A, B, C, D, E, F, G);
impl_zip_stream_tuple!(A, B, C, D, E, F, G, H);

macro_rules! impl_queryable_tuple {
    ($($name:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($name: Queryable),*> Queryable for ($($name,)*) {
            type Item = ($($name::Item,)*);

            async fn filter_state(world: &WorldHandle, state: &mut QueryFilterState) {
                $($name::filter_state(world, state).await);*
            }

            async fn get(
                world: &WorldHandle,
                state: &QueryFilterState,
                entity: Entity,
            ) -> Option<Self::Item> {
                Some(($(
                    $name::get(world, state, entity).await?,
                )*))
            }

            fn iter(world: &WorldHandle, state: &QueryFilterState) -> impl Stream<Item = Self::Item> + Send {
                ZipStream { zip: ($(
                    Box::pin($name::iter(world, state)),
                )*)}.fuse()
            }
        }
    };
}
impl_queryable_tuple!(A);
impl_queryable_tuple!(A, B);
impl_queryable_tuple!(A, B, C);
impl_queryable_tuple!(A, B, C, D);
impl_queryable_tuple!(A, B, C, D, E);
impl_queryable_tuple!(A, B, C, D, E, F);
impl_queryable_tuple!(A, B, C, D, E, F, G);
impl_queryable_tuple!(A, B, C, D, E, F, G, H);

pub struct Query<Q: Queryable> {
    state: QueryFilterState,
    world: WorldHandle,
    _marker: PhantomData<Q>,
}

impl<Q: Queryable> Query<Q> {
    pub async fn new(world: WorldHandle) -> Self {
        let mut state = QueryFilterState {
            entities_matching: world.all_entities().await,
        };

        Q::filter_state(&world, &mut state).await;

        Self {
            state,
            world,
            _marker: PhantomData,
        }
    }

    pub async fn get(&self, entity: Entity) -> Option<Q::Item> {
        Q::get(&self.world, &self.state, entity).await
    }

    pub fn iter(&self) -> impl Stream<Item = Q::Item> + use<'_, Q> {
        Q::iter(&self.world, &self.state)
    }
}

impl<Q: Queryable> HandlerParam for Query<Q> {
    type Item = Query<Q>;

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default()
    }

    async fn fetch(world: WorldHandle) -> Self::Item {
        world.query::<Q>().await
    }

    async fn can_run(_world: WorldHandle) -> bool {
        true
    }
}
