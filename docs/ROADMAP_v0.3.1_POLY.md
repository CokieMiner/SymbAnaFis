# SymbAnaFis v0.3.1 - Polynomial Coefficient Ring Architecture

## Overview

This document outlines the implementation plan for adding GiNaC-style polynomial coefficient rings to SymbAnaFis. This architectural change will dramatically improve simplification performance for polynomial expressions (10-100x faster) while maintaining our fast parsing, differentiation, and transcendental function support.

## Motivation

### Current Performance (v0.3.0 - December 2025)
| Operation | vs SymPy | vs Symbolica |
|-----------|----------|--------------|
| Parsing | 17-21x faster ✓ | 1.6-2.3x faster ✓ |
| Differentiation (includes simplify) | Mixed | 13-62x **slower** ✗ |
| Simplification only | 4-43x faster ✓ | — |
| Evaluation | ~400ns ✓ | — |

> **Note**: `diff()` always runs simplification. Symbolica's `derivative()` also auto-normalizes.

**The bottleneck**: Rule-based polynomial simplification requires many iterations of pattern matching, while Symbolica uses native polynomial arithmetic.

### Target Performance (v0.3.1)
- Full diff+simplify pipeline: **competitive with Symbolica** (within 2-5x)
- Polynomial simplification: **10-100x faster** than current implementation

---

## Architecture

### Current: Binary AST Tree
```
x² + 2x + 1 represented as:
    Add
    ├── Add
    │   ├── Pow(x, 2)
    │   └── Mul(2, x)
    └── 1
```
**Problem**: Simplifying requires pattern matching, many rules, multiple iterations.

### New: Polynomial Coefficient Ring
```
x² + 2x + 1 represented as:
    Poly { var: x, coeffs: [1, 2, 1] }
    //                      ↑  ↑  ↑
    //                     x⁰ x¹ x²
```
**Advantage**: Addition/multiplication are direct coefficient operations. Self-simplifying.

### Recursive Structure (GiNaC-style)
Coefficients can themselves be expressions, enabling multivariate polynomials:
```rust
// x² + 2xy + y² = (x + y)² 
Poly {
    var: x,
    coeffs: [
        (0, Poly { var: y, coeffs: [(2, 1)] }),  // y² (x⁰ term)
        (1, Poly { var: y, coeffs: [(1, 2)] }),  // 2y (x¹ term)
        (2, Expr::number(1)),                     // 1  (x² term)
    ]
}
```

---

## Comparison with GiNaC

This architecture aligns with GiNaC's core philosophy with one critical adaptation.

### Alignment ✓

| Feature | SymbAnaFis | GiNaC | Status |
|---------|------------|-------|--------|
| **Multivariate** | Recursive R[y][x] | Recursive | ✓ Aligned |
| **Variable Ordering** | Lexicographic ID | Canonical ordering | ✓ Aligned |
| **Arithmetic** | Coefficient-wise | Optimized algorithms | ✓ Aligned |
| **Integration** | Hybrid/Transient | Core object | Slight diff |

### Critical Difference: Sparse Representation

> [!IMPORTANT]
> **Must use sparse representation, not dense.**

| Representation | Example: x¹⁰⁰⁰ + 1 | Memory |
|----------------|-------------------|--------|
| Dense `Vec<Expr>` | 1001-element vector | O(max_degree) |
| **Sparse `Vec<(u32, Expr)>`** | 2-element vector | O(terms) |

GiNaC stores `(coefficient, exponent)` pairs. We adopt this approach:

```rust
pub struct Poly {
    pub var: InternedSymbol,
    /// Sparse: (exponent, coefficient) pairs, sorted by exponent
    pub terms: Vec<(u32, Expr)>,
}
```

## Implementation Plan

### Phase 1: Core Poly Structure (~200 lines)

#### [NEW] `src/poly.rs`

```rust
/// Sparse univariate polynomial with arbitrary expression coefficients
/// 
/// Uses GiNaC-style sparse representation for memory efficiency:
/// x¹⁰⁰⁰ + 1 stores only 2 terms, not 1001 coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct Poly {
    /// The polynomial variable (interned for O(1) comparison)
    pub var: InternedSymbol,
    /// Sparse terms: (exponent, coefficient) pairs, sorted by exponent ascending
    /// Invariant: no zero coefficients, no duplicate exponents
    pub terms: Vec<(u32, Expr)>,
}

impl Poly {
    /// Create from sparse terms, auto-merging duplicates and removing zeros
    pub fn new(var: InternedSymbol, terms: Vec<(u32, Expr)>) -> Self;
    
    /// Create from dense coefficients (for convenience)
    pub fn from_dense(var: InternedSymbol, coeffs: Vec<Expr>) -> Self;
    
    /// Degree of the polynomial (None for zero polynomial)
    pub fn degree(&self) -> Option<u32>;
    
    /// Check if this is the zero polynomial
    pub fn is_zero(&self) -> bool;
    
    /// Get the leading coefficient (highest degree term)
    pub fn leading_coeff(&self) -> Option<&Expr>;
    
    /// Get coefficient for a specific power (returns 0 if not present)
    pub fn coeff(&self, power: u32) -> Expr;
}
```

#### Polynomial Arithmetic
```rust
impl Poly {
    /// Add two polynomials in the same variable: O(n + m) where n, m are term counts
    pub fn add(&self, other: &Poly) -> Poly;
    
    /// Subtract two polynomials: O(n + m)
    pub fn sub(&self, other: &Poly) -> Poly;
    
    /// Multiply two polynomials: O(n * m) term-wise, can use Karatsuba for large polys
    pub fn mul(&self, other: &Poly) -> Poly;
    
    /// Polynomial division with remainder
    /// Returns (quotient, remainder) such that self = quotient * other + remainder
    pub fn div(&self, other: &Poly) -> Option<(Poly, Poly)>;
    
    /// Greatest Common Divisor (Euclidean algorithm)
    pub fn gcd(&self, other: &Poly) -> Poly;
    
    /// Scalar multiplication: coeff * poly
    pub fn scale(&self, coeff: &Expr) -> Poly;
    
    /// Evaluate polynomial at a value: O(n) using Horner's method adaptation
    pub fn eval(&self, value: &Expr) -> Expr;
}
```

### Phase 2: AST Integration (~100 lines)

#### [MODIFY] `src/ast.rs`

Add new ExprKind variant:
```rust
pub enum ExprKind {
    Number(f64),
    Symbol(InternedSymbol),
    FunctionCall { name: String, args: Vec<Expr> },
    Add(Arc<Expr>, Arc<Expr>),
    Sub(Arc<Expr>, Arc<Expr>),
    Mul(Arc<Expr>, Arc<Expr>),
    Div(Arc<Expr>, Arc<Expr>),
    Pow(Arc<Expr>, Arc<Expr>),
    Derivative { inner: Arc<Expr>, var: String, order: u32 },
    
    // NEW: Polynomial in coefficient ring form
    Poly(crate::poly::Poly),
}
```

#### Conversion Functions
```rust
impl Expr {
    /// Try to convert expression to polynomial form in variable `var`
    /// Returns None if expression contains `var` in non-polynomial position
    /// (e.g., sin(x), x^y where y is not a non-negative integer)
    pub fn try_as_poly(&self, var: &str) -> Option<Poly>;
    
    /// Convert polynomial back to standard AST form
    pub fn from_poly(poly: Poly) -> Expr;
    
    /// Check if expression is polynomial in variable `var`
    pub fn is_polynomial_in(&self, var: &str) -> bool;
}

### Phase 2.5: Differentiation Support (~50 lines)

#### [MODIFY] `src/differentiation/mod.rs`

Add differentiation rule for `ExprKind::Poly`:
```rust
ExprKind::Poly(poly) => {
    let diff_var_interned = get_or_intern(diff_var);
    
    if poly.var == diff_var_interned {
        // d/dx[Σ c_i x^i] = Σ i·c_i x^(i-1)
        let new_coeffs: Vec<Expr> = poly.coeffs
            .iter()
            .enumerate()
            .skip(1)  // constant term vanishes
            .map(|(i, c)| Expr::number(i as f64) * c.clone())
            .collect();
        
        Arc::new(Expr::new(ExprKind::Poly(Poly::new(poly.var.clone(), new_coeffs))))
    } else {
        // Differentiate each coefficient with respect to diff_var
        let new_coeffs: Vec<Expr> = poly.coeffs
            .iter()
            .map(|c| differentiate_inner(c.clone(), diff_var, ctx))
            .map(|arc| Arc::unwrap_or_clone(arc))
            .collect();
        
        Arc::new(Expr::new(ExprKind::Poly(Poly::new(poly.var.clone(), new_coeffs))))
    }
}
```

### Phase 3: Engine Integration (~50 lines)

#### [MODIFY] `src/simplification/engine.rs`

Add Poly handling to `apply_rules_bottom_up`:
```rust
AstKind::Poly(poly) => {
    // Simplify each coefficient recursively
    let simplified_coeffs: Vec<Expr> = poly.coeffs
        .iter()
        .map(|c| self.apply_rules_bottom_up(Arc::new(c.clone()), depth + 1))
        .map(unwrap_or_clone)
        .collect();
    
    let new_poly = Poly::new(poly.var.clone(), simplified_coeffs);
    Arc::new(Expr::new(AstKind::Poly(new_poly)))
}
```

#### [MODIFY] `src/simplification/rules/mod.rs`

Add ExprKind variant:
```rust
pub enum ExprKind {
    // ... existing
    Poly,  // NEW
}
```

### Phase 4: Detection & Conversion Rules (~150 lines)

#### [NEW] `src/simplification/rules/polynomial/mod.rs`

```rust
// High priority: detect polynomial structure and convert
rule!(DetectPolynomial, "detect_poly", 98, Algebraic, 
    &[ExprKind::Add, ExprKind::Mul, ExprKind::Pow], 
    |expr, ctx| {
        // Detect polynomials in each variable present
        for var in expr.variables() {
            if expr.is_polynomial_in(&var) {
                if let Some(poly) = expr.try_as_poly(&var) {
                    return Some(Expr::new(ExprKind::Poly(poly)));
                }
            }
        }
        None
    }
);

// Low priority: convert back to AST for final output
// IMPORTANT: Prefer contracted/factored form over expansion
// E.g., output (x+1)² instead of x² + 2x + 1
rule!(ContractPolynomial, "contract_poly", 2, Algebraic,
    &[ExprKind::Poly],
    |expr, _ctx| {
        if let ExprKind::Poly(poly) = &expr.kind {
            // Try to factor the polynomial first
            if let Some(factored) = poly.try_factor() {
                Some(factored)  // Returns Expr like (x+1)^2
            } else {
                // Fall back to standard expansion if no nice factors
                Some(Expr::from_poly(poly.clone()))
            }
        } else {
            None
        }
    }
);
```

#### Factoring in `Poly`
```rust
impl Poly {
    /// Try to factor the polynomial into a product of simpler terms
    /// Returns None if no nice factorization exists
    pub fn try_factor(&self) -> Option<Expr> {
        // Check for perfect squares: a² + 2ab + b² = (a+b)²
        // Check for difference of squares: a² - b² = (a+b)(a-b)
        // Check for common factors via GCD
        // ...
    }
}
```

#### Polynomial Operation Rules
```rust
// Poly + Poly → Poly (automatic addition)
rule!(PolyAddition, "poly_add", 95, Algebraic, &[ExprKind::Add], |expr, _ctx| {
    if let Add(a, b) = &expr.kind {
        if let (ExprKind::Poly(p1), ExprKind::Poly(p2)) = (&a.kind, &b.kind) {
            if p1.var == p2.var {
                return Some(Expr::new(ExprKind::Poly(p1.add(p2))));
            }
        }
    }
    None
});

// Poly * Poly → Poly (automatic multiplication)
rule!(PolyMultiplication, "poly_mul", 95, Algebraic, &[ExprKind::Mul], |expr, _ctx| {
    // Similar pattern
});

// Poly / Poly → Poly or Div (division when divisible)  
rule!(PolyDivision, "poly_div", 90, Algebraic, &[ExprKind::Div], |expr, _ctx| {
    // Attempt polynomial division, fall back to fraction if remainder
});
```

### Phase 5: Retire Polynomial-Specific Legacy Rules

#### [DEPRECATE] Functions to replace (Poly arithmetic handles these):

```
src/simplification/rules/algebraic/
├── addition.rs      # Polynomial term collection → Poly::add()
├── multiplication.rs # Polynomial distribution → Poly::mul()
├── distribution.rs   # Polynomial expansion → automatic in Poly
├── combined.rs       # Polynomial combinations → automatic in Poly
├── terms.rs          # Polynomial term handling → automatic in Poly
└── factoring.rs      # Polynomial factoring → Poly::gcd() + Poly::try_factor()
```

**Keep (non-polynomial operations)**:
- `power.rs` - for non-integer exponents like x^y, x^(1/2)
- `fractions.rs` - for rational functions
- `factoring.rs` - **trigonometric factoring** (sin²x + cos²x = 1) and **special patterns** (a² - b² = (a+b)(a-b) for symbolic a,b)
- `identities.rs` - algebraic identities not covered by Poly

> **Note**: The factoring.rs file contains both polynomial factoring (to be replaced) and
> trigonometric/special pattern factoring (to be kept). Split or mark deprecated sections.

---

## Verification Plan

### Unit Tests

#### [NEW] `src/tests/poly_tests.rs`
```rust
#[test]
fn test_poly_addition() {
    // (x² + 2x + 1) + (x + 1) = x² + 3x + 2
    let p1 = Poly::new(intern("x"), vec![(0, num(1)), (1, num(2)), (2, num(1))]);
    let p2 = Poly::new(intern("x"), vec![(0, num(1)), (1, num(1))]);
    let result = p1.add(&p2);
    assert_eq!(result.terms, vec![(0, num(2)), (1, num(3)), (2, num(1))]);
}

#[test]
fn test_poly_multiplication() {
    // (x+1)² = x² + 2x + 1
    let p = Poly::new(intern("x"), vec![(0, num(1)), (1, num(1))]);
    let result = p.mul(&p);
    assert_eq!(result.terms, vec![(0, num(1)), (1, num(2)), (2, num(1))]);
}

#[test]
fn test_poly_division() {
    // (x²−1)/(x−1) = x+1
    let dividend = Poly::new(intern("x"), vec![(0, num(-1)), (2, num(1))]);  // Sparse: no x¹ term!
    let divisor = Poly::new(intern("x"), vec![(0, num(-1)), (1, num(1))]);
    let (quotient, remainder) = dividend.div(&divisor).unwrap();
    assert_eq!(quotient.terms, vec![(0, num(1)), (1, num(1))]);
    assert!(remainder.is_zero());
}

#[test]
fn test_sparse_efficiency() {
    // x¹⁰⁰⁰ + 1 should only store 2 terms
    let p = Poly::new(intern("x"), vec![(0, num(1)), (1000, num(1))]);
    assert_eq!(p.terms.len(), 2);
    assert_eq!(p.degree(), Some(1000));
}
```

**Run**: `cargo test poly_tests`

### Integration Tests

#### [NEW] `src/tests/poly_integration.rs`
```rust
#[test]
fn test_full_pipeline_polynomial() {
    // (x+1)² - x² - 2x - 1 should simplify to 0
    let result = diff_and_simplify("(x+1)^2 - x^2 - 2*x - 1", "x", None).unwrap();
    assert_eq!(result.to_string(), "0");
}

#[test] 
fn test_polynomial_with_transcendentals() {
    // sin(x)² + 2*sin(x) + 1 should work with sin(x) as base
    let result = simplify_expr(parse("sin(x)^2 + 2*sin(x) + 1").unwrap());
    // Should recognize as (sin(x) + 1)²
}
```

**Run**: `cargo test poly_integration`

### Benchmark Verification

#### [MODIFY] `benches/simplification_benchmark.rs`

Add polynomial-focused benchmarks:
```rust
fn benchmark_polynomial_simplification(c: &mut Criterion) {
    let cases = [
        "(x+1)^2 - x^2 - 2*x - 1",           // Should be 0
        "(x+1)^5",                             // Binomial expansion
        "(x^2 - 1)/(x - 1)",                   // Should be x+1
        "x^3 + 3*x^2 + 3*x + 1",              // Should factor to (x+1)³
    ];
    // Compare against current master and Symbolica
}
```

**Run**: `cargo bench simplification`

### Regression Testing

All existing tests must still pass:
```bash
cargo test
```

---

## Migration Path

### v0.3.1-alpha: Add Poly, Keep Legacy
1. Add `Poly` struct and arithmetic
2. Add `ExprKind::Poly` variant
3. Add detection/conversion rules
4. Both systems coexist, A/B testing

### v0.3.1-beta: Validate Performance
1. Run benchmarks comparing both paths
2. Ensure Poly path is faster for polynomial cases
3. Fix any edge cases

### v0.3.1: Remove Legacy
1. Delete redundant algebraic rules
2. Poly is the default for polynomial simplification
3. Update documentation

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Multivariate polynomial ordering issues | Use lexicographic variable ordering based on InternedSymbol ID |
| Coefficient simplification recursion | Limit recursion depth, detect cycles |
| Non-polynomial expressions misidentified | Conservative `is_polynomial_in()` check |
| Existing tests break | Keep legacy rules during alpha, gradual migration |

---

## Timeline Estimate

| Phase | Effort | Description |
|-------|--------|-------------|
| Phase 1: Core Poly | 3-4 hours | Struct + arithmetic |
| Phase 2: AST Integration | 1-2 hours | ExprKind variant + conversions |
| Phase 3: Engine Integration | 1 hour | Match arm + rule enum |
| Phase 4: Detection Rules | 2-3 hours | Conversion rules |
| Phase 5: Delete Legacy | 1 hour | Remove old rules |
| Testing & Benchmarks | 2-3 hours | Validation |
| **Total** | **10-14 hours** | ~2-3 focused sessions |

---

## Success Criteria

- [ ] `cargo test` passes (all existing tests)
- [ ] New `poly_tests` pass
- [ ] Benchmark: polynomial simplification 10x+ faster
- [ ] Benchmark: full pipeline within 5x of Symbolica
- [ ] No regression in transcendental function handling
