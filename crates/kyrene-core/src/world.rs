use std::sync::Arc;

use downcast_rs::DowncastSync;

use crate::{
    component::{Component, Components, Mut, Ref},
    entity::{Entities, Entity},
    event::DynEvent,
    handler::{EventHandlerFn, EventHandlers},
    lock::Mutex,
    plugin::Plugin,
    resource::Resources,
    util::TypeIdMap,
    world_view::WorldView,
};

pub struct World {
    pub entities: Entities,
    pub components: Components,
    pub resources: Resources,
    pub event_handlers: EventHandlers,
}

#[allow(clippy::derivable_impls)]
impl Default for World {
    fn default() -> Self {
        let mut this = Self {
            entities: Entities::default(),
            components: Components::default(),
            resources: Resources::default(),
            event_handlers: EventHandlers::default(),
        };
        this.event::<WorldStartup>();
        this.event::<WorldTick>();
        this.event::<WorldShutdown>();
        this
    }
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_plugin<T: Plugin>(&mut self, plugin: T) {
        pollster::block_on(plugin.build(self));
    }

    pub fn entity(&mut self) -> Entity {
        self.entities.alloc()
    }

    pub async fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        self.components.insert(entity, component).await
    }

    pub async fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(entity).await
    }

    pub async fn get<T: Component>(&mut self, entity: Entity) -> Option<Ref<T>> {
        self.components.get(entity).await
    }

    pub async fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<Mut<T>> {
        self.components.get_mut(entity).await
    }

    pub async fn insert_resource<T: Component>(&mut self, resource: T) -> Option<T> {
        self.resources.insert(resource).await
    }

    pub async fn remove_resource<T: Component>(&mut self) -> Option<T> {
        self.resources.remove::<T>().await
    }

    pub async fn get_resource<T: Component>(&mut self) -> Option<Ref<T>> {
        self.resources.get::<T>().await
    }

    pub async fn get_resource_mut<T: Component>(&mut self) -> Option<Mut<T>> {
        self.resources.get_mut::<T>().await
    }

    pub fn event<T: Component>(&mut self) -> DynEvent {
        self.event_handlers.event::<T>()
    }

    pub fn fire_event<T: Component + Clone>(&mut self, payload: T) {
        self.event_handlers.event::<T>().fire(payload);
    }

    pub fn add_event_handler<T, F, M>(&mut self, handler: F) -> DynEvent
    where
        T: DowncastSync,
        F: EventHandlerFn<M, Event = T> + 'static,
        M: 'static,
    {
        self.event_handlers.add_handler(handler)
    }

    pub fn run(mut self) {
        #[cfg(debug_assertions)]
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        #[cfg(not(debug_assertions))]
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async move {
            let (tx, mut op_rx) = tokio::sync::mpsc::unbounded_channel();
            let view = WorldView { tx };

            let mut total_listeners = TypeIdMap::default();
            let listeners_ready = Arc::new(Mutex::new(TypeIdMap::default()));

            // spawn all event listeners
            let mut event_handlers = self.event_handlers.clone();
            for (event_type_id, mut handlers) in event_handlers.handlers.drain() {
                let event = event_handlers.events.remove(&event_type_id).unwrap();
                total_listeners.insert(event_type_id, handlers.len());
                for handler in handlers.drain(..) {
                    tokio::spawn({
                        let view = view.clone();
                        let event = event.clone();
                        let listeners_ready = listeners_ready.clone();
                        let mut listener = event.listen();

                        async move {
                            *listeners_ready
                                .lock()
                                .await
                                .entry(event_type_id)
                                .or_insert(0usize) += 1;

                            loop {
                                let payload = listener.next().await;
                                handler.run_dyn(view.clone(), payload).await;
                            }
                        }
                    });
                }
            }

            // wait for all event listeners to be ready and listening
            loop {
                if listeners_ready
                    .lock()
                    .await
                    .iter()
                    .all(|(k, v)| *v == total_listeners[k])
                {
                    break;
                }

                tokio::task::yield_now().await;
            }

            self.fire_event(WorldStartup);

            // spawn WorldTick task
            let mut tick = 0;
            tokio::spawn({
                let view = view.clone();
                async move {
                    loop {
                        tick += 1;
                        view.fire_event(WorldTick { tick }).await;
                    }
                }
            });

            // await and apply deferred ops
            tokio::spawn(async move {
                loop {
                    while let Some(op) = op_rx.recv().await {
                        op.run(&mut self).await;
                    }
                }
            });

            loop {
                tokio::task::yield_now().await;
            }

            // todo: fire WorldShutdown event
        });
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WorldTick {
    pub tick: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct WorldStartup;

#[derive(Clone, Copy, Debug)]
pub struct WorldShutdown;
