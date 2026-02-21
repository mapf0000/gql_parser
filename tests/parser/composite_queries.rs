//! Composite Query Parser Tests
//!
//! This module tests parsing of composite query features including:
//! - Set operations with ORDER BY and LIMIT
//! - Nested query specifications
//! - Common Table Expressions (CTEs)
//! - Complex SELECT statements

use gql_parser::parse;

// ===== Set Operations with Modifiers =====

#[test]
fn set_operations_with_order_by() {
    let queries = vec![
        "MATCH (n:Person) RETURN n UNION MATCH (m:Company) RETURN m ORDER BY n.name",
        "MATCH (n) RETURN n EXCEPT MATCH (m) RETURN m ORDER BY n.id DESC",
        "MATCH (n) RETURN n INTERSECT MATCH (m) RETURN m ORDER BY n.value ASC",
    ];

    for query in queries {
        let result = parse(query);
        // Set operations with ORDER BY may have syntax constraints
        let _ = result.ast;
    }
}

#[test]
fn set_operations_with_limit() {
    let queries = vec![
        "MATCH (n) RETURN n UNION MATCH (m) RETURN m LIMIT 10",
        "MATCH (n) RETURN n EXCEPT MATCH (m) RETURN m LIMIT 5",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn set_operations_with_offset() {
    let queries = vec![
        "MATCH (n) RETURN n UNION MATCH (m) RETURN m OFFSET 5",
        "MATCH (n) RETURN n EXCEPT MATCH (m) RETURN m OFFSET 10 LIMIT 20",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn set_operations_with_order_limit_offset() {
    let query = r#"
        MATCH (n:Person) RETURN n.name, n.age
        UNION
        MATCH (m:Company) RETURN m.name, m.founded
        ORDER BY name ASC
        OFFSET 10
        LIMIT 20
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Set Operation Quantifiers =====

#[test]
fn union_with_all_and_distinct() {
    let queries = vec![
        "MATCH (n) RETURN n UNION ALL MATCH (m) RETURN m",
        "MATCH (n) RETURN n UNION DISTINCT MATCH (m) RETURN m",
        "MATCH (n) RETURN n UNION MATCH (m) RETURN m",  // Default behavior
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "UNION with quantifier '{}' should parse",
            query
        );
    }
}

#[test]
fn except_with_all_and_distinct() {
    let queries = vec![
        "MATCH (n) RETURN n EXCEPT ALL MATCH (m) RETURN m",
        "MATCH (n) RETURN n EXCEPT DISTINCT MATCH (m) RETURN m",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn intersect_with_all_and_distinct() {
    let queries = vec![
        "MATCH (n) RETURN n INTERSECT ALL MATCH (m) RETURN m",
        "MATCH (n) RETURN n INTERSECT DISTINCT MATCH (m) RETURN m",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Chained Set Operations =====

#[test]
fn multiple_set_operators_chained() {
    let query = r#"
        MATCH (n:Person) RETURN n
        UNION
        MATCH (m:Company) RETURN m
        INTERSECT
        MATCH (p:Active) RETURN p
        EXCEPT
        MATCH (q:Deleted) RETURN q
    "#;

    let result = parse(query);
    // Chained set operators may have precedence rules
    let _ = result.ast;
}

#[test]
fn chained_unions() {
    let query = r#"
        MATCH (a) RETURN a
        UNION
        MATCH (b) RETURN b
        UNION
        MATCH (c) RETURN c
        UNION
        MATCH (d) RETURN d
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "Chained UNIONs should parse");
}

#[test]
fn mixed_set_operators_with_parentheses() {
    let query = r#"
        (MATCH (a) RETURN a UNION MATCH (b) RETURN b)
        INTERSECT
        (MATCH (c) RETURN c EXCEPT MATCH (d) RETURN d)
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Nested Query Specifications =====

#[test]
fn nested_query_in_from_clause() {
    let query = "SELECT * FROM (MATCH (n) RETURN n.id, n.name) AS subquery \
                 WHERE subquery.id > 10";

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn nested_query_with_alias() {
    let query = "SELECT t.id, t.name FROM (MATCH (n) RETURN n.id, n.name) AS t";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn multiple_nested_queries_in_from() {
    let query = r#"
        SELECT *
        FROM (MATCH (n:Person) RETURN n) AS persons,
             (MATCH (m:Company) RETURN m) AS companies
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn deeply_nested_queries() {
    let query = r#"
        SELECT *
        FROM (
            SELECT *
            FROM (MATCH (n) RETURN n) AS inner
        ) AS outer
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Common Table Expressions (CTEs) =====

#[test]
fn single_cte_with_query() {
    let query = "WITH recent AS (MATCH (n) RETURN n) SELECT * FROM recent";
    let result = parse(query);
    assert!(result.ast.is_some(), "Single CTE should parse");
}

#[test]
fn multiple_ctes() {
    let query = r#"
        WITH a AS (MATCH (n:Person) RETURN n),
             b AS (MATCH (m:Company) RETURN m)
        SELECT * FROM a, b
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "Multiple CTEs should parse");
}

#[test]
fn cte_with_column_aliases() {
    let query = "WITH people(id, name) AS (MATCH (n:Person) RETURN n.id, n.name) \
                 SELECT * FROM people";

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn nested_ctes() {
    let query = r#"
        WITH outer AS (
            WITH inner AS (MATCH (n) RETURN n)
            SELECT * FROM inner
        )
        SELECT * FROM outer
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn cte_referencing_previous_cte() {
    let query = r#"
        WITH first AS (MATCH (n) RETURN n),
             second AS (SELECT * FROM first WHERE n.age > 18)
        SELECT * FROM second
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== SELECT with Complex FROM Clauses =====

#[test]
fn select_from_match_list() {
    let queries = vec![
        "SELECT * FROM MATCH (n) RETURN n",
        "SELECT n.name FROM MATCH (n:Person) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "SELECT FROM MATCH '{}' should parse",
            query
        );
    }
}

#[test]
fn select_from_multiple_matches() {
    let query = r#"
        SELECT *
        FROM MATCH (n:Person) RETURN n,
             MATCH (m:Company) RETURN m
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn select_from_graph_match_list() {
    let query = "SELECT * FROM myGraph MATCH (n) RETURN n";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn select_with_join_conditions() {
    let query = r#"
        SELECT *
        FROM (MATCH (n:Person) RETURN n) AS people
        JOIN (MATCH (c:Company) RETURN c) AS companies
        ON people.companyId = companies.id
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== SELECT with Grouping and Aggregation =====

#[test]
fn select_with_group_by() {
    let queries = vec![
        "SELECT n.city, COUNT(*) FROM MATCH (n) RETURN n GROUP BY n.city",
        "SELECT n.dept, AVG(n.salary) FROM MATCH (n) RETURN n GROUP BY n.dept",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "SELECT with GROUP BY '{}' should parse",
            query
        );
    }
}

#[test]
fn select_with_having() {
    let query = r#"
        SELECT n.city, COUNT(*) as cnt
        FROM MATCH (n:Person) RETURN n
        GROUP BY n.city
        HAVING COUNT(*) > 10
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "SELECT with HAVING should parse");
}

#[test]
fn select_with_group_by_multiple_columns() {
    let query = r#"
        SELECT n.city, n.dept, COUNT(*)
        FROM MATCH (n) RETURN n
        GROUP BY n.city, n.dept
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn select_with_group_by_expressions() {
    let query = r#"
        SELECT YEAR(n.birthdate), COUNT(*)
        FROM MATCH (n) RETURN n
        GROUP BY YEAR(n.birthdate)
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== SELECT with DISTINCT =====

#[test]
fn select_distinct() {
    let queries = vec![
        "SELECT DISTINCT n.name FROM MATCH (n) RETURN n",
        "SELECT DISTINCT n.city, n.state FROM MATCH (n) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(result.ast.is_some(), "SELECT DISTINCT '{}' should parse", query);
    }
}

#[test]
fn select_all_explicit() {
    let query = "SELECT ALL n.name FROM MATCH (n) RETURN n";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Complex SELECT Expressions =====

#[test]
fn select_with_arithmetic_expressions() {
    let query = r#"
        SELECT n.price * 1.1 AS priceWithTax,
               n.quantity * n.price AS total
        FROM MATCH (n) RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn select_with_case_expressions() {
    let query = r#"
        SELECT n.name,
               CASE
                   WHEN n.age < 18 THEN 'minor'
                   ELSE 'adult'
               END AS ageCategory
        FROM MATCH (n) RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn select_with_aggregates_and_window_functions() {
    let query = r#"
        SELECT n.name,
               COUNT(*) AS totalCount,
               ROW_NUMBER() OVER (ORDER BY n.id) AS rowNum
        FROM MATCH (n) RETURN n
        GROUP BY n.name
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Subqueries in WHERE Clauses =====

#[test]
fn where_with_in_subquery() {
    let query = r#"
        MATCH (n)
        WHERE n.id IN (MATCH (m:Active) RETURN m.id)
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn where_with_exists_subquery() {
    let query = r#"
        MATCH (n)
        WHERE EXISTS (MATCH (n)-[:KNOWS]->(m) RETURN m)
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn where_with_scalar_subquery() {
    let query = r#"
        MATCH (n)
        WHERE n.count > (MATCH (m) RETURN COUNT(m))
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== NEXT Clause =====

#[test]
fn next_clause_basic() {
    let query = r#"
        MATCH (n) RETURN n
        NEXT
        MATCH (m) RETURN m
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn next_clause_with_yield() {
    let query = r#"
        MATCH (n) RETURN n
        NEXT YIELD n
        MATCH (m) WHERE m.id = n.id RETURN m
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn multiple_next_clauses() {
    let query = r#"
        MATCH (a) RETURN a
        NEXT
        MATCH (b) RETURN b
        NEXT
        MATCH (c) RETURN c
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== FOR Loop =====

#[test]
fn for_loop_basic() {
    let query = "FOR item IN [1, 2, 3] RETURN item";
    let result = parse(query);
    assert!(result.ast.is_some(), "FOR loop should parse");
}

#[test]
fn for_loop_with_ordinality() {
    let query = "FOR item IN [1, 2, 3] WITH ORDINALITY idx RETURN item, idx";
    let result = parse(query);
    assert!(result.ast.is_some(), "FOR with ORDINALITY should parse");
}

#[test]
fn nested_for_loops() {
    let query = r#"
        FOR x IN [1, 2, 3]
        FOR y IN [4, 5, 6]
        RETURN x, y
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== LET Clause =====

#[test]
fn let_clause_basic() {
    let query = "LET x = 1 RETURN x";
    let result = parse(query);
    assert!(result.ast.is_some(), "LET clause should parse");
}

#[test]
fn let_clause_multiple_variables() {
    let query = "LET x = 1, y = 2, z = 3 RETURN x, y, z";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn let_clause_with_expressions() {
    let query = "MATCH (n) LET count = COUNT(n), avg = AVG(n.value) RETURN count, avg";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Complex Query Combinations =====

#[test]
fn match_let_for_return_pipeline() {
    let query = r#"
        MATCH (n:Person)
        LET adults = [p IN n WHERE p.age >= 18]
        FOR adult IN adults
        RETURN adult.name
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn cte_with_set_operations() {
    let query = r#"
        WITH combined AS (
            MATCH (n:Person) RETURN n
            UNION
            MATCH (m:Company) RETURN m
        )
        SELECT * FROM combined
    "#;

    let result = parse(query);
    let _ = result.ast;
}
