use kyrene::prelude::*;
use kyrene_core::{handler::IntoHandlerConfig, world::WorldStartup};
use kyrene_graphics::{
    window::{WindowSettings, WinitPlugin},
    WgpuPlugin,
};

#[derive(Debug, Clone)]
struct FooEvent {
    entity: Entity,
}

async fn foo_event_handler(world: WorldView, event: Arc<FooEvent>) {
    let mut counter = world.get_mut::<i32>(event.entity).await.unwrap();
    *counter += 1;
    info!("Handler 1: {} -> {}", *counter - 1, *counter);
}

async fn foo_event_handler_2(_world: WorldView, _event: Arc<FooEvent>) {
    info!("Handler 2");
}

async fn startup(world: WorldView, _event: Arc<WorldStartup>) {
    let entity = world.entity().await;

    world.insert::<i32>(entity, 0).await;
    world.insert::<f32>(entity, 0.0).await;

    let entity2 = world.entity().await;
    world.insert::<i32>(entity2, 1).await;

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

fn main() {
    let mut world = World::new();
    world.add_plugin(WinitPlugin);
    world.add_plugin(WgpuPlugin);

    world.add_event_handler(startup);

    world.add_event::<FooEvent>();
    world.add_event_handler(foo_event_handler);
    world.add_event_handler(foo_event_handler_2.after(foo_event_handler));

    world.run_winit(WindowSettings::default());
    // world.run();
}
