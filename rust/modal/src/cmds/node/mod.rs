//! Node management commands for the Modal CLI.
//!
//! This module provides commands for creating, running, and managing
//! Modality network nodes including miners, observers, and validators.

pub mod address;
pub mod clear;
pub mod clear_storage;
pub mod compare;
pub mod config;
pub mod create;
pub mod info;
pub mod inspect;
pub mod kill;
pub mod logs;
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

