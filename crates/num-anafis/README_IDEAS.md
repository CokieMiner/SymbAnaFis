# Future Types — Ideas & Notes

## CliffordNumber

Dense Clifford algebra multivector with inline fast path for small algebras.

- **Signature `(p, q, r)`**: p generators squaring to +1, q to -1, r to 0
- **Storage**: inline `[Scalar; 16]` for n ≤ 4 generators, heap `Vec<Scalar>` for larger
- **Geometric product**: standard blade multiplication with sign tracking
- **Convenience constructors**: complex `i`, dual `eps`, split-complex `j`, basis vectors `e1/e2/e3`
- **Should implement**: `Number` trait component-wise for scalar-only values, Clifford-specific for general multivectors
- **Transcendentals**: `sin`, `cos`, `exp`, `ln`, `sqrt` extended to complex, dual, and split-complex algebras
  - Use reflection/Euler formulas per signature
  - Dual numbers: `f(a + bε) = f(a) + b·f'(a)·ε`
  - Complex numbers: standard complex analysis formulas
  - Split-complex: hyperbolic variants

## Interval

Interval arithmetic `[lo, hi]` for rigorous error bounding.

- **Rounding**: outward rounding on every operation (lo rounds down, hi rounds up)
- **All `Number` trait functions**: must be monotonicity-aware
  - Monotone increasing (exp, sinh): `[f(lo), f(hi)]`
  - Monotone decreasing (exp_neg): `[f(hi), f(lo)]`
  - Non-monotone (sin, cos): requires critical point analysis
- **Intersection / union** operations
- **Width / midpoint** queries
