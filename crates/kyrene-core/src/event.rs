use std::{
    collections::VecDeque,
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
    time::{Duration, Instant},
};

use petgraph::prelude::*;

use crate::{
    handler::{DynEventHandlers, IntoHandlerConfig},
    lock::Mutex,
    prelude::{Component, WorldHandle},
    util::{FxHashMap, TypeInfo},
};

pub struct EventInner<T: Component> {
    event: Arc<T>,
    delta_time: Option<Duration>,
}

impl<T: Component> EventInner<T> {
    pub fn delta_time(&self) -> Option<Duration> {
        self.delta_time
    }
}

impl<T: Component> Deref for EventInner<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

pub struct Event<T: Component>(Arc<EventInner<T>>);

impl<T: Component> Event<T> {
    pub(crate) fn from_dyn_event(event: DynEvent) -> Self {
        assert_eq!(event.type_id, TypeInfo::of::<T>());
        Self(Arc::new(EventInner {
            event: event
                .event
                .downcast_arc()
                .unwrap_or_else(|_| unreachable!()),
            delta_time: event.delta_time,
        }))
    }

    pub fn event(&self) -> &T {
        &self.0.event
    }
}

impl<T: Component> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Component> Deref for Event<T> {
    type Target = EventInner<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct DynEvent {
    pub(crate) type_id: TypeInfo,
    pub(crate) event: Arc<dyn Component>,
    pub(crate) delta_time: Option<Duration>,
}

pub struct EventDispatcher<T: Component> {
    event: DynEventDispatcher,
    _marker: PhantomData<T>,
}

impl<T: Component> Clone for EventDispatcher<T> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: Component> EventDispatcher<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            event: DynEventDispatcher::new::<T>(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn from_dyn_event(event: DynEventDispatcher) -> Self {
        assert_eq!(event.type_id, TypeInfo::of::<T>());
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

pub(crate) struct DynEventDispatcher {
    pub(crate) handlers: DynEventHandlers,
    type_id: TypeInfo,
    last_fired: Arc<Mutex<Option<Instant>>>,
}

impl Clone for DynEventDispatcher {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            type_id: self.type_id,
            last_fired: self.last_fired.clone(),
        }
    }
}

impl DynEventDispatcher {
    pub fn new<T: Component>() -> Self {
        Self {
            handlers: DynEventHandlers::new::<T>(),
            type_id: TypeInfo::of::<T>(),
            last_fired: Arc::new(Mutex::new(None)),
        }
    }

    pub fn add_handler<T, F, M>(&self, handler: F)
    where
        T: Component,
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        assert_eq!(TypeInfo::of::<T>(), self.type_id);
        self.handlers.insert(handler);
    }

    pub async fn fire<T: Component>(
        &self,
        world: WorldHandle,
        event: T,
        await_all_handlers: bool,
    ) -> usize {
        assert_eq!(
            TypeInfo::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        let event: Arc<dyn Component> = Arc::new(event);

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

            let delta_time = {
                let mut last_fired = self.last_fired.try_lock().unwrap();
                let delta_time = last_fired.map(|t| t.elapsed());
                last_fired.replace(Instant::now());
                delta_time
            };

            let event = DynEvent {
                type_id: self.type_id,
                delta_time,
                event: event.clone(),
            };

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
