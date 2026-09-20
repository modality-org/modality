//! Node and local development commands for Modal CLI.

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

pub mod address;
pub mod clear;
pub mod clear_storage;
pub mod compare;
pub mod config;
pub mod contract_get;
pub mod create;
pub mod info;
pub mod inspect;
pub mod kill;
pub mod local;
pub mod logs;
pub mod net_mining_sync;
pub mod net_storage;
pub mod pid;
pub mod ping;
pub mod restart;
pub mod run;
pub mod run_miner;
pub mod run_noop;
pub mod run_observer;
pub mod run_validator;
pub mod runner;
pub mod start;
pub mod stats;
pub mod stop;
pub mod sync;
