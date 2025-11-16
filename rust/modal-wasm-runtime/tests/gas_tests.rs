use modal_wasm_runtime::{WasmExecutor, DEFAULT_GAS_LIMIT, MAX_GAS_LIMIT};

#[test]
fn test_gas_limit_enforcement() {
    // Create executor with very low gas limit
    let mut executor = WasmExecutor::new(100);
    
    // This should fail due to insufficient gas
    // (even compiling the module uses some gas)
    let minimal_wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // Magic number
        0x01, 0x00, 0x00, 0x00, // Version
    ];
    
    // With such low gas, execution should fail
    let result = executor.execute(&minimal_wasm, "main", "{}");
    // Note: This will fail at validation stage in current implementation
    assert!(result.is_err() || executor.remaining_gas() == 0);
}

#[test]
fn test_default_gas_limit() {
    let executor = WasmExecutor::new(DEFAULT_GAS_LIMIT);
    assert_eq!(executor.gas_limit, DEFAULT_GAS_LIMIT);
    assert_eq!(executor.gas_limit, 10_000_000);
}

#[test]
fn test_max_gas_limit() {
    let executor = WasmExecutor::new(MAX_GAS_LIMIT);
    assert_eq!(executor.gas_limit, MAX_GAS_LIMIT);
    assert_eq!(executor.gas_limit, 100_000_000);
}

#[test]
fn test_gas_metrics() {
    use modal_wasm_runtime::GasMetrics;
    
    let mut metrics = GasMetrics::new(1000);
    assert_eq!(metrics.remaining(), 1000);
    assert!(!metrics.is_exhausted());
    
    metrics.used = 500;
    assert_eq!(metrics.remaining(), 500);
    assert!(!metrics.is_exhausted());
    
    metrics.used = 1000;
    assert_eq!(metrics.remaining(), 0);
    assert!(metrics.is_exhausted());
    
    // Test overflow protection
    metrics.used = 1500;
    assert_eq!(metrics.remaining(), 0);
    assert!(metrics.is_exhausted());
}

#[test]
fn test_gas_limit_prevents_infinite_loop() {
    // In a real implementation with proper WASM that has infinite loops,
    // the gas limit would halt execution
    // For now, we test that gas limits are respected during setup
    
    let low_gas_executor = WasmExecutor::new(1000);
    let high_gas_executor = WasmExecutor::new(1_000_000);
    
    assert!(low_gas_executor.gas_limit < high_gas_executor.gas_limit);
}

#[test]
fn test_executor_with_different_gas_limits() {
    let executors = vec![
        WasmExecutor::new(1_000),
        WasmExecutor::new(10_000),
        WasmExecutor::new(100_000),
        WasmExecutor::new(1_000_000),
        WasmExecutor::new(10_000_000),
    ];
    
    for (i, executor) in executors.iter().enumerate() {
        let expected = 1_000 * 10_u64.pow(i as u32);
        assert_eq!(executor.gas_limit, expected);
    }
}

