//! Contract hub server commands for Modal CLI.

#![allow(clippy::type_complexity)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::manual_filter_map)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::redundant_locals)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::manual_unwrap_or_default)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(unused_mut)]
#![allow(clippy::infinite_iter)]
#![allow(clippy::lines_filter_map_ok)]

pub mod core;
pub mod handler;
pub mod model_validator;
pub mod rest;
pub mod start;
