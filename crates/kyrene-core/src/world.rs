use std::sync::Arc;

use downcast_rs::DowncastSync;
use tracing::level_filters::LevelFilter;

use crate::{
    component::{Component, Components, Mut, Ref},
    entity::{Entities, Entity},
    event::Event,
    handler::{EventHandlerFn, Events},
    lock::RwLock,
    plugin::Plugin,
    resource::Resources,
    world_view::WorldView,
};

pub struct World {
    entities: Entities,
    components: Components,
    resources: Resources,
    events: Events,
}

#[allow(clippy::derivable_impls)]
impl Default for World {
    fn default() -> Self {
        let mut this = Self {
            entities: Entities::default(),
            components: Components::default(),
            resources: Resources::default(),
            events: Events::default(),
        };
        this.add_event::<WorldStartup>();
        this.add_event::<WorldTick>();
        this.add_event::<WorldShutdown>();
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

    pub fn has_resource<T: Component>(&self) -> bool {
        self.resources.contains::<T>()
    }

    pub async fn get_resource<T: Component>(&mut self) -> Option<Ref<T>> {
        self.resources.get::<T>().await
    }

    pub async fn get_resource_mut<T: Component>(&mut self) -> Option<Mut<T>> {
        self.resources.get_mut::<T>().await
    }

    #[track_caller]
    pub fn add_event<T: Component>(&mut self) -> Event<T> {
        self.events.add_event::<T>()
    }

    pub fn get_event<T: Component>(&self) -> Option<Event<T>> {
        self.events.get_event::<T>()
    }

    pub fn add_event_handler<T, F, M>(&mut self, handler: F)
    where
        T: DowncastSync,
        F: EventHandlerFn<M, Event = T> + 'static,
        M: 'static,
    {
        self.events.add_handler(handler);
    }

    pub fn into_world_view(self) -> WorldView {
        WorldView {
            world: Arc::new(RwLock::new(self)),
        }
    }

    pub fn run(self) {
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(LevelFilter::DEBUG)
                .finish(),
        )
        .unwrap();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let view = self.into_world_view();

        runtime.block_on(async move {
            view.fire_event(WorldStartup, true).await;

            // spawn WorldTick task
            let mut tick = 0;
            tokio::spawn({
                let view = view.clone();
                async move {
                    loop {
                        tick += 1;
                        view.fire_event(WorldTick { tick }, true).await;
                    }
                }
            });

            loop {
                tokio::task::yield_now().await;
            }
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
