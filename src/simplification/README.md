# Simplification Rules

This document describes all the simplification rules implemented in the SymbAnaFis symbolic computation library.

## Overview

The simplification system applies rules in a bottom-up manner through the expression tree, using multiple passes until no further simplifications are possible. Rules are organized into modules by mathematical domain.

## Rule Application Order

Rules are applied in this sequence:
1. **Numeric rules** - Basic arithmetic simplifications
2. **Algebraic rules** - Like terms combination and exponent manipulation
3. **Trigonometric rules** - Trig identities and exact values
4. **Hyperbolic rules** - Hyperbolic function identities
5. **Logarithmic/Exponential rules** - Log and exp identities
6. **Root rules** - Square root and cube root simplifications

## Numeric Rules (`numeric.rs`)

### Arithmetic Operations
- **Addition**: `0 + x = x`, `x + 0 = x`
- **Subtraction**: `x - 0 = x`
- **Multiplication**: `0 * x = 0`, `x * 0 = 0`, `1 * x = x`, `x * 1 = x`
- **Division**: `x / 1 = x`, `0 / x = 0` (for x ≠ 0)

### Power Rules
- **Zero power**: `x^0 = 1`
- **Unit power**: `x^1 = x`
- **Zero base**: `0^x = 0` (for x > 0)
- **Unit base**: `1^x = 1`

### Constant Folding
All binary operations with numeric constants are evaluated:
- `a + b`, `a - b`, `a * b`, `a / b`, `a^b` where a,b are numbers

## Algebraic Rules (`algebraic.rs`)

### Like Terms Combination
- **Addition**: `x + x = 2x`, `2x + 3x = 5x`
- **Multiplication**: `x * x = x^2`, `x^2 * x^3 = x^5`
- **Normalized terms**: Expressions like `2*cosh(x)*sinh(x)` and `-2*sinh(x)*cosh(x)` are recognized as like terms after normalizing multiplication order

### Factoring
- **Common factor**: `x*y + x*z = x*(y+z)`
- **Perfect squares**: `x^2 + 2*x*y + y^2 = (x+y)^2`

### Cancellation
- **Subtraction**: `x - x = 0`
- **Division**: `x / x = 1`

### Exponent Manipulation
- **Power of power**: `(x^a)^b = x^(a*b)`
- **Division with powers**: `x^n / x^m = x^(n-m)`
- **Division by power**: `x^n / x = x^(n-1)`
- **Division of power**: `x / x^n = x^(1-n)`

### Negative Term Handling
- **Addition with negation**: `a + (-b) = a - b`
- **Subtraction conversion**: `x - y = x + (-1)*y`

### Identity Operations
- **Multiplication by one**: `1 * x = x`, `x * 1 = x`
- **Addition with zero**: `x + 0 = x`, `0 + x = x`

### Division Simplification
- **Factor cancellation**: `(a * b) / (a * c) → b / c`
- **Exact match cancellation**: Common factors in numerator and denominator are cancelled
- **Power reduction**: 
  - `x / x^n → 1 / x^(n-1)`
  - `x^n / x → x^(n-1) / 1`
  - `x^n / x^m → x^(n-m)`
- **Division structure preservation**: When multiplications contain divisions (e.g., `a * (b/c) * d`), they are flattened to `(a*b*d)/c` and the division structure is preserved through subsequent simplification passes
- **Nested division simplification**:
  - `(x/y) / (z/a) → (x*a) / (y*z)`
  - `x / (y/z) → (x*z) / y`
  - `(x/y) / z → x / (y*z)`

#### Critical Implementation Details

1. **Structure Preservation**: The multiplication simplification includes a critical fix (lines 137-145 in `algebraic.rs`): when `try_flatten_mul_div` successfully creates a single `Div` expression from a multiplication containing divisions, it is returned immediately without further processing by `combine_mul_terms` or `combine_power_terms`. This prevents the division structure from being broken apart.

2. **Power Expansion Strategy**: To enable cancellation in expressions like `a / (a*b)^n`, the denominator is temporarily expanded to `a^n * b^n` using `expand_pow_mul`.
   - **Loop Prevention**: To avoid infinite loops (expand → combine → expand), the expanded form is **only kept if a cancellation actually occurs**. If no factors are cancelled, the original unexpanded denominator is returned. This breaks the cycle between `expand_pow_mul` and `combine_power_terms`.

### Canonical Ordering
- Terms are sorted and combined in canonical form
- Multiplication factors are normalized by sorting subexpressions

## Trigonometric Rules (`trig.rs`)

### Exact Values
- **sin(0) = 0**, **sin(π) = 0**, **sin(π/2) = 1**
- **sin(π/6) = 1/2**, **sin(π/4) = √2/2**, **sin(π/3) = √3/2**
- **cos(0) = 1**, **cos(π) = -1**, **cos(π/2) = 0**
- **cos(π/6) = √3/2**, **cos(π/4) = √2/2**, **cos(π/3) = 1/2**
- **tan(0) = 0**, **tan(π) = 0**, **tan(π/4) = 1**
- **tan(π/6) = √3/3**, **tan(π/3) = √3**
- **cot(π/4) = 1**, **cot(π/2) = 0**
- **sec(0) = 1**, **csc(π/2) = 1**, **csc(π/6) = 2**

### Parity and Sign Rules
- **sin(-x) = -sin(x)**
- **cos(-x) = cos(x)**
- **tan(-x) = -tan(x)**
- **cot(-x) = -cot(x)**
- **sec(-x) = sec(x)**
- **csc(-x) = -csc(x)**

### Inverse Function Composition
- **sin(asin(x)) = x**, **asin(sin(x)) = x**
- **cos(acos(x)) = x**, **acos(cos(x)) = x**
- **tan(atan(x)) = x**, **atan(tan(x)) = x**

### Cofunction Identities
- **sin(π/2 - x) = cos(x)**
- **cos(π/2 - x) = sin(x)**
- **tan(π/2 - x) = cot(x)**
- **cot(π/2 - x) = tan(x)**

### Periodicity
- **sin(x + 2kπ) = sin(x)**
- **cos(x + 2kπ) = cos(x)**

### Reflection and Shift Identities
- **sin(π - x) = sin(x)**
- **sin(π + x) = -sin(x)**
- **sin(3π/2 - x) = -cos(x)**
- **sin(3π/2 + x) = -cos(x)**
- **cos(π - x) = -cos(x)**
- **cos(π + x) = -cos(x)**
- **cos(3π/2 - x) = -sin(x)**
- **cos(3π/2 + x) = sin(x)**

### Double Angle Formulas
- **sin(2x) = 2* sin(x) *cos(x)**
- **tan(2x) = 2*tan(x)/(1 - tan²(x))**

### Pythagorean Identities
- **sin²(x) + cos²(x) = 1**
- **1 + tan²(x) = sec²(x)**
- **1 + cot²(x) = csc²(x)**

## Hyperbolic Rules (`hyperbolic.rs`)

### Exact Values
- **sinh(0) = 0**, **cosh(0) = 1**
- **tanh(0) = 0**, **sech(0) = 1**

### Parity Rules
- **sinh(-x) = -sinh(x)**
- **cosh(-x) = cosh(x)**
- **tanh(-x) = -tanh(x)**

### Hyperbolic Pythagorean Identities
- **cosh²(x) - sinh²(x) = 1**
- **sinh²(x) + cosh²(x) = cosh(2x)**
- **1 - tanh²(x) = sech²(x)**
- **coth²(x) - 1 = csch²(x)**

### Hyperbolic Ratio Identities
- **sinh(x)/cosh(x) = tanh(x)**
- **cosh(x)/sinh(x) = coth(x)**
- **1/cosh(x) = sech(x)**
- **1/sinh(x) = csch(x)**

### Exponential Form Recognition
- **(e^x - e^-x)/2 = sinh(x)**
- **(e^x + e^-x)/2 = cosh(x)**
- **(e^x - e^-x)/(e^x + e^-x) = tanh(x)**
- **(e^x + e^-x)/(e^x - e^-x) = coth(x)**
- **2/(e^x + e^-x) = sech(x)**
- **2/(e^x - e^-x) = csch(x)**
- **(e^x + (-1)*e^-x)/2 = sinh(x)** (after algebraic simplification)
- **(e^x + e^-x)/(e^x + (-1)*e^-x) = coth(x)** (after algebraic simplification)

### Canonical Form Handling
All identities also recognize forms after algebraic simplification:
- **cosh²(x) + (-1)*sinh²(x) = 1**
- **(-1)*sinh²(x) + cosh²(x) = 1**
- **1 + (-1)*tanh²(x) = sech²(x)**
- **(-1)*tanh²(x) + 1 = sech²(x)**
- **coth²(x) + (-1) = csch²(x)**
- **(-1) + coth²(x) = csch²(x)**

## Logarithmic/Exponential Rules (`log_exp.rs`)

### Exponential Rules
- **exp(0) = 1**
- **exp(ln(x)) = x**

### Logarithmic Rules
- **ln(1) = 0**
- **ln(exp(x)) = x**
- **ln(x^n) = n * ln(x)**
- **log10(x^n) = n * log10(x)**
- **log2(x^n) = n * log2(x)**

### Logarithm Base Rules
- **log₁₀(1) = 0**, **log₁₀(10) = 1**
- **log₂(1) = 0**, **log₂(2) = 1**

## Root Rules (`roots.rs`)

### Square Root Rules
- **√0 = 0**, **√1 = 1**
- **√(x²) = x** (assuming x ≥ 0)

### Cube Root Rules
- **∛0 = 0**, **∛1 = 1**
- **∛(x³) = x**

### Root Simplification
- **General Powers**: `sqrt(x^n)` simplifies to `x^(n/2)` if `n` is even. `cbrt(x^n)` simplifies to `x^(n/3)` if `n` is a multiple of 3.
- **Nested Roots**: `sqrt(sqrt(x))` simplifies to `x^(1/4)`.
- **Power to Root Conversion**: `x^(1/2)` → `sqrt(x)`, `x^(1/3)` → `cbrt(x)`, `x^0.5` → `sqrt(x)`.

### Hyperbolic Simplification
- **Inverse Composition**:
    - `sinh(asinh(x))` -> `x`
    - `cosh(acosh(x))` -> `x`
    - `tanh(atanh(x))` -> `x`

### Logarithmic and Exponential Simplification
- **Combination Rules**:
    - `ln(a) + ln(b)` -> `ln(a * b)`
    - `ln(a) - ln(b)` -> `ln(a / b)`
    - `exp(a) * exp(b)` -> `exp(a + b)`

### Trigonometric Simplification
- **Sum/Difference Combination**:
    - `sin(x)cos(y) + cos(x)sin(y)` -> `sin(x + y)`
    - `sin(x)cos(y) - cos(x)sin(y)` -> `sin(x - y)`
    - `cos(x)cos(y) - sin(x)sin(y)` -> `cos(x + y)`
    - `cos(x)cos(y) + sin(x)sin(y)` -> `cos(x - y)`


## Future Enhancements (TODO)

- **Polynomial GCD**: Implement greatest common divisor for polynomials to enable further factorization and simplification of rational expressions.
- **Advanced Factoring**: Add support for factoring higher-degree polynomials, including irreducible polynomials and special cases.
- **Advanced Inverse Trig/Hyperbolic**: Extend inverse trigonometric and hyperbolic function simplifications, such as compositions and identities involving multiple arguments.

## Implementation Details

- All rules are applied recursively bottom-up through the expression tree
- The system uses cycle detection to prevent infinite loops
- Rules are applied in multiple passes until convergence
- Numeric precision uses ε = 1e-10 for floating-point comparisons
- The system preserves exact symbolic forms when possible
- **Canonical form handling**: Many rules recognize both original forms (e.g., `a - b`) and canonical forms after algebraic simplification (e.g., `a + (-1)*b`)
- **Recursive simplification**: Subexpressions are simplified before applying rules to the current level
- **Expression normalization**: Multiplication terms are sorted and normalized for consistent term combination
- **Negative term recognition**: Rules handle expressions with explicit negative coefficients (e.g., `a + (-b)`)
- **Identity preservation**: Operations like `1 * x` and `x * 1` are reduced to `x` for cleaner output

### Display Correctness

The display module (`display.rs`) includes critical fixes to ensure mathematical correctness:

- **Power base parenthesization**: When displaying `x^n`, if `x` is a `Mul`, `Div`, `Add`, or `Sub` expression, it is parenthesized to avoid operator precedence ambiguity. For example:
  - `(C * R)^2` displays as `(C * R)^2`, not `C * R^2` (which would mean `C * (R^2)`)
  - `(a / b)^n` displays as `(a / b)^n`, not `a / b^n` (which would mean `a / (b^n)`)
- **Division denominator parenthesization**: Denominators containing `Mul`, `Div`, `Add`, or `Sub` are parenthesized:
  - `a / (b * c)` displays correctly, not `a / b * c` (which would mean `(a / b) * c`)
  
These fixes ensure that the displayed form matches the internal expression tree structure and can be parsed back correctly without ambiguity.
