use std::time::Duration;

use kyrene_core::{entity::Entity, tokio, world::World, world_view::WorldView};

#[derive(Debug, Clone)]
struct FooEvent {
    entity: Entity,
}

async fn foo_event_handler(world: WorldView, event: FooEvent) {
    let mut counter = world.get::<i32>(event.entity).await.unwrap();
    println!("{:?}", *counter);
    *counter += 1;
}

#[kyrene::main]
async fn main() {
    let mut world = World::new();

    let event = world.add_event_handler(foo_event_handler);

    let entity = world.entity();
    world.insert::<i32>(entity, 0);

    kyrene::spawn(async move {
        loop {
            event.fire(FooEvent { entity });
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    world.run().await;
}
