//! Optional num-traits identity interoperability.

use num_traits::{One, Zero};

use super::Expr;

impl Zero for Expr {
    fn zero() -> Self {
        Self::number(0)
    }

    fn is_zero(&self) -> bool {
        self.is_zero_num()
    }
}

impl One for Expr {
    fn one() -> Self {
        Self::number(1)
    }

    fn is_one(&self) -> bool {
        self.is_one_num()
    }
}
