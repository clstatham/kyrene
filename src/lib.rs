pub use kyrene_core as core;
pub use kyrene_util as util;

pub use kyrene_macro::main;

pub mod prelude {
    pub use crate::main;
    pub use kyrene_core::prelude::*;
    pub use std::time::{Duration, Instant};
}
