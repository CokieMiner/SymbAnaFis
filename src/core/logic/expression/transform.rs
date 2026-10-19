//! Memoized DAG reconstruction and original-node substitution.

use alloc::{vec, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{
    ConcreteMatcher, ExactSources, Expr, FunctionDefinition, Node, Polynomial, Substitution,
    SubstitutionInput, Substitutions, Symbol,
};

impl Substitution {
    /// Replace exact nodes and disjoint explicit or counted arithmetic groups.
    ///
    /// A matched node replaces its entire subtree without traversing its
    /// descendants. Compact polynomial monomials use their logical expression
    /// projection. Every surviving edge to a shared node receives the same
    /// memoized result; destinations are not substituted again.
    ///
    /// Matching uses exact power groups that preserve branches and zero exclusions,
    /// common additive factors, and scalar coefficient quotients.
    /// Quotients are reduced before approximate
    /// promotion. Unmatched bases remain original traversal inputs; generated
    /// coefficient quotients and exponent literals are kept out of selection.
    ///
    /// # Panics
    /// Logical projection and changed-parent reconstruction can propagate
    /// numeric backend capacity panics.
    #[must_use]
    pub fn apply_all(&self, expression: &Expr) -> Expr {
        if self.from() == self.to() {
            return expression.clone();
        }
        let matcher = ConcreteMatcher::new(ExactSources::One(self));
        expression.substitute_logical_with(|node| matcher.select(node))
    }
}

impl Substitutions {
    /// Apply all entries simultaneously, matching each original node once.
    ///
    /// Matched parents take precedence over descendant substitutions. Neither
    /// inserted destinations nor nodes created by rebuilding are matched
    /// during this pass. Empty sets and unchanged nodes retain their pointers.
    ///
    /// # Panics
    /// Logical projection and changed-parent reconstruction can propagate
    /// numeric backend capacity panics.
    #[must_use]
    pub fn apply_all(&self, expression: &Expr) -> Expr {
        if self
            .iter()
            .all(|(source, destination)| source == destination)
        {
            return expression.clone();
        }
        let matcher = ConcreteMatcher::new(ExactSources::Many(self));
        expression.substitute_logical_with(|node| matcher.select(node))
    }
}

impl Expr {
    /// Transform distinct logical nodes once, after their children.
    ///
    /// Polynomial monomials are materialized as shared handles before callbacks
    /// run. Each distinct original logical node is visited once, bottom-up;
    /// inserted replacements are not traversed. Unchanged polynomials retain
    /// their pointers. Additive storage groups do not create extra callback visits.
    ///
    /// An error discards the partial output and leaves the input DAG intact.
    /// Callback side effects preceding the error are not rolled back.
    ///
    /// # Errors
    /// Returns the first error supplied by the callback.
    ///
    /// # Panics
    /// Propagates callback and numeric backend capacity panics.
    pub fn map<Error>(
        &self,
        mut transform: impl FnMut(&Self) -> Result<Option<Self>, Error>,
    ) -> Result<Self, Error> {
        let stored = Node::descendants(self);
        if !matches!(self.node(), Node::Poly(_))
            && !stored
                .iter()
                .any(|node| matches!(node.node(), Node::Poly(_)))
        {
            let mut memo = HashMap::new();
            for original in stored {
                let rebuilt = Node::rebuild_with(original, |child| {
                    memo.get(child).expect("children precede their parent")
                });
                let replacement = transform(&rebuilt)?.unwrap_or(rebuilt);
                memo.insert(original, replacement);
            }
            let rebuilt = Node::rebuild_with(self, |child| {
                memo.get(child).expect("children precede their parent")
            });
            return Ok(transform(&rebuilt)?.unwrap_or(rebuilt));
        }
        drop(stored);
        let (order, projections) = Node::logical_order(self);
        let mut memo = HashMap::new();
        for original in order {
            let rebuilt = Node::rebuild_logical(&original, &memo, &projections);
            let replacement = transform(&rebuilt)?.unwrap_or(rebuilt);
            memo.insert(original, replacement);
        }
        let rebuilt = Node::rebuild_logical(self, &memo, &projections);
        Ok(transform(&rebuilt)?.unwrap_or(rebuilt))
    }

    /// Replace symbol expression nodes with this identity across all DAG edges.
    /// A derivative's differentiation-variable metadata retains its identity.
    ///
    /// # Panics
    /// Rebuilding changed parents can propagate numeric backend capacity panics.
    #[must_use]
    pub fn substitute_symbol(&self, symbol: Symbol<'_>, replacement: &Self) -> Self {
        self.substitute_with(|node| (node.as_symbol() == Some(symbol)).then(|| replacement.clone()))
    }

    /// Substitute symbol identities simultaneously in one DAG pass.
    /// Selection uses original nodes; inserted values and parents that normalize
    /// to an inserted symbol are not substituted again in this pass.
    ///
    /// # Panics
    /// Rebuilding changed parents can propagate numeric backend capacity panics.
    #[must_use]
    pub fn substitute_symbols(&self, replacements: &HashMap<Symbol<'_>, Self>) -> Self {
        self.substitute_with(|node| {
            node.as_symbol()
                .and_then(|symbol| replacements.get(&symbol))
                .cloned()
        })
    }

    /// Replace symbol expression nodes with this name across context identities.
    /// Differentiation-variable metadata retains its identity.
    /// Use `substitute_symbol` when the identity matters.
    ///
    /// # Panics
    /// Rebuilding changed parents can propagate numeric backend capacity panics.
    #[must_use]
    pub fn substitute(&self, name: &str, replacement: &Self) -> Self {
        self.substitute_with(|node| {
            if let Node::Symbol(data) = node.node()
                && data.name() == Some(name)
            {
                Some(replacement.clone())
            } else {
                None
            }
        })
    }

    fn substitute_with(&self, mut select: impl FnMut(&Self) -> Option<Self>) -> Self {
        let mut memo = HashMap::new();
        for original in Node::descendants(self) {
            let replacement = select(original).unwrap_or_else(|| {
                Node::rebuild_with(original, |child| {
                    memo.get(child).expect("children precede their parent")
                })
            });
            memo.insert(original, replacement);
        }
        select(self).unwrap_or_else(|| {
            Node::rebuild_with(self, |child| {
                memo.get(child).expect("children precede their parent")
            })
        })
    }
}

impl SubstitutionInput<'_> {
    pub fn expand_children(
        &self,
        pending: &mut Vec<(Self, bool)>,
        projections: &mut HashMap<Expr, Vec<Expr>>,
    ) {
        let original = match self {
            Self::Stored(original) => *original,
            Self::Logical(original) => original,
        };
        if matches!(original.node(), Node::Poly(_))
            || matches!(original.node(), Node::Sum(terms) if Node::compact_group(terms).is_some())
        {
            let terms = original.children().collect::<Vec<_>>();
            pending.extend(
                terms
                    .iter()
                    .cloned()
                    .map(|term| (Self::Logical(term), false)),
            );
            projections.insert(original.clone(), terms);
            return;
        }
        match self {
            Self::Stored(stored) => {
                pending.extend(Node::children(stored).map(|child| (Self::Stored(child), false)));
            }
            Self::Logical(projected) => {
                pending.extend(
                    Node::children(projected)
                        .cloned()
                        .map(|child| (Self::Logical(child), false)),
                );
            }
        }
    }
}

/// Produce a children-before-parent sequence, visiting each canonical node once.
impl Node {
    /// Visit strict descendants before their parents; retain the root separately.
    pub fn descendants(root: &Expr) -> Vec<&Expr> {
        let mut visited = HashSet::new();
        let mut pending: Vec<_> = Self::children(root).map(|child| (child, false)).collect();
        let mut result = Vec::new();
        while let Some((expression, ready)) = pending.pop() {
            if ready {
                result.push(expression);
            } else if visited.insert(expression) {
                pending.push((expression, true));
                pending.extend(Self::children(expression).map(|child| (child, false)));
            }
        }
        result
    }

    /// Search unique nodes with early exit, without constructing a full postorder.
    pub fn any(root: &Expr, mut predicate: impl FnMut(&Expr) -> bool) -> bool {
        let mut visited = HashSet::new();
        let mut pending = vec![root];
        while let Some(expression) = pending.pop() {
            if visited.insert(expression) {
                if predicate(expression) {
                    return true;
                }
                pending.extend(Self::children(expression));
            }
        }
        false
    }

    pub fn logical_order(root: &Expr) -> (Vec<Expr>, HashMap<Expr, Vec<Expr>>) {
        let mut visited = HashSet::new();
        let mut pending = Vec::new();
        let mut result = Vec::new();
        let mut projections = HashMap::new();
        SubstitutionInput::Stored(root).expand_children(&mut pending, &mut projections);
        while let Some((input, ready)) = pending.pop() {
            let expression = match &input {
                SubstitutionInput::Stored(expression) => *expression,
                SubstitutionInput::Logical(expression) => expression,
            };
            if ready {
                result.push(expression.clone());
            } else if visited.insert(expression.clone()) {
                pending.push((input.clone(), true));
                input.expand_children(&mut pending, &mut projections);
            }
        }
        (result, projections)
    }

    pub fn rebuild_logical(
        original: &Expr,
        memo: &HashMap<Expr, Expr>,
        projections: &HashMap<Expr, Vec<Expr>>,
    ) -> Expr {
        let get = |child: &Expr| {
            memo.get(child)
                .expect("logical children precede their parent")
        };
        projections.get(original).map_or_else(
            || Self::rebuild_with(original, get),
            |terms| {
                if terms.iter().all(|term| term == get(term)) {
                    original.clone()
                } else {
                    Expr::sum(terms.iter().map(|term| get(term).clone()).collect())
                }
            },
        )
    }

    fn rebuild_with<'mapped>(original: &Expr, get: impl Fn(&Expr) -> &'mapped Expr) -> Expr {
        if Self::children(original).all(|child| child == get(child)) {
            return original.clone();
        }
        let mapped = |child: &Expr| get(child).clone();
        match original.node() {
            Self::Number(_) | Self::ImaginaryUnit | Self::Pi | Self::Euler | Self::Symbol(_) => {
                original.clone()
            }
            Self::Function { name, args } => {
                FunctionDefinition::construct(name.clone(), args.iter().map(mapped).collect())
            }
            Self::Sum(args) => Expr::sum(args.iter().map(mapped).collect()),
            Self::Poly(polynomial) => Expr::sum(Self::mapped_polynomial(polynomial, mapped)),
            Self::Product(args) => Expr::product(args.iter().map(mapped).collect()),
            Self::Div(left, right) => Self::normalize_quotient(mapped(left), mapped(right)),
            Self::Pow(base, exponent) => Self::normalize_power(mapped(base), mapped(exponent)),
            Self::Derivative { inner, var, order } => {
                Expr::derivative(mapped(inner), var.symbol(), order.get())
            }
        }
    }

    /// Iterate borrowed edges without allocating a child list.
    pub fn children(expression: &Expr) -> impl Iterator<Item = &Expr> {
        let polynomial = if let Self::Poly(polynomial) = expression.node() {
            Some(polynomial)
        } else {
            None
        };
        let (many, first, second): (&[Expr], Option<&Expr>, Option<&Expr>) = match expression.node()
        {
            Self::Number(_) | Self::ImaginaryUnit | Self::Pi | Self::Euler | Self::Symbol(_) => {
                (&[], None, None)
            }
            Self::Function { args, .. } | Self::Sum(args) | Self::Product(args) => {
                (args, None, None)
            }
            Self::Div(left, right) | Self::Pow(left, right) => (&[], Some(left), Some(right)),
            Self::Derivative { inner, .. } => (&[], Some(inner), None),
            Self::Poly(_) => (&[], None, None),
        };
        many.iter()
            .chain(first)
            .chain(second)
            .chain(polynomial.into_iter().flat_map(Polynomial::children))
    }
}
