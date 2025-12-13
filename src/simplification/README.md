# Simplification Rules

This document describes all the simplification rules implemented in the SymbAnaFis symbolic computation library.

## Overview

The simplification system applies rules in a bottom-up manner through the expression tree, using multiple passes until no further simplifications are possible. Rules are organized into modules by mathematical domain and applied in priority order (higher priority first).

### Priority Strategy: Expand → Cancel → Compact

Rules follow an **expand → cancel → compact** ordering strategy:

| Priority Range | Phase | Purpose |
|----------------|-------|--------|
| **85-95** | Expansion | Distribute, expand powers, flatten nested structures |
| **70-84** | Identity/Cancellation | x^0→1, x^1→x, x/x→1, x^a/x^b→x^(a-b) |
| **40-69** | Compression/Consolidation | Combine terms, factor, compact a^n/b^n→(a/b)^n |
| **1-39** | Canonicalization | Sort terms, normalize display form |

This ordering ensures expressions are first expanded to expose cancellation opportunities, then simplified via identities and cancellations, and finally compacted into canonical form.

## Rule Categories

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

#### Expansion Phase (85-95)
- **div_div_flatten** (priority: 92) - Rule for flattening nested divisions: (a/b)/(c/d) -> (a*d)/(b*c)
- **combine_nested_fraction** (priority: 91) - Rule for combining nested fractions: (a + b/c) / d → (a*c + b) / (c*d)
- **expand_power_for_cancellation** (priority: 92) - Rule for expanding powers to enable cancellation: (a*b)^n / a -> a^n * b^n / a
- **negative_exponent_to_fraction** (priority: 90) - Rule for x^-n -> 1/x^n where n > 0
- **polynomial_expansion** (priority: 89) - Rule for expanding polynomials (a+b)^n for n=2,3 when beneficial
- **power_of_quotient** (priority: 88) - Rule for (a/b)^n -> a^n / b^n when expansion enables simplification
- **power_expansion** (priority: 86) - Rule for expanding powers: (a*b)^n -> a^n * b^n when beneficial
- **mul_div_combination** (priority: 85) - Rule for a * (b / c) -> (a * b) / c
- **expand_difference_of_squares_product** (priority: 85) - Rule for expanding (a+b)(a-b) -> a² - b² when beneficial

#### Identity/Cancellation Phase (70-84)
- **power_zero** (priority: 80) - Rule for x^0 = 1 (when x != 0)
- **power_one** (priority: 80) - Rule for x^1 = x
- **simplify_negative_one** (priority: 80) - Rule for (-1)*1 = -1, (-1)*(-1) = 1
- **exp_ln** (priority: 80) - Rule for exp(ln(x)) -> x **[alters domain]**
- **ln_exp** (priority: 80) - Rule for ln(exp(x)) -> x **[alters domain]**
- **exp_mul_ln** (priority: 80) - Rule for exp(a * ln(b)) -> b^a **[alters domain]**
- **abs_sign_mul** (priority: 80) - Rule for abs(x) * sign(x) -> x
- **div_self** (priority: 78) - Rule for x / x = 1 (when x != 0) **[alters domain]**
- **fraction_cancellation** (priority: 76) - Rule for cancelling common terms in fractions: (a*b)/(a*c) -> b/c **[alters domain]**
- **power_power** (priority: 75) - Rule for (x^a)^b -> x^(a*b). Returns abs(x) when (x^even)^(1/even) simplifies to x^1
- **power_mul** (priority: 75) - Rule for x^a * x^b -> x^(a+b)
- **power_div** (priority: 75) - Rule for x^a / x^b -> x^(a-b)

#### Compression/Consolidation Phase (40-69)
- **power_collection** (priority: 60) - Rule for collecting powers in multiplication: x^a * x^b -> x^(a+b)
- **combine_factors** (priority: 58) - Rule for combining like factors in multiplication: x * x -> x^2
- **common_exponent_div** (priority: 55) - Rule for x^a / y^a -> (x/y)^a (compaction)
- **common_exponent_mul** (priority: 55) - Rule for x^a * y^a -> (x*y)^a (compaction)
- **combine_like_terms_addition** (priority: 52) - Rule for combining like terms in addition: 2x + 3x -> 5x
- **fraction_to_end** (priority: 50) - Rule for ((1/a) * b) / c -> b / (a * c)
- **combine_terms** (priority: 50) - Rule for combining like terms in addition: 2x + 3x -> 5x
- **distribute_negation** (priority: 50) - Rule for distributing negation: -(A + B) -> -A - B
- **perfect_square** (priority: 48) - Rule for perfect squares: a^2 + 2ab + b^2 -> (a+b)^2
- **factor_difference_of_squares** (priority: 46) - Rule for factoring difference of squares: a^2 - b^2 -> (a-b)(a+b)
- **add_fraction** (priority: 45) - Rule for adding fractions: a + b/c -> (a*c + b)/c
- **numeric_gcd_factoring** (priority: 42) - Rule for factoring out numeric GCD: 2*a + 2*b -> 2*(a+b)
- **perfect_cube** (priority: 40) - Rule for perfect cubes: a^3 + 3a^2b + 3ab^2 + b^3 -> (a+b)^3
- **common_term_factoring** (priority: 40) - Rule for factoring out common terms: ax + bx -> x(a+b)
- **common_power_factoring** (priority: 39) - Rule for factoring out common powers: x³ + x² -> x²(x + 1)

#### Absolute Value & Sign Rules
- **abs_numeric** (priority: 95) - Rule for absolute value of numeric constants: abs(5) -> 5, abs(-3) -> 3
- **sign_numeric** (priority: 95) - Rule for sign of numeric constants: sign(5) -> 1, sign(-3) -> -1, sign(0) -> 0
- **abs_abs** (priority: 90) - Rule for nested absolute value: abs(abs(x)) -> abs(x)
- **abs_neg** (priority: 90) - Rule for absolute value of negation: abs(-x) -> abs(x)
- **sign_abs** (priority: 85) - Rule for sign of absolute value: sign(abs(x)) -> 1 (for x != 0)
- **abs_square** (priority: 85) - Rule for absolute value of square: abs(x^2) -> x^2
- **abs_pow_even** (priority: 85) - Rule for abs(x)^(even) -> x^(even)
- **sign_sign** (priority: 85) - Rule for nested sign: sign(sign(x)) -> sign(x)
- **e_pow_ln** (priority: 85) - Rule for e^(ln(x)) -> x (handles Symbol("e") form) **[alters domain]**
- **e_pow_mul_ln** (priority: 85) - Rule for e^(a*ln(b)) -> b^a (handles Symbol("e") form) **[alters domain]**

#### Canonicalization Phase (1-39)
- **canonicalize** (priority: 15) - Rule for canonicalizing expressions (sorting terms)
- **canonicalize_multiplication** (priority: 15) - Rule for canonical ordering of multiplication terms
- **canonicalize_addition** (priority: 15) - Rule for canonical ordering of addition terms
- **canonicalize_subtraction** (priority: 15) - Rule for canonical ordering in subtraction
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
- **pythagorean_complements** (priority: 70) - Rule for 1 - cos²(x) = sin²(x), 1 - sin²(x) = cos²(x). Also handles the canonicalized forms `-cos²(x) + 1` and `-sin²(x) + 1`.
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
- **ln_exp_identity** (priority: 90) - Rule for ln(exp(x)) = x **[alters domain]**
- **log_power** (priority: 90) - Rule for log(x^n) = n * log(x). For even integer exponents, uses abs: log(x^2) = 2*log(abs(x)). Odd exponents **[alters domain]**
- **log_combination** (priority: 85) - Rule for ln(a) + ln(b) = ln(a*b) and ln(a) - ln(b) = ln(a/b)

### Root Rules (Category: Root)
These handle square root and cube root simplifications.

#### Identity/Cancellation Phase (70-84)
- **sqrt_power** (priority: 85) - Rule for sqrt(x^n) = x^(n/2). Returns abs(x) when sqrt(x^(even)) simplifies to x^1
- **cbrt_power** (priority: 85) - Rule for cbrt(x^n) = x^(n/3)

#### Compression/Consolidation Phase (40-69)
- **sqrt_mul** (priority: 56) - Rule for sqrt(x) * sqrt(y) = sqrt(x*y) **[alters domain]**
- **sqrt_div** (priority: 56) - Rule for sqrt(x)/sqrt(y) = sqrt(x/y) **[alters domain]**
- **normalize_roots** (priority: 50) - Rule that applies the monolithic root normalization

## Domain Safety

Some simplification rules make assumptions about the domain of validity for the expressions they transform. These rules are marked with **[alters domain]** in the rule descriptions above.

When domain-safe mode is enabled, rules that alter domains are skipped to ensure that simplifications remain valid across the entire complex plane or real line, depending on the context. This prevents incorrect simplifications that might introduce new singularities or restrict the domain inappropriately.

### Enabling Domain-Safe Mode

Use the builder pattern to enable domain-safe mode:

```rust
use symb_anafis::Simplify;

Simplify::new()
    .domain_safe(true)
    .simplify_str("sqrt(x^2)")?;  // → abs(x)
```

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
diff("e*x", "x", Some(&["e"]), None)?;
// Here "e" is treated as a constant coefficient, not Euler's number
```

## Rule Count Summary

| Category | Count |
|----------|-------|
| Numeric | 13 |
| Algebraic | 51 |
| Trigonometric | 23 |
| Hyperbolic | 21 |
| Exponential | 9 |
| Root | 6 |
| **Total** | **123** |

## Implementation Details

- All rules are applied recursively bottom-up through the expression tree
- The system uses cycle detection to prevent infinite loops
- Rules are applied in multiple passes until convergence
- Numeric precision uses ε = 1e-10 for floating-point comparisons
- The system preserves exact symbolic forms when possible
- **Rule priority ordering**: Higher priority numbers run first (e.g., priority 95 runs before 40). Key priority tiers:
  - 85-95: Expansion rules (flatten, distribute, expand powers)
  - 70-84: Identity/Cancellation rules (x^0=1, x/x=1, power arithmetic)
  - 40-69: Compression/Consolidation rules (combine terms, factor, compact)
  - 1-39: Canonicalization rules (sort terms, normalize display)
- **Expand → Cancel → Compact strategy**: Rules are ordered so expressions are first expanded to expose simplification opportunities, then simplified via identities and cancellations, and finally compacted into canonical form. This ordering reduces the need for complex conditional checks in individual rules.
- **Factoring vs Distribution balance**: `CommonTermFactoringRule` (priority 40) runs before distribution rules to ensure factored forms are preserved. Distribution is handled by `MulDivCombinationRule` (priority 85) which combines multiplication with division without conflicting with factoring rules.
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
