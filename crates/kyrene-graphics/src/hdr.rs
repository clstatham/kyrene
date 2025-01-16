use std::sync::Arc;

use kyrene_core::{
    handler::{Res, ResMut},
    plugin::Plugin,
    prelude::{World, WorldView},
};

use crate::{
    bind_group::{
        BindGroup, BindGroupLayout, BindGroupLayouts, CreateBindGroup, ResourceBindGroupPlugin,
    },
    pipeline::{
        CreateRenderPipeline, PipelineLayout, RenderPipeline, RenderPipelinePlugin, RenderPipelines,
    },
    texture::{texture_format, GpuTexture},
    window::WindowSettings,
    ActiveCommandEncoder, CurrentFrame, Device, InitRenderResources, Render,
};

#[derive(Clone)]
pub struct HdrRenderTarget {
    pub texture: GpuTexture,
    pub sampler: Arc<wgpu::Sampler>,
}

impl HdrRenderTarget {
    pub fn create(window_settings: &WindowSettings, device: &Device) -> Self {
        let texture = GpuTexture::new(
            device,
            Some("Hdr Render Target"),
            window_settings.width,
            window_settings.height,
            texture_format::HDR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        );

        let sampler = Arc::new(device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));

        Self { texture, sampler }
    }

    pub fn color_target(&self) -> &Arc<wgpu::TextureView> {
        &self.texture.view
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.texture = GpuTexture::new(
            device,
            Some("Hdr Render Target"),
            width,
            height,
            texture_format::HDR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        );
    }
}

impl CreateBindGroup for HdrRenderTarget {
    fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
        BindGroupLayout::new(
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Hdr Render Target Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            }),
        )
    }

    fn create_bind_group(&self, device: &Device, layout: &BindGroupLayout) -> BindGroup<Self> {
        BindGroup::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.sampler.as_ref()),
                },
            ],
            label: Some("Hdr Render Target Bind Group"),
        }))
    }
}

pub struct HdrRenderPipeline;

impl CreateRenderPipeline for HdrRenderPipeline {
    fn create_render_pipeline_layout(
        device: &Device,
        bind_group_layouts: &mut BindGroupLayouts,
    ) -> PipelineLayout {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HDR Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layouts.get_or_create::<HdrRenderTarget>(device)],
            push_constant_ranges: &[],
        });

        PipelineLayout::new(layout)
    }

    fn create_render_pipeline(device: &Device, layout: &PipelineLayout) -> RenderPipeline {
        let shader = wgpu::include_wgsl!("hdr.wgsl");
        let shader_module = device.create_shader_module(shader);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HDR Render Pipeline"),
            layout: Some(layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("hdr_vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("hdr_fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format::VIEW_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        RenderPipeline::new(pipeline)
    }
}

pub async fn init_hdr_target(
    world: WorldView,
    _event: Arc<InitRenderResources>,
    window_settings: Res<WindowSettings>,
    device: Res<Device>,
) {
    if world.has_resource::<HdrRenderTarget>().await {
        return;
    }

    let hdr_target = HdrRenderTarget::create(&window_settings, &device);
    world.insert_resource(hdr_target).await;
}

pub async fn render_hdr(
    _world: WorldView,
    _event: Arc<Render>,
    mut encoder: ResMut<ActiveCommandEncoder>,
    current_frame: Res<CurrentFrame>,
    pipelines: Res<RenderPipelines>,
    bind_group: Res<BindGroup<HdrRenderTarget>>,
) {
    let pipeline = pipelines.get_pipeline_for::<HdrRenderPipeline>().unwrap();
    let Some(current_frame) = current_frame.inner.as_ref() else {
        return;
    };

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HDR Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &current_frame.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &***bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

pub struct HdrPlugin;

impl Plugin for HdrPlugin {
    async fn build(self, world: &mut World) {
        world.add_plugin(RenderPipelinePlugin::<HdrRenderPipeline>::default());
        world.add_plugin(ResourceBindGroupPlugin::<HdrRenderTarget>::default());

        world.add_event_handler(init_hdr_target);
        world.add_event_handler(render_hdr);
    }
}
