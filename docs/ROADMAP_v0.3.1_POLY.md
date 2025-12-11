# SymbAnaFis v0.3.1 - Polynomial Coefficient Ring Architecture

## Overview

This document outlines the implementation plan for adding GiNaC-style polynomial coefficient rings to SymbAnaFis. This architectural change will dramatically improve simplification performance for polynomial expressions (10-100x faster) while maintaining our fast parsing, differentiation, and transcendental function support.

## Motivation

### Current Performance (v0.3.0)
| Operation | vs SymPy | vs Symbolica |
|-----------|----------|--------------|
| Parsing | 120-190x faster ✓ | 1.5-2.3x faster ✓ |
| AST Differentiation | — | 1.7-2.9x faster ✓ |
| Full Diff+Simplify | — | 17-73x **slower** ✗ |
| Evaluation | 32-3886x faster ✓ | — |

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
        Poly { var: y, coeffs: [0, 0, 1] },  // y² (x⁰ term)
        Poly { var: y, coeffs: [0, 2] },     // 2y (x¹ term)
        Expr::number(1),                      // 1  (x² term)
    ]
}
```

---

## Implementation Plan

### Phase 1: Core Poly Structure (~200 lines)

#### [NEW] `src/poly.rs`

```rust
/// Dense univariate polynomial with arbitrary expression coefficients
#[derive(Debug, Clone, PartialEq)]
pub struct Poly {
    /// The polynomial variable (interned for O(1) comparison)
    pub var: InternedSymbol,
    /// Coefficients: coeffs[i] is the coefficient of var^i
    /// Invariant: trailing zeros are trimmed (no zero leading coefficient)
    pub coeffs: Vec<Expr>,
}

impl Poly {
    /// Create from coefficients, auto-trimming trailing zeros
    pub fn new(var: InternedSymbol, coeffs: Vec<Expr>) -> Self;
    
    /// Degree of the polynomial (-1 for zero polynomial)
    pub fn degree(&self) -> i32;
    
    /// Check if this is the zero polynomial
    pub fn is_zero(&self) -> bool;
    
    /// Get the leading coefficient
    pub fn leading_coeff(&self) -> Option<&Expr>;
}
```

#### Polynomial Arithmetic
```rust
impl Poly {
    /// Add two polynomials in the same variable: O(max(deg_a, deg_b))
    pub fn add(&self, other: &Poly) -> Poly;
    
    /// Subtract two polynomials: O(max(deg_a, deg_b))
    pub fn sub(&self, other: &Poly) -> Poly;
    
    /// Multiply two polynomials (convolution): O(deg_a * deg_b)
    pub fn mul(&self, other: &Poly) -> Poly;
    
    /// Polynomial division with remainder: O(deg_a * deg_b)
    /// Returns (quotient, remainder) such that self = quotient * other + remainder
    pub fn div(&self, other: &Poly) -> Option<(Poly, Poly)>;
    
    /// Greatest Common Divisor (Euclidean algorithm)
    pub fn gcd(&self, other: &Poly) -> Poly;
    
    /// Scalar multiplication: coeff * poly
    pub fn scale(&self, coeff: &Expr) -> Poly;
    
    /// Evaluate polynomial at a value: Horner's method O(degree)
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

// Low priority: expand back to AST for final output
rule!(ExpandPolynomial, "expand_poly", 2, Algebraic,
    &[ExprKind::Poly],
    |expr, _ctx| {
        if let ExprKind::Poly(poly) = &expr.kind {
            Some(Expr::from_poly(poly.clone()))
        } else {
            None
        }
    }
);
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

### Phase 5: Delete Legacy Rules

#### [DELETE] Files to remove (replaced by Poly arithmetic):

```
src/simplification/rules/algebraic/
├── addition.rs      # ~100 lines → Poly::add()
├── multiplication.rs # ~150 lines → Poly::mul()
├── distribution.rs   # ~80 lines → automatic in Poly
├── combined.rs       # ~200 lines → automatic in Poly
├── terms.rs          # ~150 lines → automatic in Poly
└── factoring.rs      # ~100 lines → Poly::gcd() + factorization
```

**Keep**: `power.rs` (for non-integer exponents), `fractions.rs` (for rational functions)

---

## Verification Plan

### Unit Tests

#### [NEW] `src/tests/poly_tests.rs`
```rust
#[test]
fn test_poly_addition() {
    // [1, 2, 1] + [1, 1] = [2, 3, 1]
    let p1 = Poly::new(intern("x"), vec![num(1), num(2), num(1)]);
    let p2 = Poly::new(intern("x"), vec![num(1), num(1)]);
    let result = p1.add(&p2);
    assert_eq!(result.coeffs, vec![num(2), num(3), num(1)]);
}

#[test]
fn test_poly_multiplication() {
    // [1, 1] * [1, 1] = [1, 2, 1]  (i.e., (x+1)² = x² + 2x + 1)
    let p = Poly::new(intern("x"), vec![num(1), num(1)]);
    let result = p.mul(&p);
    assert_eq!(result.coeffs, vec![num(1), num(2), num(1)]);
}

#[test]
fn test_poly_division() {
    // [−1, 0, 1] / [−1, 1] = [1, 1]  (i.e., (x²−1)/(x−1) = x+1)
    let dividend = Poly::new(intern("x"), vec![num(-1), num(0), num(1)]);
    let divisor = Poly::new(intern("x"), vec![num(-1), num(1)]);
    let (quotient, remainder) = dividend.div(&divisor).unwrap();
    assert_eq!(quotient.coeffs, vec![num(1), num(1)]);
    assert!(remainder.is_zero());
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
