//! Tests for aggregate function parsing (Sprint 9).

use gql_parser::ast::{
    AggregateFunction, BinarySetFunctionType, Expression, GeneralSetFunctionType, SetQuantifier,
};
use gql_parser::lexer::Lexer;
use gql_parser::parser::expression::parse_expression;

#[test]
fn test_count_star() {
    let source = "COUNT(*)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "COUNT(*) should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::CountStar { .. } => {
                // Success!
            }
            _ => panic!("Expected CountStar, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_count_expr() {
    let source = "COUNT(n.id)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "COUNT(expr) should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Count);
                assert!(gsf.quantifier.is_none());
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_count_distinct() {
    let source = "COUNT(DISTINCT n.id)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(
        result.is_ok(),
        "COUNT(DISTINCT expr) should parse successfully"
    );
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Count);
                assert_eq!(gsf.quantifier, Some(SetQuantifier::Distinct));
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_count_all() {
    let source = "COUNT(ALL n.id)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "COUNT(ALL expr) should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Count);
                assert_eq!(gsf.quantifier, Some(SetQuantifier::All));
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_avg_function() {
    let source = "AVG(n.age)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "AVG should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Avg);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_max_function() {
    let source = "MAX(n.salary)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "MAX should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Max);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_min_function() {
    let source = "MIN(n.salary)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "MIN should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Min);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_sum_function() {
    let source = "SUM(n.amount)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "SUM should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Sum);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_collect_list_function() {
    let source = "COLLECT_LIST(n.name)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "COLLECT_LIST should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::CollectList);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_stddev_samp_function() {
    let source = "STDDEV_SAMP(n.amount)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    if let Err(e) = &result {
        panic!("STDDEV_SAMP parsing failed: {:?}", e);
    }
    assert!(result.is_ok(), "STDDEV_SAMP should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::StddevSamp);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_stddev_pop_function() {
    let source = "STDDEV_POP(n.amount)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "STDDEV_POP should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::StddevPop);
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_percentile_cont_function() {
    let source = "PERCENTILE_CONT(0.5, n.age)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "PERCENTILE_CONT should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::BinarySetFunction(bsf) => {
                assert_eq!(bsf.function_type, BinarySetFunctionType::PercentileCont);
                assert_eq!(bsf.quantifier, None);
            }
            _ => panic!("Expected BinarySetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_percentile_disc_function() {
    let source = "PERCENTILE_DISC(0.95, n.age)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "PERCENTILE_DISC should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::BinarySetFunction(bsf) => {
                assert_eq!(bsf.function_type, BinarySetFunctionType::PercentileDisc);
                assert_eq!(bsf.quantifier, None);
            }
            _ => panic!("Expected BinarySetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_percentile_disc_distinct_function() {
    let source = "PERCENTILE_DISC(DISTINCT 0.95, n.age)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(
        result.is_ok(),
        "PERCENTILE_DISC(DISTINCT ...) should parse successfully"
    );
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::BinarySetFunction(bsf) => {
                assert_eq!(bsf.function_type, BinarySetFunctionType::PercentileDisc);
                assert_eq!(bsf.quantifier, Some(SetQuantifier::Distinct));
            }
            _ => panic!("Expected BinarySetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_percentile_single_arg_rejected() {
    let source = "PERCENTILE_CONT(0.5)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(
        result.is_err(),
        "single-argument percentile should be rejected"
    );
}

#[test]
fn test_percentile_within_group_rejected() {
    let source = "PERCENTILE_CONT(0.5, n.age) WITHIN GROUP (ORDER BY n.age)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(
        result.is_err(),
        "WITHIN GROUP percentile syntax is not in the OpenGQL grammar snapshot"
    );
}

#[test]
fn test_avg_with_distinct() {
    let source = "AVG(DISTINCT n.price)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(result.is_ok(), "AVG(DISTINCT) should parse successfully");
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Avg);
                assert_eq!(gsf.quantifier, Some(SetQuantifier::Distinct));
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}

#[test]
fn test_aggregate_with_complex_expression() {
    let source = "SUM(n.quantity * n.price)";
    let lexer_result = Lexer::new(source).tokenize();
    let result = parse_expression(&lexer_result.tokens);

    assert!(
        result.is_ok(),
        "Aggregate with complex expression should parse successfully"
    );
    let expr = result.unwrap();

    match expr {
        Expression::AggregateFunction(agg) => match *agg {
            AggregateFunction::GeneralSetFunction(gsf) => {
                assert_eq!(gsf.function_type, GeneralSetFunctionType::Sum);
                // The inner expression should be a binary multiplication
                match *gsf.expression {
                    Expression::Binary(..) => {} // Success!
                    _ => panic!("Expected Binary expression inside SUM"),
                }
            }
            _ => panic!("Expected GeneralSetFunction, got {:?}", agg),
        },
        _ => panic!("Expected AggregateFunction, got {:?}", expr),
    }
}
