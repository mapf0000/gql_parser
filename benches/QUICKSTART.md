# Benchmark Quick Start Guide

## TL;DR

```bash
# Run all benchmarks
cargo bench

# Run specific category
cargo bench simple_queries
cargo bench complex_queries
cargo bench stress_tests

# Quick validation
cargo bench -- --test

# Using the helper script
./benches/run_benchmarks.sh --simple --open
```

## What Gets Benchmarked

### Core Performance Metrics
- **Lexer**: Tokenization speed (1-5 µs for simple queries)
- **Parser**: AST construction (10-50 µs for simple queries)
- **Validator**: Semantic validation (1.5-2x parser time)
- **End-to-End**: Full pipeline (lex + parse + validate)

### Query Categories

| Category | Examples | Typical Performance |
|----------|----------|---------------------|
| Simple Queries | `MATCH (n) RETURN n` | 1-5 µs |
| WHERE Clauses | `WHERE n.age > 18 AND n.city = 'NYC'` | 5-20 µs |
| Complex Patterns | Quantified paths, EXISTS predicates | 20-100 µs |
| Aggregations | `COUNT`, `SUM`, `AVG`, `GROUP BY` | 10-50 µs |
| DDL Operations | `CREATE GRAPH`, `INSERT` | 20-100 µs |
| Large Queries | 100+ return items, 50+ conditions | 200-1000 µs |
| Deep Nesting | 30+ expression levels | 100-500 µs |

### Benchmark Groups

```
simple_queries/          Basic MATCH/RETURN patterns (6 benchmarks)
where_clauses/           WHERE conditions (6 benchmarks)
complex_queries/         Multi-clause queries (6 benchmarks)
aggregation/             COUNT, SUM, AVG, GROUP BY (4 benchmarks)
ddl_operations/          CREATE, INSERT statements (6 benchmarks)
large_queries/           100+ elements (4 benchmarks)
deep_nesting/            Expression nesting (4 benchmarks)
wide_patterns/           Many properties/labels (2 benchmarks)
parse_and_validate/      Full validation pipeline (5 benchmarks)
sample_corpus/           Real-world examples (4 benchmarks)
lexer_only/              Tokenization only (3 benchmarks)
throughput/              Queries per second (3 benchmarks)
pipeline_stages/         Stage-by-stage comparison (3 benchmarks)
```

Total: **56 individual benchmarks**

## Reading Results

```
simple_queries/minimal   time:   [1.0168 µs 1.0205 µs 1.0214 µs]
                        thrpt:  [979.01 Kelem/s 979.90 Kelem/s 983.47 Kelem/s]
```

- **time**: [lower bound, estimate, upper bound] - 95% confidence interval
- **thrpt**: Throughput (elements/queries per second)
- Lower time = better performance
- Higher throughput = better performance

### Performance Changes

```
simple_queries/minimal   time:   [1.0168 µs 1.0205 µs 1.0214 µs]
                        change: [-5.2% -3.1% -1.0%] (p = 0.002 < 0.05)
                        Performance has improved.
```

- **change**: [lower, estimate, upper] percentage change vs baseline
- **p-value**: Statistical significance (< 0.05 = significant)
- Negative change = faster (improvement)
- Positive change = slower (regression)

## Common Use Cases

### 1. Local Development
```bash
# Quick validation before commit
cargo bench -- --test

# Full benchmark run
cargo bench
```

### 2. Performance Optimization
```bash
# Save baseline before changes
cargo bench -- --save-baseline before

# Make your changes...

# Compare after changes
cargo bench -- --baseline before
```

### 3. CI/CD Integration
```bash
# In CI pipeline
cargo bench --no-fail-fast -- --test

# Or for detailed comparison
cargo bench -- --baseline main
```

### 4. Profiling Specific Queries
```bash
# Benchmark specific pattern
cargo bench -- "complex_queries/path_pattern"

# Multiple patterns
cargo bench -- "(simple|complex)_queries"
```

## Performance Targets

Based on benchmark results, here are reasonable performance expectations:

| Operation | Target | Excellent | Needs Investigation |
|-----------|--------|-----------|---------------------|
| Simple query | < 50 µs | < 10 µs | > 100 µs |
| Complex query | < 200 µs | < 100 µs | > 500 µs |
| Large query (100+ items) | < 1 ms | < 500 µs | > 2 ms |
| Parse + validate | < 300 µs | < 150 µs | > 1 ms |
| Throughput | > 10K qps | > 50K qps | < 5K qps |

## Optimization Tips

If benchmarks show performance issues:

1. **Profile the code**: Use `cargo flamegraph` or `perf`
2. **Check allocations**: Use `cargo-instruments` or `valgrind`
3. **Review algorithms**: Look for O(n²) operations
4. **Benchmark incrementally**: Test each change in isolation
5. **Compare baselines**: Always measure before and after

## Advanced Usage

### Custom Benchmark Configuration
Edit `benches/parser_benchmarks.rs`:

```rust
fn bench_my_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_group");

    // Adjust sample size (default: 100)
    group.sample_size(50);

    // Adjust measurement time (default: 5s)
    group.measurement_time(std::time::Duration::from_secs(10));

    // Your benchmarks...

    group.finish();
}
```

### Benchmark Specific Files
```rust
use std::fs;

fn bench_real_query_file(c: &mut Criterion) {
    let query = fs::read_to_string("queries/my_query.gql").unwrap();

    c.bench_function("my_real_query", |b| {
        b.iter(|| parse(black_box(&query)));
    });
}
```

## Continuous Monitoring

### GitHub Actions Example
```yaml
name: Benchmarks

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run benchmarks
        run: cargo bench -- --test
      - name: Compare with main
        if: github.ref != 'refs/heads/main'
        run: |
          git fetch origin main
          git checkout origin/main
          cargo bench -- --save-baseline main
          git checkout -
          cargo bench -- --baseline main
```

## Troubleshooting

### Issue: Benchmarks take too long
**Solution**: Use `--quick` flag or reduce sample size

### Issue: Results are inconsistent
**Solution**:
- Close other applications
- Run multiple times: `cargo bench -- --sample-size 200`
- Check system load: `top` or `htop`

### Issue: Out of memory
**Solution**: Run benchmark groups separately:
```bash
cargo bench simple_queries
cargo bench complex_queries
# etc.
```

## Resources

- Full documentation: [benches/README.md](README.md)
- Criterion.rs guide: https://bheisler.github.io/criterion.rs/book/
- Rust performance book: https://nnethercote.github.io/perf-book/
