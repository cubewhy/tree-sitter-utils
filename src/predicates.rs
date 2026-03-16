//! [`NodePredicate`] trait and built-in predicate constructors.

use crate::input::Input;

/// A predicate over an [`Input`] — returns `true` when the node matches.
///
/// Any `F: Fn(Input<'_, Ctx>) -> bool + Send + Sync` implements this trait
/// automatically via the blanket implementation, so plain closures work too.
///
/// Use the free-function constructors ([`kind_is`], [`kind_is_not`], …) to
/// obtain named predicate values, or supply any compatible closure directly.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{NodePredicate, kind_is};
///
/// fn accepts_pred<Ctx: Copy, P: NodePredicate<Ctx>>(_: P) {}
/// accepts_pred::<(), _>(kind_is(&["identifier"]));
/// accepts_pred::<(), _>(|input: tree_sitter_combinator::Input<()>| input.node.kind() == "identifier");
/// ```
pub trait NodePredicate<Ctx>: Send + Sync {
    /// Test whether the predicate holds for the given input.
    fn test(&self, input: Input<'_, Ctx>) -> bool;
}

/// Blanket impl: every `Fn(Input<'_, Ctx>) -> bool + Send + Sync` is a predicate.
impl<Ctx, F> NodePredicate<Ctx> for F
where
    F: Fn(Input<'_, Ctx>) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        self(input)
    }
}

// ---------------------------------------------------------------------------
// Built-in predicate structs
// ---------------------------------------------------------------------------

/// Predicate: `true` when `node.kind()` is one of the given static `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is;
/// let pred = kind_is(&["identifier", "type_identifier"]);
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct KindIs(pub &'static [&'static str]);

impl<Ctx> NodePredicate<Ctx> for KindIs {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        self.0.contains(&input.node.kind())
    }
}

/// Predicate: `true` when `node.kind()` is **not** in the given `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is_not;
/// let pred = kind_is_not(&["comment", "ERROR"]);
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct KindIsNot(pub &'static [&'static str]);

impl<Ctx> NodePredicate<Ctx> for KindIsNot {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        !self.0.contains(&input.node.kind())
    }
}

/// Predicate: `true` when `node.parent()` has the given `kind`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::has_parent_kind;
/// let pred = has_parent_kind("function_definition");
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct HasParentKind(pub &'static str);

impl<Ctx> NodePredicate<Ctx> for HasParentKind {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        input.node.parent().is_some_and(|p| p.kind() == self.0)
    }
}

/// Predicate: `true` when the node depth (root = 0) is at most `max`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::node_depth_lte;
/// let pred = node_depth_lte(3);
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct NodeDepthLte(pub usize);

impl<Ctx> NodePredicate<Ctx> for NodeDepthLte {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        let mut depth = 0usize;
        let mut current = input.node;
        while let Some(parent) = current.parent() {
            depth += 1;
            if depth > self.0 {
                return false;
            }
            current = parent;
        }
        depth <= self.0
    }
}

// ---------------------------------------------------------------------------
// Public constructor functions
// ---------------------------------------------------------------------------

/// Returns a predicate that is `true` when `node.kind()` is in `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is;
/// let _ = kind_is(&["identifier"]);
/// ```
#[inline]
pub fn kind_is(kinds: &'static [&'static str]) -> KindIs {
    KindIs(kinds)
}

/// Returns a predicate that is `true` when `node.kind()` is **not** in `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is_not;
/// let _ = kind_is_not(&["comment", "ERROR"]);
/// ```
#[inline]
pub fn kind_is_not(kinds: &'static [&'static str]) -> KindIsNot {
    KindIsNot(kinds)
}

/// Returns a predicate that is `true` when the node's parent has `kind`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::has_parent_kind;
/// let _ = has_parent_kind("call_expression");
/// ```
#[inline]
pub fn has_parent_kind(kind: &'static str) -> HasParentKind {
    HasParentKind(kind)
}

/// Returns a predicate that is `true` when the node's tree-depth ≤ `max`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::node_depth_lte;
/// let _ = node_depth_lte(5);
/// ```
#[inline]
pub fn node_depth_lte(max: usize) -> NodeDepthLte {
    NodeDepthLte(max)
}

/// Predicate: `true` when **any strict ancestor** of the node has the given
/// `kind`.
///
/// Unlike [`HasParentKind`], which only inspects the immediate parent, this
/// predicate walks the full ancestry chain from the node up to the root and
/// returns `true` as soon as it finds a node whose kind matches.
///
/// The node itself is **not** tested — only its strict ancestors.
///
/// This is the predicate counterpart of the java parser's `find_ancestor`
/// utility. Use it to write guards such as:
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, has_ancestor_kind, Input};
///
/// // Only fire when the node lives somewhere inside an `argument_list`.
/// let h = handler_fn(|_: Input<()>| "inside arg list".to_owned())
///     .when(has_ancestor_kind("argument_list"));
/// let _ = h;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct HasAncestorKind(pub &'static str);

impl<Ctx> NodePredicate<Ctx> for HasAncestorKind {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        let mut current = input.node.parent();
        while let Some(ancestor) = current {
            if ancestor.kind() == self.0 {
                return true;
            }
            current = ancestor.parent();
        }
        false
    }
}

/// Returns a predicate that is `true` when **any strict ancestor** of the
/// node has the given `kind`.
///
/// See [`HasAncestorKind`] for the full semantics.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::has_ancestor_kind;
/// let _ = has_ancestor_kind("lambda_expression");
/// ```
#[inline]
pub fn has_ancestor_kind(kind: &'static str) -> HasAncestorKind {
    HasAncestorKind(kind)
}
