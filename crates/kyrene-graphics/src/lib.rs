use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use bind_group::BindGroupLayouts;
use camera::{insert_view_target, GpuCamera, InsertViewTarget, ViewTarget};
use hdr::HdrPlugin;
use kyrene_core::{
    entity::Entity,
    handler::{Res, ResMut},
    plugin::Plugin,
    prelude::WorldView,
    world::World,
};
use pipeline::RenderPipelines;
use texture::texture_format::{DEPTH_FORMAT, VIEW_FORMAT};
use window::{RedrawRequested, WindowCreated};

pub mod bind_group;
pub mod camera;
pub mod hdr;
pub mod pipeline;
pub mod texture;
pub mod window;

#[macro_export]
macro_rules! wrap_wgpu {
    ($t:ident < $mark:ident : $tr:ident >) => {
        pub struct $t<$mark: $tr>(
            ::std::sync::Arc<wgpu::$t>,
            ::std::marker::PhantomData<$mark>,
        );

        impl<$mark: $tr> $t<$mark> {
            pub fn new(inner: wgpu::$t) -> Self {
                Self(::std::sync::Arc::new(inner), ::std::marker::PhantomData)
            }
        }

        impl<$mark: $tr> Clone for $t<$mark> {
            fn clone(&self) -> Self {
                Self(self.0.clone(), self.1)
            }
        }

        impl<$mark: $tr> ::std::ops::Deref for $t<$mark> {
            type Target = wgpu::$t;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };

    ($t:ident) => {
        #[derive(Clone)]
        pub struct $t(::std::sync::Arc<wgpu::$t>);

        impl $t {
            pub fn new(inner: wgpu::$t) -> Self {
                Self(::std::sync::Arc::new(inner))
            }
        }

        impl ::std::ops::Deref for $t {
            type Target = wgpu::$t;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<wgpu::$t> for $t {
            fn from(inner: wgpu::$t) -> Self {
                Self::new(inner)
            }
        }
    };
}

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

wrap_wgpu!(Device);
wrap_wgpu!(Queue);

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

impl DerefMut for ActiveCommandEncoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.encoder
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
    let WindowCreated {
        window,
        surface,
        adapter,
        device,
        queue,
    } = &*event;

    let window = window.clone();
    let surface = surface.clone();
    let adapter = adapter.clone();

    let caps = surface.get_capabilities(&adapter);

    surface.configure(
        device,
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
    world.insert_resource(device.clone()).await;
    world.insert_resource(queue.clone()).await;
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
        world.add_event_handler(begin_render);
        world.add_event_handler(insert_view_target);
        world.add_event_handler(end_render);
        world.add_event_handler(post_render);

        world.insert_resource(CurrentFrame::default()).await;
        world.insert_resource(BindGroupLayouts::default()).await;
        world.insert_resource(RenderPipelines::default()).await;

        world.add_plugin(HdrPlugin);
    }
}

pub struct BeginRender;

async fn redraw_requested(world: WorldView, _event: Arc<RedrawRequested>) {
    if !world.has_resource::<Device>().await {
        return;
    }
    tracing::trace!("redraw_requested");
    world.fire_event(InitRenderResources, true).await;
    world.fire_event(PreRender, true).await;
    world.fire_event(Render, true).await;
    world.fire_event(PostRender, true).await;
}

pub async fn pre_render(world: WorldView, _event: Arc<PreRender>) {
    tracing::trace!("pre_render");

    world.fire_event(BeginRender, true).await;
    world
        .query_iter::<(Entity, &GpuCamera)>(|world, (camera, _)| async move {
            world.fire_event(InsertViewTarget { camera }, true).await;
        })
        .await;
}

pub async fn post_render(world: WorldView, _event: Arc<PostRender>) {
    tracing::trace!("post_render");

    world.fire_event(EndRender, true).await;
}

pub async fn begin_render(
    world: WorldView,
    _event: Arc<BeginRender>,
    mut current_frame: ResMut<CurrentFrame>,
    surface: Res<WindowSurface>,
    device: Res<Device>,
    mut command_buffers: ResMut<CommandBuffers>,
) {
    if current_frame.inner.is_some() {
        return;
    }

    tracing::trace!("begin_render");

    let view_targets = world.entities_with::<ViewTarget>().await;
    for entity in view_targets {
        world.remove::<ViewTarget>(entity).await;
    }

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

pub struct EndRender;

pub async fn end_render(
    world: WorldView,
    _event: Arc<EndRender>,
    mut command_buffers: ResMut<CommandBuffers>,
    mut current_frame: ResMut<CurrentFrame>,
    queue: Res<Queue>,
) {
    let Some(current_frame) = current_frame.inner.take() else {
        return;
    };

    tracing::trace!("end_render");

    let CurrentFrameInner {
        surface_texture, ..
    } = current_frame;

    if let Some(encoder) = world.remove_resource::<ActiveCommandEncoder>().await {
        command_buffers.enqueue(encoder.finish());
    }

    let command_buffers: Vec<wgpu::CommandBuffer> =
        std::mem::take(&mut command_buffers.command_buffers);

    queue.submit(command_buffers);

    let surface_texture = Arc::into_inner(surface_texture).unwrap();
    surface_texture.present();
}
