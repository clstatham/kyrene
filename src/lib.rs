pub use kyrene_asset as asset;
pub use kyrene_core as core;
pub use kyrene_graphics as graphics;

pub mod prelude {
    pub use kyrene_core::prelude::*;
    pub use kyrene_graphics::window::RunWindow;
    pub use std::time::{Duration, Instant};
}
