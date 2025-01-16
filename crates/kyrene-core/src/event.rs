use std::{any::TypeId, collections::VecDeque, marker::PhantomData, sync::Arc};

use downcast_rs::{impl_downcast, DowncastSync};
use petgraph::prelude::*;

use crate::{
    handler::{DynEventHandlers, IntoHandlerConfig},
    prelude::WorldHandle,
    util::FxHashMap,
};

pub trait Event: DowncastSync {}
impl_downcast!(Event);
impl<T: DowncastSync> Event for T {}

pub struct EventDispatcher<T: Event> {
    event: DynEvent,
    _marker: PhantomData<T>,
}

impl<T: Event> Clone for EventDispatcher<T> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: Event> EventDispatcher<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            event: DynEvent::new::<T>(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn from_dyn_event(event: DynEvent) -> Self {
        assert_eq!(event.type_id, TypeId::of::<T>());
        Self {
            event,
            _marker: PhantomData,
        }
    }

    pub fn add_handler<F, M>(&self, handler: F)
    where
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        self.event.add_handler(handler);
    }

    pub async fn fire(&self, world: WorldHandle, event: T, await_all_handlers: bool) -> usize {
        self.event.fire::<T>(world, event, await_all_handlers).await
    }
}

pub(crate) struct DynEvent {
    pub(crate) handlers: DynEventHandlers,
    type_id: TypeId,
}

impl Clone for DynEvent {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            type_id: self.type_id,
        }
    }
}

impl DynEvent {
    pub fn new<T: Event>() -> Self {
        Self {
            handlers: DynEventHandlers::new::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn add_handler<T, F, M>(&self, handler: F)
    where
        T: Event,
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        assert_eq!(TypeId::of::<T>(), self.type_id);
        self.handlers.insert(handler);
    }

    pub async fn fire<T: Event>(
        &self,
        world: WorldHandle,
        event: T,
        await_all_handlers: bool,
    ) -> usize {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        let event: Arc<dyn DowncastSync> = Arc::new(event);

        let handlers = self.handlers.handlers.read().await;
        let mut join_handles = Vec::new();

        // kahn's algorithm to process as many as possible at a time

        let mut in_degrees = FxHashMap::default();
        let mut queue = VecDeque::new();

        for node in handlers.node_indices() {
            let in_degree = handlers
                .neighbors_directed(node, Direction::Incoming)
                .count();
            in_degrees.insert(node, in_degree);

            if in_degree == 0 {
                queue.push_back(node);
            }
        }

        while !queue.is_empty() {
            let mut batch = Vec::new();

            for _ in 0..queue.len() {
                let node = queue.pop_front().unwrap();
                batch.push(node);

                for neighbor in handlers.neighbors_directed(node, Direction::Outgoing) {
                    let in_degree = in_degrees.get_mut(&neighbor).unwrap();
                    *in_degree -= 1;

                    if *in_degree == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }

            for node in batch {
                let handler = handlers[node].clone();
                let jh = tokio::spawn({
                    let world = world.clone();
                    let event = event.clone();
                    async move {
                        if handler.meta.can_run(&world).await {
                            handler.handler.run_dyn(world, event).await;
                        }
                    }
                });
                join_handles.push(jh);
            }

            if await_all_handlers {
                for handle in join_handles.drain(..) {
                    handle.await.unwrap();
                }
            }
        }

        handlers.node_count()
    }
}
