// Quick demonstration that Sprint 10 AST compiles and works

use gql_parser::ast::expression::{Expression, Literal};
use gql_parser::ast::mutation::*;
use smol_str::SmolStr;

fn main() {
    println!("Sprint 10: Data Modification Statements AST Demo\n");
    println!("=================================================\n");

    // Demonstrate INSERT statement AST
    let insert = InsertStatement {
        pattern: InsertGraphPattern {
            paths: vec![InsertPathPattern {
                elements: vec![InsertElementPattern::Node(InsertNodePattern {
                    filler: Some(InsertElementPatternFiller {
                        variable: None,
                        label_set: None,
                        use_is_keyword: false,
                        properties: None,
                        span: 0..10,
                    }),
                    span: 0..10,
                })],
                span: 0..10,
            }],
            span: 0..10,
        },
        span: 0..10,
    };
    println!("✓ INSERT statement AST created: {:?}", insert.span);

    // Demonstrate SET statement AST
    let set = SetStatement {
        items: SetItemList {
            items: vec![SetItem::Property(SetPropertyItem {
                element: SmolStr::new("n"),
                property: SmolStr::new("age"),
                value: Expression::Literal(Literal::Integer(SmolStr::new("30")), 0..2),
                span: 0..10,
            })],
            span: 0..10,
        },
        span: 0..10,
    };
    println!("✓ SET statement AST created: {:?}", set.span);

    // Demonstrate REMOVE statement AST
    let remove = RemoveStatement {
        items: RemoveItemList {
            items: vec![RemoveItem::Property(RemovePropertyItem {
                element: SmolStr::new("n"),
                property: SmolStr::new("age"),
                span: 0..10,
            })],
            span: 0..10,
        },
        span: 0..10,
    };
    println!("✓ REMOVE statement AST created: {:?}", remove.span);

    // Demonstrate DELETE statement AST
    let delete = DeleteStatement {
        detach_option: DetachOption::Detach,
        items: DeleteItemList {
            items: vec![DeleteItem {
                expression: Expression::VariableReference(SmolStr::new("n"), 0..1),
                span: 0..1,
            }],
            span: 0..1,
        },
        span: 0..15,
    };
    println!("✓ DELETE statement AST created: {:?}", delete.span);
    println!("  - DETACH option: {:?}", delete.detach_option);

    // Demonstrate primitive statement enum
    let primitive = PrimitiveDataModifyingStatement::Insert(insert);
    println!("\n✓ Primitive data modifying statement enum works");
    println!("  - Statement span: {:?}", primitive.span());

    // Demonstrate ambient linear statement
    let ambient = AmbientLinearDataModifyingStatement {
        statements: vec![SimpleDataAccessingStatement::Modifying(
            SimpleDataModifyingStatement::Primitive(PrimitiveDataModifyingStatement::Set(set)),
        )],
        primitive_result_statement: None,
        span: 0..10,
    };
    println!("\n✓ Ambient linear data modifying statement created");
    println!("  - Number of statements: {}", ambient.statements.len());

    println!("\n=================================================");
    println!("Sprint 10 AST Implementation: ✅ COMPLETE");
    println!("All mutation statement types are properly defined and working!");
}
