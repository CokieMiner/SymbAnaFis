//! Structural queries over distinct DAG nodes.

use core::{convert::Infallible, iter::once};

use alloc::{format, string::String, sync::Arc, vec::Vec};

use crate::collections::HashSet;

use super::{Expr, Node, Postorder, Symbol, SymbolData, Visit};

impl Expr {
    /// The number of distinct nodes reachable from this expression, itself
    /// included. A shared subexpression counts once.
    #[must_use]
    pub fn node_count(&self) -> usize {
        // Each reachable node owns a nonempty allocation, so their count plus
        // the root fits in the address space.
        Node::descendants(self).len().saturating_add(1)
    }

    /// The number of nodes on the longest stored path from this expression,
    /// so a leaf has depth one. Compact polynomial terms are not expanded.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        let Ok(depth) = Postorder::stored(self).evaluate(|node| {
            // An acyclic path uses distinct live allocations, each larger than
            // one byte, so its length fits `usize` and cannot saturate.
            Ok::<_, Infallible>(
                node.values()
                    .copied()
                    .max()
                    .unwrap_or(0_usize)
                    .saturating_add(1),
            )
        });
        depth
    }

    /// Whether `symbol` occurs, as a symbol or as a derivative variable.
    #[must_use]
    pub fn contains_symbol(&self, symbol: Symbol<'_>) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| data.id() == symbol.id())
        })
    }

    /// Whether a symbol named `name` from any context occurs, as a symbol or as
    /// a derivative variable.
    #[must_use]
    pub fn contains_var(&self, name: &str) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| data.name() == Some(name))
        })
    }

    /// Whether a variable occurs whose name is not in `excluded`. An anonymous
    /// symbol is named `$id`.
    #[must_use]
    pub fn has_free_variables(&self, excluded: &HashSet<String>) -> bool {
        // Format an anonymous name only when some `$id` name is excluded.
        let anonymous = excluded.iter().any(|name| name.starts_with('$'));
        Node::any(self, |node| {
            variable(node).is_some_and(|data| {
                data.name().map_or_else(
                    || !anonymous || !excluded.contains(&format!("${}", data.id())),
                    |name| !excluded.contains(name),
                )
            })
        })
    }

    /// The distinct symbol identities that occur, derivative variables
    /// included. Equal names from different contexts stay distinct.
    #[must_use]
    pub fn symbols(&self) -> HashSet<Symbol<'_>> {
        Node::descendants(self)
            .into_iter()
            .chain(once(self))
            .filter_map(|node| variable(node).map(SymbolData::symbol))
            .collect()
    }

    /// Whether a symbol identity outside `excluded` occurs.
    #[must_use]
    pub fn has_free_symbols(&self, excluded: &HashSet<Symbol<'_>>) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| !excluded.contains(&data.symbol()))
        })
    }

    /// The name of this symbol, which stays available after the name is unbound.
    #[must_use]
    pub fn symbol_name(&self) -> Option<&str> {
        if let Node::Symbol(data) = self.node() {
            data.name()
        } else {
            None
        }
    }

    /// The variable names in lexicographic order.
    #[must_use]
    pub fn variables_ordered(&self) -> Vec<String> {
        let mut variables: Vec<_> = self.variables().into_iter().collect();
        variables.sort();
        variables
    }

    /// The distinct variable names; an anonymous symbol appears as `$id`.
    #[must_use]
    pub fn variables(&self) -> HashSet<String> {
        Node::descendants(self)
            .into_iter()
            .chain(once(self))
            .filter_map(|node| {
                variable(node).map(|data| {
                    data.name()
                        .map_or_else(|| format!("${}", data.id()), String::from)
                })
            })
            .collect()
    }

    /// Fold over each distinct logical node once, children before parents.
    /// Compact polynomial terms are visited as ordinary terms.
    ///
    /// # Panics
    /// Propagates callback panics.
    pub fn fold<Value>(&self, initial: Value, visit: impl FnMut(Value, &Self) -> Value) -> Value {
        let order = Postorder::logical(self);
        order
            .descendants()
            .iter()
            .map(Visit::expression)
            .chain(once(self))
            .fold(initial, visit)
    }
}

fn variable(expression: &Expr) -> Option<&Arc<SymbolData>> {
    match expression.node() {
        Node::Symbol(data) | Node::Derivative { var: data, .. } => Some(data),
        Node::Number(_)
        | Node::ImaginaryUnit
        | Node::Pi
        | Node::Euler
        | Node::Function { .. }
        | Node::Sum(_)
        | Node::Poly(_)
        | Node::Product(_)
        | Node::Div(..)
        | Node::Pow(..) => None,
    }
}
