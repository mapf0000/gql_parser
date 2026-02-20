//! Comprehensive tests for Aggregation and GROUP BY Validation (VAL_TESTS.md Section 5)
//!
//! This test suite covers:
//! - A. GROUP BY Semantics
//! - B. HAVING Clause validation
//! - C. Nested Aggregation Detection
//! - D. Aggregation Context validation
//! - E. Aggregate Function Variations

use gql_parser::diag::DiagSeverity;
use gql_parser::parse;
use gql_parser::semantic::validator::{SemanticValidator, ValidationConfig};

// ==================== A. GROUP BY Semantics ====================

#[test]
fn test_groupby_missing_non_aggregated_in_select_error() {
    // Non-aggregated expressions in SELECT must be in GROUP BY
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age)";
    let config = ValidationConfig {
        strict_mode: true,
        ..Default::default()
    };
    let validator = SemanticValidator::with_config(config);
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should fail: n.country is not aggregated and not in GROUP BY
        assert!(
            !outcome.is_success(),
            "Should fail: n.country not in GROUP BY"
        );

        let has_groupby_error = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Error
                && (d.message.contains("GROUP BY") || d.message.contains("aggregat"))
        });
        assert!(
            has_groupby_error,
            "Should have GROUP BY related error. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_groupby_valid_single_column() {
    // Valid GROUP BY with single column
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) GROUP BY n.country";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed: n.country is in GROUP BY
        assert!(
            outcome.is_success(),
            "Should succeed: valid GROUP BY. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_groupby_valid_multiple_columns() {
    // Valid GROUP BY with multiple columns
    let source = "MATCH (n:Person) SELECT n.country, n.city, AVG(n.age) GROUP BY n.country, n.city";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: all non-aggregated columns in GROUP BY. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_groupby_missing_column_error() {
    // Missing one column from GROUP BY
    let source = "MATCH (n:Person) SELECT n.country, n.city, AVG(n.age) GROUP BY n.country";
    let config = ValidationConfig {
        strict_mode: true,
        ..Default::default()
    };
    let validator = SemanticValidator::with_config(config);
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should fail: n.city not in GROUP BY
        assert!(
            !outcome.is_success(),
            "Should fail: n.city not in GROUP BY"
        );
    }
}

#[test]
fn test_groupby_with_expression() {
    // GROUP BY with expression (not just simple property reference)
    let source = "MATCH (n:Person) SELECT n.age / 10, COUNT(*) GROUP BY n.age / 10";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed: expression in SELECT matches GROUP BY
        assert!(
            outcome.is_success(),
            "Should succeed: expression in GROUP BY. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_groupby_only_aggregates_no_groupby_needed() {
    // When SELECT contains only aggregates, GROUP BY not required
    let source = "MATCH (n:Person) SELECT AVG(n.age), COUNT(*)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: only aggregates, no GROUP BY needed. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ==================== B. HAVING Clause ====================

#[test]
fn test_having_with_groupby_and_aggregate() {
    // Valid HAVING with GROUP BY and aggregate condition
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) GROUP BY n.country HAVING AVG(n.age) > 30";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: HAVING with aggregate. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_having_with_count_condition() {
    // HAVING with COUNT condition
    let source = "MATCH (n:Person) SELECT n.country, COUNT(*) GROUP BY n.country HAVING COUNT(*) > 5";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: HAVING with COUNT. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_having_non_aggregated_not_in_groupby_error() {
    // HAVING with non-aggregated expression not in GROUP BY should error
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) GROUP BY n.country HAVING n.city = 'NYC'";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should fail: n.city not in GROUP BY
        assert!(
            !outcome.is_success(),
            "Should fail: n.city not in GROUP BY but used in HAVING"
        );

        let has_error = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Error
                && (d.message.contains("GROUP BY") || d.message.contains("HAVING"))
        });
        assert!(
            has_error,
            "Should have GROUP BY/HAVING related error"
        );
    }
}

#[test]
fn test_having_with_grouped_column() {
    // HAVING with grouped column is valid
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) GROUP BY n.country HAVING n.country = 'USA'";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: HAVING uses grouped column. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_having_without_groupby_only_aggregates() {
    // HAVING without GROUP BY but with only aggregates is valid
    let source = "MATCH (n:Person) SELECT COUNT(*) HAVING COUNT(*) > 5";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: HAVING with aggregate, no GROUP BY needed. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_having_multiple_conditions() {
    // HAVING with multiple conditions combined with AND/OR
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age), COUNT(*) GROUP BY n.country HAVING AVG(n.age) > 30 AND COUNT(*) > 10";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: HAVING with multiple aggregate conditions. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ==================== C. Nested Aggregation Detection ====================

#[test]
fn test_nested_aggregation_count_sum_error() {
    // COUNT(SUM(x)) - nested aggregation should error
    let source = "MATCH (n:Person) RETURN COUNT(SUM(n.age))";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: nested aggregation COUNT(SUM(...))"
        );

        let has_nested_error = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Error
                && (d.message.contains("Nested aggregation") || d.message.contains("nested aggregation"))
        });
        assert!(
            has_nested_error,
            "Should have nested aggregation error. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_nested_aggregation_avg_max_error() {
    // AVG(MAX(y)) - nested aggregation should error
    let source = "MATCH (n:Person) RETURN AVG(MAX(n.salary))";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: nested aggregation AVG(MAX(...))"
        );

        let has_nested_error = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Error
                && (d.message.contains("Nested aggregation") || d.message.contains("nested aggregation"))
        });
        assert!(has_nested_error, "Should have nested aggregation error");
    }
}

#[test]
fn test_nested_aggregation_min_count_error() {
    // MIN(COUNT(*)) - nested aggregation should error
    let source = "MATCH (n:Person) RETURN MIN(COUNT(*))";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: nested aggregation MIN(COUNT(...))"
        );
    }
}

#[test]
fn test_multiple_aggregates_same_level_valid() {
    // AVG(x) + SUM(y) - multiple aggregates at same level is valid
    let source = "MATCH (n:Person) RETURN AVG(n.age) + SUM(n.salary)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: multiple aggregates at same level. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_aggregate_with_arithmetic_valid() {
    // AVG(n.age) * 2 - arithmetic on aggregate result is valid
    let source = "MATCH (n:Person) RETURN AVG(n.age) * 2";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: arithmetic on aggregate. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ==================== D. Aggregation Context ====================

#[test]
fn test_aggregate_in_where_error() {
    // Aggregates not allowed in WHERE clause - must use HAVING
    let source = "MATCH (n:Person) WHERE COUNT(*) > 5 RETURN n";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: aggregate in WHERE clause"
        );

        let has_where_error = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Error
                && (d.message.contains("WHERE")
                    || d.message.contains("HAVING")
                    || d.message.contains("aggregat"))
        });
        assert!(
            has_where_error,
            "Should suggest HAVING instead of WHERE. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_aggregate_in_filter_error() {
    // Aggregates not allowed in FILTER clause
    let source = "MATCH (n:Person) FILTER AVG(n.age) > 30 RETURN n";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: aggregate in FILTER clause"
        );
    }
}

#[test]
fn test_aggregate_in_select_valid() {
    // Aggregates allowed in SELECT/RETURN
    let source = "MATCH (n:Person) SELECT COUNT(*), AVG(n.age), MAX(n.salary)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: aggregates in SELECT. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_aggregate_in_orderby_with_groupby_valid() {
    // Aggregates allowed in ORDER BY when used with GROUP BY
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) GROUP BY n.country ORDER BY AVG(n.age) DESC";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: aggregate in ORDER BY with GROUP BY. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_aggregate_in_groupby_error() {
    // Aggregates not allowed in GROUP BY clause
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) GROUP BY AVG(n.age)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should fail: can't use aggregate in GROUP BY
        assert!(
            !outcome.is_success(),
            "Should fail: aggregate in GROUP BY clause"
        );

        let has_groupby_error = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Error
                && (d.message.contains("GROUP BY") || d.message.contains("aggregat"))
        });
        assert!(
            has_groupby_error,
            "Should have GROUP BY related error. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ==================== E. Aggregate Function Variations ====================

#[test]
fn test_count_star() {
    // COUNT(*) - count all rows
    let source = "MATCH (n:Person) RETURN COUNT(*)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: COUNT(*). Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_count_expression() {
    // COUNT(expr) - count non-null values
    let source = "MATCH (n:Person) RETURN COUNT(n.age)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: COUNT(expr). Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_count_distinct() {
    // COUNT(DISTINCT expr) - count distinct values
    let source = "MATCH (n:Person) RETURN COUNT(DISTINCT n.country)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: COUNT(DISTINCT ...). Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_statistical_aggregates_stddev_samp() {
    // STDDEV_SAMP - sample standard deviation
    let source = "MATCH (n:Person) RETURN STDDEV_SAMP(n.age)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: STDDEV_SAMP. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_statistical_aggregates_stddev_pop() {
    // STDDEV_POP - population standard deviation
    let source = "MATCH (n:Person) RETURN STDDEV_POP(n.salary)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: STDDEV_POP. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_percentile_cont() {
    // PERCENTILE_CONT - continuous percentile
    let source = "MATCH (n:Person) RETURN PERCENTILE_CONT(n.age, 0.5)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // May or may not be implemented yet
        if !outcome.is_success() {
            let has_undefined_error = outcome.diagnostics.iter().any(|d| {
                d.severity == DiagSeverity::Error
                    && (d.message.contains("PERCENTILE_CONT") || d.message.contains("Undefined"))
            });
            if !has_undefined_error {
                panic!("Expected either success or 'undefined function' error. Diagnostics: {:?}",
                    outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
            }
        }
    }
}

#[test]
fn test_percentile_disc() {
    // PERCENTILE_DISC - discrete percentile
    let source = "MATCH (n:Person) RETURN PERCENTILE_DISC(n.age, 0.5)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // May or may not be implemented yet
        if !outcome.is_success() {
            let has_undefined_error = outcome.diagnostics.iter().any(|d| {
                d.severity == DiagSeverity::Error
                    && (d.message.contains("PERCENTILE_DISC") || d.message.contains("Undefined"))
            });
            if !has_undefined_error {
                panic!("Expected either success or 'undefined function' error. Diagnostics: {:?}",
                    outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
            }
        }
    }
}

#[test]
fn test_collect_list_aggregation() {
    // COLLECT or COLLECT_LIST for list aggregation
    let source = "MATCH (n:Person) RETURN COLLECT(n.name)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: COLLECT. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_sum_distinct() {
    // SUM(DISTINCT expr) - sum of distinct values
    let source = "MATCH (n:Person) RETURN SUM(DISTINCT n.salary)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: SUM(DISTINCT ...). Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_avg_distinct() {
    // AVG(DISTINCT expr) - average of distinct values
    let source = "MATCH (n:Person) RETURN AVG(DISTINCT n.age)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: AVG(DISTINCT ...). Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_multiple_distinct_aggregates() {
    // Multiple DISTINCT aggregates in same query
    let source = "MATCH (n:Person) SELECT COUNT(DISTINCT n.country), AVG(DISTINCT n.age)";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: multiple DISTINCT aggregates. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ==================== Additional Edge Cases ====================

#[test]
fn test_groupby_with_having_and_orderby() {
    // Complex query with GROUP BY, HAVING, and ORDER BY
    let source = "MATCH (n:Person) SELECT n.country, AVG(n.age) AS avg_age GROUP BY n.country HAVING AVG(n.age) > 30 ORDER BY avg_age DESC";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: complex GROUP BY with HAVING and ORDER BY. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_empty_group_handling() {
    // Query that might produce empty groups
    let source = "MATCH (n:Person) WHERE n.age > 100 SELECT n.country, COUNT(*) GROUP BY n.country";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: empty groups are valid. Diagnostics: {:?}",
            outcome.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
