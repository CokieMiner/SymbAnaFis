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

### Cancellation
- **Subtraction**: `x - x = 0`
- **Division**: `x / x = 1`

### Exponent Manipulation
- **Power of power**: `(x^a)^b = x^(a*b)`
- **Division with powers**: `x^n / x^m = x^(n-m)`
- **Division by power**: `x^n / x = x^(n-1)`
- **Division of power**: `x / x^n = x^(1-n)`

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
- **sin(asin(x)) = x**
- **cos(acos(x)) = x**
- **tan(atan(x)) = x**
- **asin(sin(x)) = x**
- **acos(cos(x)) = x**

### Cofunction Identities
- **sin(π/2 - x) = cos(x)**
- **cos(π/2 - x) = sin(x)**

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
- **sin(2x) = 2*sin(x)*cos(x)**
- **cos(2x) = cos²(x) - sin²(x)**
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
- **1 - tanh²(x) = sech²(x)**
- **coth²(x) - 1 = csch²(x)**

### Exponential Form Recognition
- **(e^x - e^-x)/2 = sinh(x)**
- **(e^x + e^-x)/2 = cosh(x)**
- **(e^x - e^-x)/(e^x + e^-x) = tanh(x)**
- **(e^x + e^-x)/(e^x - e^-x) = coth(x)**
- **2/(e^x + e^-x) = sech(x)**
- **2/(e^x - e^-x) = csch(x)**

## Logarithmic/Exponential Rules (`log_exp.rs`)

### Exponential Rules
- **exp(0) = 1**
- **exp(ln(x)) = x**

### Logarithmic Rules
- **ln(1) = 0**
- **ln(exp(x)) = x**

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

## Implementation Notes

- All rules are applied recursively bottom-up through the expression tree
- The system uses cycle detection to prevent infinite loops
- Rules are applied in multiple passes until convergence
- Numeric precision uses ε = 1e-10 for floating-point comparisons
- The system preserves exact symbolic forms when possible