use std::{ops::Deref, sync::Arc};

use kyrene_core::{plugin::Plugin, prelude::WorldView, world::World};
use kyrene_winit::{RedrawRequested, Window};
use texture::texture_format::{DEPTH_FORMAT, VIEW_FORMAT};

pub mod texture;

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

pub struct WgpuInstance {
    pub instance: wgpu::Instance,
}

impl Deref for WgpuInstance {
    type Target = wgpu::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

pub struct WgpuAdapter {
    pub adapter: wgpu::Adapter,
}

impl Deref for WgpuAdapter {
    type Target = wgpu::Adapter;

    fn deref(&self) -> &Self::Target {
        &self.adapter
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
    pub surface: wgpu::Surface<'static>,
}

impl Deref for WindowSurface {
    type Target = wgpu::Surface<'static>;

    fn deref(&self) -> &Self::Target {
        &self.surface
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

async fn create_surface(world: &mut World, window: &Window) {
    if world.has_resource::<WindowSurface>() {
        return;
    }

    let window = &**window;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window).unwrap())
            .unwrap()
    };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let mut required_limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
    required_limits.max_push_constant_size = 256;

    let (device, queue) = adapter
        .request_device(
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
        )
        .await
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

    world.insert_resource(WgpuInstance { instance }).await;
    world.insert_resource(WgpuAdapter { adapter }).await;
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
    }
}

async fn redraw_requested(world: WorldView, _event: Arc<RedrawRequested>) {
    world.fire_event(InitRenderResources, true).await;
    world.fire_event(PreRender, true).await;
    world.fire_event(Render, true).await;
    world.fire_event(PostRender, true).await;
}
