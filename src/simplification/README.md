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

## AST Structure Impact

All rules are designed to work with the N-ary AST structure:

- **`Sum(Vec<Arc<Expr>>)`**: Flat commutative addition
  - `a + b + c` is `Sum([a, b, c])`, NOT `Add(Add(a, b), c)`
  - `a - b` is `Sum([a, Product([-1, b])])`
  - Flattening happens automatically in constructors

- **`Product(Vec<Arc<Expr>>)`**: Flat commutative multiplication
  - `a * b * c` is `Product([a, b, c])`, NOT `Mul(Mul(a, b), c)`
  - Flattening happens automatically in constructors

- **`Div(Arc<Expr>, Arc<Expr>)`**: Binary, non-commutative
  - `a / b` is stored as-is, NOT `Mul(a, Pow(b, -1))`

- **`Pow(Arc<Expr>, Arc<Expr>)`**: Binary, non-commutative
  - `a^b` is stored as-is

- **Canonical order**: Numbers first, then sorted by expression comparison
  - Numbers are combined into first position during construction

## Rule Categories

Rules are grouped by category and listed in priority order within each category.

---

## Rule Categories

### Numeric Rules (Category: Numeric)

These handle basic arithmetic identities and constant folding.

#### Identity/Cancellation Phase (Priority 100)

- **`sum_identity`** (priority: 100) - Removes zero terms from sums: `x + 0 = x`, `0 + x = x`
  - Handles flat `Sum` correctly
- **`product_zero`** (priority: 100) - Zero in product makes result zero: `0 * x = 0`, `x * 0 = 0`
  - Handles flat `Product` correctly
- **`product_identity`** (priority: 100) - Removes one factors from products: `1 * x = x`, `x * 1 = x`
  - Handles flat `Product` correctly
- **`div_one`** (priority: 100) - Division by one: `x / 1 = x`
- **`zero_div`** (priority: 100) - Zero divided by anything: `0 / x = 0`
- **`pow_zero`** (priority: 100) - Power of zero: `x^0 = 1` (when x != 0)
- **`pow_one`** (priority: 100) - Power of one: `x^1 = x`
- **`zero_pow`** (priority: 100) - Zero to a power: `0^x = 0`
- **`one_pow`** (priority: 100) - One to a power: `1^x = 1`

#### Expansion/Normalization Phase (Priority 90-95)

- **`normalize_sign_div`** (priority: 95) - Normalizes signs in division: `x / -y → -x / y`
  - Checks for `Product([-1, y])` pattern correctly for N-ary Product
- **`perfect_square`** (priority: 95) - Evaluates perfect squares: `√9 = 3`
- **`perfect_cube`** (priority: 95) - Evaluates perfect cubes: `∛8 = 2`
- **`eval_numeric_func`** (priority: 95) - Evaluates functions with numeric arguments
  - Handles constant evaluation via function registry

#### Constant Folding Phase (Priority 90)

- **`constant_fold_sum`** (priority: 90) - Combines numeric terms in sums
  - Iterates flat `Sum` terms directly
- **`constant_fold_product`** (priority: 90) - Combines numeric factors in products
  - Iterates flat `Product` factors directly
- **`constant_fold_div`** (priority: 90) - Evaluates numeric divisions
- **`constant_fold_pow`** (priority: 90) - Evaluates numeric powers

#### Compaction Phase (Priority 80)

- **`product_half`** (priority: 80) - Converts `0.5 * x` to `x/2`
  - Checks `Product([0.5, x])` pattern correctly
- **`fraction_simplify`** (priority: 80) - Simplifies fractions with integer coefficients using GCD

**Total Numeric Rules: 15**

---

### Algebraic Rules (Category: Algebraic)

These handle polynomial operations, factoring, absolute value, sign functions, and structural transformations.

#### Identity/Cancellation Phase (Priority 70-84)

- **`exp_ln`** (priority: 80) - Rule for `exp(ln(x)) -> x` **[alters domain]**
  - Handles function nesting correctly
- **`ln_exp`** (priority: 80) - Rule for `ln(exp(x)) -> x` **[alters domain]**
  - Handles function nesting correctly
- **`exp_mul_ln`** (priority: 80) - Rule for `exp(a * ln(b)) -> b^a` **[alters domain]**
  - Handles Product in argument correctly
- **`div_self`** (priority: 78) - Rule for `x / x = 1` (when x != 0) **[alters domain]**
  - Checks both Div sides directly
- **`power_zero`** (priority: 80) - Rule for `x^0 = 1`
  - Handles Pow directly
- **`power_one`** (priority: 80) - Rule for `x^1 = x`
  - Handles Pow directly
- **`power_power`** (priority: 75) - Rule for `(x^a)^b -> x^(a*b)`
  - Flattens nested powers
- **`power_product`** (priority: 75) - Rule for `x^a * x^b -> x^(a+b)`
  - distinct from `power_collection` (priority 60), handles pairwise combinations during identity phase
- **`power_div`** (priority: 75) - Rule for `x^a / x^b -> x^(a-b)`
  - Core cancellation logic for powers

#### Expansion Phase (Priority 85-92)

- **`expand_power_for_cancellation`** (priority: 92) - Rule for expanding powers to enable cancellation: `(a*b)^n / a -> a^n * b^n / a`
  - Handles Product in Pow base correctly
- **`negative_exponent_to_fraction`** (priority: 90) - Rule for `x^-n -> 1/x^n` where n > 0
  - Handles `Product([-1, a, b, ...])` pattern correctly
- **`polynomial_expansion`** (priority: 89) - Rule for expanding polynomials `(a+b)^n` for n=2,3,4
  - **IMPORTANT**: This rule expands BOTH symbolic AND numeric terms. For example:
    - `(x + y)^2 -> x^2 + 2*x*y + y^2` ✅
    - `(1 + 2)^2 -> 1 + 4 + 4` ✅
    - `(x + 1)^2 -> x^2 + 2*x + 1` ✅
  - `(x^2 + 1)^2 -> x^4 + 2*x^2 + 1` ✅
  - **The rule does NOT prevent expansion - it only restricts to 2-term Sums and n=2,3,4**
- **`power_of_quotient`** (priority: 88) - Rule for `(a/b)^n -> a^n / b^n` when expansion enables simplification
- **`power_expansion`** (priority: 86) - Rule for expanding powers: `(a*b)^n -> a^n * b^n` when beneficial
  - Handles Product in Pow base correctly
- **`product_div_combination`** (priority: 85) - Rule for `a * (b / c) -> (a * b) / c`
  - Handles Product with nested Div correctly
- **`div_div_flatten`** (priority: 92) - Rule for flattening nested divisions: `(a/b)/(c/d) -> (a*d)/(b*c)`
  - Handles Div in Div sides correctly
- **`combine_nested_fraction`** (priority: 91) - Rule for combining nested fractions: `(a + b/c) / d -> (a*c + b)/(c*d)`
  - Handles Sum containing Div correctly

#### Compression/Consolidation Phase (Priority 40-76)

- **`power_collection`** (priority: 60) - Rule for collecting powers in multiplication: `x^a * x^b -> x^(a+b)`
  - Groups by base in flat Product correctly
- **`combine_factors`** (priority: 58) - Rule for combining like factors: `x * x -> x^2`
  - Groups by base in flat Product correctly
- **`common_exponent_div`** (priority: 55) - Rule for `x^a / y^a -> (x/y)^a` (compaction)
  - Handles Pow with same exponent correctly
- **`common_exponent_product`** (priority: 55) - Rule for `x^a * y^a -> (x*y)^a` (compaction)
  - Handles Pow with same exponent correctly
- **`combine_like_terms_in_sum`** (priority: 52) - Rule for combining like terms in addition: `2x + 3x -> 5x`
  - Groups terms by base in flat Sum correctly
- **`combine_terms`** (priority: 50) - Rule for combining like terms in addition (duplicate of above)
- **`fraction_to_end`** (priority: 50) - Rule for `((1/a) * b) / c -> b / (a * c)`
  - Moves Divs to outermost level correctly
- **`add_fraction`** (priority: 45) - Rule for adding fractions: `a + b/c -> (a*c + b)/c`
  - Handles 2-term Sum with fractions correctly
- **`factor_difference_of_squares`** (priority: 46) - Rule for factoring difference of squares: `a^2 - b^2 -> (a-b)(a+b)`
  - Checks for `Product([-1, b^2])` pattern correctly (subtraction in N-ary)
- **`numeric_gcd_factoring`** (priority: 42) - Rule for factoring out numeric GCD: `2*a + 2*b -> 2*(a+b)`
  - Handles flat Sum correctly
- **`common_term_factoring`** (priority: 40) - Rule for factoring out common terms: `ax + bx -> x(a+b)`
  - Handles flat Sum correctly
- **`common_power_factoring`** (priority: 39) - Rule for factoring out common powers: `x³ + x² -> x²(x + 1)`
  - Handles flat Sum correctly
- **`fraction_cancellation`** (priority: 76) - Rule for cancelling common terms in fractions: `(a*b)/(a*c) -> b/c`
  - Handles Div sides correctly, extracts factors as Vec
- **`perfect_square_factoring`** (priority: 100) - Rule for perfect square factoring: `a^2 + 2ab + b^2 -> (a+b)^2`
  - Handles flat Sum with 3 terms correctly
- **`perfect_cube_factoring`** (priority: 40) - Rule for sum/difference of cubes: `a^3 + b^3 = (a+b)(a²-ab+b²)` and `a^3 - b^3 = (a-b)(a²+ab+b²)`
  - Handles flat Sum and Poly correctly

#### Canonicalization Phase (Priority 5-15)

- **`canonicalize_product`** (priority: 15) - Rule for canonical ordering of multiplication terms
  - Sorts flat Product factors correctly
- **`canonicalize_sum`** (priority: 15) - Rule for canonical ordering of addition terms
  - Sorts flat Sum terms correctly
- **`simplify_negative_product`** (priority: 80) - Rule for normalizing negative terms in products
  - Handles pairs of `-1` in flat Product correctly

#### Absolute Value & Sign Rules (Priority 85-95)

- **`abs_numeric`** (priority: 95) - Rule for absolute value of numeric constants: `abs(5) -> 5`, `abs(-3) -> 3`
- **`sign_numeric`** (priority: 95) - Rule for sign of numeric constants: `sign(5) -> 1`, `sign(-3) -> -1`, `sign(0) -> 0`
- **`abs_abs`** (priority: 90) - Rule for nested absolute value: `abs(abs(x)) -> abs(x)`
- **`abs_neg`** (priority: 90) - Rule for absolute value of negation: `abs(-x) -> abs(x)`
  - Checks for `Product([-1, x])` pattern correctly
- **`abs_square`** (priority: 85) - Rule for absolute value of square: `abs(x^2) -> x^2`
  - Checks for Pow with even exponent correctly
- **`abs_pow_even`** (priority: 85) - Rule for `abs(x)^(even) -> x^(even)`
  - Checks for Pow with even exponent correctly
- **`sign_sign`** (priority: 85) - Rule for nested sign: `sign(sign(x)) -> sign(x)`
- **`sign_abs`** (priority: 85) - Rule for sign of absolute value: `sign(abs(x)) -> 1` (for x != 0)
- **`abs_sign_mul`** (priority: 80) - Rule for `abs(x) * sign(x) -> x`
  - Handles Product pattern correctly

#### Fractions (Priority 76-92)

- **`div_div_flatten`** (priority: 92) - Rule for flattening nested divisions: `(a/b)/(c/d) -> (a*d)/(b*c)`
- **`combine_nested_fraction`** (priority: 91) - Rule for combining nested fractions: `(a + b/c) / d -> (a*c + b)/(c*d)`

**Total Algebraic Rules: 47**

---

### Trigonometric Rules (Category: Trigonometric)

These handle trigonometric identities, exact values, and transformations.

#### Exact Values & Identities (Priority 95)

- **`sin_zero`** (priority: 95) - Rule for `sin(0) = 0`
- **`cos_zero`** (priority: 95) - Rule for `cos(0) = 1`
- **`tan_zero`** (priority: 95) - Rule for `tan(0) = 0`
- **`sin_pi`** (priority: 95) - Rule for `sin(π) = 0`
  - Uses helper function to check for `π`
- **`cos_pi`** (priority: 95) - Rule for `cos(π) = -1`
  - Uses helper function to check for `π`
- **`sin_pi_over_two`** (priority: 95) - Rule for `sin(π/2) = 1`
  - Checks for `Div(π, 2)` in N-ary Sum correctly
- **`cos_pi_over_two`** (priority: 95) - Rule for `cos(π/2) = 0`
  - Checks for `Div(π, 2)` in N-ary Sum correctly
- **`trig_exact_values`** (priority: 95) - Rule for `sin(π/6) = 1/2`, `cos(π/3) = 1/2`, etc.
  - Handles numeric and symbolic inputs like `Div(π, 6)`

#### Pythagorean Identities (Priority 70-80)

- **`pythagorean_identity`** (priority: 80) - Rule for `sin²(x) + cos²(x) = 1`
  - Handles 2-term Sum correctly
- **`pythagorean_complements`** (priority: 70) - Rule for `1 - cos²(x) = sin²(x)` and `1 - sin²(x) = cos²(x)`
  - Checks for `Product([-1, cos²(x)])` pattern correctly (canonical subtraction)
- **`pythagorean_tangent`** (priority: 70) - Rule for `tan²(x) + 1 = sec²(x)` and `cot²(x) + 1 = csc²(x)`
  - Handles 2-term Sum correctly

#### Inverse & Composition (Priority 85-90)

- **`inverse_trig_identity`** (priority: 90) - Rule for `sin(asin(x)) = x` and `cos(acos(x)) = x` **[alters domain]**
- **`inverse_trig_composition`** (priority: 85) - Rule for `asin(sin(x)) = x` and `acos(cos(x)) = x` **[alters domain]**

#### Cofunction & Periodicity (Priority 80-85)

- **`cofunction_identity`** (priority: 85) - Rule for `sin(π/2 - x) = cos(x)` and `cos(π/2 - x) = sin(x)`
  - Handles `Sum([Div(π, 2), ...])` patterns correctly
- **`trig_periodicity`** (priority: 85) - Rule for periodicity: `sin(x + 2kπ) = sin(x)`
  - Handles `Sum` containing `2πk` correctly
- **`trig_reflection`** (priority: 80) - Rule for reflection: `sin(π - x) = sin(x)`, `cos(π - x) = -cos(x)`
  - Handles `Sum([π, Product([-1, x])])` patterns correctly
- **`trig_three_pi_over_two`** (priority: 80) - Rule for `sin(3π/2 - x) = -cos(x)`, `cos(3π/2 - x) = -sin(x)`
  - Handles `Sum([Div(3π, 2), ...])` patterns correctly
- **`trig_neg_arg`** (priority: 90) - Rule for `sin(-x) = -sin(x)`, `cos(-x) = cos(x)`, etc.
  - Checks for `Product([-1, x])` pattern correctly

#### Angle Transformations (Priority 70-90)

- **`cos_double_angle_difference`** (priority: 85) - Rule for cosine double angle in difference form: `cos²(x) - sin²(x) = cos(2x)`
- **`trig_sum_difference`** (priority: 70) - Rule for sum/difference identities: `sin(x+y)`, `cos(x-y)`, etc.
- **`trig_product_to_double_angle`** (priority: 90) - Rule for `(cos(x)-sin(x))*(cos(x)+sin(x)) = cos(2x)`
- **`sin_product_to_double_angle`** (priority: 90) - Rule for `k*sin(x)*cos(x) = (k/2)*sin(2x)` (contraction to canonical form)

#### Triple Angle (Priority 70)

- **`trig_triple_angle`** (priority: 70) - Rule for triple angle folding: `3sin(x) - 4sin³(x) -> sin(3x)`
  - Handles both flat Sum and Poly forms correctly

#### Reciprocal Conversions (Priority 85)

- **`one_cos_to_sec`** (priority: 85) - Rule for `1/cos(x) -> sec(x)` and `1/cos(x)^n -> sec(x)^n`
  - Handles Div with numerator 1 correctly
- **`one_sin_to_csc`** (priority: 85) - Rule for `1/sin(x) -> csc(x)` and `1/sin(x)^n -> csc(x)^n`
  - Handles Div with numerator 1 correctly
- **`sin_cos_to_tan`** (priority: 85) - Rule for `sin(x)/cos(x) -> tan(x)`
  - Handles Div correctly
- **`cos_sin_to_cot`** (priority: 85) - Rule for `cos(x)/sin(x) -> cot(x)`
  - Handles Div correctly

**Total Trigonometric Rules: 23**

---

### Hyperbolic Rules (Category: Hyperbolic)

These handle hyperbolic function identities and exponential forms.

#### Exact Values & Identities (Priority 95)

- **`sinh_zero`** (priority: 95) - Rule for `sinh(0) = 0`
- **`cosh_zero`** (priority: 95) - Rule for `cosh(0) = 1`
- **`sinh_asinh_identity`** (priority: 95) - Rule for `sinh(asinh(x)) = x`
- **`cosh_acosh_identity`** (priority: 95) - Rule for `cosh(acosh(x)) = x`
- **`tanh_atanh_identity`** (priority: 95) - Rule for `tanh(atanh(x)) = x`
- **`hyperbolic_identity`** (priority: 95) - Rule for `cosh²(x) - sinh²(x) = 1` and related identities

#### Negation (Priority 90)

- **`sinh_negation`** (priority: 90) - Rule for `sinh(-x) = -sinh(x)`
  - Checks for `Product([-1, x])` pattern correctly
- **`cosh_negation`** (priority: 90) - Rule for `cosh(-x) = cosh(x)`
  - Checks for `Product([-1, x])` pattern correctly
- **`tanh_negation`** (priority: 90) - Rule for `tanh(-x) = -tanh(x)`
  - Checks for `Product([-1, x])` pattern correctly

#### Ratio Conversions (Priority 85)

- **`sinh_cosh_to_tanh`** (priority: 85) - Rule for `sinh(x)*cosh(x) -> sinh(2x)/2`
  - Handles Div correctly
- **`cosh_sinh_to_coth`** (priority: 85) - Rule for `cosh(x)*sinh(x) -> sinh(2x)/2`
  - Handles Div correctly
- **`one_cosh_to_sech`** (priority: 85) - Rule for `1/cosh(x) -> sech(x)`
  - Handles Div with numerator 1 correctly
- **`one_sinh_to_csch`** (priority: 85) - Rule for `1/sinh(x) -> csch(x)`
  - Handles Div with numerator 1 correctly
- **`one_tanh_to_coth`** (priority: 85) - Rule for `1/tanh(x) -> coth(x)`
  - Handles Div with numerator 1 correctly

#### Exponential Conversions (Priority 80)

All exponential conversion rules handle different term orderings in commutative operations (Addition):
- `Sum([e^x, e^(-x)])` → sinh(x)
- `Sum([e^x, e^(-x)])` (reversed) → sinh(x)
- `Div(e^x, e^x + e^(-x))` → tanh(x)
- `Div(e^x + e^(-x), e^x - e^(-x))` → tanh(x)` (reversed)
- `Div(2, e^x + e^(-x))` → sech(x)`
- `Div(2, e^x - e^(-x))` → sech(x)` (reversed)
- `Div(e^x, e^x - e^(-x))` → csch(x)` (and reversed)
- `Div(e^x + e^(-x), e^x - e^(-x))` → coth(x)` (and reversed)

- **`sinh_from_exp`** (priority: 80) - Rule for `(e^x - e^(-x)) / 2 -> sinh(x)`
  - Handles Div patterns correctly with both term orderings
- **`cosh_from_exp`** (priority: 80) - Rule for `(e^x + e^(-x)) / 2 -> cosh(x)`
  - Handles Div patterns correctly with both term orderings
- **`tanh_from_exp`** (priority: 80) - Rule for `(e^x - e^(-x)) / (e^x + e^(-x)) -> tanh(x)`
  - Handles Div patterns correctly with both denominator orderings
- **`sech_from_exp`** (priority: 80) - Rule for `2 / (e^x + e^(-x)) -> sech(x)`
  - Handles both denominator orderings correctly
- **`csch_from_exp`** (priority: 80) - Rule for `2 / (e^x - e^(-x)) -> csch(x)`
  - Handles both term orderings in commutative numerators
- **`coth_from_exp`** (priority: 80) - Rule for `(e^x + e^(-x)) / (e^x - e^(-x)) -> coth(x)`
  - Handles both term orderings in numerator and denominator

#### Advanced Identities (Priority 70)

- **`hyperbolic_triple_angle`** (priority: 70) - Rule for `4sinh³(x) + 3sinh(x) -> sinh(3x)`, etc.
  - Handles both flat Sum and Poly forms correctly

**Total Hyperbolic Rules: 21**

---

### Exponential Rules (Category: Exponential)

These handle logarithmic and exponential function identities.

#### Identity/Cancellation Phase (Priority 95)

- **`ln_one`** (priority: 95) - Rule for `ln(1) = 0`
- **`ln_e`** (priority: 95) - Rule for `ln(e) = 1` (when e is Euler's number, not a user variable)
- **`exp_zero`** (priority: 95) - Rule for `exp(0) = 1`
- **`exp_to_e_pow`** (priority: 95) - Rule for `exp(x) -> e^x`

#### Identities (Priority 90)

- **`exp_ln_identity`** (priority: 90) - Rule for `exp(ln(x)) = x` (for x > 0) **[alters domain]**
- **`ln_exp_identity`** (priority: 90) - Rule for `ln(exp(x)) = x` **[alters domain]**

#### Power Rules (Priority 90)

- **`log_power`** (priority: 90) - Rule for `ln(x^n) = n * ln(x)` (also supports `log10`, `log2`, and generic `log(b, x)`)
  - For even integer exponents, uses abs: `log(x^2) = 2*log(abs(x))`
  - Respects domain-safe mode for odd exponents
  - Handles Pow correctly

#### Base Values (Priority 95)

- **`log_base_values`** (priority: 95) - Rule for specific log values: `log10(1)=0`, `log10(10)=1`, `log2(1)=0`, `log2(2)=1`. Also supports `log(b, 1)=0` and `log(b, b)=1`.

#### Combination (Priority 85)

- **`log_combination`** (priority: 85) - Rule for `ln(a) + ln(b) = ln(ab)` and `ln(a) - ln(b) = ln(a/b)`
  - Handles 2-term Sum correctly

**Total Exponential Rules: 9**

---

### Root Rules (Category: Root)

These handle square root and cube root simplifications.

#### Identity/Cancellation Phase (Priority 85)

- **`sqrt_power`** (priority: 85) - Rule for `sqrt(x^n) = x^(n/2)`
  - Returns `abs(x)` when `sqrt(x^2)` simplifies to `x^1`
- **`cbrt_power`** (priority: 85) - Rule for `cbrt(x^n) = x^(n/3)`
  - Returns `x` when exponent simplifies to `1`

#### Compression/Consolidation Phase (Priority 50-84)

- **`sqrt_extract_square`** (priority: 84) - Rule for `sqrt(a * x^2) -> |x| * sqrt(a)`
  - Handles Product patterns correctly

#### Domain-Altering Rules (Priority 56)

- **`sqrt_product`** (priority: 56) - Rule for `sqrt(x) * sqrt(y) = sqrt(xy)` **[alters domain]**
  - Handles Product iteration correctly
- **`sqrt_div`** (priority: 56) - Rule for `sqrt(x)/sqrt(y) = sqrt(x/y)` **[alters domain]**
  - Handles Div correctly

#### Canonicalization (Priority 50)

- **`normalize_roots`** (priority: 50) - Rule that applies monolithic root normalization
  - `sqrt(x) -> x^(1/2)`, `cbrt(x) -> x^(1/3)`

**Total Root Rules: 6**

---

## Domain Safety

Some simplification rules make assumptions about the domain of validity for expressions they transform. These rules are marked with **[alters domain]** in rule descriptions above.

When domain-safe mode is enabled, rules that alter domains are skipped to ensure that simplifications remain valid across the entire complex plane or real line, depending on context. This prevents incorrect simplifications that might introduce new singularities or restrict domain inappropriately.

### Enabling Domain-Safe Mode

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

## Constant Handling and Symbols

The simplification system handles mathematical constants (`e`, `pi`, `tau`, `inf`) consistently using stable UIDs. 

### Multi-character Symbols

If you use multi-character symbols in your expressions (e.g., `alpha`, `beta`), you must pass them to the **parser** so it doesn't treat them as `a*l*p*h*a`. However, once parsed into an `Expr`, the simplifier treats all symbols (other than constants) as variables.

### "e" vs Variables

In older versions, `known_symbols` was used to distinguish between Euler's number `e` and a user variable named `e`. This is no longer necessary:
- The symbol `e` is **always** Euler's number in the simplifier context.
- If you need a variable named `e`, it is recommended to use a different name to avoid ambiguity with the mathematical constant.

## Rule Count Summary

| Category | Count |
|----------|-------|
| Numeric | 15 |
| Algebraic | 47 |
| Trigonometric | 23 |
| Hyperbolic | 21 |
| Exponential | 9 |
| Root | 6 |
| **Total** | **121** |

## Implementation Details

- All rules are applied recursively bottom-up through the expression tree
- The system uses cycle detection to prevent infinite loops
- Rules are applied in multiple passes until convergence
- Numeric precision uses ε = 1e-14 (EPSILON) for floating-point comparisons
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
