//! Shared utilities for Modal CLI domain crates.

pub mod dir;
pub mod output;

pub use dir::resolve_node_dir;
pub use output::{OutputFormat, format_output};
