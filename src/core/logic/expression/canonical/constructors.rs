//! Public constructors and literal predicates.

use core::num::NonZeroU32;

use alloc::{sync::Arc, vec::Vec};

use num_anafis::{Number, Real};

use crate::errors::SymbolError;

use super::{ExactIdentity, Expr, Node, NumericValue, SYMBOLS, Symbol, SymbolData};

impl Expr {
    /// `self` raised to `exponent`, in canonical form.
    ///
    /// Exact \(x^{1/2}\) becomes \(\sqrt{x}\) and \(e^x\) becomes \(\exp(x)\).
    /// A known negative real exponent moves the power to a denominator; an
    /// exponent of unknown sign stays a power.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Self>) -> Self {
        Node::normalize_power(self.clone(), exponent.into())
    }

    /// The canonical sum: flattened, ordered, finite literals folded, and equal
    /// terms collected. A zero coefficient keeps its base, so an undefined term
    /// is not erased.
    #[must_use]
    pub fn sum(terms: Vec<Self>) -> Self {
        Node::normalize_sum(terms)
    }

    /// The canonical product: flattened, ordered, finite literals folded, and
    /// denominators collected. Numeric coefficients stay outside shared symbolic
    /// quotients, and a zero factor does not erase symbolic or nonfinite factors.
    #[must_use]
    pub fn product(factors: Vec<Self>) -> Self {
        Node::normalize_product(factors)
    }

    /// A `NumAnaFis` literal, keeping its representation tier and float bits.
    #[must_use]
    pub fn number(value: impl Into<Number>) -> Self {
        let literal = NumericValue::new(value.into());
        if literal.gaussian() == Some((0, 1)) {
            Self::i()
        } else {
            Self::intern(Node::Number(literal))
        }
    }

    /// The exact imaginary unit \(i\).
    #[must_use]
    pub fn i() -> Self {
        Self::intern(Node::ImaginaryUnit)
    }

    /// The exact constant \(\pi\); it is not a symbol.
    #[must_use]
    pub fn pi() -> Self {
        Self::intern(Node::Pi)
    }

    /// Euler's number \(e\), exact; it is not a symbol.
    #[must_use]
    pub fn e() -> Self {
        Self::intern(Node::Euler)
    }

    /// The global symbol named `name`, created on first use.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when a new identity cannot be allocated.
    pub fn symbol(name: impl AsRef<str>) -> Result<Self, SymbolError> {
        Ok(Self::intern(Node::Symbol(Arc::clone(
            SYMBOLS.intern(name.as_ref())?,
        ))))
    }

    /// The symbol identity, if this expression is a symbol. The handle borrows
    /// this expression, which owns the metadata, so no registry lookup occurs.
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let x = {
    ///     let context = Context::new().expect("context");
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

    /// Whether this is a zero literal, including float zeros of either sign.
    #[must_use]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_zero)
    }

    /// Whether this is a literal equal to one.
    #[must_use]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_one)
    }

    /// Whether this is a literal equal to minus one.
    #[must_use]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_neg_one)
    }

    /// The literal value of a number or of the imaginary unit.
    #[must_use]
    pub fn as_number(&self) -> Option<&Number> {
        self.literal().map(NumericValue::value)
    }
}

impl ExactIdentity for Expr {
    fn is_exact_zero(&self) -> bool {
        self.literal().and_then(NumericValue::gaussian) == Some((0, 0))
    }

    fn is_exact_one(&self) -> bool {
        self.literal().and_then(NumericValue::gaussian) == Some((1, 0))
    }

    fn exact_ratio(&self) -> Option<(i64, u64)> {
        match self.as_number()?.components() {
            (Real::Int(numerator), Real::Int(0)) => Some((*numerator, 1)),
            (Real::Rational(ratio), Real::Int(0)) => Some((*ratio.numer(), *ratio.denom())),
            _ => None,
        }
    }

    fn positive_factor(&self) -> Option<(&Self, i64)> {
        match self.node() {
            Node::Pow(base, exponent) => match exponent.exact_ratio() {
                Some((count, 1)) if count > 0 => Some((base, count)),
                _ => None,
            },
            Node::Number(_) | Node::ImaginaryUnit => None,
            Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Derivative { .. } => Some((self, 1)),
        }
    }
}

impl SymbolData {
    /// The symbol expression for this metadata.
    pub fn to_expr(self: Arc<Self>) -> Expr {
        Expr::intern(Node::Symbol(self))
    }

    /// The unevaluated derivative \(\partial^{n} f / \partial v^{n}\) of
    /// \(f =\) `inner` in this variable \(v\), with \(n =\) `order`. Nested
    /// derivatives in the same variable combine,
    /// \(\partial_{v}^{m} \partial_{v}^{n} f = \partial_{v}^{m+n} f\), while
    /// \(m + n\) fits `u32`.
    pub fn derivative_of(self: Arc<Self>, mut inner: Expr, mut order: NonZeroU32) -> Expr {
        while let Node::Derivative {
            inner: child,
            var: previous,
            order: count,
        } = inner.node()
        {
            if *previous != self {
                break;
            }
            let Some(combined) = order.checked_add(count.get()) else {
                break;
            };
            order = combined;
            inner = child.clone();
        }
        Expr::intern(Node::Derivative {
            inner,
            var: self,
            order,
        })
    }
}
