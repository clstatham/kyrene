use std::{any::TypeId, future::Future, marker::PhantomData, sync::Arc};

use downcast_rs::DowncastSync;
use futures::{future::BoxFuture, FutureExt};
use kyrene_util::TypeIdMap;

use crate::world_view::WorldView;

use super::DynEvent;

pub async fn await_and_handle_event(
    world: WorldView,
    event: &DynEvent,
    handler_fn: &dyn EventHandler,
) {
    let listener = event.listen();
    let payload = listener.await;
    handler_fn.run_dyn(world, payload).await;
}

pub trait EventHandler: Send + Sync {
    fn run_dyn(&self, world: WorldView, event: Arc<dyn DowncastSync>) -> BoxFuture<'static, ()>;
}

pub trait EventHandlerFn<M>: Send + Sync {
    type Event: DowncastSync;

    fn run(&self, world: WorldView, event: Self::Event) -> BoxFuture<'static, ()>;
}

pub trait IntoEventHandler<M>: Send + Sync {
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
        let event = Arc::into_inner(event).unwrap();
        self.func.run(world, event).boxed()
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

impl<Func, Fut, T> EventHandlerFn<fn(WorldView, T)> for Func
where
    Func: Fn(WorldView, T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + Sync + 'static,
    T: DowncastSync,
{
    type Event = T;

    fn run(&self, world: WorldView, event: Self::Event) -> BoxFuture<'static, ()> {
        (self)(world, event).boxed()
    }
}

#[derive(Default, Clone)]
pub struct EventHandlers {
    pub(crate) handlers: TypeIdMap<Vec<Arc<dyn EventHandler>>>,
    pub(crate) events: TypeIdMap<DynEvent>,
}

impl EventHandlers {
    pub fn add_handler<T, F, M>(&mut self, handler: F) -> DynEvent
    where
        T: DowncastSync,
        F: EventHandlerFn<M, Event = T> + 'static,
        M: 'static,
    {
        self.handlers
            .entry(TypeId::of::<T>())
            .or_default()
            .push(handler.into_event_handler());
        self.add_event::<T>()
    }

    pub fn add_event<T>(&mut self) -> DynEvent
    where
        T: DowncastSync,
    {
        if let Some(event) = self.events.get(&TypeId::of::<T>()) {
            event.clone()
        } else {
            let event = DynEvent::new::<T>();
            self.events.insert(TypeId::of::<T>(), event.clone());
            event
        }
    }

    pub fn get_event<T>(&self) -> Option<DynEvent>
    where
        T: DowncastSync,
    {
        self.events.get(&TypeId::of::<T>()).cloned()
    }
}
