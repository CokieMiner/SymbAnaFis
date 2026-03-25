//! Operator overloading for `Expr`.
//!
//! Contains `Add`, `Sub`, `Mul`, `Div`, and `Neg` implementations
//! for `Expr` combined with other `Expr`s, references, and primitives.

use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::Arc;

use crate::Expr;

// ============================================================================
// Operator Overloading Macro
// ============================================================================

/// Implement binary operations for expressions
macro_rules! impl_binary_ops_expr {
    ($lhs:ty, $rhs:ty, $to_lhs:expr, $to_rhs:expr) => {
        impl Add<$rhs> for $lhs {
            type Output = Expr;
            #[inline]
            fn add(self, rhs: $rhs) -> Expr {
                Expr::add_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Sub<$rhs> for $lhs {
            type Output = Expr;
            #[inline]
            fn sub(self, rhs: $rhs) -> Expr {
                Expr::sub_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Mul<$rhs> for $lhs {
            type Output = Expr;
            #[inline]
            fn mul(self, rhs: $rhs) -> Expr {
                Expr::mul_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Div<$rhs> for $lhs {
            type Output = Expr;
            #[inline]
            fn div(self, rhs: $rhs) -> Expr {
                Expr::div_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
    };
}

// Expr operations
impl_binary_ops_expr!(Expr, Expr, |s: Expr| s, |r: Expr| r);
impl_binary_ops_expr!(Expr, f64, |s: Expr| s, |r: f64| Expr::number(r));
impl_binary_ops_expr!(Expr, &Expr, |s: Expr| s, |r: &Expr| r.clone());

// &Expr operations (reference)
impl_binary_ops_expr!(&Expr, &Expr, |e: &Expr| e.clone(), |r: &Expr| r.clone());
impl_binary_ops_expr!(&Expr, Expr, |e: &Expr| e.clone(), |r: Expr| r);
impl_binary_ops_expr!(&Expr, f64, |e: &Expr| e.clone(), |r: f64| Expr::number(r));

// =============================================================================
// f64 operators (Expr on right side)
// =============================================================================

impl Add<Expr> for f64 {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add_expr(Expr::number(self), rhs)
    }
}

impl Mul<Expr> for f64 {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs)
    }
}

impl Sub<Expr> for f64 {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs)
    }
}

impl Div<Expr> for f64 {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div_expr(Expr::number(self), rhs)
    }
}

// f64 on left side with &Expr
impl Add<&Expr> for f64 {
    type Output = Expr;
    fn add(self, rhs: &Expr) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.clone())
    }
}

impl Mul<&Expr> for f64 {
    type Output = Expr;
    fn mul(self, rhs: &Expr) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.clone())
    }
}

impl Sub<&Expr> for f64 {
    type Output = Expr;
    fn sub(self, rhs: &Expr) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.clone())
    }
}

impl Div<&Expr> for f64 {
    type Output = Expr;
    fn div(self, rhs: &Expr) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.clone())
    }
}

// =============================================================================
// i32 operators
// =============================================================================

impl_binary_ops_expr!(Expr, i32, |s: Expr| s, |r: i32| Expr::number(f64::from(r)));
impl_binary_ops_expr!(&Expr, i32, |e: &Expr| e.clone(), |r: i32| Expr::number(
    f64::from(r)
));

// i32 on left side with Expr
impl Add<Expr> for i32 {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Sub<Expr> for i32 {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Mul<Expr> for i32 {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Div<Expr> for i32 {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs)
    }
}

// i32 on left side with &Expr
impl Add<&Expr> for i32 {
    type Output = Expr;
    fn add(self, rhs: &Expr) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Sub<&Expr> for i32 {
    type Output = Expr;
    fn sub(self, rhs: &Expr) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Mul<&Expr> for i32 {
    type Output = Expr;
    fn mul(self, rhs: &Expr) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Div<&Expr> for i32 {
    type Output = Expr;
    fn div(self, rhs: &Expr) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

// =============================================================================
// Arc<Expr> operations
// =============================================================================

impl Add<Arc<Self>> for Expr {
    type Output = Self;
    fn add(self, rhs: Arc<Self>) -> Self {
        Self::add_expr(self, Self::from(rhs))
    }
}

impl Add<&Arc<Self>> for Expr {
    type Output = Self;
    fn add(self, rhs: &Arc<Self>) -> Self {
        Self::add_expr(self, Self::from(rhs))
    }
}

impl Sub<Arc<Self>> for Expr {
    type Output = Self;
    fn sub(self, rhs: Arc<Self>) -> Self {
        Self::sub_expr(self, Self::from(rhs))
    }
}

impl Sub<&Arc<Self>> for Expr {
    type Output = Self;
    fn sub(self, rhs: &Arc<Self>) -> Self {
        Self::sub_expr(self, Self::from(rhs))
    }
}

impl Mul<Arc<Self>> for Expr {
    type Output = Self;
    fn mul(self, rhs: Arc<Self>) -> Self {
        Self::mul_expr(self, Self::from(rhs))
    }
}

impl Mul<&Arc<Self>> for Expr {
    type Output = Self;
    fn mul(self, rhs: &Arc<Self>) -> Self {
        Self::mul_expr(self, Self::from(rhs))
    }
}

impl Div<Arc<Self>> for Expr {
    type Output = Self;
    fn div(self, rhs: Arc<Self>) -> Self {
        Self::div_expr(self, Self::from(rhs))
    }
}

impl Div<&Arc<Self>> for Expr {
    type Output = Self;
    fn div(self, rhs: &Arc<Self>) -> Self {
        Self::div_expr(self, Self::from(rhs))
    }
}

// =============================================================================
// Negation operators
// =============================================================================

impl Neg for Expr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::mul_expr(Self::number(-1.0), self)
    }
}

impl Neg for &Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.clone())
    }
}
