//! Ready-to-use AST visitors.

pub mod collecting;
pub mod span;
pub mod variable;

pub use collecting::{AstNode, CollectingVisitor};
pub use span::SpanCollector;
pub use variable::VariableCollector;
