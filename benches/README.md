# Parser Benchmarks

This directory contains comprehensive end-to-end benchmarks for the GQL parser using [Criterion.rs](https://github.com/bheisler/criterion.rs).

## Overview

The benchmark suite covers the following categories:

### 1. **Simple Queries** (`simple_queries`, `where_clauses`)
- Basic MATCH/RETURN patterns
- Single and multiple WHERE conditions
- Common query patterns

### 2. **Complex Queries** (`complex_queries`, `aggregation`)
- Multi-clause queries
- Path patterns and quantified paths
- EXISTS predicates
- UNION operations
- Aggregation functions (COUNT, SUM, AVG, etc.)

### 3. **DDL Operations** (`ddl_operations`)
- CREATE GRAPH statements
- CREATE SCHEMA statements
- INSERT operations
- Schema and catalog operations

### 4. **Stress Tests** (`large_queries`, `deep_nesting`, `wide_patterns`)
- Large queries (100+ return items, 50+ WHERE conditions)
- Deep nesting (up to 30 levels of expressions)
- Wide patterns (many properties, labels)
- Large collections (500+ element lists)

### 5. **Semantic Validation** (`parse_and_validate`)
- Full parsing + validation pipeline
- Valid and invalid queries
- Different query types

### 6. **Real-world Samples** (`sample_corpus`)
- Based on official GQL sample corpus
- Common real-world query patterns

### 7. **Pipeline Stages** (`pipeline_stages`, `lexer_only`)
- Isolated lexer performance
- Parser-only performance
- Full validation pipeline
- Stage-by-stage comparison

### 8. **Throughput** (`throughput`)
- Queries per second measurements
- Different complexity levels

## Running Benchmarks

### Run All Benchmarks
```bash
cargo bench
```

### Run Specific Benchmark Group
```bash
# Simple queries only
cargo bench simple_queries

# Complex queries only
cargo bench complex_queries

# Stress tests only
cargo bench stress_tests

# Parse and validate pipeline
cargo bench parse_and_validate
```

### Run with Specific Filter
```bash
# Run all benchmarks containing "where"
cargo bench -- where

# Run benchmarks matching a pattern
cargo bench -- "complex_queries/(path_pattern|quantified_path)"
```

### Test Benchmarks (Fast Validation)
```bash
# Run benchmarks in test mode (quick validation without statistics)
cargo bench -- --test
```

### Generate HTML Reports
```bash
cargo bench
# Reports are generated in target/criterion/
# Open target/criterion/report/index.html in your browser
```

## Interpreting Results

Criterion provides detailed statistics for each benchmark:

```
simple_queries/minimal   time:   [12.345 µs 12.567 µs 12.789 µs]
                        change: [-2.5% +0.3% +3.1%] (p = 0.45 > 0.05)
                        No change in performance detected.
```

- **time**: Estimated time with confidence interval [lower, estimate, upper]
- **change**: Performance change compared to previous run
- **p-value**: Statistical significance of the change

### Key Metrics

- **Time**: Lower is better (typically in microseconds or milliseconds)
- **Throughput**: Higher is better (queries/second or elements/second)
- **Variance**: Lower variance indicates more consistent performance
- **Outliers**: Severe outliers may indicate performance issues

## Benchmark Configuration

The benchmarks use the following configuration:

- **Sample Size**: 100 iterations for most benchmarks (50 for expensive ones)
- **Warm-up Time**: 3 seconds
- **Measurement Time**: 5 seconds
- **Significance Level**: 0.05
- **Noise Threshold**: 0.01

You can modify these in `parser_benchmarks.rs` by adjusting the Criterion group settings.

## Continuous Performance Monitoring

Release baseline numbers are tracked in `docs/BENCHMARK_BASELINE.md`.

### Baseline Comparison

Save a baseline for comparison:
```bash
cargo bench -- --save-baseline my-baseline
```

Compare against the baseline:
```bash
cargo bench -- --baseline my-baseline
```

### Regression Detection

To detect performance regressions in CI:

1. Save baseline on main branch
2. Run benchmarks on feature branch with `--baseline`
3. Check for significant performance changes

Example CI integration:
```bash
# On main branch
cargo bench -- --save-baseline main

# On feature branch
cargo bench -- --baseline main
```

## Adding New Benchmarks

To add a new benchmark:

1. Create a new benchmark function in `parser_benchmarks.rs`:
   ```rust
   fn bench_my_feature(c: &mut Criterion) {
       let mut group = c.benchmark_group("my_feature");

       group.bench_function("my_test", |b| {
           b.iter(|| parse(black_box("MATCH (n) RETURN n")));
       });

       group.finish();
   }
   ```

2. Add it to the `criterion_group!` macro:
   ```rust
   criterion_group!(
       benches,
       // ... existing benchmarks ...
       bench_my_feature,
   );
   ```

## Performance Guidelines

Based on benchmark results, here are general performance characteristics:

- **Simple queries**: < 50 µs
- **Complex queries**: 50-200 µs
- **Large queries** (100+ elements): 200-1000 µs
- **Deep nesting** (30+ levels): 100-500 µs
- **Parse + validate**: 1.5-2x parser-only time

These are approximate ranges and may vary depending on hardware.

## Troubleshooting

### Benchmarks Take Too Long

Reduce sample size for expensive benchmarks:
```rust
group.sample_size(10);  // Default is 100
```

### Inconsistent Results

Ensure your system is under low load:
- Close other applications
- Disable CPU frequency scaling
- Use `cargo bench -- --noplot` to skip report generation

### Criterion Not Found

Install Criterion dependencies:
```bash
cargo clean
cargo bench
```

## Further Reading

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [GQL Parser Documentation](../README.md)
