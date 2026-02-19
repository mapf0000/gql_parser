//! Type table for tracking expression types.

use std::collections::HashMap;

/// Unique identifier for an expression node in the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(usize);

impl ExprId {
    /// Creates a new expression ID.
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// GQL type representation.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Integer type.
    Int,

    /// Floating-point type.
    Float,

    /// String type.
    String,

    /// Boolean type.
    Boolean,

    /// Date type.
    Date,

    /// Time type.
    Time,

    /// Timestamp type.
    Timestamp,

    /// Duration type.
    Duration,

    /// Node type (optionally with labels).
    Node(Option<Vec<String>>),

    /// Edge type (optionally with labels).
    Edge(Option<Vec<String>>),

    /// Path type.
    Path,

    /// List type with element type.
    List(Box<Type>),

    /// Record type with field names and types.
    Record(Vec<(String, Type)>),

    /// Union of multiple types.
    Union(Vec<Type>),

    /// Null type.
    Null,

    /// Any type (unknown or dynamic).
    Any,
}

impl Type {
    /// Returns true if this type is numeric (Int or Float).
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    /// Returns true if this type is comparable.
    pub fn is_comparable(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Float
                | Type::String
                | Type::Boolean
                | Type::Date
                | Type::Time
                | Type::Timestamp
                | Type::Duration
        )
    }

    /// Returns true if this type is a boolean.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Type::Boolean)
    }

    /// Returns true if this type is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Type::String)
    }

    /// Returns true if this type is a node.
    pub fn is_node(&self) -> bool {
        matches!(self, Type::Node(_))
    }

    /// Returns true if this type is an edge.
    pub fn is_edge(&self) -> bool {
        matches!(self, Type::Edge(_))
    }

    /// Returns true if this type is a path.
    pub fn is_path(&self) -> bool {
        matches!(self, Type::Path)
    }

    /// Returns true if this type is a list.
    pub fn is_list(&self) -> bool {
        matches!(self, Type::List(_))
    }

    /// Returns true if this type is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Type::Null)
    }

    /// Returns true if this type is compatible with another type for assignment/comparison.
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        // Same types are compatible
        if self == other {
            return true;
        }

        // Any type is compatible with everything
        if matches!(self, Type::Any) || matches!(other, Type::Any) {
            return true;
        }

        // Int can be promoted to Float
        if matches!(self, Type::Int) && matches!(other, Type::Float) {
            return true;
        }
        if matches!(self, Type::Float) && matches!(other, Type::Int) {
            return true;
        }

        // Null is compatible with any type
        if matches!(self, Type::Null) || matches!(other, Type::Null) {
            return true;
        }

        // Union types
        if let Type::Union(types) = self {
            return types.iter().any(|t| t.is_compatible_with(other));
        }
        if let Type::Union(types) = other {
            return types.iter().any(|t| self.is_compatible_with(t));
        }

        // Node/Edge types with different labels
        match (self, other) {
            (Type::Node(_), Type::Node(_)) => return true,
            (Type::Edge(_), Type::Edge(_)) => return true,
            _ => {}
        }

        false
    }

    /// Returns a human-readable name for this type.
    pub fn name(&self) -> String {
        match self {
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::String => "String".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::Date => "Date".to_string(),
            Type::Time => "Time".to_string(),
            Type::Timestamp => "Timestamp".to_string(),
            Type::Duration => "Duration".to_string(),
            Type::Node(Some(labels)) => format!("Node:{}", labels.join("|")),
            Type::Node(None) => "Node".to_string(),
            Type::Edge(Some(labels)) => format!("Edge:{}", labels.join("|")),
            Type::Edge(None) => "Edge".to_string(),
            Type::Path => "Path".to_string(),
            Type::List(elem_type) => format!("List<{}>", elem_type.name()),
            Type::Record(fields) => {
                let field_strs: Vec<_> = fields
                    .iter()
                    .map(|(name, ty)| format!("{}: {}", name, ty.name()))
                    .collect();
                format!("Record<{}>", field_strs.join(", "))
            }
            Type::Union(types) => {
                let type_names: Vec<_> = types.iter().map(|t| t.name()).collect();
                format!("Union<{}>", type_names.join(", "))
            }
            Type::Null => "Null".to_string(),
            Type::Any => "Any".to_string(),
        }
    }
}

/// Type constraint for an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeConstraint {
    /// Must be a numeric type.
    Numeric,

    /// Must be a comparable type.
    Comparable,

    /// Must be a boolean type.
    Boolean,

    /// Must be a string type.
    String,

    /// Must be a node type.
    Node,

    /// Must be an edge type.
    Edge,

    /// Must be a graph element (node or edge).
    GraphElement,

    /// Must be a list type.
    List,

    /// Must be an exact type.
    Exact(Type),
}

/// Type table tracking expression types and constraints.
#[derive(Debug, Clone)]
pub struct TypeTable {
    /// Inferred types for expressions.
    types: HashMap<ExprId, Type>,

    /// Type constraints for expressions.
    constraints: HashMap<ExprId, Vec<TypeConstraint>>,

    /// Next expression ID to assign.
    next_id: usize,
}

impl TypeTable {
    /// Creates a new empty type table.
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            constraints: HashMap::new(),
            next_id: 0,
        }
    }

    /// Allocates a new expression ID.
    pub fn alloc_expr_id(&mut self) -> ExprId {
        let id = ExprId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Sets the type for an expression.
    pub fn set_type(&mut self, expr_id: ExprId, ty: Type) {
        self.types.insert(expr_id, ty);
    }

    /// Gets the type of an expression.
    pub fn get_type(&self, expr_id: ExprId) -> Option<&Type> {
        self.types.get(&expr_id)
    }

    /// Adds a constraint for an expression.
    pub fn add_constraint(&mut self, expr_id: ExprId, constraint: TypeConstraint) {
        self.constraints
            .entry(expr_id)
            .or_default()
            .push(constraint);
    }

    /// Gets the constraints for an expression.
    pub fn get_constraints(&self, expr_id: ExprId) -> Option<&[TypeConstraint]> {
        self.constraints.get(&expr_id).map(|v| v.as_slice())
    }

    /// Checks if an expression satisfies all its constraints.
    pub fn satisfies_constraints(&self, expr_id: ExprId) -> bool {
        let ty = match self.get_type(expr_id) {
            Some(ty) => ty,
            None => return false,
        };

        let constraints = match self.get_constraints(expr_id) {
            Some(c) => c,
            None => return true,
        };

        constraints.iter().all(|constraint| match constraint {
            TypeConstraint::Numeric => ty.is_numeric(),
            TypeConstraint::Comparable => ty.is_comparable(),
            TypeConstraint::Boolean => ty.is_boolean(),
            TypeConstraint::String => ty.is_string(),
            TypeConstraint::Node => ty.is_node(),
            TypeConstraint::Edge => ty.is_edge(),
            TypeConstraint::GraphElement => ty.is_node() || ty.is_edge(),
            TypeConstraint::List => ty.is_list(),
            TypeConstraint::Exact(expected) => ty.is_compatible_with(expected),
        })
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_compatibility() {
        assert!(Type::Int.is_compatible_with(&Type::Int));
        assert!(Type::Int.is_compatible_with(&Type::Float));
        assert!(Type::Float.is_compatible_with(&Type::Int));
        assert!(Type::Int.is_compatible_with(&Type::Any));
        assert!(Type::Any.is_compatible_with(&Type::String));
        assert!(Type::Null.is_compatible_with(&Type::Int));
    }

    #[test]
    fn test_type_table() {
        let mut tt = TypeTable::new();

        let expr_id = tt.alloc_expr_id();
        tt.set_type(expr_id, Type::Int);

        assert_eq!(tt.get_type(expr_id), Some(&Type::Int));
    }

    #[test]
    fn test_type_constraints() {
        let mut tt = TypeTable::new();

        let expr_id = tt.alloc_expr_id();
        tt.set_type(expr_id, Type::Int);
        tt.add_constraint(expr_id, TypeConstraint::Numeric);

        assert!(tt.satisfies_constraints(expr_id));
    }
}
