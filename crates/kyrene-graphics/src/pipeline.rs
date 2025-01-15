use std::{marker::PhantomData, sync::Arc};

use kyrene_core::{
    define_atomic_id,
    plugin::Plugin,
    prelude::{Component, WorldView},
    util::{FxHashMap, TypeIdMap},
};

use crate::{bind_group::BindGroupLayouts, wrap_wgpu, Device, InitRenderResources};

define_atomic_id!(PipelineId);

wrap_wgpu!(PipelineLayout);
wrap_wgpu!(RenderPipeline);

#[derive(Default)]
pub struct RenderPipelines {
    layout_cache: FxHashMap<PipelineId, PipelineLayout>,
    pipeline_cache: FxHashMap<PipelineId, RenderPipeline>,
    ids: TypeIdMap<PipelineId>,
}

impl RenderPipelines {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_id_for<T>(&self) -> Option<PipelineId>
    where
        T: CreateRenderPipeline,
    {
        self.ids.get_for::<T>().copied()
    }

    pub fn get_layout(&self, id: PipelineId) -> Option<&PipelineLayout> {
        self.layout_cache.get(&id)
    }

    pub fn get_pipeline(&self, id: PipelineId) -> Option<&RenderPipeline> {
        self.pipeline_cache.get(&id)
    }

    pub fn get_layout_for<T>(&self) -> Option<&PipelineLayout>
    where
        T: CreateRenderPipeline,
    {
        self.ids
            .get_for::<T>()
            .and_then(|id| self.layout_cache.get(id))
    }

    pub fn get_pipeline_for<T>(&self) -> Option<&RenderPipeline>
    where
        T: CreateRenderPipeline,
    {
        self.ids
            .get_for::<T>()
            .and_then(|id| self.pipeline_cache.get(id))
    }

    pub fn create_for<T>(
        &mut self,
        device: &Device,
        bind_group_layouts: &mut BindGroupLayouts,
    ) -> PipelineId
    where
        T: CreateRenderPipeline,
    {
        if let Some(id) = self.ids.get_for::<T>() {
            return *id;
        }

        let id = PipelineId::new();

        let layout = T::create_render_pipeline_layout(device, bind_group_layouts);
        let pipeline = T::create_render_pipeline(device, &layout);
        self.layout_cache.insert(id, layout);
        self.pipeline_cache.insert(id, pipeline);
        self.ids.insert_for::<T>(id);

        id
    }

    pub fn insert(&mut self, layout: PipelineLayout, pipeline: RenderPipeline) -> PipelineId {
        let id = PipelineId::new();
        self.layout_cache.insert(id, layout);
        self.pipeline_cache.insert(id, pipeline);
        id
    }
}

pub trait CreateRenderPipeline: Component + Sized {
    fn create_render_pipeline_layout(
        device: &Device,
        bind_group_layouts: &mut BindGroupLayouts,
    ) -> PipelineLayout;

    fn create_render_pipeline(device: &Device, layout: &PipelineLayout) -> RenderPipeline;
}

pub struct RenderPipelinePlugin<T: CreateRenderPipeline>(PhantomData<T>);

impl<T: CreateRenderPipeline> Default for RenderPipelinePlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: CreateRenderPipeline> Plugin for RenderPipelinePlugin<T> {
    async fn build(self, world: &mut kyrene_core::prelude::World) {
        world.add_event_handler(create_render_pipeline::<T>);
    }
}

pub async fn create_render_pipeline<T: CreateRenderPipeline>(
    world: WorldView,
    _event: Arc<InitRenderResources>,
) {
    let mut pipelines = world.get_resource_mut::<RenderPipelines>().await.unwrap();

    if pipelines.get_id_for::<T>().is_some() {
        return;
    }

    let device = world.get_resource::<Device>().await.unwrap();
    let mut bind_group_layouts = world.get_resource_mut::<BindGroupLayouts>().await.unwrap();
    pipelines.create_for::<T>(&device, &mut bind_group_layouts);
}
