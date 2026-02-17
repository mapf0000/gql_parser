//! AST foundation types: spans and spanned nodes.

use std::ops::Range;

/// A span representing a range in source text.
/// This is the canonical span type used throughout the parser.
pub type Span = Range<usize>;

/// A value with an associated source span.
///
/// `Spanned<T>` pairs a syntax node or token with its location in source text.
/// This is the primary building block for AST nodes that need positional information
/// for diagnostics and error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The wrapped value.
    pub node: T,
    /// The span in source text where this node appears.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new spanned value.
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    /// Maps the inner value while preserving the span.
    pub fn map<U, F>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }

    /// Extracts the inner value, discarding the span.
    pub fn into_inner(self) -> T {
        self.node
    }

    /// Returns a reference to the span.
    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl<T> AsRef<T> for Spanned<T> {
    fn as_ref(&self) -> &T {
        &self.node
    }
}

impl<T> AsMut<T> for Spanned<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_basic_properties() {
        let span: Span = 5..10;
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 10);
        assert_eq!(span.len(), 5);
    }

    #[test]
    fn span_empty() {
        let span: Span = 5..5;
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn spanned_new() {
        let spanned = Spanned::new(42, 0..2);
        assert_eq!(spanned.node, 42);
        assert_eq!(spanned.span, 0..2);
    }

    #[test]
    fn spanned_map() {
        let spanned = Spanned::new(5, 10..15);
        let mapped = spanned.map(|x| x * 2);
        assert_eq!(mapped.node, 10);
        assert_eq!(mapped.span, 10..15);
    }

    #[test]
    fn spanned_into_inner() {
        let spanned = Spanned::new("hello", 0..5);
        assert_eq!(spanned.into_inner(), "hello");
    }

    #[test]
    fn spanned_accessors() {
        let mut spanned = Spanned::new(100, 20..25);
        assert_eq!(*spanned.as_ref(), 100);
        assert_eq!(spanned.span(), &(20..25));

        *spanned.as_mut() = 200;
        assert_eq!(spanned.node, 200);
    }

    #[test]
    fn spanned_clone_and_eq() {
        let s1 = Spanned::new("test", 0..4);
        let s2 = s1.clone();
        assert_eq!(s1, s2);
    }
}
