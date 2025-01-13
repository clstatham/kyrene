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
    println!("{:?}", *counter);
    *counter += 1;
}

async fn foo_event_handler_2(world: WorldView, event: Arc<FooEvent>) {
    let mut counter = world.get_mut::<f32>(event.entity).await.unwrap();
    println!("{:?}", *counter);
    *counter += 1.0;
}

#[derive(Debug, Clone)]
struct BarEvent {
    entity: Entity,
    value: i32,
}

async fn bar_event_handler(world: WorldView, event: Arc<BarEvent>) {
    let mut counter = world.get_mut::<i32>(event.entity).await.unwrap();
    *counter += event.value;
}

async fn world_tick_handler(world: WorldView, event: Arc<WorldTick>) {
    let Some(player) = world.get_resource::<CurrentPlayer>().await else {
        return;
    };

    let entity = player.0;

    world.fire_event(FooEvent { entity }).await;

    if event.tick % 4 == 0 {
        world.fire_event(BarEvent { entity, value: 4 }).await;
    }
}

async fn startup(world: WorldView, _event: Arc<WorldStartup>) {
    let entity = world.entity().await;

    world.insert::<i32>(entity, 0).await;
    world.insert::<f32>(entity, 0.0).await;

    world.insert_resource(CurrentPlayer(entity)).await;
}

fn main() {
    let mut world = World::new();

    world.add_event_handler(startup);
    world.add_event_handler(world_tick_handler);

    world.add_event_handler(foo_event_handler);
    world.add_event_handler(foo_event_handler_2);

    world.add_event_handler(bar_event_handler);

    world.run_winit(WindowSettings::default());
}
