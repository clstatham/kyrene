use std::{future::Future, marker::PhantomData, sync::Arc};

use downcast_rs::DowncastSync;
use futures::{future::BoxFuture, FutureExt};
use petgraph::prelude::*;

use crate::{
    event::{DynEvent, Event},
    lock::RwLock,
    util::{FxHashSet, TypeIdMap, TypeInfo},
    world_view::WorldView,
};

pub(crate) trait EventHandler: Send + Sync {
    fn run_dyn(&self, world: WorldView, event: Arc<dyn DowncastSync>) -> BoxFuture<'static, ()>;
}

pub trait EventHandlerFn<M>: Send + Sync + 'static {
    type Event: DowncastSync;

    fn run(&self, world: WorldView, event: Arc<Self::Event>) -> BoxFuture<'static, ()>;
}

pub(crate) trait IntoEventHandler<M>: Send + Sync {
    type EventHandler: EventHandler;

    fn into_event_handler(self) -> Arc<Self::EventHandler>;
}

pub struct FunctionEventHandler<M, F>
where
    F: EventHandlerFn<M>,
{
    func: Arc<F>,
    _marker: PhantomData<fn() -> M>,
}

impl<M, F> FunctionEventHandler<M, F>
where
    F: EventHandlerFn<M>,
{
    pub fn new(func: F) -> Self {
        Self {
            func: Arc::new(func),
            _marker: PhantomData,
        }
    }
}

impl<M, F> EventHandler for FunctionEventHandler<M, F>
where
    F: EventHandlerFn<M>,
{
    fn run_dyn(&self, world: WorldView, event: Arc<dyn DowncastSync>) -> BoxFuture<'static, ()> {
        let event: Arc<<F as EventHandlerFn<M>>::Event> = event.into_any_arc().downcast().unwrap();
        self.func.run(world, event)
    }
}

pub struct FunctionEventHandlerMarker;

impl<M, F> IntoEventHandler<(FunctionEventHandlerMarker, M)> for F
where
    F: EventHandlerFn<M>,
{
    type EventHandler = FunctionEventHandler<M, F>;

    fn into_event_handler(self) -> Arc<Self::EventHandler> {
        Arc::new(FunctionEventHandler::new(self))
    }
}

impl<Func, Fut, T> EventHandlerFn<fn(WorldView, Arc<T>)> for Func
where
    Func: Fn(WorldView, Arc<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + Sync + 'static,
    T: DowncastSync,
{
    type Event = T;

    fn run(&self, world: WorldView, event: Arc<Self::Event>) -> BoxFuture<'static, ()> {
        (self)(world, event).boxed()
    }
}

#[derive(Clone)]
pub(crate) struct DynEventHandlers {
    pub event_type_id: TypeInfo,
    pub handlers: Arc<RwLock<StableDiGraph<Arc<dyn EventHandler>, ()>>>,
    pub index_cache: Arc<RwLock<TypeIdMap<NodeIndex>>>,
}

impl DynEventHandlers {
    pub fn new<T: DowncastSync>() -> Self {
        Self {
            event_type_id: TypeInfo::of::<T>(),
            handlers: Arc::new(RwLock::new(StableDiGraph::new())),
            index_cache: Arc::new(RwLock::new(TypeIdMap::default())),
        }
    }

    pub fn insert<T, F, M>(&self, handler: F) -> NodeIndex
    where
        T: DowncastSync,
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        assert_eq!(TypeInfo::of::<T>(), self.event_type_id);
        let config = handler.finish();
        let index = self.handlers.blocking_write().add_node(config.handler);
        self.index_cache
            .blocking_write()
            .insert(config.handler_type_id, index);

        for opt in config.options {
            let mut handlers = self.handlers.blocking_write();
            let index_cache = self.index_cache.blocking_read();
            match opt {
                HandlerAddOption::After(first) => {
                    let first = *index_cache.get(&first).unwrap();
                    handlers.add_edge(first, index, ());
                }
                HandlerAddOption::Before(second) => {
                    let second = *index_cache.get(&second).unwrap();
                    handlers.add_edge(index, second, ());
                }
            }
        }

        index
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlerAddOption {
    After(TypeInfo),
    Before(TypeInfo),
}

pub struct HandlerConfig<T: DowncastSync> {
    handler_type_id: TypeInfo,
    handler: Arc<dyn EventHandler>,
    options: FxHashSet<HandlerAddOption>,
    _marker: PhantomData<T>,
}

impl<T: DowncastSync> HandlerConfig<T> {
    pub fn new<F, M>(handler: F) -> Self
    where
        F: EventHandlerFn<M, Event = T>,
        M: 'static,
    {
        Self {
            handler_type_id: TypeInfo::of::<F>(),
            handler: handler.into_event_handler(),
            options: FxHashSet::default(),
            _marker: PhantomData,
        }
    }

    pub fn after<F2, M2>(mut self, _handler: F2) -> Self
    where
        F2: EventHandlerFn<M2, Event = T>,
        M2: 'static,
    {
        self.options
            .insert(HandlerAddOption::After(TypeInfo::of::<F2>()));
        self
    }

    pub fn before<F2, M2>(mut self, _handler: F2) -> Self
    where
        F2: EventHandlerFn<M2, Event = T>,
        M2: 'static,
    {
        self.options
            .insert(HandlerAddOption::Before(TypeInfo::of::<F2>()));
        self
    }
}

pub trait IntoHandlerConfig<M>: Sized + 'static {
    type Event: DowncastSync;

    fn finish(self) -> HandlerConfig<Self::Event>;

    fn after<F2, M2>(self, handler: F2) -> HandlerConfig<Self::Event>
    where
        F2: EventHandlerFn<M2, Event = Self::Event>,
        M2: 'static,
    {
        self.finish().after(handler)
    }

    fn before<F2, M2>(self, handler: F2) -> HandlerConfig<Self::Event>
    where
        F2: EventHandlerFn<M2, Event = Self::Event>,
        M2: 'static,
    {
        self.finish().before(handler)
    }
}

impl<T, F, M> IntoHandlerConfig<M> for F
where
    T: DowncastSync,
    F: EventHandlerFn<M, Event = T>,
    M: 'static,
{
    type Event = T;

    fn finish(self) -> HandlerConfig<T> {
        HandlerConfig::new(self)
    }
}

impl<T: DowncastSync> IntoHandlerConfig<()> for HandlerConfig<T> {
    type Event = T;

    fn finish(self) -> HandlerConfig<Self::Event> {
        self
    }
}

#[derive(Default, Clone)]
pub(crate) struct Events {
    pub entries: TypeIdMap<DynEvent>,
}

impl Events {
    pub fn add_event<T: DowncastSync>(&mut self) -> Event<T> {
        if let Some(event) = self.get_event::<T>() {
            return event;
        }
        let event = DynEvent::new::<T>();
        self.entries.insert_for::<T>(event.clone());
        Event::from_dyn_event(event)
    }

    pub fn get_event<T: DowncastSync>(&self) -> Option<Event<T>> {
        let event = self.entries.get_for::<T>()?.clone();
        Some(Event::from_dyn_event(event))
    }

    pub fn has_event<T: DowncastSync>(&self) -> bool {
        self.entries.contains_type::<T>()
    }

    pub fn add_handler<T, F, M>(&mut self, handler: F)
    where
        T: DowncastSync,
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        let event = self.add_event::<T>();
        event.add_handler(handler);
    }
}
