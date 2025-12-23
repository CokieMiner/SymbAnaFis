# symb_anafis API Reference

A comprehensive guide to the symb_anafis symbolic mathematics library.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Symbol Management](#symbol-management)
3. [Core Functions](#core-functions)
4. [Builder Pattern API](#builder-pattern-api)
5. [Expression Output](#expression-output)
6. [Uncertainty Propagation](#uncertainty-propagation)
7. [Custom Functions](#custom-functions)
8. [Evaluation](#evaluation)
9. [Vector Calculus](#vector-calculus)
10. [Parallel Evaluation](#parallel-evaluation)
11. [Compilation & Performance](#compilation--performance)
12. [Built-in Functions](#built-in-functions)
13. [Expression Syntax](#expression-syntax)
14. [Error Handling](#error-handling)

---

## Quick Start

### Rust

```rust
use symb_anafis::{diff, simplify};

// Differentiate
let result = diff("x^3 + sin(x)", "x", None, None)?;
// Result: "3x^2 + cos(x)"

// Simplify
let result = simplify("sin(x)^2 + cos(x)^2", None, None)?;
// Result: "1"
```

### Python

```python
import symb_anafis

result = symb_anafis.diff("x^3 + sin(x)", "x")  # "3x^2 + cos(x)"
result = symb_anafis.simplify("sin(x)^2 + cos(x)^2")  # "1"
```

---

## Symbol Management

Symbols are **interned** for O(1) comparison and **Copy** for natural operator usage.

> **Tip:** `Symbol` implements `Copy`, so `a + a` works without `.clone()`!

### Creating Symbols

| Function | Behavior | Use Case |
|----------|----------|----------|
| `symb("x")` | Get or create - never errors | General use, parser |
| `symb_new("x")` | Create only - errors if exists | Strict control |
| `symb_get("x")` | Get only - errors if not found | Retrieve existing |
| `Symbol::anon()` | Create anonymous symbol | Temporary computation |

```rust
use symb_anafis::{symb, symb_new, symb_get, Symbol};

// symb() - always works, idempotent
let x1 = symb("x");  // Creates "x"
let x2 = symb("x");  // Returns same "x", no error
assert_eq!(x1.id(), x2.id());  // true - same symbol!

// symb_new() - strict create
let y = symb_new("y")?;     // Ok - creates "y"
let y2 = symb_new("y");     // Err(DuplicateName)

// symb_get() - strict get
let z = symb_get("z");      // Err(NotFound)
let y3 = symb_get("y")?;    // Ok - same as y

// Anonymous symbols
let temp = Symbol::anon();  // Unique ID, no name
```

### Registry Management

```rust
use symb_anafis::{symbol_exists, symbol_count, symbol_names, remove_symbol, clear_symbols};

// Check if symbol exists
if symbol_exists("x") {
    println!("x is registered");
}

// Count registered symbols
let count = symbol_count();  // e.g., 5

// List all symbol names
let names = symbol_names();  // e.g., ["x", "y", "z"]

// Remove a specific symbol
remove_symbol("x");  // Returns true if removed

// Clear all symbols (use with caution!)
clear_symbols();
```

### SymbolContext: Isolated Symbol Registries

For isolated symbol namespaces (useful for testing, avoiding collisions, or multi-tenant systems):

```rust
use symb_anafis::SymbolContext;

// Create isolated contexts
let ctx1 = SymbolContext::new();
let ctx2 = SymbolContext::new();

// Same name, different contexts = DIFFERENT symbols!
let x1 = ctx1.symb("x");  // id: 1234
let x2 = ctx2.symb("x");  // id: 5678 (different!)

// Build expressions using context symbols
let y = ctx1.symb("y");
let expr = x1 + y;  // Uses symbols from ctx1

// Use with builders to ensure correct symbol resolution
let diff = Diff::new()
    .with_context(&ctx1)
    .diff_str("x^2 + y", "x")?;
```

**SymbolContext API:**

| Method | Description |
|--------|-------------|
| `SymbolContext::new()` | Create new isolated context |
| `ctx.symb("x")` | Get or create symbol |
| `ctx.symb_new("x")` | Create only (errors if exists) |
| `ctx.get("x")` | Get if exists (returns `Option<Symbol>`) |
| `ctx.contains("x")` | Check if symbol exists |
| `ctx.len()` | Count symbols in context |
| `ctx.is_empty()` | Check if context has no symbols |
| `ctx.symbol_names()` | List all symbol names |
| `ctx.remove("x")` | Remove a symbol |
| `ctx.clear()` | Remove all symbols |
| `ctx.anon()` | Create anonymous symbol (unique ID, no name) |

**Global Context Accessor:**

```rust
use symb_anafis::global_context;

// Get the singleton global context
let ctx = global_context();
let x = ctx.symb("x");  // Equivalent to symb("x")
```

---

## Core Functions

### `diff(formula, var, fixed_vars, custom_functions)`

Differentiate an expression with respect to a variable.

| Parameter | Type | Description |
|-----------|------|-------------|
| `formula` | `&str` | Expression to differentiate |
| `var` | `&str` | Variable to differentiate with respect to |
| `fixed_vars` | `Option<&[&str]>` | Constants (won't be differentiated) |
| `custom_functions` | `Option<&[&str]>` | User-defined function names |

```rust
// Treat "a" as a constant
diff("a * x^2", "x", Some(&["a"]), None)?;
// Result: "2*a*x"
```

### `simplify(formula, fixed_vars, custom_functions)`

Simplify an expression algebraically.

```rust
simplify("x^2 + 2*x + 1", None, None)?;
// Result: "(x + 1)^2"
```

### `parse(formula, fixed_vars, custom_functions)`

Parse a string into an `Expr` AST.

```rust
use symb_anafis::parse;
use std::collections::HashSet;

let expr = parse("x^2 + 1", &HashSet::new(), &HashSet::new())?;
```

---

## Builder Pattern API

For fine-grained control, use `Diff` and `Simplify` builders.

### `Diff` Builder

```rust
use symb_anafis::{Diff, symb};

let result = Diff::new()
    .domain_safe(true)       // Preserve mathematical domains
    .skip_simplification(true) // Return raw derivative (faster, for benchmarks)
    .max_depth(200)          // AST depth limit
    .max_nodes(50000)        // Node count limit
    .with_context(&ctx)      // Use specific symbol context
    .fixed_var(&symb("a"))   // Single constant
    .custom_fn("f")          // Register function name
    .diff_str("a * f(x)", "x")?;
```

**`Diff` Builder Methods:**

| Method | Description |
|--------|-------------|
| `domain_safe(bool)` | If true, prevents simplifications that change the domain (e.g., `x/x` $\ne$ `1` if x=0). |
| `skip_simplification(bool)` | If true, returns the raw unsimplified derivative. Useful for benchmarking or inspecting the raw output of derivation rules. |
| `fixed_var(&Symbol)` | Registers a variable as constant during differentiation. |
| `fixed_vars(&[&Symbol])` | Registers multiple constants. |
| `custom_fn(name)` | Registers a custom function name so the parser recognizes it. |
| `custom_derivative(name, fn)` | Defines a derivative rule for a custom function. |
| `custom_eval(name, fn)` | Defines a numeric evaluation rule for a custom function. |
| `custom_fn_multi(name, fn)` | Defines a multi-argument custom function with partial derivatives. |
| `with_context(&SymbolContext)` | Sets the symbol context for variable resolution. |
| `max_depth(usize)` | Sets the limit for expression tree depth (prevents stack overflow). |
| `max_nodes(usize)` | Sets the limit for total nodes (prevents DoS). |
```

### `Simplify` Builder

```rust
use symb_anafis::Simplify;

let result = Simplify::new()
    .domain_safe(true)
    .fixed_var(&symb("a"))
    .simplify_str("sqrt(x^2) + a")?;
// With domain_safe: "abs(x) + a"
// Without: "x + a"
```

**`Simplify` Builder Methods:**

| Method | Description |
|--------|-------------|
| `domain_safe(bool)` | If true, prevents simplifications that change the domain. |
| `fixed_var(&Symbol)` | Registers a variable as constant. |
| `fixed_vars(&[&Symbol])` | Registers multiple constants. |
| `custom_fn(name)` | Registers a custom function name. |
| `custom_eval(name, fn)` | Defines a numeric evaluation rule for a custom function. |
| `with_context(&SymbolContext)` | Sets the symbol context. |
| `max_depth(usize)` | Sets the limit for expression tree depth. |
| `max_nodes(usize)` | Sets the limit for total nodes. |
```

### Type-Safe Expressions

Build expressions programmatically:

```rust
use symb_anafis::{symb, Diff, Expr};

let x = symb("x");

// Symbol is Copy - no clone() needed for operators!
let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
let expr2 = x + x;  // Works! Symbol is Copy.

let derivative = Diff::new().differentiate(expr, &x)?;
```

### Expression Inspection

Since `Expr` internals are private to enforce invariants, use accessors to inspect properties:

```rust
let expr = symb("x") + symb("y");

// Unique ID for debugging/caching
let id = expr.id();

// Structural hash for fast equality checks
let hash = expr.hash();

// Access the internal structure (consumes the Expr)
if let ExprKind::Sum(terms) = expr.into_kind() {
    println!("Sum of {} terms", terms.len());
}
```

---

## Expression Output

Format expressions for different output contexts.

### LaTeX Output

```rust
use symb_anafis::sym;

let x = symb("x");
let sigma = symb("sigma");
let expr = x.pow(2.0) / sigma;  // All methods take &self!

println!("{}", expr.to_latex());
// Output: \frac{x^{2}}{\sigma}
```

**LaTeX Features:**
| Expression | LaTeX Output |
|------------|--------------|
| `a / b` | `\frac{a}{b}` |
| `x^n` | `x^{n}` |
| `a * b` | `a \cdot b` |
| `sin(x)` | `\sin\left(x\right)` |
| `sqrt(x)` | `\sqrt{x}` |
| `pi`, `alpha`, etc. | `\pi`, `\alpha`, etc. |

### Unicode Output

```rust
let expr = symb("x").pow(2.0) + symb("pi");
println!("{}", expr.to_unicode());
// Output: x² + π
```

**Unicode Features:**
- Superscripts for integer powers: `x²`, `x³`, `x⁻¹`
- Greek letters: `pi` → `π`, `sigma` → `σ`, `alpha` → `α`
- Proper minus sign: `−`
- Middle dot for multiplication: `·`
- Infinity symbol: `∞`

---

## Uncertainty Propagation

Compute uncertainty propagation using the standard formula:
σ_f = √(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))

### Basic Usage

```rust
use symb_anafis::{symb, uncertainty_propagation};

let x = symb("x");
let y = symb("y");
let expr = x + y;  // Symbol is Copy - no & needed!

// Returns: sqrt(sigma_x^2 + sigma_y^2)
let sigma = uncertainty_propagation(&expr, &["x", "y"], None)?;
println!("{}", sigma.to_latex());
```

### Numeric Covariance

```rust
use symb_anafis::{uncertainty_propagation, CovarianceMatrix, CovEntry};

let cov = CovarianceMatrix::diagonal(vec![
    CovEntry::Num(1.0),  // σ_x² = 1
    CovEntry::Num(4.0),  // σ_y² = 4
])?;  // Returns Result

let sigma = uncertainty_propagation(&expr, &["x", "y"], Some(&cov))?;
// For f = x + y: σ_f = sqrt(1 + 4) = sqrt(5)
```

### Correlated Variables

When variables are correlated (e.g., both depend on temperature), the full formula includes cross-terms:

**σ_f² = Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)**

The **covariance matrix** for 2 variables is:
```
       |  Cov(x,x)   Cov(x,y)  |     |  σ_x²        ρ·σ_x·σ_y  |
Cov =  |                       |  =  |                         |
       |  Cov(y,x)   Cov(y,y)  |     |  ρ·σ_x·σ_y  σ_y²        |
```

Where **ρ** is the correlation coefficient (-1 to +1).

**Example: Fully symbolic correlation**

```rust
use symb_anafis::{symb, CovEntry, CovarianceMatrix, Expr};

let sigma_x = symb("sigma_x");
let sigma_y = symb("sigma_y");
let rho = symb("rho");  // correlation coefficient

// Build the full 2x2 covariance matrix
let cov = CovarianceMatrix::new(vec![
    vec![
        CovEntry::Symbolic(sigma_x.pow(2.0)),              // [0,0]: σ_x²
        CovEntry::Symbolic(rho * sigma_x * sigma_y),       // [0,1]: ρ·σ_x·σ_y
    ],
    vec![
        CovEntry::Symbolic(rho * sigma_x * sigma_y),       // [1,0]: ρ·σ_x·σ_y
        CovEntry::Symbolic(sigma_y.pow(2.0)),              // [1,1]: σ_y²
    ],
])?;  // Returns Result - validates matrix dimensions

let sigma = uncertainty_propagation(&expr, &["x", "y"], Some(&cov))?;
// Result includes cross-terms with ρ
```

**Example: Numeric correlation**

```rust
// Known values: σ_x = 0.1, σ_y = 0.2, ρ = 0.5
let sigma_x = 0.1;
let sigma_y = 0.2;
let rho = 0.5;

let cov = CovarianceMatrix::new(vec![
    vec![
        CovEntry::Num(sigma_x.powi(2)),                    // σ_x² = 0.01
        CovEntry::Num(rho * sigma_x * sigma_y),            // ρ·σ_x·σ_y = 0.01
    ],
    vec![
        CovEntry::Num(rho * sigma_x * sigma_y),            // ρ·σ_x·σ_y = 0.01
        CovEntry::Num(sigma_y.powi(2)),                    // σ_y² = 0.04
    ],
])?;
```

### Relative Uncertainty

```rust
use symb_anafis::relative_uncertainty;

// Returns σ_f / |f|
let rel = relative_uncertainty(&expr, &["x", "y"], None)?;
```

---

## Custom Functions

### Single-Argument Custom Derivatives

Define how to differentiate `f(u)`:

```rust
use symb_anafis::{Diff, Expr};

let diff = Diff::new()
    .custom_derivative("f", |inner, _var, inner_prime| {
        // d/dx[f(u)] = 2u * u'  (chain rule automatic!)
        Expr::number(2.0) * inner.clone() * inner_prime.clone()
    });

diff.diff_str("f(x^2)", "x")?;  // Result: 4x³
```

**Parameters:**
- `inner`: The argument expression (e.g., `x^2` in `f(x^2)`)
- `_var`: The differentiation variable
- `inner_prime`: Derivative of the argument (e.g., `2x`)

### Custom Numeric Evaluation

Allow `f(3)` to evaluate to a number:

```rust
let diff = Diff::new()
    .custom_eval("f", |args| Some(args[0].powi(2) + 1.0))  // f(x) = x² + 1
    .custom_derivative("f", |inner, _var, inner_prime| {
        Expr::number(2.0) * inner.clone() * inner_prime.clone()
    });
```

### Multi-Argument Custom Functions

For functions with 2+ arguments, define **partial derivatives**:

```rust
use symb_anafis::{Diff, CustomFn, Expr};

// F(x, y) = x * sin(y)
let my_fn = CustomFn::new(2)  // 2-arity
    .eval(|args| Some(args[0] * args[1].sin()))
    .partial(0, |args| Expr::func("sin", args[1].clone()))      // ∂F/∂x
    .partial(1, |args| args[0].clone() * Expr::func("cos", args[1].clone()));  // ∂F/∂y

let diff = Diff::new().custom_fn_multi("F", my_fn);
diff.diff_str("F(t, t^2)", "t")?;  // Chain rule applied automatically

// Inspect arity
assert_eq!(my_fn.arity(), 2);
```

### Nested Custom Functions

**Yes, custom functions can call other custom functions!**

```rust
use symb_anafis::{Diff, Expr};

let diff = Diff::new()
    .custom_derivative("f", |inner, _var, inner_prime| {
        // d/dx[f(u)] = 2u * u'
        Expr::number(2.0) * inner.clone() * inner_prime.clone()
    })
    .custom_derivative("g", |inner, _var, inner_prime| {
        // d/dx[g(u)] = 3u² * u'
        Expr::number(3.0) * inner.clone().pow_of(2.0) * inner_prime.clone()
    });

// f(g(x)) differentiates using chain rule:
// d/dx[f(g(x))] = f'(g(x)) * g'(x)
diff.diff_str("f(g(x))", "x")?;
```

---

## Evaluation

### `evaluate_str`

Substitute values into an expression (supports **partial evaluation**):

```rust
use symb_anafis::evaluate_str;

// Full evaluation
evaluate_str("x * y + 1", &[("x", 3.0), ("y", 2.0)])?;
// Result: "7"

// Partial evaluation (y stays symbolic)
evaluate_str("x * y + 1", &[("x", 3.0)])?;
// Result: "3y + 1"
```

### `Expr::evaluate`

For direct expression evaluation:

```rust
use std::collections::HashMap;

let expr = parse("x^2 + y", ...)?;
let mut vars = HashMap::new();
vars.insert("x", 3.0);

let result = expr.evaluate(&vars);  // Returns: 9 + y (Expr)
```

### `evaluate_with_custom`

Evaluate with custom function implementations:

```rust
let mut custom_evals = HashMap::new();
custom_evals.insert("f".to_string(), 
    Arc::new(|args: &[f64]| Some(args[0].powi(2) + 1.0)));

let result = expr.evaluate_with_custom(&vars, &custom_evals);
```

---

## Vector Calculus

### Gradient

```rust
use symb_anafis::gradient_str;

let grad = gradient_str("x^2 + y^2", &["x", "y"])?;
// grad = ["2x", "2y"]
```

### Hessian Matrix

```rust
use symb_anafis::hessian_str;

let hess = hessian_str("x^2 * y", &["x", "y"])?;
// hess = [["2y", "2x"], ["2x", "0"]]
```

### Jacobian Matrix

```rust
use symb_anafis::jacobian_str;

let jac = jacobian_str(&["x^2 + y", "x * y"], &["x", "y"])?;
// jac = [["2x", "1"], ["y", "x"]]
```

### Type-Safe Versions

```rust
use symb_anafis::{symb, gradient, hessian, jacobian};

let x = symb("x");
let y = symb("y");
let expr = x.pow(2.0) + y.pow(2.0);  // All methods take &self!

let grad = gradient(&expr, &[&x, &y]);  // Vec<Expr>
```

---

## Parallel Evaluation

> Requires `parallel` feature: `symb_anafis = { features = ["parallel"] }`

Evaluate multiple expressions at multiple points in parallel using Rayon.

### `eval_parallel!` Macro

Clean syntax for mixed-type parallel evaluation:

```rust
use symb_anafis::parallel::{eval_parallel, SKIP};

let x = symb("x");
let y = symb("y");
let expr1 = &x.pow(2.0);
let expr2 = &(x.clone() + y.clone());

// Evaluate at multiple points
let results = eval_parallel!(
    [expr1, expr2],           // Expressions
    [["x"], ["x", "y"]],      // Variables per expression
    [
        [[1.0, 2.0, 3.0]],                    // expr1: x values
        [[1.0, 2.0], [10.0, 20.0]]            // expr2: x and y values
    ]
);
// results[0] = [1.0, 4.0, 9.0]
// results[1] = [11.0, 22.0]
```

### `SKIP` Constant for Partial Evaluation

Use `SKIP` to keep variables symbolic:

```rust
use symb_anafis::parallel::{eval_parallel!, SKIP};

let results = eval_parallel!(
    [&(x * y)],
    [["x", "y"]],
    [[[2.0, SKIP, 4.0], [3.0, 5.0, 6.0]]]
);
// Point 0: 2*3 = Num(6.0)
// Point 1: SKIP*5 = Expr("5*x") - symbolic result!
// Point 2: 4*6 = Num(24.0)
```

### Return Types: `EvalResult`

Results are `EvalResult` enum preserving type information:

```rust
pub enum EvalResult {
    Num(f64),       // Fully evaluated numeric result
    Expr(Expr),     // Symbolic result (when SKIP used)
}
```

### Direct Function: `evaluate_parallel`

For programmatic use without macro:

```rust
use symb_anafis::parallel::{evaluate_parallel, ExprInput, VarInput, Value};

let results = evaluate_parallel(
    &[ExprInput::Ref(&expr)],
    &[VarInput::Slice(&["x"])],
    &[&[&[Value::Num(1.0), Value::Num(2.0)]]]
);
```

---

## Compilation & Performance

For tight loops or massive repeated evaluation, use the `CompiledEvaluator`. It compiles the expression tree into a flat bytecode that interpreted efficiently, avoiding tree traversal overhead and enabling SIMD optimizations.

### `CompiledEvaluator`

```rust
use symb_anafis::Expr;
// Create an expression
let expr = symb("x").sin() * symb("x").pow(2.0);

// Compile it (requires list of parameter names in order)
let compiled = expr.compile(&["x"])?;

// Evaluate repeatedly (much faster than expr.evaluate)
let result = compiled.evaluate(&[0.5]); // Result at x=0.5
```

**Methods:**

| Method | Description |
|--------|-------------|
| `compile(expr, params)` | Compile an expression for given parameters. |
| `compile_with_context(expr, params, ctx)` | Compile with a custom function context. |
| `evaluate(args)` | Evaluate at a single point. |
| `eval_batch(cols, out)` | Evaluate multiple points (SIMD logic). |

### `FunctionContext` (Custom Functions in Compiler)

To use custom functions within compiled expressions, register them in a `FunctionContext`.

```rust
use symb_anafis::{FunctionContext, FunctionDefinition};

// 1. Create a context
let ctx = FunctionContext::new();

// 2. Register a function
ctx.register("my_sq", FunctionDefinition {
    name: "my_sq",
    arity: 1..=1,
    // Numeric evaluation logic
    eval: |args| Some(args[0] * args[0]),
    // Derivative logic (required by struct, though mostly used by Diff)
    derivative: |args, _| crate::Expr::number(0.0), // Simplified for example
})?;

// 3. Compile with context
// Note: Requires manual parsing or construction if using custom fns not in default parser
// (Parser needs custom_functions context too)
// For manual construction:
let expr = Expr::func("my_sq", symb("x"));

let compiled = CompiledEvaluator::compile_with_context(
    &expr, 
    &["x"], 
    Some(&ctx)
)?;

let result = compiled.evaluate(&[3.0]); // 9.0
```

---

## Built-in Functions

| Category | Functions |
|----------|-----------|
| **Trig** | `sin`, `cos`, `tan`, `cot`, `sec`, `csc` |
| **Inverse Trig** | `asin`, `acos`, `atan`, `acot`, `asec`, `acsc` |
| **Hyperbolic** | `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch` |
| **Inverse Hyperbolic** | `asinh`, `acosh`, `atanh`, `acoth`, `asech`, `acsch` |
| **Exp/Log** | `exp`, `ln`, `log`, `log10`, `log2`, `exp_polar` |
| **Roots** | `sqrt`, `cbrt` |
| **Error Functions** | `erf`, `erfc` |
| **Gamma Family** | `gamma`, `digamma`, `trigamma`, `tetragamma`, `polygamma(n, x)`, `beta(a, b)` |
| **Zeta** | `zeta`, `zeta_deriv(n, s)` |
| **Bessel** | `besselj(n, x)`, `bessely(n, x)`, `besseli(n, x)`, `besselk(n, x)` |
| **Elliptic Integrals** | `elliptic_k`, `elliptic_e` |
| **Orthogonal Polynomials** | `hermite(n, x)`, `assoc_legendre(l, m, x)` |
| **Spherical Harmonics** | `spherical_harmonic(l, m, θ, φ)`, `ynm(l, m, θ, φ)` |
| **Other** | `abs`, `sign`, `signum`, `sinc`, `LambertW`, `lambertw`, `floor`, `ceil`, `round` |

> **Note:** All functions have both **numeric evaluation** and **symbolic differentiation** rules. Multi-argument functions like `besselj(n, x)` differentiate with respect to `x` (treating `n` as constant).

### Using Built-in Functions

```rust
use symb_anafis::{symb, Diff, Expr};

let x = symb("x");

// All Symbol methods take &self - no clone() needed!
let expr = x.sin();                  // sin(x)
let expr = x.pow(2.0);               // x²
let expr = x.gamma();                // gamma(x)
// Reuse x freely: all methods borrow, nothing is consumed!

// Multi-argument functions
let expr = x.besselj(0);             // J_0(x) - shorthand
let expr = Expr::call("besselj", [Expr::number(0.0), x.into()]);  // Explicit

// Differentiate special functions
let result = Diff::new().diff_str("gamma(x)", "x")?;
// Result: digamma(x) * gamma(x)

let result = Diff::new().diff_str("besselj(0, x)", "x")?;
// Result: (besselj(-1, x) - besselj(1, x)) / 2  (Bessel recurrence)
```

---

## Expression Syntax

| Element | Syntax | Example |
|---------|--------|---------|
| Variables | Any identifier | `x`, `y`, `sigma` |
| Numbers | Integer/decimal/scientific | `1`, `3.14`, `1e-5` |
| Addition | `+` | `x + 1` |
| Subtraction | `-` | `x - 1` |
| Multiplication | `*` | `x * y` |
| Division | `/` | `x / y` |
| Power | `^` | `x^2` |
| Function calls | `name(args)` | `sin(x)`, `log(x, 10)` |
| Constants | `pi`, `e` | Auto-recognized |
| Implicit mult | Adjacent terms | `2x`, `(x+1)(x-1)` |
| Partial derivative | `∂_f(x)/∂_x` | Output notation |

### Operator Precedence

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| Highest | `^` (power) | Right |
| | `*`, `/` | Left |
| Lowest | `+`, `-` | Left |

---

## Error Handling

All functions return `Result<T, DiffError>`:

```rust
use symb_anafis::{Diff, DiffError};

let diff = Diff::new();
match diff.diff_str("invalid syntax ((", "x") {
    Ok(result) => println!("Result: {}", result),
    Err(DiffError::InvalidSyntax { msg, .. }) => println!("Parse error: {}", msg),
    Err(e) => println!("Other error: {:?}", e),
}
```

---