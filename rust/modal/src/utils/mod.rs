//! Shared utilities for the Modal CLI.
//!
//! This module provides common functionality used across multiple CLI commands,
//! including directory resolution, output formatting, and other helpers.

pub mod dir;
pub mod output;

pub use dir::resolve_node_dir;
// Re-export output utilities for use in contract commands
#[allow(unused_imports)]
pub use output::{OutputFormat, format_output};

