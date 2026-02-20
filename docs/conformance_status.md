# Conformance Status

Generated: 2026-02-20

## Summary

- Total tracked rows: 23
- Closed rows: 15
- Open rows: 8
- Closed DB/Catalog No rows: 15
- Open DB/Catalog Yes or Partial rows: 8

## Row Status

| ID | Milestone | Workstream | DB/Catalog | Status | Owner | Tests |
| --- | --- | --- | --- | --- | --- | --- |
| M1-1 | Milestone1 | ConformanceMatrix | No | closed | parser-core | `tests/conformance_matrix_tests.rs::conformance_matrix_rows_are_well_formed` |
| M1-2 | Milestone1 | ConformanceStatusOutput | No | closed | parser-core | `tests/conformance_matrix_tests.rs::generated_status_lists_every_row` |
| M1-3 | Milestone1 | OwnershipTraceability | No | closed | parser-core | `tests/conformance_matrix_tests.rs::every_matrix_row_has_owner_and_traceability` |
| M3-5-1 | Milestone3 | GraphTypeModeling | No | closed | parser-core | `tests/graph_type_tests.rs::test_graph_type_parser_supports_abstract_inheritance_and_constraints` |
| M3-5-2 | Milestone3 | GraphTypeModeling | No | closed | parser-core | `tests/graph_type_tests.rs::test_graph_type_parser_supports_abstract_inheritance_and_constraints|tests/graph_type_tests.rs::test_graph_type_parser_supports_edge_constraints_and_inheritance` |
| M3-5-3 | Milestone3 | GraphTypeModeling | No | closed | parser-core | `tests/graph_type_tests.rs::test_graph_type_parser_supports_abstract_inheritance_and_constraints|tests/graph_type_tests.rs::test_graph_type_parser_supports_edge_constraints_and_inheritance` |
| M3-5-4 | Milestone3 | GraphTypeModeling | No | closed | parser-core | `tests/graph_type_tests.rs::test_graph_type_parser_supports_abstract_inheritance_and_constraints` |
| M3-4-1 | Milestone3 | SchemaSemantics | Yes | open | schema-integration | `tests/semantic_validator_tests.rs::test_valid_query_no_errors` |
| M3-4-2 | Milestone3 | SchemaSemantics | Yes | open | schema-integration | `tests/semantic_validator_scope_and_agg_tests.rs::test_group_by_no_aggregation` |
| M3-4-3 | Milestone3 | SchemaSemantics | Yes | open | schema-integration | `tests/semantic_validator_tests.rs::test_unknown_property_validation` |
| M3-4-4 | Milestone3 | SchemaSemantics | Yes | open | schema-integration | `tests/semantic_validator_tests.rs::test_unknown_label_validation` |
| M4-6-1-1 | Milestone4 | PatternHardening | No | closed | parser-core | `tests/pattern_tests.rs::parse_deeply_nested_quantifiers_on_parenthesized_patterns` |
| M4-6-1-2 | Milestone4 | PatternHardening | No | closed | parser-core | `tests/pattern_tests.rs::parse_path_expression_mixed_union_and_multiset_alternation_has_stable_precedence|tests/pattern_tests.rs::parse_simplified_mixed_union_and_multiset_alternation_has_stable_precedence` |
| M4-6-1-3 | Milestone4 | PatternHardening | No | closed | parser-core | `tests/stress_tests.rs::deeply_nested_parenthesized_quantifiers_stress|tests/stress_tests.rs::mixed_union_multiset_alternation_stress` |
| M4-6-2-1 | Milestone4 | FunctionCompleteness | No | closed | parser-core | `src/parser/expression.rs::parses_specialized_string_and_list_functions|src/parser/expression.rs::parses_temporal_functions|tests/case_insensitive_tests.rs::built_in_function_keywords_case_insensitive` |
| M4-6-2-2 | Milestone4 | FunctionCompleteness | Partial | open | semantic-core | `tests/semantic_validator_tests.rs::test_expression_validation_function_call` |
| M5-7-1-1 | Milestone5 | RecoveryPolish | No | closed | parser-core | `src/parser/program.rs::synchronize_top_level_resumes_at_statement_start|src/parser/program.rs::synchronize_top_level_skips_semicolon_run` |
| M5-7-1-2 | Milestone5 | RecoveryPolish | No | closed | parser-core | `src/parser/program.rs::append_diags_dedup_suppresses_adjacent_duplicate_diagnostics|tests/pattern_tests.rs::parse_reports_single_chained_quantifier_diagnostic` |
| M5-7-1-3 | Milestone5 | RecoveryPolish | No | closed | parser-core | `src/parser/mod.rs::parser_never_panics_on_random_inputs|src/parser/program.rs::parse_recovers_after_invalid_top_level_token` |
| M5-7-2-1 | Milestone5 | NestedQuantifierRobustness | No | closed | parser-core | `tests/stress_tests.rs::quantifier_rollback_ambiguity_fuzz_regression|tests/stress_tests.rs::deeply_nested_parenthesized_quantifiers_stress` |
| M5-7-3-1 | Milestone5 | TypeInferenceQuality | Partial | open | semantic-core | `tests/semantic_validator_tests.rs::test_expression_validation_valid_simple_expression` |
| M5-7-3-2 | Milestone5 | TypeInferenceQuality | Partial | open | semantic-core | `tests/semantic_validator_tests.rs::test_expression_validation_function_call` |
| M5-7-3-3 | Milestone5 | TypeInferenceQuality | Partial | open | semantic-core | `tests/semantic_validator_tests.rs::test_aggregation_multiple_functions` |
