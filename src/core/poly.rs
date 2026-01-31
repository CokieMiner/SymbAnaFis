//! Univariate polynomial representation
//!
//! Provides efficient polynomial operations with any expression as base.
//! The base is stored once; terms are just (power, coefficient) pairs.

use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use std::sync::Arc;

// =============================================================================
// POLYNOMIAL
// =============================================================================

/// Univariate polynomial in a single base expression
///
/// Represents: Σ `coeff_i` * `base^pow_i`
///
/// Example: 3*sin(x)² + 2*sin(x) + 1 is stored as:
/// - base: sin(x)
/// - terms: [(0, 1.0), (1, 2.0), (2, 3.0)]
#[derive(Debug, Clone, PartialEq)]
pub struct Polynomial {
    /// The base expression (variable, function, etc.) - stored ONCE
    base: Arc<Expr>,
    /// Terms as (power, coefficient) pairs, sorted by power ascending
    /// Invariant: no duplicate powers, no zero coefficients
    terms: Vec<(u32, f64)>,
}

/// Helper to format coefficient
fn format_coeff(n: f64) -> String {
    if {
        #[allow(clippy::float_cmp, reason = "Checking for exact integer via trunc")]
        let is_int = n.trunc() == n;
        is_int
    } && n.abs() < 1e10
    {
        #[allow(clippy::cast_possible_truncation, reason = "Checked is_int above")]
        let n_int = n as i64;
        format!("{n_int}")
    } else {
        format!("{n}")
    }
}

impl std::fmt::Display for Polynomial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.terms.is_empty() {
            return write!(f, "0");
        }

        let mut first = true;
        for &(pow, coeff) in &self.terms {
            if first {
                if coeff < 0.0 {
                    write!(f, "-")?;
                    self.write_term(f, pow, coeff.abs())?;
                } else {
                    self.write_term(f, pow, coeff)?;
                }
                first = false;
            } else if coeff < 0.0 {
                write!(f, " - ")?;
                self.write_term(f, pow, coeff.abs())?;
            } else {
                write!(f, " + ")?;
                self.write_term(f, pow, coeff)?;
            }
        }
        Ok(())
    }
}

impl Polynomial {
    /// Write a single term (coefficient * base^power)
    fn write_term(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        pow: u32,
        coeff: f64,
    ) -> std::fmt::Result {
        // Determine if base needs parentheses when raised to a power
        let needs_parens_pow = matches!(
            self.base.kind,
            ExprKind::Sum(_)
                | ExprKind::Product(_)
                | ExprKind::Div(_, _)
                | ExprKind::Poly(_)
                | ExprKind::Pow(_, _)
        ) || (matches!(self.base.kind, ExprKind::Number(n) if n < 0.0));

        if pow == 0 {
            write!(f, "{}", format_coeff(coeff))
        } else if (coeff - 1.0).abs() < EPSILON {
            if pow == 1 {
                write!(f, "{}", self.base)
            } else if needs_parens_pow {
                write!(f, "({})^{}", self.base, pow)
            } else {
                write!(f, "{}^{}", self.base, pow)
            }
        } else if pow == 1 {
            // Determine if base needs parentheses when multiplied by coefficient
            let needs_parens_mul = matches!(self.base.kind, ExprKind::Sum(_) | ExprKind::Poly(_));
            if needs_parens_mul {
                write!(f, "{}*({})", format_coeff(coeff), self.base)
            } else {
                write!(f, "{}*{}", format_coeff(coeff), self.base)
            }
        } else if needs_parens_pow {
            write!(f, "{}*({})^{}", format_coeff(coeff), self.base, pow)
        } else {
            write!(f, "{}*{}^{}", format_coeff(coeff), self.base, pow)
        }
    }

    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /// Create a zero polynomial with given base
    pub const fn zero(base: Arc<Expr>) -> Self {
        Self {
            base,
            terms: Vec::new(),
        }
    }

    /// Create a constant polynomial
    pub fn constant(c: f64) -> Self {
        if c.abs() < EPSILON {
            // Zero polynomial - use dummy base
            Self {
                base: Arc::new(Expr::number(1.0)),
                terms: Vec::new(),
            }
        } else {
            Self {
                base: Arc::new(Expr::number(1.0)),
                terms: vec![(0, c)],
            }
        }
    }

    /// Create polynomial for a simple expression: base^1
    pub fn from_base(base: Expr) -> Self {
        Self {
            base: Arc::new(base),
            terms: vec![(1, 1.0)],
        }
    }

    /// Create polynomial: coeff * base^pow
    pub fn term(base: Arc<Expr>, pow: u32, coeff: f64) -> Self {
        if coeff.abs() < EPSILON {
            Self::zero(base)
        } else {
            Self {
                base,
                terms: vec![(pow, coeff)],
            }
        }
    }

    // =========================================================================
    // ACCESSORS
    // =========================================================================

    /// Get the base expression
    #[inline]
    pub fn base(&self) -> &Expr {
        &self.base
    }

    /// Update the base expression
    pub(crate) fn set_base(&mut self, base: Arc<Expr>) {
        self.base = base;
    }

    /// Get terms as (power, coefficient) pairs
    #[inline]
    pub fn terms(&self) -> &[(u32, f64)] {
        &self.terms
    }

    /// Check if polynomial is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Check if polynomial is a constant
    pub fn is_constant(&self) -> bool {
        self.terms.is_empty() || (self.terms.len() == 1 && self.terms[0].0 == 0)
    }

    /// Get constant value if polynomial is constant
    pub fn as_constant(&self) -> Option<f64> {
        if self.terms.is_empty() {
            Some(0.0)
        } else if self.terms.len() == 1 && self.terms[0].0 == 0 {
            Some(self.terms[0].1)
        } else {
            None
        }
    }

    /// Get the degree (highest power)
    #[inline]
    pub fn degree(&self) -> u32 {
        self.terms.last().map_or(0, |(p, _)| *p)
    }

    /// Number of terms
    pub const fn term_count(&self) -> usize {
        self.terms.len()
    }

    /// Take the base expression, replacing it with a dummy
    pub(crate) fn take_base(&mut self) -> Arc<Expr> {
        let dummy = Arc::new(Expr::number(0.0));
        std::mem::replace(&mut self.base, dummy)
    }

    /// Get coefficient for a given power (0 if not present)
    pub fn coeff(&self, pow: u32) -> f64 {
        self.terms
            .binary_search_by_key(&pow, |(p, _)| *p)
            .map(|i| self.terms[i].1)
            .unwrap_or(0.0)
    }

    /// Get leading coefficient (highest power term)
    pub fn leading_coeff(&self) -> f64 {
        self.terms.last().map_or(0.0, |(_, c)| *c)
    }

    // =========================================================================
    // OPERATIONS
    // =========================================================================

    /// Add a term (power, coeff) to the polynomial
    pub(crate) fn add_term(&mut self, pow: u32, coeff: f64) {
        if coeff.abs() < EPSILON {
            return;
        }
        match self.terms.binary_search_by_key(&pow, |(p, _)| *p) {
            Ok(i) => {
                self.terms[i].1 += coeff;
                if self.terms[i].1.abs() < EPSILON {
                    self.terms.remove(i);
                }
            }
            Err(i) => {
                self.terms.insert(i, (pow, coeff));
            }
        }
    }

    /// Try to add another polynomial to this one (in-place)
    /// Returns true if successful (compatible bases), false otherwise
    pub(crate) fn try_add_assign(&mut self, other: &Self) -> bool {
        // Same base (by pointer or value): simple merge
        if Arc::ptr_eq(&self.base, &other.base) || self.base == other.base {
            for &(pow, coeff) in &other.terms {
                self.add_term(pow, coeff);
            }
            return true;
        }

        // Other is constant: can add to any polynomial
        if other.is_constant() {
            self.add_term(0, other.as_constant().unwrap_or(0.0));
            return true;
        }

        // Self is constant: adopt other's base and terms, then add our constant
        if self.is_constant() {
            let my_const = self.as_constant().unwrap_or(0.0);
            self.base = Arc::clone(&other.base);
            self.terms.clone_from(&other.terms);
            if my_const.abs() >= EPSILON {
                self.add_term(0, my_const);
            }
            return true;
        }

        // Incompatible non-constant bases: cannot merge
        false
    }

    /// Add two polynomials (must have same base)
    pub fn add(&self, other: &Self) -> Self {
        debug_assert!(
            Arc::ptr_eq(&self.base, &other.base) || self.base == other.base,
            "Cannot add polynomials with different bases"
        );

        // Pre-allocate with combined capacity
        let mut result = Self {
            base: Arc::clone(&self.base),
            terms: Vec::with_capacity(self.terms.len() + other.terms.len()),
        };
        result.terms.clone_from(&self.terms);
        for &(pow, coeff) in &other.terms {
            result.add_term(pow, coeff);
        }
        result
    }

    /// Subtract polynomials
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.negate())
    }

    /// Negate polynomial
    pub fn negate(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            terms: self.terms.iter().map(|&(p, c)| (p, -c)).collect(),
        }
    }

    /// Multiply two polynomials (convolution)
    pub fn mul(&self, other: &Self) -> Self {
        debug_assert!(
            Arc::ptr_eq(&self.base, &other.base) || self.base == other.base,
            "Cannot multiply polynomials with different bases"
        );

        if self.is_zero() || other.is_zero() {
            return Self::zero(Arc::clone(&self.base));
        }

        // Efficient multiplication: Collect all terms then sort and merge
        // This is O(N*M log(N*M)) which is better than O(N*M * (N+M)) of insertion

        let mut all_terms = Vec::with_capacity(self.terms.len() * other.terms.len());

        for &(p1, c1) in &self.terms {
            for &(p2, c2) in &other.terms {
                all_terms.push((p1 + p2, c1 * c2));
            }
        }

        // Sort by power
        all_terms.sort_by_key(|&(pow, _)| pow);

        // Merge terms with same power
        let mut merged_terms = Vec::with_capacity(all_terms.len());
        if let Some(&(mut current_pow, mut current_coeff)) = all_terms.first() {
            for &(next_pow, next_coeff) in all_terms.iter().skip(1) {
                if next_pow == current_pow {
                    current_coeff += next_coeff;
                } else {
                    if current_coeff.abs() >= EPSILON {
                        merged_terms.push((current_pow, current_coeff));
                    }
                    current_pow = next_pow;
                    current_coeff = next_coeff;
                }
            }
            if current_coeff.abs() >= EPSILON {
                merged_terms.push((current_pow, current_coeff));
            }
        }

        Self {
            base: Arc::clone(&self.base),
            terms: merged_terms,
        }
    }

    /// Multiply by a scalar
    pub fn scale(&self, scalar: f64) -> Self {
        if scalar.abs() < EPSILON {
            return Self::zero(Arc::clone(&self.base));
        }
        Self {
            base: Arc::clone(&self.base),
            terms: self.terms.iter().map(|&(p, c)| (p, c * scalar)).collect(),
        }
    }

    /// Differentiate polynomial: d/d(base)
    /// Each term c*base^n becomes n*c*base^(n-1)
    pub fn derivative(&self) -> Self {
        let mut result = Self::zero(Arc::clone(&self.base));
        for &(pow, coeff) in &self.terms {
            if pow > 0 {
                result.add_term(pow - 1, coeff * f64::from(pow));
            }
        }
        result
    }

    /// Make polynomial monic (leading coefficient = 1)
    pub fn make_monic(&self) -> Self {
        let lc = self.leading_coeff();
        if lc.abs() < EPSILON || (lc - 1.0).abs() < EPSILON {
            return self.clone();
        }
        self.scale(1.0 / lc)
    }

    // =========================================================================
    // DIVISION (for GCD)
    // =========================================================================

    /// Polynomial division: self / other = (quotient, remainder)
    pub fn div_rem(&self, other: &Self) -> Option<(Self, Self)> {
        if other.is_zero() {
            return None;
        }

        let div_deg = other.degree();
        let div_lc = other.leading_coeff();

        let mut quotient = Self::zero(Arc::clone(&self.base));
        let mut remainder = self.clone();

        loop {
            if remainder.is_zero() {
                break;
            }

            let rem_deg = remainder.degree();
            if rem_deg < div_deg {
                break;
            }

            let q_coeff = remainder.leading_coeff() / div_lc;
            let q_pow = rem_deg - div_deg;

            quotient.add_term(q_pow, q_coeff);

            // remainder -= q_term * other
            for &(p, c) in &other.terms {
                remainder.add_term(p + q_pow, -q_coeff * c);
            }
        }

        Some((quotient, remainder))
    }

    /// GCD using Euclidean algorithm
    /// Returns None if polynomials have different bases (cannot compute GCD)
    pub fn gcd(&self, other: &Self) -> Option<Self> {
        // GCD only makes sense for polynomials with same base
        if !Arc::ptr_eq(&self.base, &other.base) && self.base != other.base {
            return None;
        }

        let mut r0 = self.clone();
        let mut r1 = other.clone();

        loop {
            if r1.is_zero() {
                return Some(r0.make_monic());
            }

            let (_, rem) = r0.div_rem(&r1)?;
            r0 = r1;
            r1 = rem;
        }
    }

    // =========================================================================
    // CONVERSION
    // =========================================================================

    /// Try to convert an expression to polynomial form
    /// Returns None if expression is not polynomial-like
    pub fn try_from_expr(expr: &Expr) -> Option<Self> {
        match &expr.kind {
            // Constants
            ExprKind::Number(n) => Some(Self::constant(*n)),

            // Symbols, function calls, or derivatives become base^1
            ExprKind::Symbol(_) | ExprKind::FunctionCall { .. } | ExprKind::Derivative { .. } => {
                Some(Self::from_base(expr.clone()))
            }
            ExprKind::Sum(terms) => {
                if terms.is_empty() {
                    return Some(Self::constant(0.0));
                }

                // Convert first term to establish base
                let mut result = Self::try_from_expr(&terms[0])?;

                // Add remaining terms (must have compatible bases)
                for term in terms.iter().skip(1) {
                    let term_poly = Self::try_from_expr(term)?;
                    if !result.try_add_assign(&term_poly) {
                        return None;
                    }
                }

                Some(result)
            }

            // Product: convert each factor and multiply
            ExprKind::Product(factors) => {
                if factors.is_empty() {
                    return Some(Self::constant(1.0));
                }

                // Convert first factor
                let mut result = Self::try_from_expr(&factors[0])?;

                // Multiply remaining factors
                for factor in factors.iter().skip(1) {
                    let factor_poly = Self::try_from_expr(factor)?;
                    result = Self::try_mul(&result, &factor_poly)?;
                }

                Some(result)
            }

            // Power with integer exponent
            ExprKind::Pow(base_expr, exp) => {
                if let ExprKind::Number(n) = &exp.kind
                    && *n >= 0.0
                    && n.fract().abs() < EPSILON
                {
                    // Exponent n >= 0 is already validated
                    #[allow(
                        clippy::float_cmp,
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "Safe conversion of validated non-negative integer float to u32 for polynomial power"
                    )]
                    let pow = *n as u32;
                    // Reuse existing Arc instead of deep cloning
                    return Some(Self {
                        base: Arc::clone(base_expr),
                        terms: vec![(pow, 1.0)],
                    });
                }
                // Non-integer power: treat entire expression as base
                Some(Self::from_base(expr.clone()))
            }

            // Division by constant
            ExprKind::Div(num, den) => {
                if let ExprKind::Number(d) = &den.kind
                    && d.abs() > EPSILON
                {
                    let mut result = Self::try_from_expr(num)?;
                    result = result.scale(1.0 / d);
                    return Some(result);
                }
                // Non-constant divisor: treat as opaque
                Some(Self::from_base(expr.clone()))
            }

            // Already a polynomial: extract
            ExprKind::Poly(poly) => Some(poly.clone()),
        }
    }

    /// Try to multiply two polynomials, handling base compatibility
    fn try_mul(a: &Self, b: &Self) -> Option<Self> {
        // Multiply by constant
        if a.is_constant() {
            return Some(b.scale(a.as_constant().unwrap_or(0.0)));
        }
        if b.is_constant() {
            return Some(a.scale(b.as_constant().unwrap_or(0.0)));
        }

        // Same base: normal multiply
        // Arc::ptr_eq provides O(1) short-circuit for squaring (p.mul(&p)) and cloned polynomials
        if Arc::ptr_eq(&a.base, &b.base) || a.base == b.base {
            return Some(a.mul(b));
        }

        // Different bases: cannot combine as single polynomial
        None
    }

    /// Convert back to expression
    pub fn to_expr(&self) -> Expr {
        match self.terms.len() {
            0 => Expr::number(0.0),
            1 => self.term_to_expr(self.terms[0].0, self.terms[0].1),
            _ => {
                let term_exprs: Vec<Expr> = self
                    .terms
                    .iter()
                    .map(|&(pow, coeff)| self.term_to_expr(pow, coeff))
                    .collect();
                Expr::sum(term_exprs)
            }
        }
    }

    /// Convert a single term to expression: coeff * base^pow
    fn term_to_expr(&self, pow: u32, coeff: f64) -> Expr {
        if pow == 0 {
            return Expr::number(coeff);
        }

        // Use Arc-based constructors to avoid deep cloning
        let base_pow = if pow == 1 {
            Expr::unwrap_arc(Arc::clone(&self.base))
        } else {
            Expr::pow_from_arcs(
                Arc::clone(&self.base),
                Arc::new(Expr::number(f64::from(pow))),
            )
        };

        if (coeff - 1.0).abs() < EPSILON {
            base_pow
        } else if (coeff + 1.0).abs() < EPSILON {
            base_pow.negate()
        } else {
            Expr::product(vec![Expr::number(coeff), base_pow])
        }
    }

    /// Differentiate with respect to a variable (chain rule)
    pub fn derivative_expr(&self, diff_var: &str) -> Expr {
        // Get polynomial derivative
        let poly_deriv = self.derivative();
        let poly_expr = poly_deriv.to_expr();

        // Check if base contains the differentiation variable
        if self.base.contains_var(diff_var) {
            // Chain rule: d/dx[P(u)] = P'(u) * u'
            let u_prime = self.base.derive_raw(diff_var);

            if u_prime.is_zero_num() {
                Expr::number(0.0)
            } else if u_prime.is_one_num() {
                poly_expr
            } else {
                Expr::product(vec![poly_expr, u_prime])
            }
        } else {
            // Base doesn't contain diff_var, derivative is 0
            Expr::number(0.0)
        }
    }

    /// Convert polynomial terms to `Vec<Expr>` (compatibility method)
    /// Returns each term as a separate expression: coeff * base^pow
    pub fn to_expr_terms(&self) -> Vec<Expr> {
        self.terms
            .iter()
            .map(|&(pow, coeff)| self.term_to_expr(pow, coeff))
            .collect()
    }

    /// Get leading coefficient for the first term (for display sign detection)
    pub fn first_coeff(&self) -> Option<f64> {
        self.terms.first().map(|(_, c)| *c)
    }

    /// Negate a specific term's coefficient (for display purposes)
    /// Returns a new polynomial with the negated term
    pub fn with_negated_first(&self) -> Self {
        if self.terms.is_empty() {
            return self.clone();
        }
        let mut new_terms = self.terms.clone();
        new_terms[0].1 = -new_terms[0].1;
        Self {
            base: Arc::clone(&self.base),
            terms: new_terms,
        }
    }
}

impl Default for Polynomial {
    fn default() -> Self {
        Self::constant(0.0)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
// Standard test relaxations: unwrap/panic for assertions, precision loss for math
#[allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;
    use crate::core::known_symbols as ks;

    #[test]
    fn test_poly_constant() {
        let p = Polynomial::constant(5.0);
        assert!(p.is_constant());
        assert_eq!(p.as_constant(), Some(5.0));
        assert_eq!(p.degree(), 0);
    }

    #[test]
    fn test_poly_from_base() {
        let x = Expr::symbol("x");
        let p = Polynomial::from_base(x);
        assert!(!p.is_constant());
        assert_eq!(p.degree(), 1);
        assert_eq!(p.coeff(1), 1.0);
    }

    #[test]
    fn test_poly_add() {
        let x = Arc::new(Expr::symbol("x"));
        let p1 = Polynomial::term(Arc::clone(&x), 2, 3.0); // 3x²
        let p2 = Polynomial::term(Arc::clone(&x), 1, 2.0); // 2x
        let sum = p1.add(&p2);

        assert_eq!(sum.term_count(), 2);
        assert_eq!(sum.coeff(2), 3.0);
        assert_eq!(sum.coeff(1), 2.0);
    }

    #[test]
    fn test_poly_mul() {
        // (x + 1) * (x + 1) = x² + 2x + 1
        let x = Arc::new(Expr::symbol("x"));
        let mut p = Polynomial::term(Arc::clone(&x), 1, 1.0);
        p.add_term(0, 1.0); // x + 1

        let result = p.mul(&p);
        assert_eq!(result.degree(), 2);
        assert_eq!(result.coeff(0), 1.0);
        assert_eq!(result.coeff(1), 2.0);
        assert_eq!(result.coeff(2), 1.0);
    }

    #[test]
    fn test_poly_derivative() {
        // d/dx(3x² + 2x + 1) = 6x + 2
        let x = Arc::new(Expr::symbol("x"));
        let mut p = Polynomial::zero(Arc::clone(&x));
        p.add_term(2, 3.0);
        p.add_term(1, 2.0);
        p.add_term(0, 1.0);

        let deriv = p.derivative();
        assert_eq!(deriv.degree(), 1);
        assert_eq!(deriv.coeff(1), 6.0);
        assert_eq!(deriv.coeff(0), 2.0);
    }

    #[test]
    fn test_poly_div_rem() {
        // (x² - 1) / (x - 1) = (x + 1), remainder 0
        let x = Arc::new(Expr::symbol("x"));

        let mut p1 = Polynomial::zero(Arc::clone(&x));
        p1.add_term(2, 1.0);
        p1.add_term(0, -1.0); // x² - 1

        let mut p2 = Polynomial::zero(Arc::clone(&x));
        p2.add_term(1, 1.0);
        p2.add_term(0, -1.0); // x - 1

        let (q, r) = p1.div_rem(&p2).unwrap();
        assert_eq!(q.degree(), 1);
        assert!(r.is_zero() || r.is_constant());
    }

    #[test]
    fn test_poly_gcd() {
        // GCD of (x² - 1) and (x - 1) should be (x - 1)
        let x = Arc::new(Expr::symbol("x"));

        let mut p1 = Polynomial::zero(Arc::clone(&x));
        p1.add_term(2, 1.0);
        p1.add_term(0, -1.0);

        let mut p2 = Polynomial::zero(Arc::clone(&x));
        p2.add_term(1, 1.0);
        p2.add_term(0, -1.0);

        let gcd = p1.gcd(&p2).unwrap();
        assert_eq!(gcd.degree(), 1);
    }

    #[test]
    fn test_poly_with_function_base() {
        // Polynomial in sin(x)
        let sin_x = Expr::func_symbol(ks::get_symbol(ks::KS.sin), Expr::symbol("x"));
        let base = Arc::new(sin_x);

        let mut p = Polynomial::zero(Arc::clone(&base));
        p.add_term(2, 1.0);
        p.add_term(0, -1.0); // sin(x)² - 1

        assert_eq!(p.degree(), 2);
        assert_eq!(p.to_string(), "-1 + sin(x)^2");
    }

    #[test]
    fn test_poly_roundtrip() {
        let expr = Expr::sum(vec![
            Expr::pow_static(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
            Expr::number(1.0),
        ]);

        let poly = Polynomial::try_from_expr(&expr);
        assert!(poly.is_some());

        let p = poly.unwrap();
        assert_eq!(p.degree(), 2);
    }

    #[test]
    fn test_poly_display() {
        let x = Arc::new(Expr::symbol("x"));
        let mut p = Polynomial::zero(x);
        p.add_term(2, 3.0);
        p.add_term(1, -2.0);
        p.add_term(0, 1.0);

        let s = p.to_string();
        assert!(s.contains('3') && s.contains('x'));
    }
}
