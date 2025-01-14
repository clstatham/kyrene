use std::future::IntoFuture;

pub mod component;
pub mod entity;
#[macro_use]
pub mod event;
pub mod handler;
pub mod intern;
pub mod label;
pub mod loan;
pub mod lock;
pub mod plugin;
pub mod query;
pub mod resource;
pub mod util;
pub mod world;
pub mod world_view;

#[doc(hidden)]
pub extern crate tokio;

pub mod prelude {
    pub use crate::{
        block_on,
        component::{Component, Ref},
        entity::Entity,
        event::Event,
        lock::{MappedMutexGuard, Mutex, MutexGuard},
        plugin::Plugin,
        util::{FxHashMap, FxHashSet, TypeIdMap, TypeIdSet},
        world::{World, WorldTick},
        world_view::WorldView,
    };
    pub use futures::StreamExt;
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
