use std::{any::TypeId, marker::PhantomData, sync::Arc};

use downcast_rs::DowncastSync;

use crate::{handler::DynEventHandlers, prelude::WorldView};

pub struct Event<T: DowncastSync = ()> {
    event: DynEvent,
    _marker: PhantomData<T>,
}

impl<T: DowncastSync> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: DowncastSync> Event<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            event: DynEvent::new::<T>(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn from_dyn_event(event: DynEvent) -> Self {
        assert_eq!(event.type_id, TypeId::of::<T>());
        Self {
            event,
            _marker: PhantomData,
        }
    }

    pub fn fire_blocking(&self, world: WorldView, tag: T) -> usize {
        self.event.fire_blocking(world, tag)
    }

    pub async fn fire(&self, world: WorldView, tag: T, await_all_handlers: bool) -> usize {
        self.event.fire(world, tag, await_all_handlers).await
    }
}

pub(crate) struct DynEvent {
    pub(crate) handlers: DynEventHandlers,
    type_id: TypeId,
}

impl Clone for DynEvent {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            type_id: self.type_id,
        }
    }
}

impl DynEvent {
    pub fn new<T: DowncastSync>() -> Self {
        Self {
            handlers: DynEventHandlers::new(),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn fire_blocking<T: DowncastSync>(&self, world: WorldView, tag: T) -> usize {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        let tag: Arc<dyn DowncastSync> = Arc::new(tag);

        let handlers = self.handlers.handlers.blocking_read();

        for handler in handlers.iter() {
            pollster::block_on(handler.run_dyn(world.clone(), tag.clone()));
        }

        handlers.len()
    }

    pub async fn fire<T: DowncastSync>(
        &self,
        world: WorldView,
        tag: T,
        await_all_handlers: bool,
    ) -> usize {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        let tag: Arc<dyn DowncastSync> = Arc::new(tag);

        let handlers = self.handlers.handlers.read().await;
        let mut join_handles = Vec::new();

        for handler in handlers.iter() {
            let jh = tokio::spawn(handler.run_dyn(world.clone(), tag.clone()));
            join_handles.push(jh);
        }

        if await_all_handlers {
            for handle in join_handles {
                handle.await.unwrap();
            }
        }

        handlers.len()
    }
}
