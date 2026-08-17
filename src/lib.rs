//! Library crate for `purge-deps`.
//!
//! The binary in `src/main.rs` parses CLI flags and calls [`run`].

mod app;

pub use app::cli;
pub use app::presets::Preset;
pub use app::{run, Config, Error, Report};
