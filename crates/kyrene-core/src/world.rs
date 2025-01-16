use std::sync::Arc;

use tracing::level_filters::LevelFilter;

use crate::{
    bundle::Bundle,
    component::{Component, Components, Mut, Ref},
    entity::{Entities, Entity},
    event::{Event, EventDispatcher},
    handler::{Events, IntoHandlerConfig},
    lock::RwLock,
    plugin::Plugin,
    resource::Resources,
    util::TypeInfo,
    world_handle::WorldHandle,
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

    pub fn entity_iter(&self) -> impl Iterator<Item = Entity> + use<'_> {
        self.components.entity_iter()
    }

    pub async fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        self.components.insert(entity, component).await
    }

    pub fn insert_bundle<T: Bundle>(&mut self, entity: Entity, bundle: T) {
        self.components.insert_bundle(entity, bundle);
    }

    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> Entity {
        let entity = self.entity();
        self.insert_bundle(entity, bundle);
        entity
    }

    pub async fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(entity).await
    }

    pub async fn get<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        self.components.get(entity).await
    }

    pub async fn get_mut<T: Component>(&self, entity: Entity) -> Option<Mut<T>> {
        self.components.get_mut(entity).await
    }

    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.components.has::<T>(entity)
    }

    pub fn entities_with<T: Component>(&self) -> impl Iterator<Item = Entity> + use<'_, T> {
        self.components.entities_with::<T>()
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

    pub(crate) fn has_resource_dyn(&self, resource_type_id: TypeInfo) -> bool {
        self.resources.contains_dyn(resource_type_id)
    }

    pub async fn get_resource<T: Component>(&self) -> Option<Ref<T>> {
        self.resources.get::<T>().await
    }

    pub async fn get_resource_mut<T: Component>(&self) -> Option<Mut<T>> {
        self.resources.get_mut::<T>().await
    }

    pub async fn await_resource<T: Component>(&mut self) -> Ref<T> {
        self.resources.wait_for::<T>().await
    }

    pub async fn await_resource_mut<T: Component>(&mut self) -> Mut<T> {
        self.resources.wait_for_mut::<T>().await
    }

    #[track_caller]
    pub fn add_event<T: Event>(&mut self) -> EventDispatcher<T> {
        self.events.add_event::<T>()
    }

    pub fn get_event<T: Event>(&self) -> Option<EventDispatcher<T>> {
        self.events.get_event::<T>()
    }

    pub fn has_event<T: Event>(&self) -> bool {
        self.events.has_event::<T>()
    }

    #[track_caller]
    pub fn add_event_handler<T, F, M>(&mut self, handler: F)
    where
        T: Event,
        F: IntoHandlerConfig<M, Event = T> + 'static,
        M: 'static,
    {
        self.events.add_handler(handler);
    }

    pub fn into_world_handle(self) -> WorldHandle {
        WorldHandle {
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

        let world = self.into_world_handle();

        runtime.block_on(async move {
            world.fire_event(WorldStartup, true).await;

            // spawn WorldTick task
            let mut tick = 0;
            tokio::spawn({
                let world = world.clone();
                async move {
                    loop {
                        tick += 1;
                        world.fire_event(WorldTick { tick }, true).await;
                    }
                }
            });

            loop {
                tokio::task::yield_now().await;
            }
        });
    }
}

pub struct WorldTick {
    pub tick: u64,
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct WorldStartup;

#[derive(Clone, Copy, Debug, Hash)]
pub struct WorldShutdown;
