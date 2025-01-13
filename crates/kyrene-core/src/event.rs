use std::{any::TypeId, sync::Arc};

use downcast_rs::DowncastSync;
pub use event_listener::{IntoNotification, Listener, Notification};

#[derive(Debug)]
pub struct Event<T: Send + Sync = ()>(Arc<event_listener::Event<T>>);

impl<T: Send + Sync> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Send + Sync> Event<T> {
    pub fn new() -> Self {
        Self(Arc::new(event_listener::Event::<T>::with_tag()))
    }

    pub fn fire(&self, tag: T)
    where
        T: Clone,
    {
        self.0.notify(usize::MAX.tag(tag));
    }

    pub fn fire_with<F>(&self, tag: F)
    where
        F: FnMut() -> T,
    {
        self.0.notify(usize::MAX.tag_with(tag));
    }

    pub fn listen(&self) -> EventListener<T> {
        self.0.listen()
    }
}

impl<T: Send + Sync> Default for Event<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub type EventListener<T> = event_listener::EventListener<T>;

#[macro_export]
macro_rules! stack_listener {
    ($event:ident => $listener:ident) => {
        ::event_listener::listener!($event.0 => $listener)
    };
}

#[derive(Clone)]
pub struct DynEvent {
    event: Event<Arc<dyn DowncastSync>>,
    type_id: TypeId,
}

impl DynEvent {
    pub fn new<T: DowncastSync>() -> Self {
        Self {
            event: Event::new(),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn listen(&self) -> EventListener<Arc<dyn DowncastSync>> {
        self.event.listen()
    }

    #[track_caller]
    pub fn fire<T: DowncastSync + Clone>(&self, tag: T) {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        let tag: Arc<_> = Arc::new(tag);
        self.event.fire(tag);
    }

    #[track_caller]
    pub fn fire_with<T: DowncastSync, F: FnMut() -> T>(&self, mut tag: F) {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        self.event.fire_with(move || {
            let tag: Arc<dyn DowncastSync> = Arc::new(tag());
            tag
        });
    }
}
