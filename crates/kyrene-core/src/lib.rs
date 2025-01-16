use std::future::IntoFuture;

pub mod component;
pub mod entity;
#[macro_use]
pub mod event;
pub mod handler;
pub mod intern;
pub mod label;
pub mod lock;
pub mod plugin;
pub mod query;
pub mod resource;
#[macro_use]
pub mod util;
pub mod bundle;
pub mod world;
pub mod world_handle;

#[doc(hidden)]
pub extern crate tokio;

#[doc(hidden)]
pub extern crate self as kyrene_core;

pub use kyrene_macro::Bundle;

pub mod prelude {
    pub use crate::{
        block_on,
        component::{Component, Ref},
        entity::Entity,
        event::EventDispatcher,
        handler::IntoHandlerConfig,
        lock::{MappedMutexGuard, Mutex, MutexGuard},
        plugin::Plugin,
        util::{FxHashMap, FxHashSet, TypeIdMap, TypeIdSet},
        world::{World, WorldTick},
        world_handle::WorldHandle,
    };
    pub use futures::StreamExt;
    pub use kyrene_macro::Bundle;
    pub use std::sync::Arc;
    pub use tokio;
    pub use tracing::{debug, error, info, trace, warn};
}

/// Blocks the thread until the future is ready.
///
/// The future does not necessarily have to be `Send`, `Sync`, or `'static`.
///
/// Use this sparingly and only on futures that don't run for very long, because this doesn't cooperate well with the async context.
pub fn block_on<F: IntoFuture>(fut: F) -> F::Output {
    pollster::block_on(fut)
}
