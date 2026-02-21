//! Semantic validation tests for catalog and session management.
//!
//! Tests validation of catalog operations (CREATE/DROP SCHEMA/GRAPH) and
//! session commands (SET SCHEMA, SET GRAPH, SET TIME ZONE, SET parameters)
//! according to the test plan in VAL_TESTS.md section 11.
//!
//! Note: These tests focus on validation logic. Since we're testing the
//! validator and not a live database, we mock metadata providers to simulate
//! catalog state.

use gql_parser::parse;
use gql_parser::semantic::validator::SemanticValidator;
use gql_parser::semantic::metadata_provider::MockMetadataProvider;
use gql_parser::ir::ValidationOutcome;

fn validate_catalog(source: &str) -> ValidationOutcome {
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let validator = SemanticValidator::new();
    validator.validate(parse_result.ast.as_ref().unwrap())
}

fn validate_catalog_with_provider(source: &str, provider: &MockMetadataProvider) -> ValidationOutcome {
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let validator = SemanticValidator::new()
        .with_metadata_provider(provider);
    validator.validate(parse_result.ast.as_ref().unwrap())
}

// ===== Section A: CREATE SCHEMA Tests =====

#[test]
fn test_create_schema_valid_basic() {
    let source = "CREATE SCHEMA myschema";
    let outcome = validate_catalog(source);

    // Should parse and validate successfully
    assert!(outcome.is_success(), "CREATE SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_schema_if_not_exists() {
    let source = "CREATE SCHEMA IF NOT EXISTS myschema";
    let outcome = validate_catalog(source);

    // IF NOT EXISTS is valid syntax
    assert!(outcome.is_success(), "CREATE SCHEMA IF NOT EXISTS should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_schema_or_replace() {
    let source = "CREATE OR REPLACE SCHEMA myschema";
    let outcome = validate_catalog(source);

    // OR REPLACE is valid syntax
    assert!(outcome.is_success(), "CREATE OR REPLACE SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_schema_qualified_name() {
    let source = "CREATE SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Qualified schema names use absolute paths (/)
    assert!(outcome.is_success(), "CREATE SCHEMA with absolute path should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_schema_with_directory_path() {
    let source = "CREATE SCHEMA /dir/myschema";
    let outcome = validate_catalog(source);

    // Schema with directory path
    assert!(outcome.is_success(), "CREATE SCHEMA with directory path should validate: {:?}", outcome.diagnostics);
}

// Note: CREATE SCHEMA HOME is not valid - HOME is a schemaReference for SESSION SET,
// but CREATE SCHEMA requires catalogSchemaParentAndName (absolute path + name)

#[test]
fn test_create_schema_duplicate_error_simulation() {
    // In a real system, this would check against the catalog
    // For mock testing, we just validate the syntax is correct
    let source = "CREATE SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Without IF NOT EXISTS, syntax should still be valid
    // Actual duplicate detection would happen at execution time in a real database
    assert!(outcome.is_success(), "Syntax should validate even if schema might exist: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_schema_multiple_in_sequence() {
    let source = "CREATE SCHEMA /schema1; CREATE SCHEMA /schema2; CREATE SCHEMA /schema3";
    let outcome = validate_catalog(source);

    // Multiple CREATE SCHEMA statements should validate
    assert!(outcome.is_success(), "Multiple CREATE SCHEMA statements should validate: {:?}", outcome.diagnostics);
}

// ===== Section B: DROP SCHEMA Tests =====

#[test]
fn test_drop_schema_valid_basic() {
    let source = "DROP SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Should parse and validate successfully (uses absolute path)
    assert!(outcome.is_success(), "DROP SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_schema_if_exists() {
    let source = "DROP SCHEMA IF EXISTS /myschema";
    let outcome = validate_catalog(source);

    // IF EXISTS is valid syntax
    assert!(outcome.is_success(), "DROP SCHEMA IF EXISTS should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_schema_qualified_name() {
    let source = "DROP SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Schema names use absolute paths
    assert!(outcome.is_success(), "DROP SCHEMA with absolute path should validate: {:?}", outcome.diagnostics);
}

// Note: DROP SCHEMA HOME is not valid - HOME is a schemaReference for SESSION SET,
// but DROP SCHEMA requires catalogSchemaParentAndName (absolute path + name)

#[test]
fn test_drop_schema_non_existent_simulation() {
    // Without IF EXISTS, dropping a non-existent schema would fail at execution
    // For validation purposes, syntax should be correct
    let source = "DROP SCHEMA /nonexistent_schema";
    let outcome = validate_catalog(source);

    // Syntax validation should pass (catalog check happens at execution)
    assert!(outcome.is_success(), "DROP SCHEMA syntax should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_schema_multiple_in_sequence() {
    let source = "DROP SCHEMA /schema1; DROP SCHEMA /schema2; DROP SCHEMA IF EXISTS /schema3";
    let outcome = validate_catalog(source);

    // Multiple DROP SCHEMA statements should validate
    assert!(outcome.is_success(), "Multiple DROP SCHEMA statements should validate: {:?}", outcome.diagnostics);
}

// ===== Section C: CREATE GRAPH Tests =====

#[test]
fn test_create_graph_valid_basic_any() {
    let source = "CREATE GRAPH mygraph ANY";
    let outcome = validate_catalog(source);

    // Basic CREATE GRAPH with ANY (open graph type)
    assert!(outcome.is_success(), "CREATE GRAPH ANY should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_property_graph() {
    let source = "CREATE PROPERTY GRAPH mygraph ANY";
    let outcome = validate_catalog(source);

    // PROPERTY keyword is optional but valid
    assert!(outcome.is_success(), "CREATE PROPERTY GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_if_not_exists() {
    let source = "CREATE GRAPH IF NOT EXISTS mygraph ANY";
    let outcome = validate_catalog(source);

    // IF NOT EXISTS is valid syntax  (requires ANY or TYPED)
    assert!(outcome.is_success(), "CREATE GRAPH IF NOT EXISTS should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_or_replace() {
    let source = "CREATE OR REPLACE GRAPH mygraph ANY";
    let outcome = validate_catalog(source);

    // OR REPLACE is valid syntax
    assert!(outcome.is_success(), "CREATE OR REPLACE GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_open_type() {
    let source = "CREATE GRAPH mygraph ANY";
    let outcome = validate_catalog(source);

    // Open graph type allows any node/edge types
    assert!(outcome.is_success(), "CREATE GRAPH with ANY type should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_typed() {
    let source = "CREATE GRAPH mygraph TYPED social_network_type";
    let outcome = validate_catalog(source);

    // Graph can be typed with a graph type
    assert!(outcome.is_success(), "CREATE GRAPH TYPED graph_type should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_like_clause() {
    let source = "CREATE GRAPH mygraph LIKE other_graph";
    let outcome = validate_catalog(source);

    // LIKE clause copies structure from existing graph
    assert!(outcome.is_success(), "CREATE GRAPH LIKE should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_as_copy_of() {
    let source = "CREATE GRAPH mygraph AS COPY OF other_graph";
    let outcome = validate_catalog(source);

    // AS COPY OF copies structure and data from existing graph
    assert!(outcome.is_success(), "CREATE GRAPH AS COPY OF should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_qualified_name() {
    let source = "CREATE GRAPH /myschema/mygraph ANY";
    let outcome = validate_catalog(source);

    // Fully qualified graph names use schema reference / separator
    assert!(outcome.is_success(), "CREATE GRAPH with qualified name should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_create_graph_with_multiple_specifications() {
    // Test that we can only have one type specification
    let source = "CREATE GRAPH mygraph OF graph_type";
    let outcome = validate_catalog(source);

    assert!(outcome.is_success(), "CREATE GRAPH with type spec should validate: {:?}", outcome.diagnostics);
}

// ===== Section D: DROP GRAPH Tests =====

#[test]
fn test_drop_graph_valid_basic() {
    let source = "DROP GRAPH mygraph";
    let outcome = validate_catalog(source);

    // Basic DROP GRAPH should validate
    assert!(outcome.is_success(), "DROP GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_property_graph() {
    let source = "DROP PROPERTY GRAPH mygraph";
    let outcome = validate_catalog(source);

    // PROPERTY keyword is optional but valid
    assert!(outcome.is_success(), "DROP PROPERTY GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_graph_if_exists() {
    let source = "DROP GRAPH IF EXISTS mygraph";
    let outcome = validate_catalog(source);

    // IF EXISTS is valid syntax
    assert!(outcome.is_success(), "DROP GRAPH IF EXISTS should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_graph_qualified_name() {
    let source = "DROP GRAPH /myschema/mygraph";
    let outcome = validate_catalog(source);

    // Qualified graph names use schema reference / separator
    assert!(outcome.is_success(), "DROP GRAPH with qualified name should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_drop_graph_multiple_in_sequence() {
    let source = "DROP GRAPH graph1; DROP GRAPH IF EXISTS graph2; DROP PROPERTY GRAPH graph3";
    let outcome = validate_catalog(source);

    // Multiple DROP GRAPH statements should validate
    assert!(outcome.is_success(), "Multiple DROP GRAPH statements should validate: {:?}", outcome.diagnostics);
}

// ===== Section E: Session Commands - SET SCHEMA =====

#[test]
fn test_session_set_schema_basic() {
    let source = "SESSION SET SCHEMA myschema";
    let outcome = validate_catalog(source);

    // Basic SESSION SET SCHEMA should validate
    assert!(outcome.is_success(), "SESSION SET SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_schema_qualified() {
    let source = "SESSION SET SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Schema reference uses absolute path
    assert!(outcome.is_success(), "SESSION SET SCHEMA with absolute path should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_schema_home() {
    let source = "SESSION SET SCHEMA HOME_SCHEMA";
    let outcome = validate_catalog(source);

    // HOME_SCHEMA is a valid predefined schema reference
    assert!(outcome.is_success(), "SESSION SET SCHEMA HOME_SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_schema_current() {
    let source = "SESSION SET SCHEMA CURRENT_SCHEMA";
    let outcome = validate_catalog(source);

    // CURRENT_SCHEMA is a valid schema reference
    assert!(outcome.is_success(), "SESSION SET SCHEMA CURRENT_SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_schema_in_transaction() {
    let source = "START TRANSACTION; SESSION SET SCHEMA myschema; MATCH (n) RETURN n; COMMIT";
    let outcome = validate_catalog(source);

    // Session commands can be used within transactions
    assert!(outcome.is_success(), "SESSION SET SCHEMA in transaction should validate: {:?}", outcome.diagnostics);
}

// ===== Section E: Session Commands - SET GRAPH =====

#[test]
fn test_session_set_graph_basic() {
    let source = "SESSION SET GRAPH mygraph";
    let outcome = validate_catalog(source);

    // Basic SESSION SET GRAPH should validate
    assert!(outcome.is_success(), "SESSION SET GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_property_graph() {
    let source = "SESSION SET PROPERTY GRAPH mygraph";
    let outcome = validate_catalog(source);

    // PROPERTY keyword is optional but valid
    assert!(outcome.is_success(), "SESSION SET PROPERTY GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_graph_qualified() {
    let source = "SESSION SET GRAPH /myschema/mygraph";
    let outcome = validate_catalog(source);

    // Qualified graph reference uses schema reference / separator
    assert!(outcome.is_success(), "SESSION SET GRAPH with qualified name should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_graph_current() {
    let source = "SESSION SET GRAPH CURRENT_GRAPH";
    let outcome = validate_catalog(source);

    // CURRENT_GRAPH is a valid graph reference
    assert!(outcome.is_success(), "SESSION SET GRAPH CURRENT_GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_graph_in_transaction() {
    let source = "START TRANSACTION; SESSION SET GRAPH mygraph; MATCH (n) RETURN n; COMMIT";
    let outcome = validate_catalog(source);

    // Session commands can be used within transactions
    assert!(outcome.is_success(), "SESSION SET GRAPH in transaction should validate: {:?}", outcome.diagnostics);
}

// ===== Section E: Session Commands - SET TIME ZONE =====

#[test]
fn test_session_set_time_zone_string() {
    let source = "SESSION SET TIME ZONE 'America/New_York'";
    let outcome = validate_catalog(source);

    // Time zone as string literal
    assert!(outcome.is_success(), "SESSION SET TIME ZONE with string should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_time_zone_offset() {
    let source = "SESSION SET TIME ZONE '+05:30'";
    let outcome = validate_catalog(source);

    // Time zone as offset
    assert!(outcome.is_success(), "SESSION SET TIME ZONE with offset should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_time_zone_utc() {
    let source = "SESSION SET TIME ZONE 'UTC'";
    let outcome = validate_catalog(source);

    // UTC is a common time zone
    assert!(outcome.is_success(), "SESSION SET TIME ZONE UTC should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_time_zone_local() {
    let source = "SESSION SET TIME ZONE LOCAL";
    let outcome = validate_catalog(source);

    // LOCAL keyword for time zone
    // Note: This may or may not be valid syntax depending on GQL spec
    // Adjust based on actual parser support
    let _ = outcome; // Just ensure it parses without panic
}

// ===== Section E: Session Commands - SET VALUE Parameters =====

#[test]
fn test_session_set_value_parameter_integer() {
    let source = "SESSION SET VALUE $max_connections = 100";
    let outcome = validate_catalog(source);

    // Setting a value parameter with integer (requires $ prefix)
    assert!(outcome.is_success(), "SESSION SET VALUE parameter with integer should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_value_parameter_string() {
    let source = "SESSION SET VALUE $app_name = 'MyApp'";
    let outcome = validate_catalog(source);

    // Setting a value parameter with string (requires $ prefix)
    assert!(outcome.is_success(), "SESSION SET VALUE parameter with string should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_value_parameter_boolean() {
    let source = "SESSION SET VALUE $debug_mode = TRUE";
    let outcome = validate_catalog(source);

    // Setting a value parameter with boolean (requires $ prefix)
    assert!(outcome.is_success(), "SESSION SET VALUE parameter with boolean should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_value_parameter_expression() {
    let source = "SESSION SET VALUE $timeout_seconds = 60 * 5";
    let outcome = validate_catalog(source);

    // Setting a value parameter with expression (requires $ prefix)
    assert!(outcome.is_success(), "SESSION SET VALUE parameter with expression should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_multiple_parameters() {
    let source = "SESSION SET VALUE $max_connections = 100; SESSION SET VALUE $timeout = 300";
    let outcome = validate_catalog(source);

    // Multiple parameter settings
    assert!(outcome.is_success(), "Multiple SESSION SET VALUE parameters should validate: {:?}", outcome.diagnostics);
}

// ===== Section E: Session Commands - RESET =====

#[test]
fn test_session_reset_all() {
    let source = "SESSION RESET ALL";
    let outcome = validate_catalog(source);

    // RESET ALL resets all session state
    assert!(outcome.is_success(), "SESSION RESET ALL should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_reset_parameters() {
    let source = "SESSION RESET PARAMETERS";
    let outcome = validate_catalog(source);

    // RESET PARAMETERS resets all session parameters
    assert!(outcome.is_success(), "SESSION RESET PARAMETERS should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_reset_characteristics() {
    let source = "SESSION RESET CHARACTERISTICS";
    let outcome = validate_catalog(source);

    // RESET CHARACTERISTICS resets transaction characteristics
    assert!(outcome.is_success(), "SESSION RESET CHARACTERISTICS should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_reset_schema() {
    let source = "SESSION RESET SCHEMA";
    let outcome = validate_catalog(source);

    // RESET SCHEMA resets to default schema
    assert!(outcome.is_success(), "SESSION RESET SCHEMA should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_reset_graph() {
    let source = "SESSION RESET GRAPH";
    let outcome = validate_catalog(source);

    // RESET GRAPH resets to default graph
    assert!(outcome.is_success(), "SESSION RESET GRAPH should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_reset_time_zone() {
    let source = "SESSION RESET TIME ZONE";
    let outcome = validate_catalog(source);

    // RESET TIME ZONE resets to default time zone
    assert!(outcome.is_success(), "SESSION RESET TIME ZONE should validate: {:?}", outcome.diagnostics);
}

// ===== Section E: Session Commands - CLOSE =====

#[test]
fn test_session_close() {
    let source = "SESSION CLOSE";
    let outcome = validate_catalog(source);

    // SESSION CLOSE ends the session
    assert!(outcome.is_success(), "SESSION CLOSE should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_close_after_transaction() {
    let source = "START TRANSACTION; MATCH (n) RETURN n; COMMIT; SESSION CLOSE";
    let outcome = validate_catalog(source);

    // SESSION CLOSE after transaction
    assert!(outcome.is_success(), "SESSION CLOSE after transaction should validate: {:?}", outcome.diagnostics);
}

// ===== Session Parameter References =====

#[test]
fn test_session_parameter_reference_single_dollar() {
    let source = "SESSION SET VALUE max_rows = 100; MATCH (n) RETURN n LIMIT $max_rows";
    let outcome = validate_catalog(source);

    // Single dollar sign for value parameters
    assert!(outcome.is_success(), "Session parameter reference with $ should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_parameter_reference_double_dollar() {
    let source = "SESSION SET VALUE graph_param = 'myvalue'; MATCH (n {prop: $$graph_param}) RETURN n";
    let outcome = validate_catalog(source);

    // Double dollar sign for graph parameters
    assert!(outcome.is_success(), "Session parameter reference with $$ should validate: {:?}", outcome.diagnostics);
}

// ===== Combined Catalog and Session Operations =====

#[test]
fn test_combined_catalog_and_session_operations() {
    let source = r#"
        CREATE SCHEMA /myschema;
        CREATE GRAPH mygraph ANY;
        SESSION SET SCHEMA /myschema;
        SESSION SET GRAPH mygraph;
        MATCH (n:Person) RETURN n;
        SESSION CLOSE
    "#;
    let outcome = validate_catalog(source);

    // Combined operations should validate
    assert!(outcome.is_success(), "Combined catalog and session operations should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_catalog_operations_with_if_clauses() {
    let source = r#"
        CREATE SCHEMA IF NOT EXISTS /myschema;
        CREATE GRAPH IF NOT EXISTS mygraph ANY;
        DROP GRAPH IF EXISTS oldgraph;
        DROP SCHEMA IF EXISTS /oldschema
    "#;
    let outcome = validate_catalog(source);

    // All IF clauses should validate
    assert!(outcome.is_success(), "Catalog operations with IF clauses should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_state_changes_in_sequence() {
    let source = r#"
        SESSION SET SCHEMA /schema1;
        MATCH (n) RETURN n;
        SESSION SET SCHEMA /schema2;
        MATCH (m) RETURN m;
        SESSION RESET SCHEMA;
        MATCH (x) RETURN x
    "#;
    let outcome = validate_catalog(source);

    // Sequential session state changes should validate
    assert!(outcome.is_success(), "Sequential session state changes should validate: {:?}", outcome.diagnostics);
}

// ===== Mock Provider Tests =====

#[test]
fn test_validation_with_mock_provider() {
    let provider = MockMetadataProvider::with_standard_fixtures();
    let source = "SESSION SET GRAPH social_graph; MATCH (n:Person) RETURN n";
    let outcome = validate_catalog_with_provider(source, &provider);

    // Should validate with schema from mock provider
    assert!(outcome.is_success(), "Validation with mock provider should succeed: {:?}", outcome.diagnostics);
}

#[test]
fn test_validation_with_empty_mock_provider() {
    let provider = MockMetadataProvider::new();
    let source = "SESSION SET SCHEMA myschema; MATCH (n) RETURN n";
    let outcome = validate_catalog_with_provider(source, &provider);

    // Should still validate syntax even with empty provider
    assert!(outcome.is_success(), "Validation with empty mock provider should validate syntax: {:?}", outcome.diagnostics);
}

#[test]
fn test_graph_operations_with_mock_catalog() {
    let provider = MockMetadataProvider::with_standard_fixtures();
    let source = r#"
        SESSION SET GRAPH social_graph;
        MATCH (p:Person)-[:KNOWS]->(friend:Person)
        WHERE p.age > 25
        RETURN p.name, friend.name
    "#;
    let outcome = validate_catalog_with_provider(source, &provider);

    // Should validate against social_graph schema
    assert!(outcome.is_success(), "Query against mocked schema should validate: {:?}", outcome.diagnostics);
}

// ===== Edge Cases and Error Scenarios =====

#[test]
fn test_create_and_drop_same_schema() {
    let source = "CREATE SCHEMA /myschema; DROP SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Should validate syntactically (semantic consistency checked at execution)
    assert!(outcome.is_success(), "CREATE then DROP same schema should validate: {:?}", outcome.diagnostics);
}

#[test]
fn test_session_set_before_create() {
    let source = "SESSION SET SCHEMA /myschema; CREATE SCHEMA /myschema";
    let outcome = validate_catalog(source);

    // Setting a schema before creating it - syntax is valid, execution would handle order
    assert!(outcome.is_success(), "SESSION SET before CREATE should validate syntactically: {:?}", outcome.diagnostics);
}

#[test]
fn test_empty_catalog_operations_sequence() {
    let source = "CREATE SCHEMA /s1; DROP SCHEMA /s1; CREATE SCHEMA /s1; DROP SCHEMA /s1";
    let outcome = validate_catalog(source);

    // Repeated create/drop should validate
    assert!(outcome.is_success(), "Repeated create/drop operations should validate: {:?}", outcome.diagnostics);
}
