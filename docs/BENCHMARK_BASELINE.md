# Benchmark Baseline (0.1.0)

Date: 2026-02-19
Host: local development machine
Rust: stable (edition 2024)

## Commands

```bash
cargo bench simple_queries/minimal
cargo bench where_clauses/single_condition
cargo bench complex_queries/path_pattern
cargo bench parse_and_validate/simple_valid
```

## Baseline Snapshot

These are reference numbers for regression tracking (95% confidence estimate from Criterion):

- `simple_queries/minimal`: `1.0731 us` (`[1.0695 us, 1.0774 us]`)
- `where_clauses/single_condition`: `1.9152 us` (`[1.9055 us, 1.9283 us]`)
- `complex_queries/path_pattern`: `3.8487 us` (`[3.8394 us, 3.8584 us]`)
- `parse_and_validate/simple_valid`: `2.1718 us` (`[2.1668 us, 2.1769 us]`)

Use these as coarse regression thresholds. Re-baseline when parser architecture or semantic passes change materially.
