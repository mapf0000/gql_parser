use gql_parser::ast::graph_type::EdgeTypePattern;
use gql_parser::ast::{
    EdgeReferenceValueType, GraphReferenceValueType, LocalNodeTypeAlias, NodeReferenceValueType,
};
use gql_parser::parser::types::{
    parse_edge_reference_value_type, parse_graph_reference_value_type,
    parse_node_reference_value_type,
};
use crate::common::*;

#[test]
fn graph_reference_property_graph_parses_real_nested_specification() {
    let tokens = tokenize_cleanly("PROPERTY GRAPH { NODE TYPE Person LABEL Person } NOT NULL");
    let graph_ref = parse_graph_reference_value_type(&tokens).expect("graph ref should parse");

    let GraphReferenceValueType::PropertyGraph { spec, not_null, .. } = graph_ref else {
        panic!("expected PROPERTY GRAPH typed form");
    };
    assert!(not_null, "expected NOT NULL flag");
    assert_eq!(
        spec.body.element_types.types.len(),
        1,
        "expected nested graph spec to preserve element types"
    );
}

#[test]
fn node_reference_typed_form_preserves_pattern_content() {
    let tokens = tokenize_cleanly("(n LABEL Person { name :: STRING }) NOT NULL");
    let node_ref = parse_node_reference_value_type(&tokens).expect("node ref should parse");

    let NodeReferenceValueType::Typed { spec, not_null, .. } = node_ref else {
        panic!("expected typed node reference");
    };
    assert!(not_null, "expected NOT NULL flag");

    let Some(LocalNodeTypeAlias { name, .. }) = spec.pattern.phrase.alias.as_ref() else {
        panic!("expected node alias to be preserved");
    };
    assert_eq!(name, "n");
}

#[test]
fn edge_reference_typed_form_parses_full_visual_pattern() {
    let tokens = tokenize_cleanly("(Person)-[LABEL KNOWS]->(Company) NOT NULL");
    let edge_ref = parse_edge_reference_value_type(&tokens).expect("edge ref should parse");

    let EdgeReferenceValueType::Typed { spec, not_null, .. } = edge_ref else {
        panic!("expected typed edge reference");
    };
    assert!(not_null, "expected NOT NULL flag");

    let EdgeTypePattern::Directed(pattern) = &spec.pattern else {
        panic!("expected directed edge pattern");
    };

    let Some(LocalNodeTypeAlias {
        name: left_name, ..
    }) = pattern.left_endpoint.phrase.alias.as_ref()
    else {
        panic!("expected left endpoint alias");
    };
    let Some(LocalNodeTypeAlias {
        name: right_name, ..
    }) = pattern.right_endpoint.phrase.alias.as_ref()
    else {
        panic!("expected right endpoint alias");
    };

    assert_eq!(left_name, "Person");
    assert_eq!(right_name, "Company");
}
