//! Arithmetic on Copy symbols; nodes are constructed through `Expr`.

use core::ops::{Add, Div, Mul, Neg, Sub};

use num_anafis::Number;

use super::{Expr, Symbol};

impl From<Symbol<'_>> for Expr {
    fn from(symbol: Symbol<'_>) -> Self {
        symbol.to_expr()
    }
}

impl From<&Symbol<'_>> for Expr {
    fn from(symbol: &Symbol<'_>) -> Self {
        symbol.to_expr()
    }
}

macro_rules! symbol_operators {
    ($left:ty) => {
        impl<Right: Into<Expr>> Add<Right> for $left {
            type Output = Expr;
            fn add(self, right: Right) -> Expr {
                Add::<Expr>::add(self.to_expr(), right.into())
            }
        }
        impl<Right: Into<Expr>> Sub<Right> for $left {
            type Output = Expr;
            fn sub(self, right: Right) -> Expr {
                Sub::<Expr>::sub(self.to_expr(), right.into())
            }
        }
        impl<Right: Into<Expr>> Mul<Right> for $left {
            type Output = Expr;
            fn mul(self, right: Right) -> Expr {
                Mul::<Expr>::mul(self.to_expr(), right.into())
            }
        }
        impl<Right: Into<Expr>> Div<Right> for $left {
            type Output = Expr;
            fn div(self, right: Right) -> Expr {
                Div::<Expr>::div(self.to_expr(), right.into())
            }
        }
    };
}

symbol_operators!(Symbol<'_>);
symbol_operators!(&Symbol<'_>);

macro_rules! primitive_operators {
    ($left:ty, $right:ty) => {
        impl Add<$right> for $left {
            type Output = Expr;
            fn add(self, right: $right) -> Expr {
                Add::<Expr>::add(Expr::from(self), right.to_expr())
            }
        }
        impl Sub<$right> for $left {
            type Output = Expr;
            fn sub(self, right: $right) -> Expr {
                Sub::<Expr>::sub(Expr::from(self), right.to_expr())
            }
        }
        impl Mul<$right> for $left {
            type Output = Expr;
            fn mul(self, right: $right) -> Expr {
                Mul::<Expr>::mul(Expr::from(self), right.to_expr())
            }
        }
        impl Div<$right> for $left {
            type Output = Expr;
            fn div(self, right: $right) -> Expr {
                Div::<Expr>::div(Expr::from(self), right.to_expr())
            }
        }
    };
}

primitive_operators!(f64, Symbol<'_>);
primitive_operators!(f64, &Symbol<'_>);
primitive_operators!(i32, Symbol<'_>);
primitive_operators!(i32, &Symbol<'_>);
primitive_operators!(Number, Symbol<'_>);
primitive_operators!(Number, &Symbol<'_>);
primitive_operators!(&Number, Symbol<'_>);
primitive_operators!(&Number, &Symbol<'_>);

impl Neg for Symbol<'_> {
    type Output = Expr;
    fn neg(self) -> Expr {
        Neg::neg(self.to_expr())
    }
}

impl Neg for &Symbol<'_> {
    type Output = Expr;
    fn neg(self) -> Expr {
        Neg::neg(self.to_expr())
    }
}
