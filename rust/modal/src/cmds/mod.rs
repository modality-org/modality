//! CLI command modules for the Modal CLI.
//!
//! This module organizes all CLI commands into logical submodules:
//! - `local`: Local development commands (nodes, killall)
//! - `net`: Network-related commands (info, storage, mining)
//! - `node`: Node management commands (create, run, info, etc.)
//! - `contract`: Contract management commands (create, commit, push, etc.)
//! - `hub`: Contract hub server commands
//! - `predicate`: Predicate management and testing
//! - `program`: Program management and creation
//! - `chain`: Chain validation and testing

pub mod local;
pub mod net;
pub mod node;
pub mod contract;
pub mod hub;
pub mod predicate;
pub mod program;
pub mod chain;
