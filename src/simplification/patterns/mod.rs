//! Pattern matching utilities for simplification rules
//!
//! Provides extractors for common expression patterns used by rule implementations.

use crate::{Expr, ExprKind};

// Common pattern matching utilities have been moved to helpers.rs

/// Trigonometric pattern matching utilities
pub mod trigonometric {
    use super::{Expr, ExprKind};
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
