use kyrene::prelude::*;
use kyrene_core::world::WorldStartup;
use kyrene_winit::WindowSettings;

struct CurrentPlayer(Entity);

#[derive(Debug, Clone)]
struct FooEvent {
    entity: Entity,
}

async fn foo_event_handler(world: WorldView, event: Arc<FooEvent>) {
    let mut counter = world.get_mut::<i32>(event.entity).await.unwrap();
    *counter += 1;
    info!("{} -> {}", *counter - 1, *counter);
}

async fn startup(world: WorldView, _event: Arc<WorldStartup>) {
    let entity = world.entity().await;

    world.insert::<i32>(entity, 0).await;
    world.insert::<f32>(entity, 0.0).await;

    world.insert_resource(CurrentPlayer(entity)).await;

    info!("Before events");
    world.fire_event(FooEvent { entity }, true).await;
    info!("After first");
    world.fire_event(FooEvent { entity }, true).await;
    info!("After second");
    world.fire_event(FooEvent { entity }, true).await;
    info!("After third");
}

fn main() {
    let mut world = World::new();

    world.add_event_handler(startup);
    // world.add_event_handler(world_tick_handler);

    world.add_event::<FooEvent>();
    world.add_event_handler(foo_event_handler);

    world.run_winit(WindowSettings::default());
    // world.run();
}
