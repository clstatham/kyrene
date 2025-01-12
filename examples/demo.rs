use kyrene::prelude::*;

struct CurrentPlayer(Entity);

#[derive(Debug, Clone)]
struct FooEvent {
    entity: Entity,
}

async fn foo_event_handler(world: WorldView, event: Arc<FooEvent>) {
    let mut counter = world.get::<i32>(event.entity).await.unwrap();
    println!("{:?}", *counter);
    *counter += 1;
}

async fn foo_event_handler_2(world: WorldView, event: Arc<FooEvent>) {
    let mut counter = world.get::<f32>(event.entity).await.unwrap();
    println!("{:?}", *counter);
    *counter += 1.0;
}

#[derive(Debug, Clone)]
struct BarEvent {
    entity: Entity,
    value: i32,
}

async fn bar_event_handler(world: WorldView, event: Arc<BarEvent>) {
    let mut counter = world.get::<i32>(event.entity).await.unwrap();
    *counter += event.value;
}

async fn world_tick_handler(world: WorldView, event: Arc<WorldTick>) {
    let entity = world.get_resource::<CurrentPlayer>().await.unwrap().0;

    world.fire_event(FooEvent { entity }).await;

    if event.tick % 4 == 0 {
        world.fire_event(BarEvent { entity, value: 4 }).await;
    }
}

#[kyrene::main]
async fn main() {
    let mut world = World::new();

    world.add_event_handler(world_tick_handler);

    world.add_event_handler(foo_event_handler);
    world.add_event_handler(foo_event_handler_2);

    world.add_event_handler(bar_event_handler);

    let entity = world.entity();
    world.insert::<i32>(entity, 0).await;
    world.insert::<f32>(entity, 0.0).await;

    world.insert_resource(CurrentPlayer(entity)).await;

    world.run().await;
}
