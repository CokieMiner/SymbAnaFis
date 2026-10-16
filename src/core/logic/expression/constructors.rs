//! Validated public constructors with conservative normalization.

use alloc::{sync::Arc, vec::Vec};

use num_anafis::Number;

use super::{Expr, Node, NumericValue, SYMBOLS, Symbol, SymbolData};

impl Expr {
    /// Raise this expression to a power, sharing its base.
    ///
    /// # Panics
    /// Numeric folding propagates the backend's rational capacity panics.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Self>) -> Self {
        Self::pow_static(self.clone(), exponent.into())
    }

    /// Construct an unevaluated derivative with a globally named variable.
    ///
    /// # Panics
    /// Panics if the symbol registry's native-width IDs are exhausted.
    #[must_use]
    pub fn derivative(inner: Self, var: impl AsRef<str>, order: u32) -> Self {
        let data = SYMBOLS
            .intern(var.as_ref())
            .expect("derivative variable registration failed");
        Self::intern(Node::Derivative {
            inner,
            var: data,
            order,
        })
    }

    /// Construct an owning unevaluated derivative using a validated identity.
    #[must_use]
    pub fn derivative_symbol(inner: Self, var: Symbol<'_>, order: u32) -> Self {
        Self::intern(Node::Derivative {
            inner,
            var: SymbolData::for_symbol(var),
            order,
        })
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

    /// Flatten, order, and fold finite constants in a product.
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
    ///     let expression = context.symb("x").to_expr();
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

impl SymbolData {
    /// Intern an owning symbol node directly from its published metadata.
    pub fn to_expr(self: Arc<Self>) -> Expr {
        Expr::intern(Node::Symbol(self))
    }
}
