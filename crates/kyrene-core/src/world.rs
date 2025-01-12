use downcast_rs::DowncastSync;

use crate::{
    component::{Component, Components, Ref},
    entity::{Entities, Entity},
    event::{
        handler::{await_and_handle_event, EventHandlerFn, EventHandlers},
        DynEvent,
    },
    world_view::WorldView,
};

pub struct World {
    entities: Entities,
    components: Components,
    event_handlers: EventHandlers,
}

#[allow(clippy::derivable_impls)]
impl Default for World {
    fn default() -> Self {
        Self {
            entities: Entities::default(),
            components: Components::default(),
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

    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        self.components.insert(entity, component)
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(entity)
    }

    pub async fn get<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        self.components.get_async(entity).await
    }

    pub fn add_event_handler<T, F, M>(&mut self, handler: F) -> DynEvent
    where
        T: DowncastSync,
        F: EventHandlerFn<M, Event = T> + 'static,
        M: 'static,
    {
        self.event_handlers.add_handler(handler)
    }

    pub async fn run(&mut self) {
        let mut world = std::mem::take(self);
        let (tx, mut op_rx) = tokio::sync::mpsc::unbounded_channel();
        let view = WorldView { tx };

        let mut event_handlers = world.event_handlers.clone();

        for (event_type_id, mut handlers) in event_handlers.handlers.drain() {
            let event = event_handlers.events.remove(&event_type_id).unwrap();
            for handler in handlers.drain(..) {
                tokio::spawn({
                    let view = view.clone();
                    let event = event.clone();
                    async move {
                        loop {
                            await_and_handle_event(view.clone(), &event, &*handler).await;
                        }
                    }
                });
            }
        }

        loop {
            while let Some(op) = op_rx.recv().await {
                op.run(&mut world).await;
            }
        }
    }
}
