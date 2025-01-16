use kyrene::prelude::*;
use kyrene_core::world::WorldStartup;
use kyrene_graphics::{
    window::{WindowSettings, WinitPlugin},
    WgpuPlugin,
};

#[derive(Debug, Clone)]
struct FooEvent {
    entity: Entity,
}

async fn foo_event_handler(event: Event<FooEvent>, world: WorldHandle) {
    let mut counter = world.get_mut::<i32>(event.entity).await.unwrap();
    *counter += 1;
    println!("Handler 1: {} -> {}", *counter - 1, *counter);
}

async fn foo_event_handler_2(_event: Event<FooEvent>) {
    println!("Handler 2");
}

async fn startup(_event: Event<WorldStartup>, world: WorldHandle) {
    let entity = world.spawn((0i32, 0.0f32)).await;

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

    world.fire_event(FooEvent { entity }, true).await;
}

async fn world_tick(event: Event<WorldTick>) {
    println!("{:?}", event.delta_time());
}

fn main() {
    let mut world = World::new();
    world.add_plugin(WinitPlugin);
    world.add_plugin(WgpuPlugin);

    world.add_event_handler(startup);
    world.add_event_handler(world_tick);

    world.add_event_handler(foo_event_handler);
    world.add_event_handler(foo_event_handler_2.after(foo_event_handler));

    world.run_window(WindowSettings::default());
    // world.run();
}
