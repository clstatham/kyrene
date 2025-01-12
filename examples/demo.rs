use kyrene::prelude::*;

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

#[kyrene::main]
async fn main() {
    let mut world = World::new();

    let foo_event = world.add_event_handler(foo_event_handler);
    world.add_event_handler(foo_event_handler_2);

    let bar_event = world.add_event_handler(bar_event_handler);

    let entity = world.entity();
    world.insert::<i32>(entity, 0);
    world.insert::<f32>(entity, 0.0);

    tokio::spawn(async move {
        loop {
            foo_event.fire(FooEvent { entity });
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::spawn(async move {
        loop {
            bar_event.fire(BarEvent { entity, value: 2 });
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });

    world.run().await;
}
