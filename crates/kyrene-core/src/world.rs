use downcast_rs::DowncastSync;

use crate::{
    component::{Component, Components, Ref},
    entity::{Entities, Entity},
    event::DynEvent,
    handler::{EventHandlerFn, EventHandlers},
    resource::Resources,
    world_view::WorldView,
};

pub struct World {
    entities: Entities,
    components: Components,
    resources: Resources,
    event_handlers: EventHandlers,
}

#[allow(clippy::derivable_impls)]
impl Default for World {
    fn default() -> Self {
        Self {
            entities: Entities::default(),
            components: Components::default(),
            resources: Resources::default(),
            event_handlers: EventHandlers::default(),
        }
    }
}

impl World {
    pub fn new() -> Self {
        Self::default()
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

    pub async fn insert_resource<T: Component>(&mut self, resource: T) -> Option<T> {
        self.resources.insert(resource).await
    }

    pub async fn remove_resource<T: Component>(&mut self) -> Option<T> {
        self.resources.remove::<T>().await
    }

    pub async fn get_resource<T: Component>(&mut self) -> Option<Ref<T>> {
        self.resources.get::<T>().await
    }

    pub fn event<T: Component>(&mut self) -> DynEvent {
        self.event_handlers.event::<T>()
    }

    pub fn fire_event<T: Component + Clone>(&mut self, payload: T) {
        self.event_handlers.event::<T>().fire(payload);
    }

    pub fn fire_event_with<T: Component>(&mut self, payload: impl FnMut() -> T) {
        self.event_handlers.event::<T>().fire_with(payload);
    }

    pub fn add_event_handler<T, F, M>(&mut self, handler: F) -> DynEvent
    where
        T: DowncastSync,
        F: EventHandlerFn<M, Event = T> + 'static,
        M: 'static,
    {
        self.event_handlers.add_handler(handler)
    }

    pub async fn run(mut self) {
        let (tx, mut op_rx) = tokio::sync::mpsc::unbounded_channel();
        let view = WorldView { tx };

        // spawn all event listeners
        let mut event_handlers = self.event_handlers.clone();
        for (event_type_id, mut handlers) in event_handlers.handlers.drain() {
            let event = event_handlers.events.remove(&event_type_id).unwrap();
            for handler in handlers.drain(..) {
                tokio::spawn({
                    let view = view.clone();
                    let event = event.clone();
                    async move {
                        loop {
                            let listener = event.listen();
                            let payload = listener.await;
                            handler.run_dyn(view.clone(), payload).await;
                        }
                    }
                });
            }
        }

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
        loop {
            while let Some(op) = op_rx.recv().await {
                op.run(&mut self).await;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WorldTick {
    pub tick: u64,
}
