# Simplification Rules

This document describes all the simplification rules implemented in the SymbAnaFis symbolic computation library.

## Overview

The simplification system applies rules in a bottom-up manner through the expression tree, using multiple passes until no further simplifications are possible. Rules are organized into modules by mathematical domain and applied in priority order (higher priority first).

## Rule Categories and Priorities

Rules are grouped by category and listed in priority order within each category.

### Numeric Rules (Category: Numeric)
These handle basic arithmetic identities and constant folding.

- **add_zero** (priority: 100) - Rule for adding zero: x + 0 = x, 0 + x = x
- **sub_zero** (priority: 100) - Rule for subtracting zero: x - 0 = x
- **mul_zero** (priority: 100) - Rule for multiplying by zero: 0 * x = 0, x * 0 = 0
- **mul_one** (priority: 100) - Rule for multiplying by one: 1 * x = x, x * 1 = x
- **div_one** (priority: 100) - Rule for dividing by one: x / 1 = x
- **zero_div** (priority: 100) - Rule for zero divided by something: 0 / x = 0 (when x != 0)
- **pow_zero** (priority: 100) - Rule for power of zero: x^0 = 1 (when x != 0)
- **pow_one** (priority: 100) - Rule for power of one: x^1 = x
- **zero_pow** (priority: 100) - Rule for zero to a power: 0^x = 0 (for x > 0)
- **one_pow** (priority: 100) - Rule for one to a power: 1^x = 1
- **normalize_sign_div** (priority: 95) - Rule for normalizing signs in division: x / -y -> -x / y (moves negative from denominator to numerator)
- **constant_fold** (priority: 90) - Rule for constant folding arithmetic operations. Also handles nested multiplications with multiple numeric factors: `3 * (2 * x) → 6 * x`
- **fraction_simplify** (priority: 80) - Rule for simplifying fractions with integer coefficients

### Algebraic Rules (Category: Algebraic)
These handle polynomial operations, factoring, absolute value, sign functions, and structural transformations.

- **abs_numeric** (priority: 95) - Rule for absolute value of numeric constants: abs(5) -> 5, abs(-3) -> 3
- **sign_numeric** (priority: 95) - Rule for sign of numeric constants: sign(5) -> 1, sign(-3) -> -1, sign(0) -> 0
- **div_div_flatten** (priority: 95) - Rule for flattening nested divisions: (a/b)/(c/d) -> (a*d)/(b*c)
- **combine_nested_fraction** (priority: 94) - Rule for combining nested fractions: (a + b/c) / d → (a*c + b) / (c*d), and similar patterns with subtraction. Enables cleaner output for expressions like (1 - v/c) / c → (c - v) / c²
- **power_zero** (priority: 95) - Rule for x^0 = 1 (when x != 0)
- **power_one** (priority: 95) - Rule for x^1 = x
- **expand_power_for_cancellation** (priority: 95) - Rule for expanding powers to enable cancellation: (a*b)^n / a -> a^n * b^n / a
- **negative_exponent_to_fraction** (priority: 95) - Rule for x^-n -> 1/x^n where n > 0
- **fraction_to_end** (priority: 93) - Rule for ((1/a) * b) / c -> b / (a * c), keeps one division at the end
- **polynomial_expansion** (priority: 92) - Rule for expanding polynomials (a+b)^n for n=2,3 when beneficial for cancellation
- **div_self** (priority: 90) - Rule for x / x = 1 (when x != 0) **[alters domain]**
- **fraction_cancellation** (priority: 90) - Rule for cancelling common terms in fractions: (a*b)/(a*c) -> b/c, also handles powers: x^a / x^b -> x^(a-b). In domain-safe mode, only cancels nonzero numeric constants; symbolic cancellation **[alters domain]**
- **abs_abs** (priority: 90) - Rule for nested absolute value: abs(abs(x)) -> abs(x)
- **abs_neg** (priority: 90) - Rule for absolute value of negation: abs(-x) -> abs(x)
- **power_to_sqrt** (priority: 90) - Rule for x^(1/2) -> sqrt(x)
- **power_to_cbrt** (priority: 90) - Rule for x^(1/3) -> cbrt(x)
- **distribute_mul_in_numerator** (priority: 35) - Rule for distributing multiplication in numerator for term combination. Has smart detection to avoid distributing when the numerator is already a factored product (e.g., (a+b)*(c+d)), preventing infinite loops with factoring rules. Also folds numeric constants during distribution to avoid outputs like `3 * 2 * x` (produces `6 * x` instead)
- **sign_abs** (priority: 85) - Rule for sign of absolute value: sign(abs(x)) -> 1 (for x != 0)
- **distribute_negation** (priority: 90) - Rule for distributing negation: -(A + B) -> -A - B
- **e_pow_ln** (priority: 85) - Rule for e^(ln(x)) -> x (handles Symbol("e") form) **[alters domain]**
- **e_pow_mul_ln** (priority: 85) - Rule for e^(a*ln(b)) -> b^a (handles Symbol("e") form)
- **power_power** (priority: 85) - Rule for (x^a)^b -> x^(a*b). Returns abs(x) when (x^even)^(1/even) simplifies to x^1
- **abs_square** (priority: 85) - Rule for absolute value of square: abs(x^2) -> x^2
- **abs_pow_even** (priority: 85) - Rule for abs(x)^(even) -> x^(even): abs(x)^2 -> x^2, abs(x)^4 -> x^4
- **sign_sign** (priority: 85) - Rule for nested sign: sign(sign(x)) -> sign(x)
- **sign_abs** (priority: 85) - Rule for sign of absolute value: sign(abs(x)) -> 1 (for x != 0)
- **power_mul** (priority: 85) - Rule for x^a * x^b -> x^(a+b)
- **mul_div_combination** (priority: 85) - Rule for a * (b / c) -> (a * b) / c
- **expand_difference_of_squares_product** (priority: 85) - Rule for expanding (a+b)(a-b) -> a² - b² when beneficial
- **power_div** (priority: 93) - Rule for x^a / x^b -> x^(a-b). Higher priority than polynomial_expansion (92) to cancel `(x+1)^2 / (x+1) → x+1` before expanding
- **exp_ln** (priority: 80) - Rule for exp(ln(x)) -> x **[alters domain]**
- **ln_exp** (priority: 80) - Rule for ln(exp(x)) -> x
- **exp_mul_ln** (priority: 80) - Rule for exp(a * ln(b)) -> b^a **[alters domain]**
- **abs_sign_mul** (priority: 80) - Rule for abs(x) * sign(x) -> x
- **power_collection** (priority: 80) - Rule for collecting powers in multiplication: x^a * x^b -> x^(a+b)
- **common_exponent_div** (priority: 80) - Rule for x^a / y^a -> (x/y)^a. For fractional exponents, checks non-negativity in domain-safe mode
- **common_exponent_mul** (priority: 80) - Rule for x^a * y^a -> (x*y)^a. For fractional exponents, checks non-negativity in domain-safe mode
- **combine_like_terms_addition** (priority: 80) - Rule for combining like terms in addition: 2x + 3x -> 5x, x^2 - x^2 -> 0
- **perfect_square** (priority: 50) - Rule for perfect squares: a^2 + 2ab + b^2 -> (a+b)^2
- **perfect_cube** (priority: 50) - Rule for perfect cubes: a^3 + 3a^2b + 3ab^2 + b^3 -> (a+b)^3
- **combine_terms** (priority: 45) - Rule for combining like terms in addition: 2x + 3x -> 5x
- **combine_factors** (priority: 45) - Rule for combining like factors in multiplication: x * x -> x^2
- **numeric_gcd_factoring** (priority: 42) - Rule for factoring out numeric GCD: 2*a + 2*b -> 2*(a+b)
- **power_expansion** (priority: 40) - Rule for expanding powers: (a*b)^n -> a^n * b^n
- **add_fraction** (priority: 40) - Rule for adding fractions: a + b/c -> (a*c + b)/c
- **canonicalize** (priority: 40) - Rule for canonicalizing expressions (sorting terms)
- **common_term_factoring** (priority: 40) - Rule for factoring out common terms: ax + bx -> x(a+b)
- **common_power_factoring** (priority: 39) - Rule for factoring out common powers: x³ + x² -> x²(x + 1). Only applies to pure power sums (no coefficients, no constant terms)
- **canonicalize_multiplication** (priority: 15) - Rule for canonical ordering of multiplication terms: (x*y)*z -> x*y*z
- **canonicalize_addition** (priority: 15) - Rule for canonical ordering of addition terms
- **canonicalize_subtraction** (priority: 15) - Rule for canonical ordering in subtraction
- **factor_difference_of_squares** (priority: 10) - Rule for factoring difference of squares: a^2 - b^2 -> (a-b)(a+b)
- **normalize_add_negation** (priority: 5) - Rule for normalizing addition with negation: a + (-b) -> a - b

### Trigonometric Rules (Category: Trigonometric)
These handle trigonometric identities, exact values, and transformations.

- **sin_zero** (priority: 95) - Rule for sin(0) = 0
- **cos_zero** (priority: 95) - Rule for cos(0) = 1
- **tan_zero** (priority: 95) - Rule for tan(0) = 0
- **sin_pi** (priority: 95) - Rule for sin(π) = 0
- **cos_pi** (priority: 95) - Rule for cos(π) = -1
- **sin_pi_over_two** (priority: 95) - Rule for sin(π/2) = 1
- **cos_pi_over_two** (priority: 95) - Rule for cos(π/2) = 0
- **trig_exact_values** (priority: 95) - Rule for sin(π/6) = 1/2, cos(π/3) = 1/2, etc.
- **inverse_trig_identity** (priority: 90) - Rule for sin(asin(x)) = x and cos(acos(x)) = x **[alters domain]**
- **trig_neg_arg** (priority: 90) - Rule for sin(-x) = -sin(x), cos(-x) = cos(x), etc.
- **trig_product_to_double_angle** (priority: 90) - Rule for product-to-double-angle conversions
- **cofunction_identity** (priority: 85) - Rule for sin(π/2 - x) = cos(x) and cos(π/2 - x) = sin(x)
- **inverse_trig_composition** (priority: 85) - Rule for asin(sin(x)) = x and acos(cos(x)) = x **[alters domain]**
- **trig_periodicity** (priority: 85) - Rule for periodicity: sin(x + 2kπ) = sin(x), cos(x + 2kπ) = cos(x)
- **trig_double_angle** (priority: 85) - Rule for sin(2*x) = 2*sin(x)*cos(x)
- **cos_double_angle_difference** (priority: 85) - Rule for cosine double angle in difference form
- **pythagorean_complements** (priority: 85) - Rule for 1 - cos²(x) = sin²(x), 1 - sin²(x) = cos²(x). Also handles the canonicalized forms `-cos²(x) + 1` and `-sin²(x) + 1`. Higher priority ensures trig identity fires before algebraic factoring (difference of squares)
- **pythagorean_identity** (priority: 80) - Rule for sin^2(x) + cos^2(x) = 1
- **trig_reflection** (priority: 80) - Rule for reflection: sin(π - x) = sin(x), cos(π - x) = -cos(x)
- **trig_three_pi_over_two** (priority: 80) - Rule for sin(3π/2 - x) = -cos(x), cos(3π/2 - x) = -sin(x)
- **pythagorean_tangent** (priority: 70) - Rule for tan^2(x) + 1 = sec^2(x) and cot^2(x) + 1 = csc^2(x)
- **trig_sum_difference** (priority: 70) - Rule for sum/difference identities: sin(x+y), cos(x-y), etc.
- **trig_triple_angle** (priority: 70) - Rule for triple angle folding: 3sin(x) - 4sin^3(x) -> sin(3x)

### Hyperbolic Rules (Category: Hyperbolic)
These handle hyperbolic function identities and exponential forms. All exponential conversion rules (sinh, cosh, tanh, sech, csch, coth) now properly handle different term orderings in commutative operations (Addition), making patterns more general and robust.

- **sinh_zero** (priority: 95) - Rule for sinh(0) = 0
- **cosh_zero** (priority: 95) - Rule for cosh(0) = 1
- **sinh_asinh_identity** (priority: 95) - Rule for sinh(asinh(x)) = x
- **cosh_acosh_identity** (priority: 95) - Rule for cosh(acosh(x)) = x
- **tanh_atanh_identity** (priority: 95) - Rule for tanh(atanh(x)) = x
- **hyperbolic_identity** (priority: 95) - Rule for cosh^2(x) - sinh^2(x) = 1, 1 - tanh^2(x) = sech^2(x), coth^2(x) - 1 = csch^2(x)
- **sinh_negation** (priority: 90) - Rule for sinh(-x) = -sinh(x)
- **cosh_negation** (priority: 90) - Rule for cosh(-x) = cosh(x)
- **tanh_negation** (priority: 90) - Rule for tanh(-x) = -tanh(x)
- **sinh_from_exp** (priority: 80) - Rule for converting (e^x - e^(-x)) / 2 to sinh(x). Handles both Add and Sub patterns with reversed term orderings.
- **cosh_from_exp** (priority: 80) - Rule for converting (e^x + e^(-x)) / 2 to cosh(x). Now handles reversed order: (e^(-x) + e^x) / 2.
- **tanh_from_exp** (priority: 80) - Rule for converting (e^x - e^(-x)) / (e^x + e^(-x)) to tanh(x). Denominators with reversed order (e^(-x) + e^x) are also recognized.
- **sech_from_exp** (priority: 80) - Rule for converting 2 / (e^x + e^(-x)) to sech(x). Now handles reversed denominator order (e^(-x) + e^x).
- **csch_from_exp** (priority: 80) - Rule for converting 2 / (e^x - e^(-x)) to csch(x). Handles both Add and Sub patterns with reversed term orderings.
- **coth_from_exp** (priority: 80) - Rule for converting (e^x + e^-x) / (e^x - e^-x) to coth(x). Handles different term orderings in commutative operations.
- **sinh_cosh_to_tanh** (priority: 80) - Rule for converting sinh(x)*cosh(x) to sinh(2x)/2
- **cosh_sinh_to_coth** (priority: 80) - Rule for converting cosh(x)*sinh(x) to sinh(2x)/2
- **one_sinh_to_csch** (priority: 80) - Rule for converting 1/sinh(x) to csch(x)
- **one_cosh_to_sech** (priority: 80) - Rule for converting 1/cosh(x) to sech(x)
- **one_tanh_to_coth** (priority: 80) - Rule for converting 1/tanh(x) to coth(x)
- **hyperbolic_triple_angle** (priority: 70) - Rule for triple angle folding: 4sinh^3(x) + 3sinh(x) -> sinh(3x), 4cosh^3(x) - 3cosh(x) -> cosh(3x)

### Exponential Rules (Category: Exponential)
These handle logarithmic and exponential function identities.

- **ln_one** (priority: 95) - Rule for ln(1) = 0
- **ln_e** (priority: 95) - Rule for ln(e) = 1
- **exp_zero** (priority: 95) - Rule for exp(0) = 1
- **exp_to_e_pow** (priority: 95) - Rule for exp(x) = e^x
- **log_base_values** (priority: 95) - Rule for specific log values: log10(1)=0, log10(10)=1, log2(1)=0, log2(2)=1
- **exp_ln_identity** (priority: 90) - Rule for exp(ln(x)) = x (for x > 0) **[alters domain]**
- **ln_exp_identity** (priority: 90) - Rule for ln(exp(x)) = x
- **log_power** (priority: 90) - Rule for log(x^n) = n * log(x). For even integer exponents, uses abs: log(x^2) = 2*log(abs(x)). Odd exponents **[alters domain]**
- **log_combination** (priority: 85) - Rule for ln(a) + ln(b) = ln(a*b) and ln(a) - ln(b) = ln(a/b)

### Root Rules (Category: Root)
These handle square root and cube root simplifications.

- **sqrt_power** (priority: 85) - Rule for sqrt(x^n) = x^(n/2). Returns abs(x) when sqrt(x^(even)) simplifies to x^1
- **cbrt_power** (priority: 85) - Rule for cbrt(x^n) = x^(n/3)
- **sqrt_mul** (priority: 80) - Rule for sqrt(x) * sqrt(y) = sqrt(x*y). Safe when x,y are known non-negative; otherwise **[alters domain]**
- **sqrt_div** (priority: 80) - Rule for sqrt(x)/sqrt(y) = sqrt(x/y). Safe when x,y are known non-negative; otherwise **[alters domain]**
- **power_to_root** (priority: 75) - Rule for x^(1/n) = nth root of x
- **normalize_roots** (priority: 50) - Rule that applies the monolithic root normalization

## Domain Safety

Some simplification rules make assumptions about the domain of validity for the expressions they transform. These rules are marked with **[alters domain]** in the rule descriptions above.

When domain-safe mode is enabled, rules that alter domains are skipped to ensure that simplifications remain valid across the entire complex plane or real line, depending on the context. This prevents incorrect simplifications that might introduce new singularities or restrict the domain inappropriately.

### Enabling Domain-Safe Mode

Domain-safe mode can be enabled in several ways:

1. **Environment variable**: Set `SYMB_ANAFIS_DOMAIN_SAFETY=true`
2. **Programmatically**: Use `Simplifier::new().with_domain_safe(true)`

## Debugging and Tracing

### Rule Application Tracing

To debug simplification and see which rules are being applied, set the `SYMB_TRACE` environment variable:

```bash
SYMB_TRACE=1 cargo run --example your_example
```

This will print each rule application to stderr, showing:
- The rule name being applied
- The original expression
- The simplified result

This is useful for diagnosing rule interaction issues and understanding the simplification process.

## Fixed Variables Support

The simplification system supports "fixed variables" - symbols that should be treated as user-specified constants rather than mathematical constants like `e` (Euler's number).

When a variable is marked as "fixed":
- Rules like `e_pow_ln` and `e_pow_mul_ln` will NOT apply special handling for `e`
- The symbol `e` will be treated as a regular variable/constant

### Usage

```rust
// In diff() or simplify() functions, pass fixed variables:
diff("e*x".to_string(), "x".to_string(), Some(&["e".to_string()]), None);
// Here "e" is treated as a constant coefficient, not Euler's number
```

## Rule Count Summary

| Category | Count |
|----------|-------|
| Numeric | 13 |
| Algebraic | 52 |
| Trigonometric | 23 |
| Hyperbolic | 21 |
| Exponential | 9 |
| Root | 6 |
| **Total** | **124** |

## Implementation Details

- All rules are applied recursively bottom-up through the expression tree
- The system uses cycle detection to prevent infinite loops
- Rules are applied in multiple passes until convergence
- Numeric precision uses ε = 1e-10 for floating-point comparisons
- The system preserves exact symbolic forms when possible
- **Rule priority ordering**: Higher priority numbers run first (e.g., priority 95 runs before 40). Key priority tiers:
  - 95-100: Identity rules, basic arithmetic, structure flattening
  - 85-94: Cancellation, power rules, term combination
  - 40-80: Factoring, polynomial operations, term collection
  - 5-35: Canonicalization, normalization, display cleanup
- **Factoring vs Distribution balance**: `CommonTermFactoringRule` (priority 40) runs before `DistributeMulInNumeratorRule` (priority 35), ensuring factored forms are preserved. Distribution only occurs when the numerator is not already a factored product.
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
