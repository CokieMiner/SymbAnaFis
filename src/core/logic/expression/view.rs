//! Logical views and lazy child projection.

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    iter::FusedIterator,
};

use super::{Expr, ExprChildren, ExprView, Node};

impl Expr {
    /// Inspect this expression's mathematical structure.
    /// Compact additive storage appears as a sum of lazy logical terms.
    #[must_use]
    pub fn view(&self) -> ExprView<'_> {
        match self.node() {
            Node::Number(value) => ExprView::Number(value.value()),
            Node::ImaginaryUnit => ExprView::ImaginaryUnit,
            Node::Pi => ExprView::Pi,
            Node::Euler => ExprView::Euler,
            Node::Symbol(data) => ExprView::Symbol(data.symbol()),
            Node::Function { name, args } => ExprView::Function { name, args },
            Node::Sum(_) | Node::Poly(_) => ExprView::Sum(self.children()),
            Node::Product(factors) => ExprView::Product(factors),
            Node::Div(left, right) => ExprView::Div(left, right),
            Node::Pow(base, exponent) => ExprView::Pow(base, exponent),
            Node::Derivative { inner, var, order } => ExprView::Derivative {
                inner,
                var: var.symbol(),
                order: order.get(),
            },
        }
    }

    /// Iterate immediate logical children without building a child list.
    ///
    /// Additive storage groups are flattened. Functions and products retain
    /// their ordered arguments and factors; powers expose their base and exponent.
    /// Compact monomials are reconstructed only when requested. Counting children
    /// does not reconstruct them. Returned handles can outlive this expression.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr, ExprView};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let expression = x.pow(2) + x.pow(3) + 1;
    /// assert!(matches!(expression.view(), ExprView::Sum(_)));
    /// let terms = expression.children();
    /// assert_eq!(terms.len(), 3);
    /// assert_eq!(Expr::sum(terms.collect()), expression);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    ///
    /// # Panics
    /// Reconstructing logical terms can propagate numeric backend capacity panics.
    #[must_use]
    pub fn children(&self) -> ExprChildren<'_> {
        let monomials = match self.node() {
            Node::Poly(polynomial) => polynomial.terms.iter(),
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => [].iter(),
        };
        ExprChildren {
            source: self,
            index: 0,
            monomials,
        }
    }
}

impl Debug for ExprChildren<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.debug_list().entries(self.clone()).finish()
    }
}

impl Iterator for ExprChildren<'_> {
    type Item = Expr;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(term) = self.monomials.next() {
                return Some(Node::mapped_monomial(term, Clone::clone));
            }
            let child = match self.source.node() {
                Node::Function { args, .. } | Node::Sum(args) | Node::Product(args) => {
                    args.get(self.index)
                }
                Node::Div(first, second) | Node::Pow(first, second) => match self.index {
                    0 => Some(first),
                    1 => Some(second),
                    _ => None,
                },
                Node::Derivative { inner, .. } if self.index == 0 => Some(inner),
                Node::Number(_)
                | Node::ImaginaryUnit
                | Node::Pi
                | Node::Euler
                | Node::Symbol(_)
                | Node::Poly(_)
                | Node::Derivative { .. } => None,
            }?;
            // A yielded slot belongs to an allocated vector or one of two
            // inline edges. Its successor fits in the native address space.
            self.index = self.index.saturating_add(1);
            if matches!(self.source.node(), Node::Sum(_))
                && let Node::Poly(polynomial) = child.node()
            {
                self.monomials = polynomial.terms.iter();
            } else {
                return Some(child.clone());
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.len();
        (count, Some(count))
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl ExactSizeIterator for ExprChildren<'_> {
    fn len(&self) -> usize {
        let stored = match self.source.node() {
            Node::Sum(terms) => {
                let tail = terms.get(self.index..).unwrap_or(&[]);
                // Structural order places the unique compact group between
                // lower-ranked terms and products. Locate it without a linear scan.
                let extra = Node::compact_group(tail)
                    .map_or(0, |polynomial| polynomial.terms.len().saturating_sub(1));
                tail.len().saturating_add(extra)
            }
            Node::Function { args, .. } | Node::Product(args) => {
                args.len().saturating_sub(self.index)
            }
            Node::Div(..) | Node::Pow(..) => 2_usize.saturating_sub(self.index),
            Node::Derivative { .. } => 1_usize.saturating_sub(self.index),
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Poly(_) => 0,
        };
        // A normalized sum has at most one compact additive group. Each logical
        // term occupies its own slot in that group or the outer vector, so the
        // total count fits in the address space and cannot saturate.
        stored.saturating_add(self.monomials.len())
    }
}

impl FusedIterator for ExprChildren<'_> {}
