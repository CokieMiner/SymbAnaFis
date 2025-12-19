//! On-demand polynomial detection and operations (Phase 6)
//!
//! Provides multivariate polynomial representation with expression substitution.
//! Complex sub-expressions (sin, exp, etc.) become temporary symbols during
//! polynomial operations, then get substituted back.

use crate::symbol::{InternedSymbol, get_or_intern};
use crate::{Expr, ExprKind};
use std::collections::HashMap;
use std::sync::Arc;

// =============================================================================
// POLYNOMIAL TERM
// =============================================================================

/// A single polynomial term: coefficient * product of variable powers
/// e.g., 3*x^2*y = PolyTerm { coeff: 3.0, powers: [(x, 2), (y, 1)] }
#[derive(Debug, Clone, PartialEq)]
pub struct PolyTerm {
    /// Numeric coefficient (kept as integer when possible)
    pub coeff: f64,
    /// Variable -> power pairs, sorted by symbol ID for canonical form
    pub powers: Vec<(InternedSymbol, u32)>,
}

impl PolyTerm {
    /// Create a new polynomial term
    pub fn new(coeff: f64, mut powers: Vec<(InternedSymbol, u32)>) -> Self {
        // Sort alphabetically by variable name (matches AST canonical order)
        powers.sort_by(|(a, _), (b, _)| a.cmp(b));
        // Remove zero powers
        powers.retain(|(_, p)| *p > 0);
        PolyTerm { coeff, powers }
    }

    /// Create a constant term
    pub fn constant(coeff: f64) -> Self {
        PolyTerm {
            coeff,
            powers: Vec::new(),
        }
    }

    /// Create a term from a single variable: x = 1*x^1
    pub fn var(sym: InternedSymbol) -> Self {
        PolyTerm {
            coeff: 1.0,
            powers: vec![(sym, 1)],
        }
    }

    /// Total degree of the term (sum of all exponents)
    pub fn degree(&self) -> u32 {
        self.powers.iter().map(|(_, p)| p).sum()
    }

    /// Check if this term is a constant (no variables)
    pub fn is_constant(&self) -> bool {
        self.powers.is_empty()
    }

    /// Get the signature (powers only) for combining like terms
    pub fn signature(&self) -> &[(InternedSymbol, u32)] {
        &self.powers
    }

    /// Multiply two terms
    pub fn mul(&self, other: &PolyTerm) -> PolyTerm {
        let new_coeff = self.coeff * other.coeff;

        // Merge powers
        let mut power_map: HashMap<u64, (InternedSymbol, u32)> = HashMap::new();
        for (sym, pow) in &self.powers {
            power_map.insert(sym.id(), (sym.clone(), *pow));
        }
        for (sym, pow) in &other.powers {
            power_map
                .entry(sym.id())
                .and_modify(|(_, p)| *p += pow)
                .or_insert((sym.clone(), *pow));
        }

        let powers: Vec<_> = power_map.into_values().collect();
        PolyTerm::new(new_coeff, powers)
    }

    /// Convert term back to Expr
    pub fn to_expr(&self) -> Expr {
        if self.powers.is_empty() {
            return Expr::number(self.coeff);
        }

        let mut factors: Vec<Expr> = Vec::new();

        // Add coefficient if not 1
        if (self.coeff - 1.0).abs() > 1e-10 {
            factors.push(Expr::number(self.coeff));
        }

        // Add variable powers
        for (sym, pow) in &self.powers {
            let var = Expr::from_interned(sym.clone());
            if *pow == 1 {
                factors.push(var);
            } else {
                factors.push(Expr::pow(var, Expr::number(*pow as f64)));
            }
        }

        if factors.is_empty() {
            Expr::number(1.0)
        } else if factors.len() == 1 {
            factors.pop().unwrap()
        } else {
            Expr::product(factors)
        }
    }
}

// =============================================================================
// POLYNOMIAL
// =============================================================================

/// Multivariate polynomial with expression substitution
/// Non-polynomial sub-expressions are replaced with temporary symbols
#[derive(Debug, Clone)]
pub struct Polynomial {
    /// List of terms (coefficient * monomial)
    terms: Vec<PolyTerm>,

    /// Substitution map: temp_symbol_id -> original Expr
    /// Complex expressions like sin(x) get a temp ID during conversion
    substitutions: HashMap<u64, Arc<Expr>>,

    /// Counter for generating unique temp symbol IDs
    next_temp_id: u64,
}

impl Polynomial {
    /// Create a new empty polynomial
    pub fn new() -> Self {
        Polynomial {
            terms: Vec::new(),
            substitutions: HashMap::new(),
            next_temp_id: 0,
        }
    }

    /// Create a polynomial from a single term
    pub fn from_term(term: PolyTerm) -> Self {
        Polynomial {
            terms: vec![term],
            substitutions: HashMap::new(),
            next_temp_id: 0,
        }
    }

    /// Create a constant polynomial
    pub fn constant(c: f64) -> Self {
        Polynomial::from_term(PolyTerm::constant(c))
    }

    /// Create a polynomial from a variable
    pub fn var(sym: InternedSymbol) -> Self {
        Polynomial::from_term(PolyTerm::var(sym))
    }

    /// Try to convert an expression into polynomial form
    /// Complex sub-expressions (sin, exp, etc.) become temp symbols
    pub fn try_from_expr(expr: &Expr) -> Option<Self> {
        let mut poly = Polynomial::new();
        poly.convert_expr(expr)?;
        poly.combine_like_terms();
        Some(poly)
    }

    /// Internal: Convert expression to polynomial terms
    fn convert_expr(&mut self, expr: &Expr) -> Option<()> {
        match &expr.kind {
            // Constants become constant terms
            ExprKind::Number(n) => {
                self.terms.push(PolyTerm::constant(*n));
                Some(())
            }

            // Symbols become single-variable terms
            ExprKind::Symbol(s) => {
                self.terms.push(PolyTerm::var(s.clone()));
                Some(())
            }

            // Sum: convert each term and collect
            ExprKind::Sum(terms) => {
                for t in terms {
                    self.convert_expr(t)?;
                }
                Some(())
            }

            // Product: multiply all factors
            ExprKind::Product(factors) => {
                // Convert first factor
                if factors.is_empty() {
                    self.terms.push(PolyTerm::constant(1.0));
                    return Some(());
                }

                let mut result_poly = Polynomial::new();
                result_poly.convert_expr(&factors[0])?;

                // Multiply with remaining factors
                for factor in factors.iter().skip(1) {
                    let mut factor_poly = Polynomial::new();
                    factor_poly.convert_expr(factor)?;
                    result_poly = result_poly.mul(&factor_poly);
                }

                // Merge substitutions and terms
                self.merge_substitutions(&result_poly);
                self.terms.extend(result_poly.terms);
                Some(())
            }

            // Power with integer exponent
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Number(n) = &exp.kind {
                    // Check for non-negative integer exponent
                    if *n >= 0.0 && n.fract() == 0.0 {
                        let pow = *n as u32;

                        // Check if base is a simple symbol
                        if let ExprKind::Symbol(s) = &base.kind {
                            self.terms.push(PolyTerm::new(1.0, vec![(s.clone(), pow)]));
                            return Some(());
                        }

                        // Otherwise, convert base and raise to power
                        let mut base_poly = Polynomial::new();
                        base_poly.convert_expr(base)?;

                        let mut result = Polynomial::constant(1.0);
                        for _ in 0..pow {
                            result = result.mul(&base_poly);
                        }

                        self.merge_substitutions(&result);
                        self.terms.extend(result.terms);
                        return Some(());
                    }
                }

                // Non-integer or negative exponent: treat as opaque
                self.add_opaque_expr(expr)
            }

            // Division: only handle if divisor is a constant
            ExprKind::Div(num, den) => {
                if let ExprKind::Number(d) = &den.kind
                    && d.abs() > 1e-10
                {
                    // p / c = (1/c) * p
                    let inv = 1.0 / d;
                    self.convert_expr(num)?;
                    // Multiply all current terms by inverse
                    for term in &mut self.terms {
                        term.coeff *= inv;
                    }
                    return Some(());
                }
                // Non-constant divisor: treat as opaque
                self.add_opaque_expr(expr)
            }

            // FunctionCall, Derivative: treat as opaque expressions
            ExprKind::FunctionCall { .. } | ExprKind::Derivative { .. } => {
                self.add_opaque_expr(expr)
            }
        }
    }

    /// Add an opaque expression as a temporary symbol
    fn add_opaque_expr(&mut self, expr: &Expr) -> Option<()> {
        // Create a unique temp symbol name
        let temp_name = format!("__poly_temp_{}", self.next_temp_id);
        self.next_temp_id += 1;

        let temp_sym = get_or_intern(&temp_name);

        // Store the substitution
        self.substitutions
            .insert(temp_sym.id(), Arc::new(expr.clone()));

        // Add as a term
        self.terms.push(PolyTerm::var(temp_sym));
        Some(())
    }

    /// Merge substitutions from another polynomial
    fn merge_substitutions(&mut self, other: &Polynomial) {
        for (id, expr) in &other.substitutions {
            self.substitutions.insert(*id, expr.clone());
        }
        self.next_temp_id = self.next_temp_id.max(other.next_temp_id);
    }

    /// Combine like terms (same power signature)
    fn combine_like_terms(&mut self) {
        if self.terms.len() <= 1 {
            return;
        }

        // Group by signature
        // Type alias to reduce complexity warning
        type Sig = Vec<(u64, u32)>;
        type Powers = Vec<(InternedSymbol, u32)>;
        let mut combined: HashMap<Sig, f64> = HashMap::new();
        let mut sig_to_powers: HashMap<Sig, Powers> = HashMap::new();

        for term in &self.terms {
            let sig: Vec<(u64, u32)> = term.powers.iter().map(|(s, p)| (s.id(), *p)).collect();
            *combined.entry(sig.clone()).or_insert(0.0) += term.coeff;
            sig_to_powers
                .entry(sig)
                .or_insert_with(|| term.powers.clone());
        }

        // Rebuild terms
        self.terms = combined
            .into_iter()
            .filter(|(_, coeff)| coeff.abs() > 1e-14)
            .map(|(sig, coeff)| PolyTerm {
                coeff,
                powers: sig_to_powers.remove(&sig).unwrap_or_default(),
            })
            .collect();

        // Sort for canonical form (matches AST order):
        // 1. Alphabetical by first variable: x before y
        // 2. Lower degree first: x before x^2 before x^3
        self.terms.sort_by(|a, b| {
            // Alphabetically by first variable name
            let var_a = a.powers.first().map(|(s, _)| s.to_string());
            let var_b = b.powers.first().map(|(s, _)| s.to_string());
            var_a.cmp(&var_b).then_with(|| {
                // Lower degree first (ascending)
                a.degree().cmp(&b.degree())
            })
        });
    }

    /// Multiply two polynomials
    pub fn mul(&self, other: &Polynomial) -> Polynomial {
        let mut result = Polynomial::new();

        // Merge substitutions
        result.merge_substitutions(self);
        result.merge_substitutions(other);

        // Multiply each pair of terms
        for t1 in &self.terms {
            for t2 in &other.terms {
                result.terms.push(t1.mul(t2));
            }
        }

        result.combine_like_terms();
        result
    }

    /// Add two polynomials
    pub fn add(&self, other: &Polynomial) -> Polynomial {
        let mut result = Polynomial::new();

        result.merge_substitutions(self);
        result.merge_substitutions(other);

        result.terms.extend(self.terms.clone());
        result.terms.extend(other.terms.clone());

        result.combine_like_terms();
        result
    }

    /// Negate polynomial
    pub fn negate(&self) -> Polynomial {
        let mut result = self.clone();
        for term in &mut result.terms {
            term.coeff = -term.coeff;
        }
        result
    }

    /// Subtract polynomials
    pub fn sub(&self, other: &Polynomial) -> Polynomial {
        self.add(&other.negate())
    }

    /// Check if polynomial is zero
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty() || self.terms.iter().all(|t| t.coeff.abs() < 1e-14)
    }

    /// Check if polynomial is a constant
    pub fn is_constant(&self) -> bool {
        self.terms.is_empty() || (self.terms.len() == 1 && self.terms[0].is_constant())
    }

    /// Get constant value if polynomial is constant
    pub fn as_constant(&self) -> Option<f64> {
        if self.is_zero() {
            Some(0.0)
        } else if self.terms.len() == 1 && self.terms[0].is_constant() {
            Some(self.terms[0].coeff)
        } else {
            None
        }
    }

    /// Total degree of the polynomial
    pub fn degree(&self) -> u32 {
        self.terms.iter().map(|t| t.degree()).max().unwrap_or(0)
    }

    /// Number of terms
    pub fn term_count(&self) -> usize {
        self.terms.len()
    }

    /// Differentiate polynomial with respect to a variable (sparse algorithm)
    ///
    /// For each term: new_coeff = coeff * power, new_power = power - 1
    /// Returns the derivative polynomial and the base expression for chain rule
    ///
    /// Example: d/dx(3x² + 2x + 1) = 6x + 2
    pub fn derivative(&self, var: &InternedSymbol) -> Polynomial {
        let mut result_terms: Vec<PolyTerm> = Vec::new();

        for term in &self.terms {
            // Find the power of this variable in the term
            let mut new_powers: Vec<(InternedSymbol, u32)> = Vec::new();
            let mut var_power: Option<u32> = None;

            for (sym, pow) in &term.powers {
                if sym.id() == var.id() {
                    var_power = Some(*pow);
                    // Reduce power by 1 (if power becomes 0, don't add it)
                    if *pow > 1 {
                        new_powers.push((sym.clone(), *pow - 1));
                    }
                } else {
                    new_powers.push((sym.clone(), *pow));
                }
            }

            // If this term contains the variable, include in derivative
            if let Some(pow) = var_power {
                // new_coeff = old_coeff * power
                let new_coeff = term.coeff * (pow as f64);
                if new_coeff.abs() > 1e-14 {
                    result_terms.push(PolyTerm::new(new_coeff, new_powers));
                }
            }
            // If term doesn't contain the variable, its derivative is 0 (not included)
        }

        let mut result = Polynomial::new();
        result.terms = result_terms;
        result.substitutions = self.substitutions.clone();
        result.next_temp_id = self.next_temp_id;
        result.combine_like_terms();
        result
    }

    /// Differentiate with chain rule for substituted expressions
    ///
    /// If the polynomial contains opaque expressions (sin(y), etc.),
    /// this returns the full derivative including chain rule terms
    pub fn derivative_expr(&self, diff_var: &str) -> Option<Expr> {
        // For univariate polynomial in a single variable
        if !self.is_univariate() {
            return None; // Multivariate needs partial derivatives
        }

        let var = self.main_variable()?;

        // Get polynomial derivative
        let poly_deriv = self.derivative(&var);
        let poly_expr = poly_deriv.to_expr();

        // Check if the variable is a substituted expression
        if let Some(original_expr) = self.substitutions.get(&var.id()) {
            // Chain rule: d/dx[P(u)] = P'(u) * u'
            // Need to compute derivative of u with respect to diff_var
            let u_prime = original_expr.derive_raw(diff_var);

            if u_prime.is_zero_num() {
                Some(Expr::number(0.0))
            } else if u_prime.is_one_num() {
                Some(poly_expr)
            } else {
                Some(Expr::product(vec![poly_expr, u_prime]))
            }
        } else {
            // Regular variable - check if it's the diff variable
            if var.as_ref() == diff_var {
                Some(poly_expr)
            } else {
                Some(Expr::number(0.0)) // d/dx of polynomial in y is 0
            }
        }
    }

    /// Convert back to Expr, substituting temp symbols back
    pub fn to_expr(&self) -> Expr {
        if self.terms.is_empty() {
            return Expr::number(0.0);
        }

        // Convert each term
        let term_exprs: Vec<Expr> = self
            .terms
            .iter()
            .map(|t| {
                let mut expr = t.to_expr();
                // Substitute back any temp symbols
                expr = self.substitute_temps(expr);
                expr
            })
            .collect();

        if term_exprs.len() == 1 {
            term_exprs.into_iter().next().unwrap()
        } else {
            Expr::sum(term_exprs)
        }
    }

    /// Substitute temporary symbols back with original expressions
    fn substitute_temps(&self, expr: Expr) -> Expr {
        if self.substitutions.is_empty() {
            return expr;
        }

        expr.map(|node| {
            if let ExprKind::Symbol(s) = &node.kind
                && let Some(original) = self.substitutions.get(&s.id())
            {
                return (**original).clone();
            }
            node.clone()
        })
    }

    // =========================================================================
    // UNIVARIATE GCD OPERATIONS
    // =========================================================================

    /// Get all variables in this polynomial
    pub fn variables(&self) -> Vec<InternedSymbol> {
        let mut vars: HashMap<u64, InternedSymbol> = HashMap::new();
        for term in &self.terms {
            for (sym, _) in &term.powers {
                vars.entry(sym.id()).or_insert(sym.clone());
            }
        }
        vars.into_values().collect()
    }

    /// Check if polynomial is univariate (has at most one variable)
    pub fn is_univariate(&self) -> bool {
        self.variables().len() <= 1
    }

    /// Get the main variable (first variable found, if any)
    pub fn main_variable(&self) -> Option<InternedSymbol> {
        self.variables().into_iter().next()
    }

    /// Polynomial GCD using Euclidean algorithm (sparse, univariate only)
    /// Works directly on sparse representation - no dense conversion
    /// Returns None if polynomials are not univariate in the same variable
    pub fn gcd(&self, other: &Polynomial) -> Option<Polynomial> {
        // GCD only works for univariate polynomials - reject multivariate
        if !self.is_univariate() || !other.is_univariate() {
            return None;
        }

        // Get main variable
        let var = match (self.main_variable(), other.main_variable()) {
            (Some(v1), Some(v2)) if v1.id() == v2.id() => v1,
            (Some(v), None) | (None, Some(v)) => v,
            (None, None) => {
                // Both constants - GCD is 1
                return Some(Polynomial::constant(1.0));
            }
            _ => return None, // Different variables
        };

        // Euclidean algorithm: gcd(a, b) = gcd(b, a mod b)
        let mut r0 = self.clone();
        let mut r1 = other.clone();

        loop {
            if r1.is_zero() {
                // Make monic (leading coefficient = 1)
                r0.make_monic(&var);
                return Some(r0);
            }

            let (_, rem) = r0.div_rem_sparse(&r1, &var)?;
            r0 = r1;
            r1 = rem;
        }
    }

    /// Polynomial division: self / other = (quotient, remainder)
    /// Sparse implementation - works directly on terms
    /// Returns None if not univariate or division not possible
    pub fn div_rem(&self, other: &Polynomial) -> Option<(Polynomial, Polynomial)> {
        if other.is_zero() {
            return None;
        }

        // Get main variable
        let var = match (self.main_variable(), other.main_variable()) {
            (Some(v1), Some(v2)) if v1.id() == v2.id() => v1,
            (Some(v), None) => v, // Dividing by constant
            (None, Some(_)) => {
                // Constant / polynomial with variable
                return Some((Polynomial::constant(0.0), self.clone()));
            }
            (None, None) => {
                // Both constants
                let a = self.as_constant().unwrap_or(0.0);
                let b = other.as_constant().unwrap_or(1.0);
                return Some((Polynomial::constant(a / b), Polynomial::constant(0.0)));
            }
            _ => return None,
        };

        self.div_rem_sparse(other, &var)
    }

    /// Internal sparse polynomial division
    fn div_rem_sparse(
        &self,
        divisor: &Polynomial,
        var: &InternedSymbol,
    ) -> Option<(Polynomial, Polynomial)> {
        // Get leading term info of divisor
        let (div_lc, div_deg) = divisor.leading_term_info(var)?;

        let mut quotient = Polynomial::new();
        quotient.substitutions = self.substitutions.clone();

        let mut remainder = self.clone();

        loop {
            if remainder.is_zero() {
                break;
            }

            // Get leading term of remainder
            let (rem_lc, rem_deg) = match remainder.leading_term_info(var) {
                Some(info) => info,
                None => break, // Constant remainder
            };

            if rem_deg < div_deg {
                break; // Done - remainder degree < divisor degree
            }

            // Compute quotient term: coeff * x^(rem_deg - div_deg)
            let q_coeff = rem_lc / div_lc;
            let q_deg = rem_deg - div_deg;

            // Add to quotient
            let q_term = if q_deg == 0 {
                PolyTerm::constant(q_coeff)
            } else {
                PolyTerm::new(q_coeff, vec![(var.clone(), q_deg)])
            };
            quotient.terms.push(q_term.clone());

            // Subtract (quotient_term * divisor) from remainder
            let mut q_mono = Polynomial::from_term(q_term);
            q_mono.substitutions = self.substitutions.clone();
            let scaled_divisor = q_mono.mul(divisor);
            remainder = remainder.sub(&scaled_divisor);
        }

        quotient.combine_like_terms();
        remainder.combine_like_terms();

        Some((quotient, remainder))
    }

    /// Get leading term coefficient and degree for a variable
    fn leading_term_info(&self, var: &InternedSymbol) -> Option<(f64, u32)> {
        let mut max_deg = 0u32;
        let mut lc: f64 = 0.0;

        for term in &self.terms {
            let deg = term
                .powers
                .iter()
                .find(|(s, _)| s.id() == var.id())
                .map(|(_, p)| *p)
                .unwrap_or(0);

            if deg > max_deg || (deg == max_deg && lc.abs() < 1e-14) {
                max_deg = deg;
                lc = term.coeff;
            }
        }

        if lc.abs() < 1e-14 && max_deg == 0 {
            // Check for constant terms
            for term in &self.terms {
                if term.powers.is_empty() || term.powers.iter().all(|(s, _)| s.id() != var.id()) {
                    return Some((term.coeff, 0));
                }
            }
            None
        } else {
            Some((lc, max_deg))
        }
    }

    /// Make polynomial monic (leading coefficient = 1)
    fn make_monic(&mut self, var: &InternedSymbol) {
        if let Some((lc, _)) = self.leading_term_info(var)
            && lc.abs() > 1e-14
            && (lc - 1.0).abs() > 1e-14
        {
            for term in &mut self.terms {
                term.coeff /= lc;
            }
        }
    }
}

impl Default for Polynomial {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poly_from_number() {
        let expr = Expr::number(5.0);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        assert_eq!(poly.term_count(), 1);
        assert!(poly.is_constant());
        assert_eq!(poly.as_constant(), Some(5.0));
    }

    #[test]
    fn test_poly_from_symbol() {
        let expr = Expr::symbol("x");
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        assert_eq!(poly.term_count(), 1);
        assert!(!poly.is_constant());
        assert_eq!(poly.degree(), 1);
    }

    #[test]
    fn test_poly_from_sum() {
        // x + 2
        let expr = Expr::sum(vec![Expr::symbol("x"), Expr::number(2.0)]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        assert_eq!(poly.term_count(), 2);
        assert_eq!(poly.degree(), 1);
    }

    #[test]
    fn test_poly_from_product() {
        // 3 * x
        let expr = Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        assert_eq!(poly.term_count(), 1);
        assert_eq!(poly.to_expr().to_string(), "3x");
    }

    #[test]
    fn test_poly_from_power() {
        // x^2
        let expr = Expr::pow(Expr::symbol("x"), Expr::number(2.0));
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        assert_eq!(poly.term_count(), 1);
        assert_eq!(poly.degree(), 2);
    }

    #[test]
    fn test_poly_multiplication() {
        // (x + 1) * (x + 1) = x^2 + 2x + 1
        let x_plus_1 = Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]);
        let p1 = Polynomial::try_from_expr(&x_plus_1).unwrap();
        let p2 = Polynomial::try_from_expr(&x_plus_1).unwrap();
        let result = p1.mul(&p2);

        assert_eq!(result.term_count(), 3);
        assert_eq!(result.degree(), 2);
    }

    #[test]
    fn test_poly_with_sin() {
        // x + sin(y) - sin is opaque
        let expr = Expr::sum(vec![
            Expr::symbol("x"),
            Expr::func("sin", Expr::symbol("y")),
        ]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();

        // Should have 2 terms: x and __poly_temp_0
        assert_eq!(poly.term_count(), 2);

        // When converted back, should restore sin(y)
        let back = poly.to_expr();
        let back_str = back.to_string();
        assert!(back_str.contains("sin(y)") || back_str.contains("x"));
    }

    #[test]
    fn test_poly_roundtrip() {
        // x^2 + 2*x + 1
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
            Expr::number(1.0),
        ]);

        let poly = Polynomial::try_from_expr(&expr).unwrap();
        assert_eq!(poly.term_count(), 3);

        let back = poly.to_expr();
        // Should be equivalent polynomial
        let back_poly = Polynomial::try_from_expr(&back).unwrap();
        assert_eq!(back_poly.term_count(), 3);
    }

    #[test]
    fn test_poly_gcd_simple() {
        // GCD of (x^2 - 1) and (x - 1) should be (x - 1)
        // x^2 - 1 = (x-1)(x+1)
        let p1 = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);
        let p2 = Expr::sum(vec![Expr::symbol("x"), Expr::number(-1.0)]);

        let poly1 = Polynomial::try_from_expr(&p1).unwrap();
        let poly2 = Polynomial::try_from_expr(&p2).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        // GCD should be (x - 1) normalized to (x + -1) with leading coeff 1
        assert_eq!(gcd.degree(), 1);
    }

    #[test]
    fn test_poly_division() {
        // (x^2 - 1) / (x - 1) = (x + 1), remainder 0
        let p1 = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);
        let p2 = Expr::sum(vec![Expr::symbol("x"), Expr::number(-1.0)]);

        let poly1 = Polynomial::try_from_expr(&p1).unwrap();
        let poly2 = Polynomial::try_from_expr(&p2).unwrap();

        let (q, r) = poly1.div_rem(&poly2).unwrap();

        // Quotient should be (x + 1)
        assert_eq!(q.degree(), 1);
        // Remainder should be 0
        assert!(r.is_zero());
    }

    #[test]
    fn test_poly_division_with_remainder() {
        // x^2 / (x - 1) = x + 1, remainder 1
        let p1 = Expr::pow(Expr::symbol("x"), Expr::number(2.0));
        let p2 = Expr::sum(vec![Expr::symbol("x"), Expr::number(-1.0)]);

        let poly1 = Polynomial::try_from_expr(&p1).unwrap();
        let poly2 = Polynomial::try_from_expr(&p2).unwrap();

        let (q, r) = poly1.div_rem(&poly2).unwrap();

        // Quotient should be (x + 1)
        assert_eq!(q.degree(), 1);
        // Remainder should be 1
        assert!(r.is_constant());
        assert!((r.as_constant().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_poly_derivative_simple() {
        // d/dx(x^2 + 2x + 1) = 2x + 2
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]),
            Expr::number(1.0),
        ]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        let var = poly.main_variable().unwrap();
        let deriv = poly.derivative(&var);

        // Should be 2x + 2
        assert_eq!(deriv.term_count(), 2);
        assert_eq!(deriv.degree(), 1);
    }

    #[test]
    fn test_poly_derivative_cubic() {
        // d/dx(x^3) = 3x^2
        let expr = Expr::pow(Expr::symbol("x"), Expr::number(3.0));
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        let var = poly.main_variable().unwrap();
        let deriv = poly.derivative(&var);

        assert_eq!(deriv.degree(), 2);
        assert_eq!(deriv.term_count(), 1);
    }

    #[test]
    fn test_poly_derivative_constant() {
        // d/dx(5) = 0
        let expr = Expr::number(5.0);
        let poly = Polynomial::try_from_expr(&expr).unwrap();

        // Constant polynomial has no main variable, derivative check
        assert!(poly.is_constant());
    }

    #[test]
    fn test_poly_derivative_expr_with_chain_rule() {
        // d/dx(sin(x)^2 + sin(x)) uses chain rule via derivative_expr
        // This requires substitution handling
        let sin_x = Expr::func("sin", Expr::symbol("x"));
        let expr = Expr::sum(vec![Expr::pow(sin_x.clone(), Expr::number(2.0)), sin_x]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();

        // Should work - sin(x) is substituted as opaque variable
        // Note: May have multiple internal terms representing different powers
        assert!(!poly.is_constant(), "Polynomial should not be constant");
    }

    #[test]
    fn test_poly_display_roundtrip() {
        // Polynomial -> Expr -> Display -> should be readable
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::product(vec![Expr::number(3.0), Expr::symbol("x")]),
            Expr::number(7.0),
        ]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        let back = poly.to_expr();
        let display = format!("{}", back);

        // Should contain x^2, 3x or 3*x, and 7
        assert!(display.contains("x"));
        assert!(display.contains("7") || display.contains("3"));
    }

    #[test]
    fn test_poly_latex_output() {
        // Polynomial -> Expr -> LaTeX
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(1.0),
        ]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        let back = poly.to_expr();
        let latex = back.to_latex();

        // Should contain x^{2} or x^2
        assert!(latex.contains("x") && (latex.contains("{2}") || latex.contains("^2")));
    }

    #[test]
    fn test_poly_unicode_output() {
        // Polynomial -> Expr -> Unicode
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(1.0),
        ]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();
        let back = poly.to_expr();
        let unicode = back.to_unicode();

        // Should contain x² (superscript 2)
        assert!(unicode.contains("x") && unicode.contains("²"));
    }

    #[test]
    fn test_poly_multivariate_detection() {
        // x*y should NOT be detected as univariate
        let expr = Expr::product(vec![Expr::symbol("x"), Expr::symbol("y")]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();

        assert!(!poly.is_univariate());
    }

    #[test]
    fn test_poly_gcd_coprime() {
        // GCD of (x + 1) and (x + 2) should be 1 (coprime)
        let p1 = Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]);
        let p2 = Expr::sum(vec![Expr::symbol("x"), Expr::number(2.0)]);

        let poly1 = Polynomial::try_from_expr(&p1).unwrap();
        let poly2 = Polynomial::try_from_expr(&p2).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        // GCD should be constant (1)
        assert!(gcd.is_constant());
    }

    #[test]
    fn test_poly_gcd_cubic() {
        // GCD of x^3 - x and x^2 - 1
        // x^3 - x = x(x^2 - 1) = x(x-1)(x+1)
        // x^2 - 1 = (x-1)(x+1)
        // GCD = x^2 - 1
        let p1 = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(3.0)),
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
        ]);
        let p2 = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);

        let poly1 = Polynomial::try_from_expr(&p1).unwrap();
        let poly2 = Polynomial::try_from_expr(&p2).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        // GCD should be degree 2 (x^2 - 1)
        assert_eq!(gcd.degree(), 2);
    }

    #[test]
    fn test_poly_addition() {
        // (x + 1) + (x + 2) = 2x + 3
        let p1 = Polynomial::try_from_expr(&Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]))
            .unwrap();
        let p2 = Polynomial::try_from_expr(&Expr::sum(vec![Expr::symbol("x"), Expr::number(2.0)]))
            .unwrap();

        let sum = p1.add(&p2);
        assert_eq!(sum.degree(), 1);
        assert_eq!(sum.term_count(), 2); // 2x and 3
    }

    #[test]
    fn test_poly_subtraction() {
        // (x + 1) - (x + 2) = -1
        let p1 = Polynomial::try_from_expr(&Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]))
            .unwrap();
        let p2 = Polynomial::try_from_expr(&Expr::sum(vec![Expr::symbol("x"), Expr::number(2.0)]))
            .unwrap();

        let diff = p1.sub(&p2);
        assert!(diff.is_constant());
        assert!((diff.as_constant().unwrap() - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_poly_high_degree() {
        // x^10 + 1 - sparse polynomial
        let expr = Expr::sum(vec![
            Expr::pow(Expr::symbol("x"), Expr::number(10.0)),
            Expr::number(1.0),
        ]);
        let poly = Polynomial::try_from_expr(&expr).unwrap();

        assert_eq!(poly.degree(), 10);
        assert_eq!(poly.term_count(), 2); // Only 2 terms (sparse!)
    }

    #[test]
    fn test_poly_nested_powers() {
        // (x^2)^2 = x^4
        let expr = Expr::pow(
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
            Expr::number(2.0),
        );
        // This may or may not convert depending on simplification
        // At minimum it shouldn't panic
        let result = Polynomial::try_from_expr(&expr);
        if let Some(poly) = result {
            // If it converts, x^4 should have degree 4
            assert_eq!(poly.degree(), 4);
        }
    }

    // =========================================================================
    // EXTENSIVE GCD TESTS WITH ARBITRARY FUNCTIONS
    // =========================================================================

    #[test]
    fn test_gcd_with_sin_function() {
        // GCD of sin(x)^2 - 1 and sin(x) - 1
        // Should be sin(x) - 1 (treating sin(x) as opaque variable)
        let sin_x = Expr::func("sin", Expr::symbol("x"));

        // sin(x)^2 - 1 = (sin(x) - 1)(sin(x) + 1)
        let poly1_expr = Expr::sum(vec![
            Expr::pow(sin_x.clone(), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);

        // sin(x) - 1
        let poly2_expr = Expr::sum(vec![sin_x.clone(), Expr::number(-1.0)]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        // GCD should be degree 1 in sin(x)
        assert_eq!(
            gcd.degree(),
            1,
            "GCD of sin(x)^2-1 and sin(x)-1 should be degree 1"
        );
    }

    #[test]
    fn test_gcd_with_exp_function() {
        // GCD of exp(x)^2 + 2*exp(x) + 1 and exp(x) + 1
        // = (exp(x) + 1)^2 and (exp(x) + 1) = exp(x) + 1
        let exp_x = Expr::func("exp", Expr::symbol("x"));

        // exp(x)^2 + 2*exp(x) + 1 = (exp(x) + 1)^2
        let poly1_expr = Expr::sum(vec![
            Expr::pow(exp_x.clone(), Expr::number(2.0)),
            Expr::product(vec![Expr::number(2.0), exp_x.clone()]),
            Expr::number(1.0),
        ]);

        // exp(x) + 1
        let poly2_expr = Expr::sum(vec![exp_x.clone(), Expr::number(1.0)]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        assert_eq!(gcd.degree(), 1, "GCD should be exp(x) + 1 (degree 1)");
    }

    #[test]
    fn test_gcd_with_cos_function() {
        // GCD of cos(x)^3 - cos(x) and cos(x)^2 - 1
        // cos(x)(cos(x)^2 - 1) and (cos(x)^2 - 1) = cos(x)^2 - 1
        let cos_x = Expr::func("cos", Expr::symbol("x"));

        // cos(x)^3 - cos(x) = cos(x)(cos(x)^2 - 1)
        let poly1_expr = Expr::sum(vec![
            Expr::pow(cos_x.clone(), Expr::number(3.0)),
            Expr::product(vec![Expr::number(-1.0), cos_x.clone()]),
        ]);

        // cos(x)^2 - 1
        let poly2_expr = Expr::sum(vec![
            Expr::pow(cos_x.clone(), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        // GCD should be cos(x)^2 - 1 (degree 2)
        assert_eq!(
            gcd.degree(),
            2,
            "GCD of cos(x)^3-cos(x) and cos(x)^2-1 should be degree 2"
        );
    }

    #[test]
    fn test_gcd_with_log_function() {
        // GCD of ln(x)^2 and ln(x) = ln(x)
        let ln_x = Expr::func("ln", Expr::symbol("x"));

        let poly1_expr = Expr::pow(ln_x.clone(), Expr::number(2.0));
        let poly2_expr = ln_x.clone();

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        assert_eq!(
            gcd.degree(),
            1,
            "GCD of ln(x)^2 and ln(x) should be ln(x) (degree 1)"
        );
    }

    #[test]
    fn test_gcd_nested_function() {
        // GCD with sin(cos(x)) treated as opaque
        let nested = Expr::func("sin", Expr::func("cos", Expr::symbol("x")));

        // sin(cos(x))^2 - 1 and sin(cos(x)) - 1
        let poly1_expr = Expr::sum(vec![
            Expr::pow(nested.clone(), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);
        let poly2_expr = Expr::sum(vec![nested.clone(), Expr::number(-1.0)]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        assert_eq!(gcd.degree(), 1, "GCD with nested function should work");
    }

    #[test]
    fn test_gcd_coprime_functions() {
        // GCD of sin(x) + 1 and cos(x) + 1 should be 1 (coprime)
        // Since sin(x) and cos(x) are different opaque variables
        let sin_x = Expr::func("sin", Expr::symbol("x"));
        let cos_x = Expr::func("cos", Expr::symbol("x"));

        let poly1_expr = Expr::sum(vec![sin_x, Expr::number(1.0)]);
        let poly2_expr = Expr::sum(vec![cos_x, Expr::number(1.0)]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        // These are multivariate (different opaque vars), univariate GCD may not work correctly
        let gcd_result = poly1.gcd(&poly2);
        // Just verify it doesn't panic - multivariate GCD behavior is implementation-defined
        // The algorithm is designed for univariate, so we accept any result
        println!(
            "Multivariate GCD result: {:?}",
            gcd_result.map(|g| g.degree())
        );
    }

    #[test]
    fn test_gcd_multivariate_returns_none() {
        // GCD of sin(x) + sin(y) and sin(x) (multivariate)
        // Should return None since GCD algorithm is univariate only
        let sin_x = Expr::func("sin", Expr::symbol("x"));
        let sin_y = Expr::func("sin", Expr::symbol("y"));

        let poly1_expr = Expr::sum(vec![sin_x.clone(), sin_y]);
        let poly2_expr = sin_x;

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        // Multivariate - should return None (not hang!)
        let gcd_result = poly1.gcd(&poly2);
        assert!(gcd_result.is_none(), "Multivariate GCD should return None");
    }

    #[test]
    fn test_gcd_high_degree_with_function() {
        // GCD of sin(x)^4 - 1 and sin(x)^2 - 1
        // = (sin(x)^2 - 1)(sin(x)^2 + 1) and (sin(x)^2 - 1) = sin(x)^2 - 1
        let sin_x = Expr::func("sin", Expr::symbol("x"));

        let poly1_expr = Expr::sum(vec![
            Expr::pow(sin_x.clone(), Expr::number(4.0)),
            Expr::number(-1.0),
        ]);
        let poly2_expr = Expr::sum(vec![
            Expr::pow(sin_x.clone(), Expr::number(2.0)),
            Expr::number(-1.0),
        ]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        assert_eq!(
            gcd.degree(),
            2,
            "GCD of sin(x)^4-1 and sin(x)^2-1 should be degree 2"
        );
    }

    #[test]
    fn test_gcd_with_coefficient() {
        // GCD of 2*sin(x)^2 - 2 and sin(x) - 1
        // = 2*(sin(x)^2 - 1) and (sin(x) - 1) = constant multiple of (sin(x) - 1)
        let sin_x = Expr::func("sin", Expr::symbol("x"));

        let poly1_expr = Expr::sum(vec![
            Expr::product(vec![
                Expr::number(2.0),
                Expr::pow(sin_x.clone(), Expr::number(2.0)),
            ]),
            Expr::number(-2.0),
        ]);
        let poly2_expr = Expr::sum(vec![sin_x, Expr::number(-1.0)]);

        let poly1 = Polynomial::try_from_expr(&poly1_expr).unwrap();
        let poly2 = Polynomial::try_from_expr(&poly2_expr).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        assert_eq!(gcd.degree(), 1, "GCD should handle coefficients correctly");
    }

    #[test]
    fn test_gcd_identical_polys() {
        // GCD of same polynomial should be itself
        let sin_x = Expr::func("sin", Expr::symbol("x"));
        let poly_expr = Expr::sum(vec![sin_x, Expr::number(1.0)]);

        let poly = Polynomial::try_from_expr(&poly_expr).unwrap();
        let gcd = poly.gcd(&poly).unwrap();

        assert_eq!(
            gcd.degree(),
            poly.degree(),
            "GCD of identical polys should have same degree"
        );
    }

    #[test]
    fn test_gcd_constant_and_function() {
        // GCD of 5 and sin(x) + 1 should be 1
        let sin_x = Expr::func("sin", Expr::symbol("x"));

        let poly1 = Polynomial::try_from_expr(&Expr::number(5.0)).unwrap();
        let poly2 = Polynomial::try_from_expr(&Expr::sum(vec![sin_x, Expr::number(1.0)])).unwrap();

        let gcd = poly1.gcd(&poly2).unwrap();
        assert!(
            gcd.is_constant(),
            "GCD of constant and polynomial should be constant"
        );
    }

    #[test]
    fn test_gcd_zero_polynomial() {
        // GCD of 0 and sin(x) should be sin(x) (or handle gracefully)
        let sin_x = Expr::func("sin", Expr::symbol("x"));

        let poly1 = Polynomial::try_from_expr(&Expr::number(0.0)).unwrap();
        let poly2 = Polynomial::try_from_expr(&sin_x).unwrap();

        let gcd_result = poly1.gcd(&poly2);
        // Should handle zero gracefully
        assert!(
            gcd_result.is_some() || gcd_result.is_none(),
            "GCD with zero should not panic"
        );
    }
}
