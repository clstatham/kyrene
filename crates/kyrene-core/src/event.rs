use std::{any::TypeId, marker::PhantomData, sync::Arc};

use downcast_rs::DowncastSync;
pub use event_listener::{IntoNotification, Listener, Notification};

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

    pub fn fire(&self, tag: T) {
        self.event.fire(tag);
    }

    pub fn fire_with<F>(&self, tag: F)
    where
        F: FnMut() -> T,
    {
        self.event.fire_with(tag);
    }

    pub fn listen(&self) -> EventListener<T> {
        EventListener::new(self.event.clone())
    }
}

impl<T: DowncastSync> Default for Event<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EventListener<T: DowncastSync> {
    event: DynEvent,
    listener: Option<event_listener::EventListener<Arc<dyn DowncastSync>>>,
    _marker: PhantomData<T>,
}

impl<T: DowncastSync> EventListener<T> {
    pub(crate) fn new(event: DynEvent) -> Self {
        Self {
            listener: Some(event.event.listen()),
            event,
            _marker: PhantomData,
        }
    }

    pub async fn next(&mut self) -> Arc<T> {
        let listener = self.listener.replace(self.event.event.listen()).unwrap();
        let tag = listener.await;
        DowncastSync::into_any_arc(tag).downcast().unwrap()
    }

    pub fn next_blocking(&mut self) -> Arc<T> {
        let listener = self.listener.replace(self.event.event.listen()).unwrap();
        let tag = listener.wait();
        DowncastSync::into_any_arc(tag).downcast().unwrap()
    }
}

#[macro_export]
macro_rules! stack_listener {
    ($event:ident => $listener:ident) => {
        ::event_listener::listener!($event.0 => $listener)
    };
}

#[derive(Clone)]
pub struct DynEvent {
    event: Arc<event_listener::Event<Arc<dyn DowncastSync>>>,
    type_id: TypeId,
}

impl DynEvent {
    pub fn new<T: DowncastSync>() -> Self {
        Self {
            event: Arc::new(event_listener::Event::with_tag()),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn listen(&self) -> DynEventListener {
        DynEventListener::new(self.clone())
    }

    #[track_caller]
    pub fn fire<T: DowncastSync>(&self, tag: T) {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        let tag: Arc<dyn DowncastSync> = Arc::new(tag);
        self.event.notify(usize::MAX.tag(tag));
    }

    #[track_caller]
    pub fn fire_with<T: DowncastSync, F: FnMut() -> T>(&self, mut tag: F) {
        assert_eq!(
            TypeId::of::<T>(),
            self.type_id,
            "Event Type ID mismatch; Check if you're sending the right kind of payload!"
        );
        self.event.notify(usize::MAX.tag_with(move || {
            let tag: Arc<dyn DowncastSync> = Arc::new(tag());
            tag
        }));
    }
}

pub struct DynEventListener {
    event: DynEvent,
    listener: Option<event_listener::EventListener<Arc<dyn DowncastSync>>>,
}

impl DynEventListener {
    pub(crate) fn new(event: DynEvent) -> Self {
        Self {
            listener: Some(event.event.listen()),
            event,
        }
    }

    pub async fn next(&mut self) -> Arc<dyn DowncastSync> {
        let listener = self.listener.replace(self.event.event.listen()).unwrap();
        listener.await
    }

    pub fn next_blocking(&mut self) -> Arc<dyn DowncastSync> {
        let listener = self.listener.replace(self.event.event.listen()).unwrap();
        listener.wait()
    }
}
