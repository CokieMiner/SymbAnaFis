//! Arithmetic templates with exact conversion of concrete operands.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::vec;

use num_anafis::Number;

use super::{Arithmetic, Expr, Pattern, PatternNode, Symbol};

macro_rules! pattern_operators {
    ($left:ty) => {
        impl<Right: Into<Pattern>> Add<Right> for $left {
            type Output = Pattern;
            fn add(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sum(vec![self.into(), right.into()]))
            }
        }
        impl<Right: Into<Pattern>> Sub<Right> for $left {
            type Output = Pattern;
            fn sub(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sub(self.into(), right.into()))
            }
        }
        impl<Right: Into<Pattern>> Mul<Right> for $left {
            type Output = Pattern;
            fn mul(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Product(vec![self.into(), right.into()]))
            }
        }
        impl<Right: Into<Pattern>> Div<Right> for $left {
            type Output = Pattern;
            fn div(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Div(self.into(), right.into()))
            }
        }
    };
}

pattern_operators!(Pattern);
pattern_operators!(&Pattern);

macro_rules! concrete_operators {
    ($left:ty, $right:ty) => {
        impl Add<$right> for $left {
            type Output = Pattern;
            fn add(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sum(vec![self.into(), right.into()]))
            }
        }
        impl Sub<$right> for $left {
            type Output = Pattern;
            fn sub(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sub(self.into(), right.into()))
            }
        }
        impl Mul<$right> for $left {
            type Output = Pattern;
            fn mul(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Product(vec![self.into(), right.into()]))
            }
        }
        impl Div<$right> for $left {
            type Output = Pattern;
            fn div(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Div(self.into(), right.into()))
            }
        }
    };
}

macro_rules! concrete_operands {
    ($($operand:ty),+ $(,)?) => {
        $(
            impl From<$operand> for Pattern {
                fn from(value: $operand) -> Self {
                    Self::exact(value)
                }
            }
            concrete_operators!($operand, Pattern);
            concrete_operators!($operand, &Pattern);
        )+
    };
}

concrete_operands!(
    Expr,
    &Expr,
    Symbol<'_>,
    &Symbol<'_>,
    i32,
    i64,
    f64,
    Number,
    &Number
);

impl From<&Self> for Pattern {
    fn from(pattern: &Self) -> Self {
        pattern.clone()
    }
}

impl Neg for Pattern {
    type Output = Self;
    fn neg(self) -> Self {
        PatternNode::normalize(Arithmetic::Product(vec![Self::exact(-1), self]))
    }
}

impl Neg for &Pattern {
    type Output = Pattern;
    fn neg(self) -> Pattern {
        PatternNode::normalize(Arithmetic::Product(vec![Pattern::exact(-1), self.clone()]))
    }
}
