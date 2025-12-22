//! Pattern matching utilities for simplification rules
//!
//! Provides extractors for common expression patterns used by rule implementations.

use crate::{Expr, ExprKind};

/// Common pattern matching utilities for simplification rules
pub(crate) mod common {
    use super::*;

    /// Extract coefficient and base from a multiplication term
    /// Returns (coefficient, base) where base is normalized
    pub fn extract_coefficient(expr: &Expr) -> (f64, Expr) {
        match &expr.kind {
            ExprKind::Number(n) => (*n, Expr::number(1.0)),
            ExprKind::Product(factors) => {
                // Check if first factor is a number coefficient
                if let Some(first) = factors.first()
                    && let ExprKind::Number(n) = &first.kind
                {
                    // Return coefficient and remaining factors
                    let rest: Vec<_> = factors.iter().skip(1).map(|f| (**f).clone()).collect();
                    if rest.is_empty() {
                        return (*n, Expr::number(1.0));
                    } else if rest.len() == 1 {
                        return (*n, rest.into_iter().next().unwrap());
                    } else {
                        return (*n, Expr::product(rest));
                    }
                }
                (1.0, expr.clone())
            }
            _ => (1.0, expr.clone()),
        }
    }
}

/// Trigonometric pattern matching utilities
pub(crate) mod trigonometric {
    use super::*;
    use crate::core::known_symbols::{COS, COT, CSC, SEC, SIN, TAN};
    use crate::core::symbol::InternedSymbol;

    /// Extract function name and argument if expression is a trig function
    pub fn get_trig_function(expr: &Expr) -> Option<(InternedSymbol, Expr)> {
        if let ExprKind::FunctionCall { name, args } = &expr.kind {
            if args.len() == 1 {
                match name {
                    n if n.id() == *SIN
                        || n.id() == *COS
                        || n.id() == *TAN
                        || n.id() == *COT
                        || n.id() == *SEC
                        || n.id() == *CSC =>
                    {
                        Some((name.clone(), (*args[0]).clone()))
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
