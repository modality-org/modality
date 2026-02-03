//! Modal RPC - JSON-RPC interface for hubs and networks
//!
//! Provides a common RPC interface that can be used by both
//! contract hubs and the Modal Money network.

pub mod types;
pub mod methods;
pub mod server;
pub mod client;
pub mod error;

pub use types::*;
pub use methods::*;
pub use server::{RpcServer, RpcServerConfig};
pub use client::RpcClient;
pub use error::RpcError;

/// RPC API version
pub const API_VERSION: &str = "0.1.0";

/// Default RPC port
pub const DEFAULT_PORT: u16 = 8899;
