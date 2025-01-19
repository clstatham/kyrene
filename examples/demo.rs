use kyrene::prelude::*;
use kyrene_core::{handler::Local, world::WorldStartup};
use kyrene_graphics::{
    clear_color::ClearColor,
    color::Color,
    window::{RedrawRequested, WindowSettings, WinitPlugin},
    WgpuPlugin,
};

async fn startup(_event: Event<WorldStartup>, world: WorldHandle) {
    world
        .insert_resource(ClearColor::new(Color::new(0.05, 0.05, 0.1, 1.0)))
        .await;
}

async fn world_tick(_event: Event<WorldTick>, _world: WorldHandle) {}

struct FrameTime {
    print_time: Duration,
    accum: Duration,
    n: usize,
}

impl Default for FrameTime {
    fn default() -> Self {
        Self {
            print_time: Duration::from_secs(1),
            accum: Duration::ZERO,
            n: 0,
        }
    }
}

async fn print_frame_time(event: Event<RedrawRequested>, info: Local<FrameTime>) {
    let mut info = info.get_mut().await;
    info.accum += event.delta_time().unwrap_or_default();
    info.n += 1;
    if info.accum >= info.print_time {
        let frame_time = info.accum.as_secs_f64() / info.n as f64;
        let fps = 1.0 / frame_time;
        println!(
            "Frame time: {:.3?} ({:.2} fps)",
            Duration::from_secs_f64(frame_time),
            fps
        );
        info.accum = Duration::ZERO;
        info.n = 0;
    }
}

fn main() {
    let mut world = World::new();
    world.add_plugin(WinitPlugin);
    world.add_plugin(WgpuPlugin);

    world.add_event_handler(startup);
    world.add_event_handler(print_frame_time);
    world.add_event_handler(world_tick);

    world.run_window(WindowSettings::default());
}
