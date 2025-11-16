/// Default gas limit for WASM execution (10 million instructions)
pub const DEFAULT_GAS_LIMIT: u64 = 10_000_000;

/// Maximum gas limit allowed (100 million instructions)
pub const MAX_GAS_LIMIT: u64 = 100_000_000;

/// Gas metrics for tracking execution costs
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GasMetrics {
    /// Gas used by the execution
    pub used: u64,
    /// Gas limit set for the execution
    pub limit: u64,
}

impl GasMetrics {
    pub fn new(limit: u64) -> Self {
        Self { used: 0, limit }
    }

    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    pub fn is_exhausted(&self) -> bool {
        self.used >= self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_metrics() {
        let mut metrics = GasMetrics::new(1000);
        assert_eq!(metrics.remaining(), 1000);
        assert!(!metrics.is_exhausted());

        metrics.used = 500;
        assert_eq!(metrics.remaining(), 500);
        assert!(!metrics.is_exhausted());

        metrics.used = 1000;
        assert_eq!(metrics.remaining(), 0);
        assert!(metrics.is_exhausted());

        metrics.used = 1500;
        assert_eq!(metrics.remaining(), 0);
        assert!(metrics.is_exhausted());
    }
}

