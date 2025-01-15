use std::{ops::Deref, sync::Arc};

use kyrene_core::{
    plugin::Plugin,
    prelude::{
        tokio::{self, sync::mpsc},
        World, WorldView,
    },
    world::{WorldShutdown, WorldStartup, WorldTick},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use winit::{
    dpi::LogicalSize, event::WindowEvent, event_loop::ControlFlow, window::WindowAttributes,
};

use crate::{Device, Queue};

#[derive(Clone)]
pub struct Window(Arc<winit::window::Window>);

impl Deref for Window {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct WindowSettings {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            title: "kyrene".to_string(),
            width: 800,
            height: 600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WinitEvent(winit::event::Event<()>);

impl Deref for WinitEvent {
    type Target = winit::event::Event<()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct WindowResized {
    pub new_width: u32,
    pub new_height: u32,
}

pub trait RunWindow {
    fn run_window(self, window_settings: WindowSettings);
}

impl RunWindow for World {
    fn run_window(self, window_settings: WindowSettings) {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();

        let view = self.into_world_view();

        let (tx, rx) = winit_events_channel();

        std::thread::spawn({
            let view = view.clone();
            let window_settings = window_settings.clone();
            move || {
                let sub = console_subscriber::ConsoleLayer::builder()
                    .with_default_env()
                    .spawn();
                tracing_subscriber::registry()
                    .with(sub)
                    .with(
                        tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()),
                    )
                    .init();

                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                runtime.block_on(async move {
                    view.fire_event(WorldStartup, true).await;

                    view.insert_resource(window_settings).await;

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

                    let WinitEventsRx {
                        mut window_created,
                        mut winit_event,
                        mut exiting,
                    } = rx;

                    tokio::spawn({
                        let view = view.clone();
                        async move {
                            loop {
                                let Some(window_created) = window_created.recv().await else {
                                    return;
                                };
                                view.fire_event(window_created, true).await;
                            }
                        }
                    });

                    tokio::spawn({
                        let view = view.clone();
                        async move {
                            loop {
                                let Some(winit_event) = winit_event.recv().await else {
                                    return;
                                };
                                view.fire_event(winit_event, true).await;
                            }
                        }
                    });

                    tokio::spawn({
                        let view = view.clone();
                        async move {
                            loop {
                                let Some(()) = exiting.recv().await else {
                                    return;
                                };
                                view.fire_event(WorldShutdown, true).await;
                            }
                        }
                    });

                    loop {
                        tokio::task::yield_now().await;
                    }
                });
            }
        });

        let mut winit_app = WinitApp {
            window: None,
            window_settings,
            events: tx,
        };

        event_loop.run_app(&mut winit_app).unwrap();
    }
}

pub struct WinitPlugin;

impl Plugin for WinitPlugin {
    async fn build(self, world: &mut World) {
        world.add_event::<WindowCreated>();
        world.add_event::<WinitEvent>();
        world.add_event::<WindowResized>();
        world.add_event::<WorldShutdown>();
        world.add_event::<RedrawRequested>();

        world.add_event_handler(winit_event);
    }
}

pub async fn winit_event(world: WorldView, event: Arc<WinitEvent>) {
    #[allow(clippy::single_match)]
    match &event.0 {
        winit::event::Event::WindowEvent {
            window_id: _,
            event,
        } => match event {
            WindowEvent::Resized(new_size) => {
                world
                    .fire_event(
                        WindowResized {
                            new_width: new_size.width,
                            new_height: new_size.height,
                        },
                        true,
                    )
                    .await;
            }
            WindowEvent::RedrawRequested => {
                world.fire_event(RedrawRequested, true).await;
            }
            _ => {}
        },
        _ => {}
    }
}

pub struct WindowCreated {
    pub window: Window,
    pub surface: Arc<wgpu::Surface<'static>>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Device,
    pub queue: Queue,
}

#[derive(Clone, Copy, Debug)]
pub struct RedrawRequested;

pub struct WinitEventsTx {
    window_created: mpsc::Sender<WindowCreated>,
    winit_event: mpsc::Sender<WinitEvent>,
    exiting: mpsc::Sender<()>,
}

pub struct WinitEventsRx {
    window_created: mpsc::Receiver<WindowCreated>,
    winit_event: mpsc::Receiver<WinitEvent>,
    exiting: mpsc::Receiver<()>,
}

pub fn winit_events_channel() -> (WinitEventsTx, WinitEventsRx) {
    let (window_created_tx, window_created_rx) = mpsc::channel(1);
    let (winit_event_tx, winit_event_rx) = mpsc::channel(1);
    let (exiting_tx, exiting_rx) = mpsc::channel(1);

    (
        WinitEventsTx {
            window_created: window_created_tx,
            winit_event: winit_event_tx,
            exiting: exiting_tx,
        },
        WinitEventsRx {
            window_created: window_created_rx,
            winit_event: winit_event_rx,
            exiting: exiting_rx,
        },
    )
}

struct WinitApp {
    window: Option<Window>,
    window_settings: WindowSettings,
    events: WinitEventsTx,
}

impl winit::application::ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title(&self.window_settings.title)
                    .with_inner_size(LogicalSize::new(
                        self.window_settings.width,
                        self.window_settings.height,
                    )),
            )
            .unwrap();
        let window = Window(Arc::new(window));
        self.window = Some(window.clone());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&*window).unwrap())
                .unwrap()
        };

        let adapter =
            kyrene_core::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .unwrap();

        let mut required_limits =
            wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
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

        self.events
            .window_created
            .blocking_send(WindowCreated {
                window,
                surface: Arc::new(surface),
                adapter: Arc::new(adapter),
                device: Device::new(device),
                queue: Queue::new(queue),
            })
            .unwrap();
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.events
            .winit_event
            .blocking_send(WinitEvent(winit::event::Event::DeviceEvent {
                device_id,
                event: event.clone(),
            }))
            .unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Poll);

        self.events
            .winit_event
            .blocking_send(WinitEvent(winit::event::Event::WindowEvent {
                window_id,
                event: event.clone(),
            }))
            .unwrap();

        if let Some(window) = self.window.as_ref() {
            if window.id() != window_id {
                return;
            }

            window.request_redraw();
        }

        #[allow(clippy::single_match)]
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.events.exiting.blocking_send(()).unwrap();
    }
}
