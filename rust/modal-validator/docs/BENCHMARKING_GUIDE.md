# Benchmarking Guide

This guide explains how to run and interpret the performance benchmarks for the Shoal consensus implementation.

## Quick Start

```bash
# Run all benchmarks
cd rust/modal-sequencer-consensus
cargo bench --bench consensus_benchmarks

# Run specific benchmark group
cargo bench --bench consensus_benchmarks certificate_formation

# Run with quick mode (fewer samples for faster results)
cargo bench --bench consensus_benchmarks -- --quick

# Run specific test
cargo bench --bench consensus_benchmarks -- "form_certificate/4"
```

## Benchmark Structure

The benchmark suite is organized into 9 major groups:

### 1. Certificate Formation (`certificate_formation`)
Measures vote collection and certificate assembly performance.
- **Tests**: 4, 7, 10, 16 validators
- **Metric**: Time to form certificate with quorum

### 2. DAG Insertion (`dag_insertion`)
Measures certificate insertion into the DAG with parent validation.
- **Tests**: Round 0 (genesis), round 10, round 100
- **Metric**: Time to insert certificate

### 3. DAG Path Finding (`dag_path_finding`)
Measures reachability queries in the DAG.
- **Tests**: Chain lengths of 10, 50, 100
- **Metric**: Time to check path existence

### 4. Consensus Processing (`consensus_processing`)
Measures full consensus logic including DAG ops, anchor selection, and commit detection.
- **Tests**: 4, 7, 10 validators processing genesis certificates
- **Metric**: Time to process certificate through full pipeline

### 5. Reputation Updates (`reputation_updates`)
Measures performance score calculations for all validators.
- **Tests**: 4, 10, 25, 50 validators
- **Metric**: Time to update all reputation scores

### 6. Leader Selection (`leader_selection`)
Measures weighted random leader selection based on reputation.
- **Tests**: 4, 10, 25, 50, 100 validators
- **Metric**: Time to select next leader

### 7. Transaction Ordering (`transaction_ordering`)
Measures topological sorting of committed certificates.
- **Tests**: 10, 50, 100, 500 certificates
- **Metric**: Time to produce deterministic order

### 8. Worker Batch Formation (`worker_batch_formation`)
Measures transaction collection and batch assembly.
- **Tests**: 10, 100, 1000 transactions
- **Metric**: Time to form batch

### 9. End-to-End Throughput (`end_to_end_throughput`)
Measures full protocol performance over 10 consensus rounds.
- **Tests**: 4, 7 validators
- **Metric**: Total time for 10 rounds with all validators proposing

## Viewing Results

### Terminal Output
Benchmarks display results in the terminal with:
- Mean time with confidence intervals
- Performance regression detection
- Outlier detection
- Throughput measurements

### HTML Reports
Criterion generates interactive HTML reports at:
```
rust/target/criterion/report/index.html
```

Open in browser to view:
- Time series plots
- Performance trends
- Statistical distributions
- Comparison charts

Individual benchmark reports are in:
```
rust/target/criterion/{benchmark_name}/report/index.html
```

### Accessing Reports
```bash
# macOS
open rust/target/criterion/report/index.html

# Linux
xdg-open rust/target/criterion/report/index.html

# Or start a simple HTTP server
cd rust/target/criterion
python3 -m http.server 8000
# Then visit http://localhost:8000/report/
```

## Understanding Results

### Time Measurements
- **Mean**: Average time across all samples
- **Std Dev**: Standard deviation (lower is better)
- **Confidence Interval**: 95% confidence range for the true mean

### Performance Changes
Criterion compares against baseline (previous run):
- **Improved**: Green, negative % change
- **Regressed**: Red, positive % change
- **No change**: Within noise threshold

### Outliers
- **Low severe**: Much faster than typical (rare)
- **Low mild**: Somewhat faster than typical
- **High mild**: Somewhat slower than typical
- **High severe**: Much slower than typical (investigate)

## Interpreting Benchmarks

### Good Performance Indicators
✓ Low variance (<5%)  
✓ No high outliers  
✓ Stable across runs  
✓ Linear scaling with problem size

### Warning Signs
⚠ High variance (>10%)  
⚠ Many high outliers  
⚠ Performance regression  
⚠ Super-linear scaling

## Customizing Benchmarks

### Configuration
Edit `benches/consensus_benchmarks.rs` to:
- Add new benchmark groups
- Modify test parameters (validator counts, transaction sizes)
- Adjust sample sizes
- Change warmup times

### Adding a New Benchmark
```rust
fn bench_my_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_operation");
    
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("operation_name", size),
            size,
            |b, &s| {
                b.iter(|| {
                    // Your code here
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_my_operation);
```

## Optimization Workflow

1. **Baseline**: Run benchmarks to establish baseline
2. **Profile**: Identify bottlenecks using profiling tools
3. **Optimize**: Make code changes
4. **Re-benchmark**: Compare against baseline
5. **Validate**: Ensure correctness with tests
6. **Iterate**: Repeat until targets met

### Profiling Tools
```bash
# Flamegraph profiling
cargo flamegraph --bench consensus_benchmarks

# perf (Linux)
perf record cargo bench --bench consensus_benchmarks
perf report

# Instruments (macOS)
instruments -t "Time Profiler" cargo bench --bench consensus_benchmarks
```

## Performance Targets

Based on current benchmarks, target performance for production:

| Operation | Target | Current (4 validators) |
|-----------|--------|------------------------|
| Certificate Formation | < 1µs | 0.90µs ✓ |
| Consensus Processing | < 10µs | 5.17µs ✓ |
| Leader Selection | < 5µs | 1.04µs ✓ |
| End-to-End Round | < 1ms | 249µs ✓ |

## Continuous Benchmarking

### CI Integration
```yaml
# .github/workflows/benchmark.yml
name: Benchmark
on: [push, pull_request]
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run benchmarks
        run: cargo bench --bench consensus_benchmarks
      - name: Archive results
        uses: actions/upload-artifact@v2
        with:
          name: criterion-results
          path: target/criterion
```

### Regression Detection
Set baseline for comparison:
```bash
# Save current results as baseline
cargo bench --bench consensus_benchmarks -- --save-baseline main

# Compare against baseline
cargo bench --bench consensus_benchmarks -- --baseline main
```

## Troubleshooting

### Unstable Results
- Ensure machine is idle during benchmarks
- Close unnecessary applications
- Disable CPU frequency scaling
- Run with `nice -n -20` for higher priority

### Build Errors
```bash
# Clean and rebuild
cargo clean
cargo build --release
cargo bench --bench consensus_benchmarks
```

### Missing Dependencies
```bash
# Install Criterion
cargo add --dev criterion --features async_tokio

# Update dependencies
cargo update
```

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Performance Benchmarks](./PERFORMANCE_BENCHMARKS.md) - Latest results
- [Shoal Specification](./SHOAL_SPECIFICATION.md) - Protocol details

---

**Last Updated**: October 30, 2025  
**Benchmark Version**: `modal-sequencer-consensus v0.1.0`

