use std::{any::TypeId, future::Future, marker::PhantomData, sync::Arc};

use downcast_rs::DowncastSync;
use futures::{future::BoxFuture, FutureExt};

use crate::{
    event::{DynEvent, Event},
    lock::RwLock,
    util::TypeIdMap,
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
    pub handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
}

impl DynEventHandlers {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
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
        self.entries.insert(TypeId::of::<T>(), event.clone());
        Event::from_dyn_event(event)
    }

    pub fn get_event<T: DowncastSync>(&self) -> Option<Event<T>> {
        let event = self.entries.get(&TypeId::of::<T>())?.clone();
        Some(Event::from_dyn_event(event))
    }

    pub fn has_event<T: DowncastSync>(&self) -> bool {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    #[track_caller]
    pub fn add_handler<T, F, M>(&mut self, handler: F)
    where
        T: DowncastSync,
        F: EventHandlerFn<M, Event = T>,
        M: 'static,
    {
        let handler: Arc<dyn EventHandler> = handler.into_event_handler();
        let event = self
            .entries
            .entry(TypeId::of::<T>())
            .or_insert_with(|| DynEvent::new::<T>())
            .clone();
        event.handlers.handlers.blocking_write().push(handler);
    }
}
