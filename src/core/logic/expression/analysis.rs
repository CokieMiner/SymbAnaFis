//! Queries on unique DAG nodes, independent of parked engines.

use core::iter::once;

use alloc::{format, string::String, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{Expr, Node, Symbol, SymbolData};

impl Expr {
    /// Count distinct reachable allocations, rather than repeated edges.
    #[must_use]
    pub fn node_count(&self) -> usize {
        // Each reachable node owns a nonempty allocation, so their count plus
        // the root fits in the address space.
        Node::descendants(self).len().saturating_add(1)
    }

    /// Return the longest stored DAG path, counting the root as depth one.
    /// This allocation metric does not materialize compact additive terms.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        let mut depths: HashMap<&Self, usize> = HashMap::new();
        for expression in Node::descendants(self) {
            let depth = Node::stored_depth(expression, &depths);
            depths.insert(expression, depth);
        }
        Node::stored_depth(self, &depths)
    }

    /// Test for a symbol identity, including an unevaluated derivative variable.
    #[must_use]
    pub fn contains_symbol(&self, symbol: Symbol<'_>) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| data.id() == symbol.id())
        })
    }

    /// Test for a named variable across any context identity.
    #[must_use]
    pub fn contains_var(&self, name: &str) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| data.name() == Some(name))
        })
    }

    /// Test for any variable whose display name is absent from `excluded`.
    #[must_use]
    pub fn has_free_variables(&self, excluded: &HashSet<String>) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| {
                data.name().map_or_else(
                    || !excluded.contains(&format!("${}", data.id())),
                    |name| !excluded.contains(name),
                )
            })
        })
    }

    /// Collect referenced identities without merging equal display names.
    /// Includes symbols used as unevaluated derivative variables.
    #[must_use]
    pub fn symbols(&self) -> HashSet<Symbol<'_>> {
        Node::descendants(self)
            .into_iter()
            .chain(once(self))
            .filter_map(|node| variable(node).map(SymbolData::symbol))
            .collect()
    }

    /// Test for a symbol outside an exclusion set of identities.
    #[must_use]
    pub fn has_free_symbols(&self, excluded: &HashSet<Symbol<'_>>) -> bool {
        Node::any(self, |node| {
            variable(node).is_some_and(|data| !excluded.contains(&data.symbol()))
        })
    }

    /// Borrow a named symbol's metadata, even after its registry entry is removed.
    #[must_use]
    pub fn symbol_name(&self) -> Option<&str> {
        if let Node::Symbol(data) = self.node() {
            data.name()
        } else {
            None
        }
    }

    /// Collect variable display names in lexicographic order.
    #[must_use]
    pub fn variables_ordered(&self) -> Vec<String> {
        let mut variables: Vec<_> = self.variables().into_iter().collect();
        variables.sort();
        variables
    }

    /// Collect distinct variable display names; anonymous variables use `$id`.
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

    /// Fold over distinct logical nodes in children-before-parent order.
    /// Compact monomials participate as expressions; additive storage groups
    /// do not introduce extra visits. Shared expressions are visited once.
    ///
    /// # Panics
    /// Propagates callback and numeric backend capacity panics.
    pub fn fold<Value>(&self, initial: Value, visit: impl FnMut(Value, &Self) -> Value) -> Value {
        let stored = Node::descendants(self);
        if matches!(self.node(), Node::Poly(_))
            || stored
                .iter()
                .any(|node| matches!(node.node(), Node::Poly(_)))
        {
            drop(stored);
            let (logical, _) = Node::logical_order(self);
            logical.iter().chain(once(self)).fold(initial, visit)
        } else {
            stored.into_iter().chain(once(self)).fold(initial, visit)
        }
    }
}

impl Node {
    fn stored_depth(expression: &Expr, depths: &HashMap<&Expr, usize>) -> usize {
        // An acyclic path uses distinct live allocations, each larger than one
        // byte. Its length fits usize, so this increment cannot saturate.
        Self::children(expression)
            .filter_map(|child| depths.get(child).copied())
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

fn variable(expression: &Expr) -> Option<&SymbolData> {
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
