use kyrene::prelude::*;
use kyrene_core::{handler::Local, world::WorldStartup};
use kyrene_graphics::{
    window::{RedrawRequested, WindowSettings, WinitPlugin},
    WgpuPlugin,
};

#[derive(Debug, Clone)]
struct FooEvent;

async fn foo_event_handler(_event: Event<FooEvent>, local: Local<usize>) {
    let mut local = local.get_mut().await;
    *local += 1;
    println!("Handler 1: {} -> {}", *local - 1, *local);
}

async fn foo_event_handler_2(_event: Event<FooEvent>) {
    println!("Handler 2");
}

async fn startup(_event: Event<WorldStartup>, world: WorldHandle) {
    let _entity = world.spawn((0i32, 0.0f32)).await;

    let _entity2 = world.spawn((0i32,)).await;

    world
        .query_iter::<&i32>(|_world, n| async move {
            println!("{:?}", *n);
        })
        .await;

    world
        .query_iter::<&f32>(|_world, n| async move {
            println!("{:?}", *n);
        })
        .await;

    world
        .query_iter::<(&i32, &mut f32)>(|_world, (a, mut b)| async move {
            println!("{:?}, {:?}", *a, *b);
            *b += 1.0;
            println!("{:?}, {:?}", *a, *b);
        })
        .await;
}

// async fn world_tick(event: Event<WorldTick>, world: WorldHandle) {
//     println!("{:?}", event.delta_time());
//     world.fire_event(FooEvent, true).await;
// }

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
    // world.add_event_handler(world_tick);

    world.add_event_handler(foo_event_handler);
    world.add_event_handler(foo_event_handler_2.after(foo_event_handler));

    world.run_window(WindowSettings::default());
    // world.run();
}
