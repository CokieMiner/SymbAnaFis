//! Arithmetic on shared handles and primitives.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::vec;

use num_anafis::Number;

use super::Expr;

macro_rules! handle_operators {
    ($left:ty) => {
        impl<Right: Into<Expr>> Add<Right> for $left {
            type Output = Expr;
            fn add(self, right: Right) -> Expr {
                Expr::sum(vec![self.into(), right.into()])
            }
        }
        impl<Right: Into<Expr>> Sub<Right> for $left {
            type Output = Expr;
            fn sub(self, right: Right) -> Expr {
                Expr::sum(vec![self.into(), Neg::neg(Into::<Expr>::into(right))])
            }
        }
        impl<Right: Into<Expr>> Mul<Right> for $left {
            type Output = Expr;
            fn mul(self, right: Right) -> Expr {
                Expr::product(vec![self.into(), right.into()])
            }
        }
        impl<Right: Into<Expr>> Div<Right> for $left {
            type Output = Expr;
            fn div(self, right: Right) -> Expr {
                Expr::div_expr(self.into(), right.into())
            }
        }
    };
}

handle_operators!(Expr);
handle_operators!(&Expr);

macro_rules! primitive_operators {
    ($left:ty, $right:ty) => {
        impl Add<$right> for $left {
            type Output = Expr;
            fn add(self, right: $right) -> Expr {
                Expr::sum(vec![self.into(), right.into()])
            }
        }
        impl Sub<$right> for $left {
            type Output = Expr;
            fn sub(self, right: $right) -> Expr {
                Expr::sum(vec![self.into(), Neg::neg(Expr::from(right))])
            }
        }
        impl Mul<$right> for $left {
            type Output = Expr;
            fn mul(self, right: $right) -> Expr {
                Expr::product(vec![self.into(), right.into()])
            }
        }
        impl Div<$right> for $left {
            type Output = Expr;
            fn div(self, right: $right) -> Expr {
                Expr::div_expr(self.into(), right.into())
            }
        }
    };
}

primitive_operators!(f64, Expr);
primitive_operators!(f64, &Expr);
primitive_operators!(i32, Expr);
primitive_operators!(i32, &Expr);
primitive_operators!(Number, Expr);
primitive_operators!(Number, &Expr);
primitive_operators!(&Number, Expr);
primitive_operators!(&Number, &Expr);

impl Neg for Expr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::product(vec![Self::number(-1), self])
    }
}

impl Neg for &Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::product(vec![Expr::number(-1), self.clone()])
    }
}
