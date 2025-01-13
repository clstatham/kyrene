use std::{ops::Deref, sync::Arc};

use kyrene_core::{
    event::Event,
    lock::Mutex,
    prelude::{
        tokio::{self},
        World, WorldView,
    },
    util::TypeIdMap,
    world::{WorldShutdown, WorldStartup, WorldTick},
};
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
    fn run_winit(mut self, window_settings: WindowSettings) {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();

        let window_created_event = self.event::<WindowCreated>();
        self.add_event_handler(window_created);

        let winit_event_event = self.event::<WinitEvent>();

        let window_resized_event = self.event::<WindowResized>();

        let world_shutdown_event = self.event::<WorldShutdown>();

        let redraw_requested_event = self.event::<RedrawRequested>();

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                let (tx, mut op_rx) = tokio::sync::mpsc::unbounded_channel();
                let view = WorldView::from_inner(tx);

                let mut total_listeners = TypeIdMap::default();
                let listeners_ready = Arc::new(Mutex::new(TypeIdMap::default()));

                // spawn all event listeners
                let mut event_handlers = self.event_handlers.clone();
                for (event_type_id, mut handlers) in event_handlers.handlers.drain() {
                    let event = event_handlers.events.remove(&event_type_id).unwrap();
                    total_listeners.insert(event_type_id, handlers.len());
                    for handler in handlers.drain(..) {
                        tokio::spawn({
                            let view = view.clone();
                            let event = event.clone();
                            let listeners_ready = listeners_ready.clone();
                            let mut listener = event.listen();

                            async move {
                                *listeners_ready
                                    .lock()
                                    .await
                                    .entry(event_type_id)
                                    .or_insert(0usize) += 1;

                                loop {
                                    let payload = listener.next().await;
                                    handler.run_dyn(view.clone(), payload).await;
                                }
                            }
                        });
                    }
                }

                // wait for all event listeners to be ready and listening
                loop {
                    if listeners_ready
                        .lock()
                        .await
                        .iter()
                        .all(|(k, v)| *v == total_listeners[k])
                    {
                        break;
                    }

                    tokio::task::yield_now().await;
                }

                self.fire_event(WorldStartup);

                // spawn WorldTick task
                let mut tick = 0;
                tokio::spawn({
                    let view = view.clone();
                    async move {
                        loop {
                            tick += 1;
                            view.fire_event(WorldTick { tick }).await;
                        }
                    }
                });

                tokio::spawn(async move {
                    loop {
                        while let Ok(op) = op_rx.try_recv() {
                            op.run(&mut self).await;
                        }
                    }
                });

                loop {
                    tokio::task::yield_now().await;
                }
            });
        });

        let mut winit_app = WinitApp {
            window: None,
            window_created_event,
            winit_event_event,
            window_resized_event,
            world_shutdown_event,
            redraw_requested_event,
            window_settings,
        };

        event_loop.run_app(&mut winit_app).unwrap();
    }
}

#[derive(Clone)]
struct WindowCreated(Arc<Mutex<Option<Window>>>);

async fn window_created(world: WorldView, event: Arc<WindowCreated>) {
    let window = event.0.lock().await.take().unwrap();
    world.insert_resource(window).await;
}

#[derive(Clone)]
struct RedrawRequested;

struct WinitApp {
    window: Option<Window>,
    window_created_event: Event<WindowCreated>,
    winit_event_event: Event<WinitEvent>,
    window_resized_event: Event<WindowResized>,
    world_shutdown_event: Event<WorldShutdown>,
    redraw_requested_event: Event<RedrawRequested>,
    window_settings: WindowSettings,
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
            .fire(WindowCreated(Arc::new(Mutex::new(Some(window)))));
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.winit_event_event
            .fire(WinitEvent(winit::event::Event::DeviceEvent {
                device_id,
                event: event.clone(),
            }));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Poll);

        self.winit_event_event
            .fire(WinitEvent(winit::event::Event::WindowEvent {
                window_id,
                event: event.clone(),
            }));

        if let Some(window) = self.window.as_ref() {
            if window.id() != window_id {
                return;
            }

            window.request_redraw();
        }

        match event {
            WindowEvent::Resized(size) => {
                self.window_resized_event.fire(WindowResized {
                    new_width: size.width,
                    new_height: size.height,
                });
            }
            WindowEvent::CloseRequested => {
                self.world_shutdown_event.fire(WorldShutdown);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.redraw_requested_event.fire(RedrawRequested);
            }
            _ => {}
        }
    }
}
