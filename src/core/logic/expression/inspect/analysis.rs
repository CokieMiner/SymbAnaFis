//! Structural queries over distinct DAG nodes.

use core::{convert::Infallible, iter::once};

use alloc::{format, string::String, sync::Arc, vec::Vec};

use crate::collections::{FixedSet, HashSet};

use super::{Expr, Function, Node, Postorder, Symbol, SymbolData, Visit};

impl Expr {
    /// Whether this expression contains no symbol or derivative variable.
    ///
    /// Constants include numeric literals, `i`, `pi`, `e`, and compound
    /// expressions built from them without symbols or derivative variables.
    #[must_use]
    pub fn is_constant(&self) -> bool {
        !Node::any(self, false, |node| variable(node).is_some())
    }

    /// The number of distinct nodes reachable from this expression, itself
    /// included. A shared subexpression counts once.
    #[must_use]
    pub fn node_count(&self) -> usize {
        let mut count = 0_usize;
        // Each reachable node owns a nonempty allocation, so their count fits
        // in the address space.
        Node::for_each(self, true, |_| count = count.saturating_add(1));
        count
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
        Node::any(self, false, |node| {
            variable(node).is_some_and(|data| data.id() == symbol.id())
        })
    }

    /// Whether a symbol named `name` from any context occurs, as a symbol or as
    /// a derivative variable.
    #[must_use]
    pub fn contains_var(&self, name: &str) -> bool {
        Node::any(self, false, |node| {
            variable(node).is_some_and(|data| data.name() == Some(name))
        })
    }

    /// Whether a variable occurs whose name is not in `excluded`. An anonymous
    /// symbol is named `$id`.
    #[must_use]
    pub fn has_free_variables(&self, excluded: &HashSet<String>) -> bool {
        // Format an anonymous name only when some `$id` name is excluded.
        let anonymous = excluded.iter().any(|name| name.starts_with('$'));
        Node::any(self, false, |node| {
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
        let mut symbols = HashSet::new();
        Node::for_each(self, false, |node| {
            if let Some(data) = variable(node) {
                symbols.insert(data.symbol());
            }
        });
        symbols
    }

    /// Whether a symbol identity outside `excluded` occurs.
    #[must_use]
    pub fn has_free_symbols(&self, excluded: &HashSet<Symbol<'_>>) -> bool {
        Node::any(self, false, |node| {
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

    /// The distinct function identities that occur in this expression.
    ///
    /// Builtin functions share one identity per variant; custom registrations
    /// stay distinct even when they share a spelling.
    #[must_use]
    pub fn functions(&self) -> HashSet<Function> {
        let mut functions = HashSet::new();
        Node::for_each(self, false, |node| {
            if let Node::Function { name, .. } = node.node() {
                functions.insert(name.clone());
            }
        });
        functions
    }

    /// Whether `function` occurs as a function call in this expression.
    #[must_use]
    pub fn contains_function(&self, function: &Function) -> bool {
        Node::any(
            self,
            false,
            |node| matches!(node.node(), Node::Function { name, .. } if name == function),
        )
    }

    /// The variable names in lexicographic order.
    #[must_use]
    pub fn variables_ordered(&self) -> Vec<String> {
        let (named, anonymous) = collect_variable_names(self);
        let mut variables = Vec::with_capacity(named.len().saturating_add(anonymous.len()));
        variables.extend(named.iter().copied().map(String::from));
        for id in anonymous {
            let formatted = format!("${id}");
            if !named.contains(formatted.as_str()) {
                variables.push(formatted);
            }
        }
        variables.sort_unstable();
        variables
    }

    /// The distinct variable names; an anonymous symbol appears as `$id`.
    #[must_use]
    pub fn variables(&self) -> HashSet<String> {
        let (named, anonymous) = collect_variable_names(self);
        let mut variables = HashSet::with_capacity(named.len().saturating_add(anonymous.len()));
        variables.extend(named.into_iter().map(String::from));
        variables.extend(anonymous.into_iter().map(|id| format!("${id}")));
        variables
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

fn collect_variable_names(expression: &Expr) -> (FixedSet<&str>, FixedSet<usize>) {
    // Deduplicate borrowed names and identities first, so one `String` is
    // allocated per distinct variable rather than per symbol node.
    let mut named: FixedSet<&str> = FixedSet::default();
    let mut anonymous: FixedSet<usize> = FixedSet::default();
    Node::for_each(expression, false, |node| {
        if let Some(data) = variable(node) {
            match data.name() {
                Some(name) => {
                    named.insert(name);
                }
                None => {
                    anonymous.insert(data.id());
                }
            }
        }
    });
    (named, anonymous)
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
