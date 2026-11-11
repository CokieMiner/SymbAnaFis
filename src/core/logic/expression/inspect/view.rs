//! Logical views with lazy child projection.

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    iter::FusedIterator,
    slice::{Iter, from_ref},
};

use super::{Expr, ExprView, Node};

/// A lazy iterator over an expression's immediate logical children, borrowed:
/// operand vectors, then the operands of a quotient, power, or derivative.
///
/// A sum flattens its compact group into the group's terms, which are built
/// when first reached and kept with the storage.
#[derive(Clone)]
pub struct ExprChildren<'expr> {
    operands: Iter<'expr, Expr>,
    first: Option<&'expr Expr>,
    second: Option<&'expr Expr>,
    /// The remaining terms of the compact group being flattened.
    terms: Iter<'expr, Expr>,
    /// Whether the operands are additive terms.
    sum: bool,
}

impl Expr {
    /// The expression's mathematical structure. Compact polynomial storage
    /// appears as a sum whose terms are built when first iterated.
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

    /// Iterate the immediate logical children without collecting them.
    ///
    /// Function arguments and product factors keep their order; quotients and
    /// powers yield their two operands; a derivative yields its inner
    /// expression. Compact storage is the sum of its terms, which are built
    /// when first reached and kept with the storage; counting does not build
    /// them.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr, ExprView};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let expression = x.pow(2) + x.pow(3) + 1;
    /// assert!(matches!(expression.view(), ExprView::Sum(_)));
    /// let terms = expression.children();
    /// assert_eq!(terms.len(), 3);
    /// assert_eq!(Expr::sum(terms.cloned().collect()), expression);
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    #[must_use]
    pub fn children(&self) -> ExprChildren<'_> {
        let mut children = ExprChildren {
            operands: [].iter(),
            first: None,
            second: None,
            terms: [].iter(),
            sum: false,
        };
        match self.node() {
            Node::Sum(terms) => {
                children.operands = terms.iter();
                children.sum = true;
            }
            Node::Poly(_) => {
                children.operands = from_ref(self).iter();
                children.sum = true;
            }
            Node::Function { args, .. } | Node::Product(args) => children.operands = args.iter(),
            Node::Div(left, right) | Node::Pow(left, right) => {
                children.first = Some(left);
                children.second = Some(right);
            }
            Node::Derivative { inner, .. } => children.first = Some(inner),
            Node::Number(_) | Node::ImaginaryUnit | Node::Pi | Node::Euler | Node::Symbol(_) => {}
        }
        children
    }
}

impl Debug for ExprChildren<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.debug_list().entries(self.clone()).finish()
    }
}

impl<'expr> Iterator for ExprChildren<'expr> {
    type Item = &'expr Expr;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(term) = self.terms.next() {
                return Some(term);
            }
            let child = self
                .operands
                .next()
                .or_else(|| self.first.take())
                .or_else(|| self.second.take())?;
            if self.sum
                && let Node::Poly(polynomial) = child.node()
            {
                self.terms = polynomial.projected().iter();
            } else {
                return Some(child);
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
        // Structural order places the unique compact group between lower-ranked
        // terms and products, so the remaining operands locate it without a scan.
        let flattened = if self.sum {
            Node::compact_group(self.operands.as_slice()).map_or(0, |(_, polynomial)| {
                polynomial.coefficients.len().saturating_sub(1)
            })
        } else {
            0
        };
        // Each logical term occupies its own slot in an allocated vector or one
        // of two inline edges, so the total fits in the address space.
        self.operands
            .len()
            .saturating_add(flattened)
            .saturating_add(usize::from(self.first.is_some()))
            .saturating_add(usize::from(self.second.is_some()))
            .saturating_add(self.terms.len())
    }
}

impl FusedIterator for ExprChildren<'_> {}
