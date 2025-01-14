use std::{ops::Deref, sync::Arc};

use kyrene_core::{
    event::Event,
    plugin::Plugin,
    prelude::{tokio, World, WorldView},
    world::{WorldShutdown, WorldStartup, WorldTick},
};
use tracing::level_filters::LevelFilter;
use winit::{
    dpi::LogicalSize, event::WindowEvent, event_loop::ControlFlow, window::WindowAttributes,
};

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

pub trait RunWinit {
    fn run_winit(self, window_settings: WindowSettings);
}

impl RunWinit for World {
    fn run_winit(self, window_settings: WindowSettings) {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();

        let window_created_event = self.get_event::<WindowCreated>().unwrap();

        let winit_event_event = self.get_event::<WinitEvent>().unwrap();

        let window_resized_event = self.get_event::<WindowResized>().unwrap();

        let world_shutdown_event = self.get_event::<WorldShutdown>().unwrap();

        let redraw_requested_event = self.get_event::<RedrawRequested>().unwrap();

        let view = self.into_world_view();

        std::thread::spawn({
            let view = view.clone();
            move || {
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
        });

        let mut winit_app = WinitApp {
            world: view,
            window: None,
            window_settings,
            window_created_event,
            winit_event_event,
            window_resized_event,
            world_shutdown_event,
            redraw_requested_event,
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
    }
}

#[derive(Clone)]
pub struct WindowCreated(pub Window);

#[derive(Clone, Copy, Debug)]
pub struct RedrawRequested;

struct WinitApp {
    world: WorldView,
    window: Option<Window>,
    window_settings: WindowSettings,
    window_created_event: Event<WindowCreated>,
    winit_event_event: Event<WinitEvent>,
    window_resized_event: Event<WindowResized>,
    world_shutdown_event: Event<WorldShutdown>,
    redraw_requested_event: Event<RedrawRequested>,
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
        self.window_created_event
            .fire_blocking(self.world.clone(), WindowCreated(window));
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.winit_event_event.fire_blocking(
            self.world.clone(),
            WinitEvent(winit::event::Event::DeviceEvent {
                device_id,
                event: event.clone(),
            }),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Poll);

        self.winit_event_event.fire_blocking(
            self.world.clone(),
            WinitEvent(winit::event::Event::WindowEvent {
                window_id,
                event: event.clone(),
            }),
        );

        if let Some(window) = self.window.as_ref() {
            if window.id() != window_id {
                return;
            }

            window.request_redraw();
        }

        match event {
            WindowEvent::Resized(size) => {
                self.window_resized_event.fire_blocking(
                    self.world.clone(),
                    WindowResized {
                        new_width: size.width,
                        new_height: size.height,
                    },
                );
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.redraw_requested_event
                    .fire_blocking(self.world.clone(), RedrawRequested);
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.world_shutdown_event
            .fire_blocking(self.world.clone(), WorldShutdown);
    }
}
