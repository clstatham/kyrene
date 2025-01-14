pub use kyrene_core as core;
pub use kyrene_wgpu as wgpu;
pub use kyrene_winit as winit;

pub use kyrene_macro::main;

pub mod prelude {
    pub use crate::main;
    pub use kyrene_core::prelude::*;
    pub use kyrene_winit::RunWinit;
    pub use std::time::{Duration, Instant};
}
