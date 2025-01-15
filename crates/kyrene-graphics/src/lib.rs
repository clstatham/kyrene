use std::{ops::Deref, sync::Arc};

use camera::{insert_view_target, GpuCamera, InsertViewTarget, ViewTarget};
use kyrene_core::{entity::Entity, plugin::Plugin, prelude::WorldView, world::World};
use texture::texture_format::{DEPTH_FORMAT, VIEW_FORMAT};
use window::{RedrawRequested, WindowCreated};

pub mod camera;
pub mod texture;
pub mod window;

pub struct InitRenderResources;
pub struct PreRender;
pub struct Render;
pub struct PostRender;

pub struct CurrentFrameInner {
    pub surface_texture: Arc<wgpu::SurfaceTexture>,
    pub color_view: Arc<wgpu::TextureView>,
    pub depth_view: Arc<wgpu::TextureView>,
}

#[derive(Default)]
pub struct CurrentFrame {
    pub inner: Option<CurrentFrameInner>,
}

pub struct DepthTexture {
    pub depth_texture: Arc<wgpu::Texture>,
}

impl Deref for DepthTexture {
    type Target = wgpu::Texture;

    fn deref(&self) -> &Self::Target {
        &self.depth_texture
    }
}

pub struct WgpuDevice {
    pub device: wgpu::Device,
}

impl Deref for WgpuDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

pub struct WgpuQueue {
    pub queue: wgpu::Queue,
}

impl Deref for WgpuQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

pub struct WindowSurface {
    pub surface: Arc<wgpu::Surface<'static>>,
}

impl Deref for WindowSurface {
    type Target = wgpu::Surface<'static>;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

pub struct ActiveCommandEncoder {
    pub encoder: wgpu::CommandEncoder,
}

impl Deref for ActiveCommandEncoder {
    type Target = wgpu::CommandEncoder;

    fn deref(&self) -> &Self::Target {
        &self.encoder
    }
}

impl ActiveCommandEncoder {
    pub fn finish(self) -> wgpu::CommandBuffer {
        self.encoder.finish()
    }
}

pub struct CommandBuffers {
    pub command_buffers: Vec<wgpu::CommandBuffer>,
}

impl CommandBuffers {
    pub fn enqueue(&mut self, command_buffer: wgpu::CommandBuffer) {
        self.command_buffers.push(command_buffer);
    }

    pub fn enqueue_many(&mut self, command_buffers: impl IntoIterator<Item = wgpu::CommandBuffer>) {
        self.command_buffers.extend(command_buffers);
    }
}

async fn create_surface(world: WorldView, event: Arc<WindowCreated>) {
    if world.has_resource::<WindowSurface>().await {
        return;
    }

    let WindowCreated {
        window,
        surface,
        adapter,
    } = &*event;

    let window = window.clone();
    let surface = surface.clone();
    let adapter = adapter.clone();

    let mut required_limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
    required_limits.max_push_constant_size = 256;

    let (device, queue) = kyrene_core::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::MULTIVIEW
                | wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_BINDING_ARRAY
                | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            required_limits,
            label: None,
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap();

    let caps = surface.get_capabilities(&adapter);

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: VIEW_FORMAT,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 1,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        },
    );

    let depth_texture = Arc::new(device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width: window.inner_size().width,
            height: window.inner_size().height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    }));

    world.insert_resource(WgpuDevice { device }).await;
    world.insert_resource(WgpuQueue { queue }).await;
    world.insert_resource(WindowSurface { surface }).await;
    world.insert_resource(DepthTexture { depth_texture }).await;
    world
        .insert_resource(CommandBuffers {
            command_buffers: Vec::new(),
        })
        .await;
}

pub struct WgpuPlugin;

impl Plugin for WgpuPlugin {
    async fn build(self, world: &mut World) {
        world.add_event::<InitRenderResources>();
        world.add_event::<PreRender>();
        world.add_event::<Render>();
        world.add_event::<PostRender>();

        world.add_event_handler(create_surface);
        world.add_event_handler(redraw_requested);
        world.add_event_handler(pre_render);

        world.add_event::<BeginRender>();
        world.add_event_handler(begin_render);

        world.add_event::<InsertViewTarget>();
        world.add_event_handler(insert_view_target);
    }
}

pub struct BeginRender;

async fn redraw_requested(world: WorldView, _event: Arc<RedrawRequested>) {
    world.fire_event(InitRenderResources, true).await;
    world.fire_event(PreRender, true).await;
    world.fire_event(Render, true).await;
    world.fire_event(PostRender, true).await;
}

pub async fn pre_render(world: WorldView, _event: Arc<PreRender>) {
    world.fire_event(BeginRender, true).await;
    world
        .query_iter::<(Entity, &GpuCamera)>(|world, (camera, _)| async move {
            world.fire_event(InsertViewTarget { camera }, true).await;
        })
        .await;
}

pub async fn begin_render(world: WorldView, _event: Arc<BeginRender>) {
    let Some(mut current_frame) = world.get_resource_mut::<CurrentFrame>().await else {
        return;
    };
    if current_frame.inner.is_some() {
        return;
    }

    let view_targets = world.entities_with::<ViewTarget>().await;
    for entity in view_targets {
        world.remove::<ViewTarget>(entity).await;
    }

    let surface = world.get_resource::<WindowSurface>().await.unwrap();
    let frame = match surface.get_current_texture() {
        Ok(frame) => frame,
        Err(e) => {
            panic!("Failed to acquire next surface texture: {}", e);
        }
    };

    let depth_texture = world.get_resource::<DepthTexture>().await.unwrap();

    let color_view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Depth Texture View"),
        format: Some(DEPTH_FORMAT),
        dimension: Some(wgpu::TextureViewDimension::D2),
        ..Default::default()
    });

    let device = world.get_resource::<WgpuDevice>().await.unwrap();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Initial Encoder"),
    });
    {
        let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Initial Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    let mut command_buffers = world.get_resource_mut::<CommandBuffers>().await.unwrap();
    command_buffers.enqueue(encoder.finish());

    current_frame.inner.replace(CurrentFrameInner {
        surface_texture: Arc::new(frame),
        color_view: Arc::new(color_view),
        depth_view: Arc::new(depth_view),
    });

    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    world
        .insert_resource(ActiveCommandEncoder { encoder })
        .await;
}
