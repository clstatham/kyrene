use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::sync::mpsc::{Sender, UnboundedSender};

use crate::{
    component::{Component, Ref},
    entity::Entity,
    world::World,
};

pub type WorldOpFnMut =
    dyn for<'a> FnOnce(&'a mut World) -> BoxFuture<'a, Arc<dyn Component>> + Send + Sync;

// pub type WorldOpFn<'a> =
//     dyn FnOnce(&'a World) -> BoxFuture<'a, Arc<dyn WorldOpResult<'a>>> + Send + Sync + 'a;

pub struct Deferred {
    task: Box<WorldOpFnMut>,
    tx: Sender<Arc<dyn Component>>,
}

impl Deferred {
    pub async fn run(self, world: &mut World) {
        let result = (self.task)(world).await;
        self.tx.try_send(result).unwrap();
    }
}

#[derive(Clone)]
pub struct WorldView {
    pub(crate) tx: UnboundedSender<Deferred>,
}

impl WorldView {
    /// Defers an asynchronous operation that requires mutable world access.
    ///
    /// This function takes a closure that has a `&mut World` argument and returns a future containing code to run the next time the world applies its deferred tasks.
    /// Any code after this function's await point will not be run until after this has happened.
    pub async fn defer<T, F>(&self, func: F) -> T
    where
        T: Component,
        for<'a> F: FnOnce(&'a mut World) -> BoxFuture<'a, T> + Send + Sync + 'a,
    {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        self.tx
            .send(Deferred {
                task: Box::new(|world: &mut World| {
                    {
                        func(world).map(|result| {
                            let result: Arc<dyn Component> = Arc::new(result);
                            result
                        })
                    }
                    .boxed()
                }),
                tx,
            })
            .unwrap();

        let component: Arc<dyn Component> = rx.recv().await.unwrap();
        let component: Arc<T> = component.downcast_arc().unwrap_or_else(|_| unreachable!());
        Arc::into_inner(component).unwrap()
    }

    pub async fn entity(&self) -> Entity {
        self.defer(move |world: &mut World| async move { world.entity() }.boxed())
            .await
    }

    pub async fn insert<T: Component>(&self, entity: Entity, component: T) -> Option<T> {
        self.defer(move |world: &mut World| async move { world.insert(entity, component) }.boxed())
            .await
    }

    pub async fn get<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        self.defer(move |world: &mut World| async move { world.get::<T>(entity).await }.boxed())
            .await
    }
}
