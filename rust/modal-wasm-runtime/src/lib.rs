pub mod executor;
pub mod gas;
pub mod registry;
pub mod cache;

pub use executor::WasmExecutor;
pub use gas::{GasMetrics, DEFAULT_GAS_LIMIT, MAX_GAS_LIMIT};
pub use registry::ModuleRegistry;
pub use cache::{WasmModuleCache, CacheStats};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub gas_used: u64,
    pub errors: Vec<String>,
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;

