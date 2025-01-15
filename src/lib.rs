pub use kyrene_core as core;
pub use kyrene_graphics as graphics;

pub use kyrene_macro::main;

pub mod prelude {
    pub use crate::main;
    pub use kyrene_core::prelude::*;
    pub use kyrene_graphics::window::RunWindow;
    pub use std::time::{Duration, Instant};
}
