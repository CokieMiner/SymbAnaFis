//! Binary operation constructors (Add, Sub, Mul, Div, Pow) with inline constant folding.

use std::sync::Arc;

use super::super::super::{EPSILON, Expr, ExprKind};

impl Expr {
    /// Create addition: a + b → Sum([a, b])
    #[must_use]
    pub fn add_expr(left: Self, right: Self) -> Self {
        if left.is_zero_num() {
            return right;
        }
        if right.is_zero_num() {
            return left;
        }
        if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
            return Self::number(l + r);
        }
        Self::sum(vec![left, right])
    }

    /// Create subtraction: a - b → Sum([a, Product([-1, b])])
    #[must_use]
    pub fn sub_expr(left: Self, right: Self) -> Self {
        if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
            return Self::number(l - r);
        }
        if left.is_zero_num() {
            return right.negate();
        }
        if right.is_zero_num() {
            return left;
        }
        let neg_right = Self::product(vec![Self::number(-1.0), right]);
        Self::sum(vec![left, neg_right])
    }

    /// Create multiplication: a * b → Product([a, b])
    #[must_use]
    pub fn mul_expr(left: Self, right: Self) -> Self {
        if left.is_zero_num() || right.is_zero_num() {
            return Self::number(0.0);
        }
        if left.is_one_num() {
            return right;
        }
        if right.is_one_num() {
            return left;
        }
        if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
            return Self::number(l * r);
        }

        if let (ExprKind::Poly(p1), ExprKind::Poly(p2)) = (&left.kind, &right.kind)
            && p1.base() == p2.base()
        {
            let result = p1.mul(p2);
            return Self::poly(result);
        }

        Self::product(vec![left, right])
    }

    /// Create multiplication from Arc operands (avoids Expr cloning)
    #[must_use]
    pub fn mul_from_arcs(factors: Vec<Arc<Self>>) -> Self {
        Self::product_from_arcs(factors)
    }

    /// Unwrap an `Arc<Expr>` without cloning if refcount is 1
    #[inline]
    #[must_use]
    pub fn unwrap_arc(arc: Arc<Self>) -> Self {
        Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone())
    }

    /// Create division
    #[must_use]
    pub fn div_expr(left: Self, right: Self) -> Self {
        if left == right && !left.is_zero_num() {
            return Self::number(1.0);
        }
        if let Some(m) = left.as_number()
            && let Some(n) = right.as_number()
            && n != 0.0
            && m % n == 0.0
        {
            return Self::number(m / n);
        }
        if right.is_one_num() {
            return left;
        }
        if left.is_zero_num() && !right.is_zero_num() {
            return Self::number(0.0);
        }
        Self::new(ExprKind::Div(Arc::new(left), Arc::new(right)))
    }

    /// Create division from Arc operands
    #[must_use]
    pub fn div_from_arcs(left: Arc<Self>, right: Arc<Self>) -> Self {
        if left.structural_hash() == right.structural_hash()
            && *left == *right
            && !left.is_zero_num()
        {
            return Self::number(1.0);
        }
        if right.is_one_num() {
            return Arc::try_unwrap(left).unwrap_or_else(|arc| (*arc).clone());
        }
        if left.is_zero_num() && !right.is_zero_num() {
            return Self::number(0.0);
        }
        Self::new(ExprKind::Div(left, right))
    }

    /// Create power expression (static constructor form)
    #[must_use]
    pub fn pow_static(base: Self, exponent: Self) -> Self {
        if exponent.is_zero_num() {
            return Self::number(1.0);
        }
        if exponent.is_one_num() {
            return base;
        }
        if base.is_one_num() {
            return Self::number(1.0);
        }
        if base.is_zero_num()
            && let Some(n) = exponent.as_number()
            && n > 0.0
        {
            return Self::number(0.0);
        }
        if let (Some(b), Some(e)) = (base.as_number(), exponent.as_number())
            && e >= 1.0
            && e.fract().abs() < EPSILON
        {
            let result = b.powf(e);
            if result.fract().abs() < EPSILON {
                return Self::number(result.round());
            }
        }
        Self::new(ExprKind::Pow(Arc::new(base), Arc::new(exponent)))
    }

    /// Create power from Arc operands
    #[must_use]
    pub fn pow_from_arcs(base: Arc<Self>, exponent: Arc<Self>) -> Self {
        if exponent.is_zero_num() {
            return Self::number(1.0);
        }
        if exponent.is_one_num() {
            return Arc::try_unwrap(base).unwrap_or_else(|arc| (*arc).clone());
        }
        if base.is_one_num() {
            return Self::number(1.0);
        }
        if base.is_zero_num()
            && let Some(n) = exponent.as_number()
            && n > 0.0
        {
            return Self::number(0.0);
        }
        Self::new(ExprKind::Pow(base, exponent))
    }
}
