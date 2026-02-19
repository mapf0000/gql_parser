//! End-to-End Parser Benchmarks
//!
//! This benchmark suite measures the performance of the GQL parser across various
//! query types and complexities. Benchmarks are organized into the following categories:
//!
//! - **Simple Queries**: Basic MATCH and RETURN statements
//! - **Complex Queries**: Queries with WHERE, ORDER BY, LIMIT, and multiple clauses
//! - **DDL Operations**: CREATE, INSERT, and schema operations
//! - **Stress Tests**: Large queries, deep nesting, and wide queries
//! - **Semantic Validation**: Full parsing and validation pipeline
//! - **Real-world Samples**: Official GQL sample corpus
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench
//!
//! # Run specific benchmark group
//! cargo bench simple_queries
//! cargo bench complex_queries
//! cargo bench stress_tests
//!
//! # Generate HTML reports
//! cargo bench --features html_reports
//! ```
//!
//! ## Interpreting Results
//!
//! - **Time**: Lower is better (microseconds or milliseconds)
//! - **Throughput**: Higher is better (queries/second)
//! - **Stability**: Lower variance indicates more consistent performance

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use gql_parser::{parse, parse_and_validate};

// ============================================================================
// Simple Query Benchmarks
// ============================================================================

fn bench_simple_match_return(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_queries");

    let queries = vec![
        ("minimal", "MATCH (n) RETURN n"),
        ("with_label", "MATCH (n:Person) RETURN n"),
        ("with_property", "MATCH (n {name: 'Alice'}) RETURN n"),
        (
            "with_label_property",
            "MATCH (n:Person {age: 30}) RETURN n.name",
        ),
        ("edge_pattern", "MATCH (a)-[r]->(b) RETURN a, r, b"),
        (
            "labeled_edge",
            "MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a.name, b.name",
        ),
    ];

    for (name, query) in queries {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

fn bench_simple_where_clause(c: &mut Criterion) {
    let mut group = c.benchmark_group("where_clauses");

    let queries = vec![
        (
            "single_condition",
            "MATCH (n:Person) WHERE n.age > 18 RETURN n",
        ),
        (
            "and_conditions",
            "MATCH (n:Person) WHERE n.age > 18 AND n.age < 65 RETURN n",
        ),
        (
            "or_conditions",
            "MATCH (n:Person) WHERE n.age < 18 OR n.age > 65 RETURN n",
        ),
        (
            "complex_boolean",
            "MATCH (n) WHERE (n.a > 10 AND n.b < 20) OR (n.c = 30) RETURN n",
        ),
        (
            "in_predicate",
            "MATCH (n) WHERE n.id IN [1, 2, 3, 4, 5] RETURN n",
        ),
        (
            "string_comparison",
            "MATCH (n) WHERE n.name = 'Alice' AND n.city = 'NYC' RETURN n",
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// Complex Query Benchmarks
// ============================================================================

fn bench_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_queries");

    let queries = vec![
        (
            "multi_match",
            "MATCH (a:Person) MATCH (b:Company) WHERE a.age > 25 RETURN a, b",
        ),
        (
            "with_order_limit",
            "MATCH (n:Person) WHERE n.age > 18 RETURN n.name ORDER BY n.age DESC LIMIT 10",
        ),
        (
            "path_pattern",
            "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:WORKS_AT]->(c:Company) WHERE a.age > 25 RETURN a, b, c",
        ),
        (
            "quantified_path",
            "MATCH (a:Person)-[:KNOWS]->{1,5}(b:Person) RETURN a.name, b.name",
        ),
        (
            "exists_predicate",
            "MATCH (p:Person) WHERE EXISTS { MATCH (p)-[:KNOWS]->(f:Person) WHERE f.age > 30 } RETURN p",
        ),
        (
            "union_query",
            "MATCH (n:Person) WHERE n.age > 50 RETURN n.name UNION MATCH (m:Person) WHERE m.age < 25 RETURN m.name",
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

fn bench_aggregation_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregation");

    let queries = vec![
        ("count", "MATCH (n:Person) RETURN COUNT(n)"),
        ("sum_avg", "MATCH (n:Person) RETURN SUM(n.age), AVG(n.age)"),
        (
            "group_by",
            "MATCH (n:Person) RETURN n.city, COUNT(n) ORDER BY COUNT(n) DESC",
        ),
        (
            "multiple_agg",
            "MATCH (p:Person) RETURN p.dept, COUNT(p), AVG(p.salary), MIN(p.age), MAX(p.age)",
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// DDL and Mutation Benchmarks
// ============================================================================

fn bench_ddl_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddl_operations");

    let queries = vec![
        ("create_graph", "CREATE GRAPH mySocialNetwork"),
        (
            "create_graph_typed",
            "CREATE GRAPH mySocialNetwork TYPED socialNetworkGraphType",
        ),
        (
            "create_schema",
            "CREATE SCHEMA financialSchema NEXT CREATE GRAPH MATCH (:Account)-[:TRANSFER]->(:Account)",
        ),
        (
            "insert_node",
            "INSERT (n:Person {name: 'Alice', age: 30, email: 'alice@example.com'})",
        ),
        (
            "insert_edge",
            "INSERT (a:Person {name: 'Alice'})-[:KNOWS {since: DATE '2020-01-01'}]->(b:Person {name: 'Bob'})",
        ),
        (
            "match_insert",
            "MATCH (a:Person {name: 'Alice'}) INSERT (a)-[:KNOWS]->(b:Person {name: 'Charlie'})",
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// Stress Test Benchmarks
// ============================================================================

fn bench_large_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_queries");
    group.sample_size(50); // Reduce sample size for expensive benchmarks

    // Large RETURN list
    let mut large_return = String::from("MATCH (n) RETURN ");
    for i in 0..100 {
        if i > 0 {
            large_return.push_str(", ");
        }
        large_return.push_str(&format!("n.prop{}", i));
    }

    group.bench_function("100_return_items", |b| {
        b.iter(|| parse(black_box(&large_return)));
    });

    // Many WHERE conditions
    let mut many_conditions = String::from("MATCH (n) WHERE ");
    for i in 0..50 {
        if i > 0 {
            many_conditions.push_str(" AND ");
        }
        many_conditions.push_str(&format!("n.prop{} > {}", i, i));
    }
    many_conditions.push_str(" RETURN n");

    group.bench_function("50_where_conditions", |b| {
        b.iter(|| parse(black_box(&many_conditions)));
    });

    // Multiple MATCH clauses
    let mut many_matches = String::new();
    for i in 0..50 {
        many_matches.push_str(&format!("MATCH (n{}) ", i));
    }
    many_matches.push_str("RETURN 1");

    group.bench_function("50_match_clauses", |b| {
        b.iter(|| parse(black_box(&many_matches)));
    });

    // Large IN list
    let mut large_in = String::from("MATCH (n) WHERE n.id IN [");
    for i in 0..500 {
        if i > 0 {
            large_in.push_str(", ");
        }
        large_in.push_str(&i.to_string());
    }
    large_in.push_str("] RETURN n");

    group.bench_function("500_element_in_list", |b| {
        b.iter(|| parse(black_box(&large_in)));
    });

    group.finish();
}

fn bench_deep_nesting(c: &mut Criterion) {
    let mut group = c.benchmark_group("deep_nesting");
    group.sample_size(50);

    // Deep expression nesting
    for depth in [5, 10, 20, 30].iter() {
        let mut query = String::from("MATCH (n) WHERE ");
        let mut expr = String::from("n.value");

        for i in 0..*depth {
            expr = format!("({} + {})", expr, i);
        }

        query.push_str(&expr);
        query.push_str(" RETURN n");

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_levels", depth)),
            &query,
            |b, q| {
                b.iter(|| parse(black_box(q)));
            },
        );
    }

    group.finish();
}

fn bench_wide_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_patterns");

    // Pattern with many properties
    let mut many_props = String::from("MATCH (n:Person {");
    for i in 0..50 {
        if i > 0 {
            many_props.push_str(", ");
        }
        many_props.push_str(&format!("prop{}: {}", i, i));
    }
    many_props.push_str("}) RETURN n");

    group.bench_function("50_node_properties", |b| {
        b.iter(|| parse(black_box(&many_props)));
    });

    // Pattern with many labels
    let mut many_labels = String::from("MATCH (n:");
    for i in 0..20 {
        if i > 0 {
            many_labels.push('&');
        }
        many_labels.push_str(&format!("Label{}", i));
    }
    many_labels.push_str(") RETURN n");

    group.bench_function("20_node_labels", |b| {
        b.iter(|| parse(black_box(&many_labels)));
    });

    group.finish();
}

// ============================================================================
// Semantic Validation Benchmarks
// ============================================================================

fn bench_parse_and_validate(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_and_validate");

    let queries = vec![
        ("simple_valid", "MATCH (n:Person) RETURN n"),
        (
            "complex_valid",
            "MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a.age > 25 AND b.age < 65 RETURN a.name, b.name",
        ),
        (
            "with_aggregation",
            "MATCH (n:Person) RETURN n.city, COUNT(n) ORDER BY COUNT(n) DESC",
        ),
        ("undefined_variable", "MATCH (n:Person) RETURN m"), // Will fail validation
        ("insert_query", "INSERT (n:Person {name: 'Alice', age: 30})"),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse_and_validate(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// Real-world Sample Benchmarks
// ============================================================================

fn bench_sample_corpus(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample_corpus");

    // These are based on the official GQL samples
    let samples = vec![
        (
            "create_graph",
            "CREATE GRAPH mySocialNetwork TYPED socialNetworkGraphType",
        ),
        ("session_set", "SESSION SET GRAPH CURRENT_GRAPH"),
        (
            "match_insert",
            r#"MATCH (a:Person {name: 'Alice'})
               INSERT (a)-[:KNOWS]->(b:Person {name: 'Bob', joined: DATE '2024-01-15'})"#,
        ),
        (
            "exists_predicate",
            r#"MATCH (p:Person)
               WHERE EXISTS { MATCH (p)-[:KNOWS]->(f) WHERE f.active = true }
               RETURN p.name"#,
        ),
    ];

    for (name, query) in samples {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// Lexer-only Benchmarks
// ============================================================================

fn bench_lexer_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_only");

    use gql_parser::tokenize;

    let queries = vec![
        ("simple", "MATCH (n:Person) WHERE n.age > 18 RETURN n"),
        (
            "complex",
            "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:WORKS_AT]->(c:Company) WHERE a.age > 25 AND b.salary > 50000 RETURN a, b, c",
        ),
        (
            "keywords_heavy",
            "CREATE GRAPH MATCH INSERT WHERE RETURN ORDER BY LIMIT UNION OPTIONAL EXISTS",
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| tokenize(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    let queries = vec![
        ("simple_match", "MATCH (n:Person) RETURN n"),
        ("with_where", "MATCH (n:Person) WHERE n.age > 18 RETURN n"),
        (
            "complex_path",
            "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:WORKS_AT]->(c:Company) RETURN a, b, c",
        ),
    ];

    // Measure throughput (queries per second)
    for (name, query) in queries {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, q| {
            b.iter(|| parse(black_box(q)));
        });
    }

    group.finish();
}

// ============================================================================
// Comparison Benchmarks (Lexer vs Parser vs Validation)
// ============================================================================

fn bench_pipeline_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_stages");

    use gql_parser::tokenize;

    let query = "MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a.age > 25 RETURN a.name, b.name";

    group.bench_function("01_lexer_only", |b| {
        b.iter(|| tokenize(black_box(query)));
    });

    group.bench_function("02_parse_only", |b| {
        b.iter(|| parse(black_box(query)));
    });

    group.bench_function("03_parse_and_validate", |b| {
        b.iter(|| parse_and_validate(black_box(query)));
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_simple_match_return,
    bench_simple_where_clause,
    bench_complex_queries,
    bench_aggregation_queries,
    bench_ddl_operations,
    bench_large_queries,
    bench_deep_nesting,
    bench_wide_patterns,
    bench_parse_and_validate,
    bench_sample_corpus,
    bench_lexer_only,
    bench_throughput,
    bench_pipeline_stages,
);

criterion_main!(benches);
