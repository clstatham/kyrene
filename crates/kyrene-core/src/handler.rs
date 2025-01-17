use std::{
    future::Future,
    marker::PhantomData,
    ops::{Add, Deref, DerefMut},
    sync::Arc,
};

use downcast_rs::DowncastSync;
use futures::{future::BoxFuture, FutureExt};
use petgraph::prelude::*;

use crate::{
    component::Mut,
    event::{DynEvent, DynEventDispatcher, Event, EventDispatcher},
    lock::{Read, RwLock, Write},
    prelude::{Component, Ref},
    util::{FxHashSet, TypeIdMap, TypeIdSet, TypeInfo},
    world_handle::{FromWorldHandle, WorldHandle},
};

#[derive(Default, Debug, Clone)]
pub struct EventHandlerMeta {
    pub resources_read: TypeIdSet,
    pub resources_written: TypeIdSet,
}

impl EventHandlerMeta {
    pub fn res<T: Component>(mut self) -> Self {
        self.resources_read.insert_for::<T>();
        self
    }

    pub fn res_mut<T: Component>(mut self) -> Self {
        self.resources_written.insert_for::<T>();
        self
    }

    pub fn required_resources(&self) -> impl Iterator<Item = TypeInfo> + use<'_> {
        self.resources_read
            .iter()
            .copied()
            .chain(self.resources_written.iter().copied())
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        let mut conflicts = 0;

        conflicts += self
            .resources_read
            .intersection(&other.resources_written)
            .count();

        conflicts += self
            .resources_written
            .intersection(&other.resources_read)
            .count();

        conflicts += self
            .resources_written
            .intersection(&other.resources_written)
            .count();

        conflicts == 0
    }

    pub async fn can_run(&self, world: &WorldHandle) -> bool {
        let mut can = true;
        for res in self.required_resources() {
            can &= world.has_resource_dyn(res).await;
        }
        can
    }
}

impl Add<Self> for EventHandlerMeta {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut out = self.clone();
        out.resources_read.extend(rhs.resources_read);
        out.resources_written.extend(rhs.resources_written);
        out
    }
}

pub(crate) trait EventHandler: Send + Sync {
    fn meta(&self) -> EventHandlerMeta {
        EventHandlerMeta::default()
    }

    fn init(&self, world: WorldHandle) -> BoxFuture<'static, ()>;

    fn is_initialized(&self) -> BoxFuture<'static, bool>;

    fn run_dyn(&self, world: WorldHandle, event: DynEvent) -> BoxFuture<'static, ()>;
}

pub trait EventHandlerFn<M>: Send + Sync + 'static {
    type Event: DowncastSync;
    type Param: HandlerParam + 'static;

    fn init_state(&self, world: WorldHandle) -> BoxFuture<'static, HandlerParamState<Self::Param>>;

    fn run(
        &self,
        world: WorldHandle,
        event: Event<Self::Event>,
        param: HandlerParamItem<Self::Param>,
    ) -> BoxFuture<'static, ()>;
}

pub(crate) trait IntoEventHandler<M>: Send + Sync {
    type EventHandler: EventHandler;

    fn into_event_handler(self) -> Arc<Self::EventHandler>;
}

#[allow(unused)]
pub trait HandlerParam: Send + Sync {
    type Item: HandlerParam;
    type State: Send + Sync + 'static;

    fn meta() -> EventHandlerMeta;

    fn init_state(world: WorldHandle) -> impl Future<Output = Self::State> + Send;

    fn fetch(
        world: WorldHandle,
        state: &mut Self::State,
    ) -> impl Future<Output = Self::Item> + Send;

    fn can_run(world: WorldHandle, state: &Self::State) -> impl Future<Output = bool> + Send {
        async move { true }
    }
}

pub type HandlerParamItem<T> = <T as HandlerParam>::Item;
pub type HandlerParamState<T> = <T as HandlerParam>::State;

impl HandlerParam for () {
    type Item = ();
    type State = ();

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default()
    }

    async fn init_state(_world: WorldHandle) -> Self::State {}

    async fn fetch(_world: WorldHandle, _: &mut ()) -> Self::Item {}

    async fn can_run(_world: WorldHandle, _: &()) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Res<T: Component>(Ref<T>);

impl<T: Component> Deref for Res<T> {
    type Target = Ref<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Component> HandlerParam for Res<T> {
    type Item = Res<T>;
    type State = ();

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default().res::<T>()
    }

    async fn init_state(_world: WorldHandle) -> Self::State {}

    async fn fetch(world: WorldHandle, _: &mut ()) -> Self::Item {
        Res(world.get_resource::<T>().await.unwrap())
    }

    async fn can_run(world: WorldHandle, _: &()) -> bool {
        world.has_resource::<T>().await
    }
}

impl<T: Component> HandlerParam for Option<Res<T>> {
    type Item = Option<Res<T>>;
    type State = ();

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default().res::<T>()
    }

    async fn init_state(_world: WorldHandle) -> Self::State {}

    async fn fetch(world: WorldHandle, _: &mut ()) -> Self::Item {
        Some(Res(world.get_resource::<T>().await?))
    }

    async fn can_run(_world: WorldHandle, _: &()) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct ResMut<T: Component>(Mut<T>);

impl<T: Component> Deref for ResMut<T> {
    type Target = Mut<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Component> DerefMut for ResMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Component> HandlerParam for ResMut<T> {
    type Item = ResMut<T>;
    type State = ();

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default().res_mut::<T>()
    }

    async fn init_state(_world: WorldHandle) -> Self::State {}

    async fn fetch(world: WorldHandle, _: &mut ()) -> Self::Item {
        ResMut(world.get_resource_mut::<T>().await.unwrap())
    }

    async fn can_run(world: WorldHandle, _: &()) -> bool {
        world.has_resource::<T>().await
    }
}

impl<T: Component> HandlerParam for Option<ResMut<T>> {
    type Item = Option<ResMut<T>>;
    type State = ();

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default().res_mut::<T>()
    }

    async fn init_state(_world: WorldHandle) -> Self::State {}

    async fn fetch(world: WorldHandle, _: &mut ()) -> Self::Item {
        Some(ResMut(world.get_resource_mut::<T>().await?))
    }

    async fn can_run(_world: WorldHandle, _: &()) -> bool {
        true
    }
}

pub struct Local<T: Component + FromWorldHandle>(Arc<RwLock<T>>);

impl<T: Component + FromWorldHandle> Clone for Local<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Component + FromWorldHandle> Local<T> {
    pub async fn get(&self) -> Read<T> {
        self.0.clone().read_owned().await
    }

    pub async fn get_mut(&self) -> Write<T> {
        self.0.clone().write_owned().await
    }
}

impl<T: Component + FromWorldHandle> HandlerParam for Local<T> {
    type Item = Local<T>;
    type State = Local<T>;

    fn meta() -> EventHandlerMeta {
        EventHandlerMeta::default()
    }

    async fn init_state(world: WorldHandle) -> Self::State {
        Self(Arc::new(RwLock::new(T::from_world_handle(&world).await)))
    }

    async fn fetch(_world: WorldHandle, state: &mut Self::State) -> Self::Item {
        state.clone()
    }

    async fn can_run(_world: WorldHandle, _state: &Self::State) -> bool {
        true
    }
}

macro_rules! impl_handler_param_tuple {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($param: HandlerParam),*> HandlerParam for ($($param,)*) {
            type Item = ($($param::Item,)*);
            type State = ($($param::State,)*);

            fn meta() -> EventHandlerMeta {
                let mut meta = EventHandlerMeta::default();
                $(
                    let meta2 = $param::meta();
                    assert!(meta.is_compatible(&meta2));
                    meta = meta + meta2;
                )*
                meta
            }

            async fn init_state(world: WorldHandle) -> Self::State {
                tokio::join!($($param::init_state(world.clone()),)*)
            }

            async fn fetch(world: WorldHandle, state: &mut Self::State) -> Self::Item {
                let ($($param,)*) = state;
                tokio::join!($($param::fetch(world.clone(), $param),)*)
            }

            async fn can_run(world: WorldHandle, state: &Self::State) -> bool {
                let mut can = true;
                let ($($param,)*) = state;
                $(can &= $param::can_run(world.clone(), $param).await;)*
                can
            }
        }
    };
}

impl_handler_param_tuple!(A);
impl_handler_param_tuple!(A, B);
impl_handler_param_tuple!(A, B, C);
impl_handler_param_tuple!(A, B, C, D);
impl_handler_param_tuple!(A, B, C, D, E);
impl_handler_param_tuple!(A, B, C, D, E, F);
impl_handler_param_tuple!(A, B, C, D, E, F, G);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_handler_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

pub struct FunctionEventHandler<M, F>
where
    F: EventHandlerFn<M>,
{
    func: Arc<F>,
    state: Arc<RwLock<Option<HandlerParamState<F::Param>>>>,
    _marker: PhantomData<fn() -> M>,
}

impl<M, F> FunctionEventHandler<M, F>
where
    F: EventHandlerFn<M>,
{
    pub fn new(func: F) -> Self {
        Self {
            func: Arc::new(func),
            state: Arc::new(RwLock::new(None)),
            _marker: PhantomData,
        }
    }
}

impl<M, F> EventHandler for FunctionEventHandler<M, F>
where
    F: EventHandlerFn<M>,
{
    fn init(&self, world: WorldHandle) -> BoxFuture<'static, ()> {
        let func = self.func.clone();
        let state = self.state.clone();
        async move {
            let mut state = state.write().await;
            state.replace(func.init_state(world).await);
        }
        .boxed()
    }

    fn is_initialized(&self) -> BoxFuture<'static, bool> {
        let state = self.state.clone();
        async move { state.read().await.is_some() }.boxed()
    }

    fn run_dyn(&self, world: WorldHandle, event: DynEvent) -> BoxFuture<'static, ()> {
        let event: Event<<F as EventHandlerFn<M>>::Event> = Event::from_dyn_event(event);
        let func = self.func.clone();
        let state = self.state.clone();
        async move {
            let mut state_lock = state.write().await;
            let state = state_lock.as_mut().unwrap();
            if <F::Param>::can_run(world.clone(), state).await {
                let param = <F::Param>::fetch(world.clone(), state).await;
                drop(state_lock);
                func.run(world, event, param).await;
            }
        }
        .boxed()
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

impl<Func, Fut, T> EventHandlerFn<fn(WorldHandle, Event<T>)> for Func
where
    Func: Fn(Event<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + Sync + 'static,
    T: Component,
{
    type Event = T;
    type Param = ();

    fn init_state(
        &self,
        _world: WorldHandle,
    ) -> BoxFuture<'static, HandlerParamState<Self::Param>> {
        async move {}.boxed()
    }

    fn run(
        &self,
        _world: WorldHandle,
        event: Event<Self::Event>,
        _param: HandlerParamItem<Self::Param>,
    ) -> BoxFuture<'static, ()> {
        (self)(event).boxed()
    }
}

macro_rules! impl_fn_event_handler {
    ($($param:ident),*) => {
        #[allow(unused, non_snake_case)]
        impl<Func, Fut, Event, $($param),*> EventHandlerFn<fn(Arc<Event>, $($param,)*)> for Func
        where
            Func: Fn($crate::event::Event<Event>, $($param),*) -> Fut + Send + Sync + 'static
                + Fn($crate::event::Event<Event>, $(HandlerParamItem<$param>),*) -> Fut + Send + Sync + 'static,
            $($param: HandlerParam + 'static),*,
            Fut: Future<Output = ()> + Send + Sync + 'static,
            Event: $crate::component::Component,
        {
            type Event = Event;
            type Param = ($($param),*);

            fn init_state(&self, world: WorldHandle) -> BoxFuture<'static, HandlerParamState<Self::Param>> {
                <Self::Param as HandlerParam>::init_state(world).boxed()
            }

            fn run(
                &self,
                _world: WorldHandle,
                event: $crate::event::Event<Self::Event>,
                param: HandlerParamItem<Self::Param>,
            ) -> BoxFuture<'static, ()> {
                let ($($param),*) = param;
                (self)(event, $($param),*).boxed()
            }
        }
    };
}

impl_fn_event_handler!(A);
impl_fn_event_handler!(A, B);
impl_fn_event_handler!(A, B, C);
impl_fn_event_handler!(A, B, C, D);
impl_fn_event_handler!(A, B, C, D, E);
impl_fn_event_handler!(A, B, C, D, E, F);
impl_fn_event_handler!(A, B, C, D, E, F, G);
impl_fn_event_handler!(A, B, C, D, E, F, G, H);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J, K);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_fn_event_handler!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

#[derive(Clone)]
pub(crate) struct DynEventHandler {
    pub handler: Arc<dyn EventHandler>,
    pub meta: Arc<EventHandlerMeta>,
}

#[derive(Clone)]
pub(crate) struct DynEventHandlers {
    pub event_type_id: TypeInfo,
    pub handlers: Arc<RwLock<StableDiGraph<DynEventHandler, ()>>>,
    pub index_cache: Arc<RwLock<TypeIdMap<NodeIndex>>>,
}

impl DynEventHandlers {
    pub fn new<T: Component>() -> Self {
        Self {
            event_type_id: TypeInfo::of::<T>(),
            handlers: Arc::new(RwLock::new(StableDiGraph::new())),
            index_cache: Arc::new(RwLock::new(TypeIdMap::default())),
        }
    }

    pub fn insert<T, F, M>(&self, handler: F) -> NodeIndex
    where
        T: Component,
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        assert_eq!(TypeInfo::of::<T>(), self.event_type_id);
        let config = handler.finish();
        let index = self.handlers.blocking_write().add_node(DynEventHandler {
            handler: config.handler,
            meta: config.meta,
        });
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

pub struct HandlerConfig<T: Component> {
    handler_type_id: TypeInfo,
    handler: Arc<dyn EventHandler>,
    meta: Arc<EventHandlerMeta>,
    options: FxHashSet<HandlerAddOption>,
    _marker: PhantomData<T>,
}

impl<T: Component> HandlerConfig<T> {
    pub fn new<F, M>(handler: F) -> Self
    where
        F: EventHandlerFn<M, Event = T>,
        M: 'static,
    {
        let handler = handler.into_event_handler();
        Self {
            handler_type_id: TypeInfo::of::<F>(),
            meta: Arc::new(handler.meta()),
            handler,
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
    type Event: Component;

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
    T: Component,
    F: EventHandlerFn<M, Event = T>,
    M: 'static,
{
    type Event = T;

    fn finish(self) -> HandlerConfig<T> {
        HandlerConfig::new(self)
    }
}

impl<T: Component> IntoHandlerConfig<()> for HandlerConfig<T> {
    type Event = T;

    fn finish(self) -> HandlerConfig<Self::Event> {
        self
    }
}

#[derive(Default, Clone)]
pub(crate) struct Events {
    pub entries: TypeIdMap<DynEventDispatcher>,
}

impl Events {
    pub fn add_event<T: Component>(&mut self) -> EventDispatcher<T> {
        if let Some(event) = self.get_event::<T>() {
            return event;
        }
        let event = DynEventDispatcher::new::<T>();
        self.entries.insert_for::<T>(event.clone());
        EventDispatcher::from_dyn_event(event)
    }

    pub fn get_event<T: Component>(&self) -> Option<EventDispatcher<T>> {
        let event = self.entries.get_for::<T>()?.clone();
        Some(EventDispatcher::from_dyn_event(event))
    }

    pub fn has_event<T: Component>(&self) -> bool {
        self.entries.contains_type::<T>()
    }

    pub fn add_handler<T, F, M>(&mut self, handler: F)
    where
        T: Component,
        F: IntoHandlerConfig<M, Event = T>,
        M: 'static,
    {
        let event = self.add_event::<T>();
        event.add_handler(handler);
    }
}
