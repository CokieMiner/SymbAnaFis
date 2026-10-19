//! Validated public constructors with conservative normalization.

use core::{cell::OnceCell, num::NonZeroU32};

use alloc::{sync::Arc, vec::Vec};

use num_anafis::Number;

use crate::diff::Differentiation;

use super::{Expr, Node, NumericValue, SYMBOLS, Symbol, SymbolData};

impl Expr {
    /// Raise this expression to a power through canonical scalar construction.
    /// Exact \(x^{1/2}\) uses \(\sqrt{x}\), and \(e^x\) uses \(\exp(x)\).
    /// Known negative real exponents use denominators; unknown signs stay powers.
    ///
    /// # Panics
    /// Numeric folding propagates the backend's rational capacity panics.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Self>) -> Self {
        Node::normalize_power(self.clone(), exponent.into())
    }

    /// Construct \(\partial_{var}^{order} inner\) using a validated symbol identity.
    ///
    /// Supported derivatives evaluate immediately using regular principal-branch
    /// formulas. Input-domain restrictions and limiting extensions are not stored.
    /// Analytic formulas apply where their factors are regular; unsupported cases
    /// retain derivative nodes. No separate simplification pass runs.
    /// Order zero returns `inner`. Unevaluated orders in the same variable combine
    /// within `u32` capacity; larger totals retain their nesting.
    ///
    /// # Panics
    /// Propagates custom partial callback and numeric backend capacity panics.
    #[must_use]
    pub fn derivative(inner: Self, var: Symbol<'_>, order: u32) -> Self {
        let Some(mut remaining) = NonZeroU32::new(order) else {
            return inner;
        };
        let metadata = OnceCell::new();
        let mut current = inner;
        let mut checkpoint = (order > 1).then(|| current.clone());
        let mut span = 0_u32;
        let mut window = 1_u32;
        loop {
            let Some(next) = Differentiation::new(var).evaluate(&current, |child| {
                Node::normalize_derivative(
                    child.clone(),
                    Arc::clone(metadata.get_or_init(|| SymbolData::for_symbol(var))),
                    NonZeroU32::MIN,
                )
            }) else {
                let owner = metadata
                    .into_inner()
                    .unwrap_or_else(|| SymbolData::for_symbol(var));
                return Node::normalize_derivative(current, owner, remaining);
            };
            if next.is_zero_num() || next == current {
                return next;
            }
            current = next;
            let Some(rest) = remaining.get().checked_sub(1).and_then(NonZeroU32::new) else {
                return current;
            };
            remaining = rest;
            // Deterministic partials make repeated derivative states periodic.
            // Brent's cycle detection retains one checkpoint handle.
            // Span cannot exceed the requested u32 order before it is reset.
            span = span.saturating_add(1);
            if checkpoint.as_ref() == Some(&current) {
                let Some(reduced) = remaining.get().checked_rem(span).and_then(NonZeroU32::new)
                else {
                    return current;
                };
                remaining = reduced;
                span = 0;
                checkpoint = Some(current.clone());
            } else if span == window {
                checkpoint = Some(current.clone());
                span = 0;
                window = window.saturating_mul(2);
            }
        }
    }

    /// Flatten, order, fold finite constants, and collect equal additive terms.
    /// Zero coefficients retain their base, preserving undefined values.
    ///
    /// # Panics
    /// Numeric folding propagates the backend's rational capacity panics.
    #[must_use]
    pub fn sum(terms: Vec<Self>) -> Self {
        Node::normalize_sum(terms)
    }

    /// Flatten, order, fold finite constants, and collect product denominators.
    /// Numeric weights stay outside shared symbolic quotients.
    /// A zero factor does not erase symbolic or nonfinite factors.
    ///
    /// # Panics
    /// Numeric folding propagates the backend's rational capacity panics.
    #[must_use]
    pub fn product(factors: Vec<Self>) -> Self {
        Node::normalize_product(factors)
    }

    /// Intern a `NumAnaFis` literal, preserving exact tiers and float component bits.
    #[must_use]
    pub fn number(value: impl Into<Number>) -> Self {
        let literal = NumericValue::new(value.into());
        if literal.gaussian() == Some((0, 1)) {
            Self::i()
        } else {
            Self::intern(Node::Number(literal))
        }
    }

    /// Construct the shared exact scalar imaginary unit.
    #[must_use]
    pub fn i() -> Self {
        Self::intern(Node::ImaginaryUnit)
    }

    /// Construct the shared exact mathematical constant pi, without a symbol ID.
    #[must_use]
    pub fn pi() -> Self {
        Self::intern(Node::Pi)
    }

    /// Construct the shared exact Euler number e, without a symbol ID.
    #[must_use]
    pub fn e() -> Self {
        Self::intern(Node::Euler)
    }

    /// Get or create a globally named symbol expression.
    ///
    /// # Panics
    /// Panics if the symbol registry's native-width IDs are exhausted.
    #[must_use]
    pub fn symbol(name: impl AsRef<str>) -> Self {
        Self::intern(Node::Symbol(
            SYMBOLS
                .intern(name.as_ref())
                .expect("symbol registration failed"),
        ))
    }

    /// Read a symbol's identity without consulting the registry.
    /// The Copy handle borrows this expression, which owns its metadata.
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let x = {
    ///     let context = Context::new();
    ///     let expression = context.symb("x").expect("symbol allocation").to_expr();
    ///     expression.as_symbol().expect("symbol expression")
    /// };
    /// let expression = x + 1;
    /// ```
    #[must_use]
    pub fn as_symbol(&self) -> Option<Symbol<'_>> {
        if let Node::Symbol(data) = self.node() {
            Some(data.symbol())
        } else {
            None
        }
    }

    /// Test for exactly zero, including either signed zero.
    #[must_use]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_zero)
    }

    /// Test for exactly one.
    #[must_use]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_one)
    }

    /// Test for exactly negative one.
    #[must_use]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_neg_one)
    }

    /// Read a literal, if present.
    #[must_use]
    pub fn as_number(&self) -> Option<&Number> {
        self.literal().map(NumericValue::value)
    }
}

impl Node {
    /// Combine \(\partial_{x}^{m}\partial_{x}^{n} f=\partial_{x}^{m+n} f\) within capacity.
    pub fn normalize_derivative(
        mut inner: Expr,
        var: Arc<SymbolData>,
        mut order: NonZeroU32,
    ) -> Expr {
        while let Self::Derivative {
            inner: child,
            var: previous,
            order: count,
        } = inner.node()
        {
            if previous != &var {
                break;
            }
            let Some(combined) = order.checked_add(count.get()) else {
                break;
            };
            order = combined;
            inner = child.clone();
        }
        Expr::intern(Self::Derivative { inner, var, order })
    }
}

impl SymbolData {
    /// Intern an owning symbol node directly from its published metadata.
    pub fn to_expr(self: Arc<Self>) -> Expr {
        Expr::intern(Node::Symbol(self))
    }
}
