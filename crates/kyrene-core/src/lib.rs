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
pub mod resource;
pub mod util;
pub mod world;
pub mod world_view;

#[doc(hidden)]
pub extern crate tokio;

pub mod prelude {
    pub use crate::{
        component::{Component, Ref},
        entity::Entity,
        event::Event,
        lock::{MappedMutexGuard, Mutex, MutexGuard},
        plugin::Plugin,
        util::{FxHashMap, FxHashSet, TypeIdMap, TypeIdSet},
        world::{World, WorldTick},
        world_view::WorldView,
    };
    pub use std::sync::Arc;
    pub use tokio;
    pub use tracing::{debug, error, info, trace, warn};
}
