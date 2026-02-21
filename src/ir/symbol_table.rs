//! Symbol table for tracking variable bindings and scopes.

use crate::ast::Span;
use std::collections::HashMap;

/// Unique identifier for a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

impl ScopeId {
    /// Creates a new scope ID.
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// Kind of scope boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Query scope (top-level or after WITH).
    Query,

    /// Subquery scope (nested query).
    Subquery,

    /// Clause scope (local to a clause).
    Clause,

    /// Procedure scope (procedure body).
    Procedure,

    /// For loop scope (FOR clause).
    ForLoop,
}

/// Scope representing a visibility boundary for variables.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Unique scope identifier.
    pub id: ScopeId,

    /// Parent scope (None for root scope).
    pub parent: Option<ScopeId>,

    /// Kind of scope.
    pub kind: ScopeKind,

    /// Variables defined in this scope.
    symbols: Vec<String>,
}

impl Scope {
    /// Creates a new scope.
    pub fn new(id: ScopeId, parent: Option<ScopeId>, kind: ScopeKind) -> Self {
        Self {
            id,
            parent,
            kind,
            symbols: Vec::new(),
        }
    }

    /// Adds a symbol to this scope.
    pub fn add_symbol(&mut self, name: String) {
        self.symbols.push(name);
    }

    /// Checks if a symbol is defined in this scope.
    pub fn has_symbol(&self, name: &str) -> bool {
        self.symbols.iter().any(|s| s == name)
    }

    /// Returns all symbols in this scope.
    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }
}

/// Kind of symbol (variable binding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Binding variable from MATCH pattern (e.g., `n` in `MATCH (n)`).
    BindingVariable,

    /// Variable defined by LET clause.
    LetVariable,

    /// Variable defined by FOR clause.
    ForVariable,

    /// Parameter variable.
    Parameter,
}

/// Symbol representing a variable binding.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Variable name.
    pub name: String,

    /// Kind of symbol.
    pub kind: SymbolKind,

    /// Span where the symbol was declared.
    pub declared_at: Span,

    /// Scope where the symbol is defined.
    pub scope: ScopeId,
}

impl Symbol {
    /// Creates a new symbol.
    pub fn new(name: String, kind: SymbolKind, declared_at: Span, scope: ScopeId) -> Self {
        Self {
            name,
            kind,
            declared_at,
            scope,
        }
    }
}

/// Symbol table tracking variable bindings and scopes.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// All scopes in the symbol table.
    scopes: Vec<Scope>,

    /// Current scope ID.
    current_scope: ScopeId,

    /// All symbols indexed by name.
    symbols: HashMap<String, Vec<Symbol>>,
}

impl SymbolTable {
    /// Creates a new symbol table with a root scope.
    pub fn new() -> Self {
        let root_scope = Scope::new(ScopeId(0), None, ScopeKind::Query);
        Self {
            scopes: vec![root_scope],
            current_scope: ScopeId(0),
            symbols: HashMap::new(),
        }
    }

    /// Pushes a new scope onto the scope stack.
    pub fn push_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let new_id = ScopeId(self.scopes.len());
        let new_scope = Scope::new(new_id, Some(self.current_scope), kind);
        self.scopes.push(new_scope);
        self.current_scope = new_id;
        new_id
    }

    /// Pops the current scope, returning to the parent scope.
    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope.0].parent {
            self.current_scope = parent;
        }
    }

    /// Returns the current scope ID.
    pub fn current_scope(&self) -> ScopeId {
        self.current_scope
    }

    /// Returns a reference to a scope by ID.
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id.0)
    }

    /// Defines a symbol in the current scope.
    ///
    /// Returns a reference to the newly added symbol.
    pub fn define(&mut self, name: String, kind: SymbolKind, declared_at: Span) -> &Symbol {
        let symbol = Symbol::new(name.clone(), kind, declared_at, self.current_scope);

        // Add to scope
        self.scopes[self.current_scope.0].add_symbol(name.clone());

        // Add to symbols map
        let symbols_for_name = self.symbols.entry(name).or_default();
        symbols_for_name.push(symbol);

        // Return reference to the newly added symbol (last element in the vector)
        // SAFETY: We just pushed an element, so the vector is guaranteed non-empty
        symbols_for_name.last().expect("vector is non-empty after push")
    }

    /// Looks up a symbol by name in the current scope and parent scopes.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.lookup_from(self.current_scope, name)
    }

    /// Looks up a symbol by name starting from a specific scope and walking up parent scopes.
    pub fn lookup_from(&self, starting_scope: ScopeId, name: &str) -> Option<&Symbol> {
        // Get all symbols with this name
        let symbols = self.symbols.get(name)?;

        // Walk up the scope chain from starting scope
        let mut scope_id = Some(starting_scope);
        while let Some(sid) = scope_id {
            // Find a symbol in this scope
            if let Some(symbol) = symbols.iter().find(|s| s.scope == sid) {
                return Some(symbol);
            }

            // Move to parent scope
            scope_id = self.scopes[sid.0].parent;
        }

        None
    }

    /// Checks if a symbol is defined in the current scope (not parent scopes).
    pub fn is_defined_in_current_scope(&self, name: &str) -> bool {
        self.symbols
            .get(name)
            .and_then(|symbols| symbols.iter().find(|s| s.scope == self.current_scope))
            .is_some()
    }

    /// Returns all symbols in the current scope.
    pub fn current_scope_symbols(&self) -> impl Iterator<Item = &Symbol> {
        let current_scope = self.current_scope;
        self.symbols
            .values()
            .flatten()
            .filter(move |s| s.scope == current_scope)
    }

    /// Returns all symbols with a given name across all scopes.
    pub fn lookup_all(&self, name: &str) -> Option<&[Symbol]> {
        self.symbols.get(name).map(|v| v.as_slice())
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_basic() {
        let mut st = SymbolTable::new();

        // Define a variable in root scope
        st.define("n".to_string(), SymbolKind::BindingVariable, 0..1);

        // Look up the variable
        assert!(st.lookup("n").is_some());
        assert!(st.lookup("m").is_none());
    }

    #[test]
    fn test_symbol_table_scopes() {
        let mut st = SymbolTable::new();

        // Define variable in root scope
        st.define("n".to_string(), SymbolKind::BindingVariable, 0..1);

        // Push a new scope
        st.push_scope(ScopeKind::Subquery);

        // Variable from parent scope should be visible
        assert!(st.lookup("n").is_some());

        // Define a variable in nested scope
        st.define("m".to_string(), SymbolKind::BindingVariable, 2..3);
        assert!(st.lookup("m").is_some());

        // Pop back to parent scope
        st.pop_scope();

        // Variable from parent scope still visible
        assert!(st.lookup("n").is_some());

        // Variable from nested scope not visible
        assert!(st.lookup("m").is_none());
    }

    #[test]
    fn test_symbol_table_shadowing() {
        let mut st = SymbolTable::new();

        // Define variable in root scope
        st.define("n".to_string(), SymbolKind::BindingVariable, 0..1);

        // Push a new scope
        st.push_scope(ScopeKind::Subquery);

        // Shadow the variable
        st.define("n".to_string(), SymbolKind::BindingVariable, 2..3);

        // Lookup should find the inner variable
        let symbol = st.lookup("n").unwrap();
        assert_eq!(symbol.declared_at, 2..3);
    }
}
