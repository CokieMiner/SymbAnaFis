//! Operator overloading for Symbol, Expr, and numeric types.
//!
//! Contains all the `Add`, `Sub`, `Mul`, `Div`, `Neg` implementations
//! for various type combinations.

use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::core::Expr;
use crate::core::Symbol;

// ============================================================================
// Operator Overloading Macro
// ============================================================================

/// Implement binary operations for symbols and expressions
macro_rules! impl_binary_ops {
    (symbol_ops $lhs:ty, $rhs:ty, $to_lhs:expr, $to_rhs:expr) => {
        impl Add<$rhs> for $lhs {
            type Output = Expr;
            fn add(self, rhs: $rhs) -> Expr {
                Expr::add_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Sub<$rhs> for $lhs {
            type Output = Expr;
            fn sub(self, rhs: $rhs) -> Expr {
                Expr::sub_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Mul<$rhs> for $lhs {
            type Output = Expr;
            fn mul(self, rhs: $rhs) -> Expr {
                Expr::mul_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Div<$rhs> for $lhs {
            type Output = Expr;
            fn div(self, rhs: $rhs) -> Expr {
                Expr::div_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
    };
}

// Symbol operations (value)
impl_binary_ops!(symbol_ops Symbol, Symbol, |s: Symbol| s.to_expr(), |r: Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Symbol, Expr, |s: Symbol| s.to_expr(), |r: Expr| r);
impl_binary_ops!(symbol_ops Symbol, f64, |s: Symbol| s.to_expr(), |r: f64| Expr::number(r));

// &Symbol operations (reference)
impl_binary_ops!(
    symbol_ops & Symbol,
    &Symbol,
    |s: &Symbol| s.to_expr(),
    |r: &Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Symbol,
    Symbol,
    |s: &Symbol| s.to_expr(),
    |r: Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Symbol,
    Expr,
    |s: &Symbol| s.to_expr(),
    |r: Expr| r
);
impl_binary_ops!(
    symbol_ops & Symbol,
    f64,
    |s: &Symbol| s.to_expr(),
    |r: f64| Expr::number(r)
);

// Expr operations

impl_binary_ops!(symbol_ops Expr, Symbol, |s: Expr| s, |r: Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Expr, &Symbol, |s: Expr| s, |r: &Symbol| r.to_expr());

// Symbol + &Symbol and Symbol + &Expr
impl_binary_ops!(symbol_ops Symbol, &Symbol, |s: Symbol| s.to_expr(), |r: &Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Symbol, &Expr, |s: Symbol| s.to_expr(), |r: &Expr| r.clone());

// &Expr operations (reference - allows &expr + &expr without explicit .clone())

impl_binary_ops!(
    symbol_ops & Expr,
    Symbol,
    |e: &Expr| e.clone(),
    |r: Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Expr,
    &Symbol,
    |e: &Expr| e.clone(),
    |r: &Symbol| r.to_expr()
);

impl Sub<Symbol> for f64 {
    type Output = Expr;
    fn sub(self, rhs: Symbol) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Mul<Symbol> for f64 {
    type Output = Expr;
    fn mul(self, rhs: Symbol) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Add<Symbol> for f64 {
    type Output = Expr;
    fn add(self, rhs: Symbol) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Div<Symbol> for f64 {
    type Output = Expr;
    fn div(self, rhs: Symbol) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.to_expr())
    }
}

// f64 on left side with &Symbol
impl Add<&Symbol> for f64 {
    type Output = Expr;
    fn add(self, rhs: &Symbol) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Sub<&Symbol> for f64 {
    type Output = Expr;
    fn sub(self, rhs: &Symbol) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Mul<&Symbol> for f64 {
    type Output = Expr;
    fn mul(self, rhs: &Symbol) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Div<&Symbol> for f64 {
    type Output = Expr;
    fn div(self, rhs: &Symbol) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.to_expr())
    }
}

// =============================================================================
// i32 operators (Symbol/Expr on left side)
// =============================================================================

// Symbol + i32
impl_binary_ops!(symbol_ops Symbol, i32, |s: Symbol| s.to_expr(), |r: i32| Expr::number(f64::from(r)));
impl_binary_ops!(
    symbol_ops & Symbol,
    i32,
    |s: &Symbol| s.to_expr(),
    |r: i32| Expr::number(f64::from(r))
);

// i32 on left side with Symbol
impl Add<Symbol> for i32 {
    type Output = Expr;
    fn add(self, rhs: Symbol) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Sub<Symbol> for i32 {
    type Output = Expr;
    fn sub(self, rhs: Symbol) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Mul<Symbol> for i32 {
    type Output = Expr;
    fn mul(self, rhs: Symbol) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Div<Symbol> for i32 {
    type Output = Expr;
    fn div(self, rhs: Symbol) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

// =============================================================================
// Arc<Expr> operations
// =============================================================================

// =============================================================================
// Negation operators
// =============================================================================

impl Neg for &Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.to_expr())
    }
}

impl Neg for Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        self.to_expr().negate()
    }
}
