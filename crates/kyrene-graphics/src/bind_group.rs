use std::{marker::PhantomData, sync::Arc};

use kyrene_core::{
    entity::Entity,
    plugin::Plugin,
    prelude::{Component, StreamExt, WorldView},
    util::TypeIdMap,
};

use crate::{wrap_wgpu, Device, InitRenderResources};

wrap_wgpu!(BindGroupLayout);
wrap_wgpu!(BindGroup<T: CreateBindGroup>);

#[derive(Default)]
pub struct BindGroupLayouts(TypeIdMap<BindGroupLayout>);

impl BindGroupLayouts {
    pub fn insert<T: Component>(&mut self, layout: BindGroupLayout) {
        self.0.insert_for::<T>(layout);
    }

    pub fn get<T: Component>(&self) -> Option<&BindGroupLayout> {
        self.0.get_for::<T>()
    }

    pub fn get_or_create<T: CreateBindGroup>(&mut self, device: &Device) -> BindGroupLayout {
        if let Some(layout) = self.get::<T>() {
            layout.clone()
        } else {
            let layout = T::create_bind_group_layout(device);
            self.0.insert_for::<T>(layout.clone());
            layout
        }
    }
}

pub trait CreateBindGroup: Component + Sized {
    fn create_bind_group_layout(device: &Device) -> BindGroupLayout;

    fn create_bind_group(&self, device: &Device, layout: &BindGroupLayout) -> BindGroup<Self>;
}

impl<T: CreateBindGroup> BindGroup<T> {
    pub fn create(device: &Device, data: &T, layouts: &mut BindGroupLayouts) -> Self {
        let layout = layouts.get_or_create::<T>(device);
        data.create_bind_group(device, &layout)
    }
}

pub struct ComponentBindGroupPlugin<T: CreateBindGroup>(PhantomData<T>);

impl<T: CreateBindGroup> Default for ComponentBindGroupPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: CreateBindGroup> Plugin for ComponentBindGroupPlugin<T> {
    async fn build(self, world: &mut kyrene_core::prelude::World) {
        world.add_event_handler(create_component_bind_group::<T>);
    }
}

pub async fn create_component_bind_group<T: CreateBindGroup>(
    world: WorldView,
    _event: Arc<InitRenderResources>,
) {
    let item_query = world.query::<(Entity, &T)>().await;
    let mut item_query = item_query.iter();
    while let Some((entity, item)) = item_query.next().await {
        if !world.has::<BindGroup<T>>(entity).await {
            let device = world.get_resource::<Device>().await.unwrap();
            let mut layouts = world.get_resource_mut::<BindGroupLayouts>().await.unwrap();
            let bind_group = BindGroup::create(&device, &*item, &mut layouts);
            world.insert(entity, bind_group).await;
        }
    }
}

pub struct ResourceBindGroupPlugin<T: CreateBindGroup>(PhantomData<T>);

impl<T: CreateBindGroup> Default for ResourceBindGroupPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: CreateBindGroup> Plugin for ResourceBindGroupPlugin<T> {
    async fn build(self, world: &mut kyrene_core::prelude::World) {
        world.add_event_handler(create_component_bind_group::<T>);
    }
}

pub async fn create_resource_bind_group<T: CreateBindGroup>(
    world: WorldView,
    _event: Arc<InitRenderResources>,
) {
    if !world.has_resource::<BindGroup<T>>().await {
        if let Some(item) = world.get_resource::<T>().await {
            let device = world.get_resource::<Device>().await.unwrap();
            let mut layouts = world.get_resource_mut::<BindGroupLayouts>().await.unwrap();

            let bind_group = BindGroup::create(&device, &*item, &mut layouts);
            world.insert_resource(bind_group).await;
        }
    }
}