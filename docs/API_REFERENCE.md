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
10. [Automatic Differentiation](#automatic-differentiation)
11. [Parallel Evaluation](#parallel-evaluation)
12. [Compilation & Performance](#compilation--performance)
13. [Built-in Functions](#built-in-functions)
14. [Expression Syntax](#expression-syntax)
15. [Error Handling](#error-handling)

---

## Quick Start

### Rust

```rust
use symb_anafis::{diff, simplify};

// Differentiate
let result = diff("x^3 + sin(x)", "x", &[], None)?;
// Result: "3*x^2 + cos(x)"

// Simplify
let result = simplify("sin(x)^2 + cos(x)^2", &[], None)?;
// Result: "1"
```

### Python

```python
import symb_anafis

# diff() and simplify() now return Expr objects
result = symb_anafis.diff("x^3 + sin(x)", "x")
print(result)  # "3*x^2 + cos(x)"

simplified = symb_anafis.simplify("sin(x)^2 + cos(x)^2")
print(simplified)  # "1"
```

---

## Symbol Management

Symbols are **interned** for O(1) comparison and **Copy** for natural operator usage.

> **Tip:** `Symbol` implements `Copy`, so `a + a` works without `.clone()`!

### Creating Symbols

| Function         | Behavior                       | Use Case              |
| ---------------- | ------------------------------ | --------------------- |
| `symb("x")`      | Get or create - never errors   | General use, parser   |
| `symb_new("x")`  | Create only - errors if exists | Strict control        |
| `symb_get("x")`  | Get only - errors if not found | Retrieve existing     |
| `Symbol::anon()` | Create anonymous symbol        | Temporary computation |

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

### Context: Isolated Environments

For isolated symbol namespaces and unified function registries (useful for testing, avoiding collisions, or multi-tenant systems):

```rust
use symb_anafis::Context;

// Create isolated contexts
let ctx1 = Context::new();
let ctx2 = Context::new();

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

**Context API:**

| Method                         | Description                                       |
| ------------------------------ | ------------------------------------------------- |
| `Context::new()`               | Create new isolated context                       |
| `ctx.symb("x")`                | Get or create isolated symbol                     |
| `ctx.get_symbol("x")`          | Get if exists (returns `Option<Symbol>`)          |
| `ctx.contains_symbol("x")`     | Check if symbol exists                            |
| `ctx.symbol_count()`           | Count symbols in context                          |
| `ctx.symbol_names()`           | List all symbol names                             |
| `ctx.with_symbol("x")`         | Register symbol (builder pattern)                 |
| `ctx.with_symbols(["x", "y"])` | Register multiple symbols (builder pattern)       |
| `ctx.remove_symbol("x")`       | Remove a symbol (returns `bool`)                  |
| `ctx.with_function("f", func)` | Register a user function (builder pattern)        |
| `ctx.with_function_name("f")`  | Register function name only for parser            |
| `ctx.has_function("f")`        | Check if function is registered                   |
| `ctx.function_names()`         | List all function names                           |
| `ctx.get_user_fn("f")`         | Get function definition (`Option<&UserFunction>`) |
| `ctx.is_empty()`               | Check if context has no symbols or functions      |
| `ctx.clear_all()`              | Remove all symbols and functions                  |

---

## Core Functions

### `diff(formula, var, known_symbols, custom_functions)`

Differentiate an expression with respect to a variable.

| Parameter          | Type              | Description                                |
| ------------------ | ----------------- | ------------------------------------------ |
| `formula`          | `&str`            | Expression to differentiate                |
| `var`              | `&str`            | Variable to differentiate with respect to  |
| `known_symbols`    | `&[&str]`         | Known symbols (e.g., multi-char variables) |
| `custom_functions` | `Option<&[&str]>` | User-defined function names                |

```rust
// Treat "alpha" as a single symbol (not a*l*p*h*a)
diff("alpha * x^2", "x", &["alpha"], None)?;
// Result: "2*alpha*x"
```

> [!NOTE]
> **Python API:** `diff()` returns a `PyExpr` object. Membership checks like `"x" in result` should be performed on `str(result)`.

### `simplify(formula, known_symbols, custom_functions)`

Simplify an expression algebraically.

```rust
simplify("x + x + x", &[], None)?;
// Result: "3*x"
```

> [!NOTE]
> **Python API:** `simplify()` returns a `PyExpr` object. Both `diff()` and `simplify()` in Python support duck typing for the `formula` argument (accepts `str` or `Expr`).

### `parse(formula, known_symbols, custom_functions, context)`

Parse a string into an `Expr` AST.

```rust
use symb_anafis::parse;
use std::collections::HashSet;

let expr = parse("x^2 + 1", &HashSet::new(), &HashSet::new(), None)?;
```

---

## Builder Pattern API

For fine-grained control, use `Diff` and `Simplify` builders.

### `Diff` Builder

```rust
use symb_anafis::{Diff, symb, UserFunction};

let result = Diff::new()
    .domain_safe(true)       // Preserve mathematical domains
    .skip_simplification(true) // Return raw derivative (faster, for benchmarks)
    .max_depth(200)          // AST depth limit
    .max_nodes(50000)        // Node count limit
    .with_context(&ctx)      // Use specific symbol context
    .fixed_var(&symb("a"))   // Single constant
    .user_fn("f", UserFunction::new(1..=1))  // Register custom function
    .diff_str("a * f(x)", "x", &[])?;
```

| `with_context(&Context)`      | Sets the symbol context for variable resolution.                                                                            |

> [!TIP]
> **Python API:** `fixed_var` and `fixed_vars` support duck typing. You can pass either strings or `Symbol` objects. `differentiate` also accepts both strings and `Symbol` objects for the variable argument.

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

| `with_context(&Context)`      | Sets the symbol context.                                  |

> [!TIP]
> **Python API:** `fixed_var` and `fixed_vars` in `Simplify` also support duck typing (Strings or `Symbol` objects).

### Type-Safe Expressions

Build expressions programmatically:

```rust
use symb_anafis::{symb, Diff, Expr};

let x = symb("x");

// Symbol is Copy - no clone() needed for operators!
let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
let expr2 = x + x;  // Works! Symbol is Copy.

let derivative = Diff::new().differentiate(&expr, &x)?;
```

### Expression Inspection

Since `Expr` internals are private to enforce invariants, use accessors to inspect properties:

```rust
let expr = symb("x") + symb("y");

// Unique ID for debugging/caching
let id = expr.id();

// Structural hash for fast equality checks
let hash = expr.structural_hash();
```

---

## Expression Output

Format expressions for different output contexts.

### LaTeX Output

```rust
use symb_anafis::symb;

let x = symb("x");
let sigma = symb("sigma");
let expr = x.pow(2.0) / sigma;  // All methods take &self!

println!("{}", expr.to_latex());
// Output: \frac{x^{2}}{\sigma}
```

**LaTeX Features:**
| Expression          | LaTeX Output          |
| ------------------- | --------------------- |
| `a / b`             | `\frac{a}{b}`         |
| `x^n`               | `x^{n}`               |
| `a * b`             | `a \cdot b`           |
| `sin(x)`            | `\sin\left(x\right)`  |
| `sqrt(x)`           | `\sqrt{x}`            |
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
use symb_anafis::{Diff, Expr, UserFunction};
use std::sync::Arc;

let my_func = UserFunction::new(1..=1)
    .partial(0, |args: &[Arc<Expr>]| {
        // ∂f/∂u = 2u
        Expr::number(2.0) * (*args[0]).clone()
    })
    .expect("valid arg");

let diff = Diff::new().user_fn("f", my_func);

diff.diff_str("f(x^2)", "x", &[])?;  // Result: 4x³
```


### Custom Numeric Evaluation

Allow `f(3)` to evaluate to a number:

```rust
use symb_anafis::{Diff, Expr, UserFunction};
use std::sync::Arc;

let my_func = UserFunction::new(1..=1)
    .body(|args: &[Arc<Expr>]| {
        // f(x) = x² + 1
        args[0].pow(2.0) + 1.0
    })
    .partial(0, |args: &[Arc<Expr>]| {
        // ∂f/∂x = 2x
        Expr::number(2.0) * (*args[0]).clone()
    })
    .expect("valid arg");

let diff = Diff::new().user_fn("f", my_func);
```

### Multi-Argument Custom Functions

For functions with 2+ arguments, define **partial derivatives**:

```rust
use symb_anafis::{Diff, UserFunction, Expr};
use std::sync::Arc;

// F(x, y) = x * sin(y)
let my_func = UserFunction::new(2..=2)  // exactly 2 arguments
    .body(|args: &[Arc<Expr>]| {
        // F(x, y) = x * sin(y)
        (*args[0]).clone() * (*args[1]).clone().sin()
    })
    .partial(0, |args: &[Arc<Expr>]| {
        // ∂F/∂x = sin(y)
        (*args[1]).clone().sin()
    })
    .expect("valid arg")
    .partial(1, |args: &[Arc<Expr>]| {
        // ∂F/∂y = x * cos(y)
        (*args[0]).clone() * (*args[1]).clone().cos()
    })
    .expect("valid arg");

let diff = Diff::new().user_fn("F", my_func);
diff.diff_str("F(t, t^2)", "t", &[])?;  // Chain rule applied automatically
```

### Nested Custom Functions

**Yes, custom functions can call other custom functions!**

```rust
use symb_anafis::{Diff, UserFunction, Expr};
use std::sync::Arc;

let f_func = UserFunction::new(1..=1)
    .partial(0, |args: &[Arc<Expr>]| {
        // ∂f/∂u = 2u
        Expr::number(2.0) * (*args[0]).clone()
    })
    .expect("valid arg");

let g_func = UserFunction::new(1..=1)
    .partial(0, |args: &[Arc<Expr>]| {
        // ∂g/∂u = 3u²
        Expr::number(3.0) * (*args[0]).clone().pow(2.0)
    })
    .expect("valid arg");

let diff = Diff::new()
    .user_fn("f", f_func)
    .user_fn("g", g_func);

// f(g(x)) differentiates using chain rule:
// d/dx[f(g(x))] = f'(g(x)) * g'(x)
diff.diff_str("f(g(x))", "x", &[])?;
```

### Helper Trait: `ArcExprExt`

For cleaner syntax in custom function definitions, the `ArcExprExt` trait allows calling mathematical methods directly on `Arc<Expr>` (the type of `args` elements):

```rust
use symb_anafis::ArcExprExt;

// f(x) = x^2 + sin(x)
let f = UserFunction::new(1..=1)
    .body(|args| {
        // Direct method calls on Arc<Expr>:
        args[0].pow(2.0) + args[0].sin()
    });
```

Available methods include basic math (`pow`, `sqrt`), trigonometry (`sin`, `cos`, ...), hyperbolic functions, and special functions.

### Python API

Define custom functions with a body (for correct evaluation) and partial derivatives (for differentiation).

```python
import symb_anafis
from symb_anafis import Diff, Context

def sq_body(args):
    return args[0] ** 2

def sq_partial(args):
    # ∂/∂x (x^2) = 2x
    return 2 * args[0]

# Register with both body (for evaluation) AND partials (for diff)
# Signature: (name, arity, body_callback, partials_list)
diff_builder = Diff().user_fn("sq", 1, sq_body, [sq_partial])

result = diff_builder.diff_str("sq(x)", "x")
# Result: "2*x"

# Register in Context for use in Compilation or Evaluation
ctx = Context().with_function("sq", 1, sq_body, [sq_partial])
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
use symb_anafis::parse;

let expr = parse("x^2 + y", &HashSet::new(), &HashSet::new(), None)?;
let mut vars = HashMap::new();
vars.insert("x", 3.0);

// evaluate takes 2 args: vars and custom_evals (can be empty)
let result = expr.evaluate(&vars, &HashMap::new());  // Returns: 9 + y (Expr)
```

### Evaluate with Custom Functions

Evaluate with custom function implementations:

```rust
use std::sync::Arc;
use std::collections::HashMap;

let mut custom_evals: HashMap<String, Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>> = HashMap::new();
custom_evals.insert("f".to_string(), 
    Arc::new(|args: &[f64]| Some(args[0].powi(2) + 1.0)));

let result = expr.evaluate(&vars, &custom_evals);
```

---

## Vector Calculus

### Gradient

```rust
use symb_anafis::gradient_str;

let grad = gradient_str("x^2 + y^2", &["x", "y"])?;
// grad = ["2*x", "2*y"]
```

### Hessian Matrix

```rust
use symb_anafis::hessian_str;

let hess = hessian_str("x^2 * y", &["x", "y"])?;
// hess = [["2*y", "2*x"], ["2*x", "0"]]
```

### Jacobian Matrix

```rust
use symb_anafis::jacobian_str;

let jac = jacobian_str(&["x^2 + y", "x * y"], &["x", "y"])?;
// jac = [["2*x", "1"], ["y", "x"]]
```

### Type-Safe Versions

```rust
use symb_anafis::{symb, gradient, hessian, jacobian};

let x = symb("x");
let y = symb("y");
let expr = x.pow(2.0) + y.pow(2.0);  // All methods take &self!

let grad = gradient(&expr, &[&x, &y]);  // Vec<Expr>
```

### Python API

```python
from symb_anafis import gradient, hessian, jacobian

# Gradient: [∂f/∂x, ∂f/∂y]
grad = gradient("x^2 + y^2", ["x", "y"])
# grad = ["2*x", "2*y"]

# Hessian: [[∂²f/∂x², ∂²f/∂x∂y], ...]
hess = hessian("x^2 * y", ["x", "y"])

# Jacobian: [[∂f₁/∂x, ∂f₁/∂y], [∂f₂/∂x, ∂f₂/∂y]]
jac = jacobian(["x^2 + y", "x * y"], ["x", "y"])
```

---

## Automatic Differentiation

SymbAnaFis provides dual number arithmetic for exact derivative computation through algebraic manipulation.

### Dual Numbers

Dual numbers extend real numbers with an infinitesimal `ε` where `ε² = 0`. A dual number `a + bε` carries both a value (`a`) and its derivative (`b`).

```rust
use symb_anafis::Dual;

// f(x) = x² + 3x + 1, f'(x) = 2x + 3
let x = Dual::new(2.0, 1.0);  // x + 1ε
let fx = x * x + Dual::new(3.0, 0.0) * x + Dual::new(1.0, 0.0);

println!("f(2) = {}", fx.val);   // 11.0
println!("f'(2) = {}", fx.eps);  // 7.0
```

### Transcendental Functions

All standard mathematical functions work with dual numbers:

```rust
use symb_anafis::Dual;

// f(x) = sin(x) * exp(x), f'(x) = cos(x) * exp(x) + sin(x) * exp(x)
let x = Dual::new(1.0, 1.0);
let fx = x.sin() * x.exp();

println!("f(1) = {:.6}", fx.val);   // 2.287355
println!("f'(1) = {:.6}", fx.eps);  // 3.756049
```

### Chain Rule

Dual numbers automatically apply the chain rule:

```rust
use symb_anafis::Dual;

// f(x) = sin(x² + 1), f'(x) = cos(x² + 1) * 2x
let x = Dual::new(1.5, 1.0);
let inner = x * x + Dual::new(1.0, 0.0);  // x² + 1
let fx = inner.sin();

println!("f(1.5) = {:.6}", fx.val);   // -0.108195
println!("f'(1.5) = {:.6}", fx.eps);  // -2.982389
```

### Comparison with Symbolic Differentiation

```rust
use symb_anafis::{symb, Diff, Dual};

// Symbolic differentiation
let x = symb("x");
let expr = x.pow(3.0) + 2.0 * x.pow(2.0) + x + 1.0;
let symbolic_deriv = Diff::new().differentiate(&expr, &x).unwrap();

// Dual number differentiation
let x_dual = Dual::new(2.0, 1.0);
let fx = x_dual.powi(3) + Dual::new(2.0, 0.0) * x_dual.powi(2) + x_dual + Dual::new(1.0, 0.0);

println!("Symbolic: d/dx(x³ + 2x² + x + 1) = {}", symbolic_deriv);
println!("Dual numbers: f'(2) = {}", fx.eps);  // Both give 21
```

### Python API

```python
import symb_anafis

# f(x) = x² + 3x + 1, f'(x) = 2x + 3
x = symb_anafis.Dual(2.0, 1.0)  # x + 1ε
fx = x * x + symb_anafis.Dual.constant(3.0) * x + symb_anafis.Dual.constant(1.0)

print(f"f(2) = {fx.val}")   # 11.0
print(f"f'(2) = {fx.eps}")  # 7.0

# Transcendental functions
x = symb_anafis.Dual(1.0, 1.0)
fx = x.sin() * x.exp()
print(f"f(1) = {fx.val:.6f}")   # 2.287355
print(f"f'(1) = {fx.eps:.6f}")  # 3.756049

# Chain rule
x = symb_anafis.Dual(1.5, 1.0)
inner = x * x + symb_anafis.Dual.constant(1.0)  # x² + 1
fx = inner.sin()
print(f"f(1.5) = {fx.val:.6f}")   # -0.108195
print(f"f'(1.5) = {fx.eps:.6f}")  # -2.982389
```

### Key Benefits

- **Exact derivatives** (no numerical approximation errors)
- **Handles complex expressions** automatically
- **Chain rule applied algebraically** (no explicit implementation needed)
- **Higher-order derivatives** possible
- **Composable** with other numeric operations

See `examples/dual_autodiff.rs` (Rust) and `examples/dual_autodiff.py` (Python) for comprehensive demonstrations.
### Python Native Support

`PyExpr` and `PySymbol` objects behave like native Python objects:
- **Equality**: `expr1 == expr2` compares structure.
- **Hashing**: Can be used as dictionary keys: `{x: 1, y: 2}`.
- **Numeric**: `float(expr)` works if the expression is a constant number.
- **Strings**: `str(expr)` or `repr(expr)` returns the mathematical representation.

---

## Parallel Evaluation

> Requires `parallel` feature: `symb_anafis = { features = ["parallel"] }`

Evaluate multiple expressions at multiple points in parallel using Rayon.

### `eval_parallel!` Macro

Clean syntax for mixed-type parallel evaluation:

```rust
use symb_anafis::{eval_parallel, symb};
use symb_anafis::parallel::SKIP;

let x = symb("x");
let expr = x.pow(2.0);

// Evaluate at multiple points
let results = eval_parallel!(
    exprs: ["x + y", expr],         // String or Expr inputs
    vars: [["x", "y"], ["x"]],      // Variables per expression
    values: [
        [[1.0, 2.0], [10.0, 20.0]], // expr0: x and y values  
        [[1.0, 2.0, 3.0]]           // expr1: x values
    ]
).unwrap();

// results[0] = String results (since input was string): ["11", "22"]
// results[1] = Expr results (since input was Expr): [Expr(1), Expr(4), Expr(9)]
```

### `SKIP` Constant for Partial Evaluation

Use `SKIP` to keep variables symbolic:

```rust
use symb_anafis::{eval_parallel, symb};
use symb_anafis::parallel::SKIP;

let results = eval_parallel!(
    exprs: ["x * y"],
    vars: [["x", "y"]],
    values: [[[2.0, SKIP], [3.0, 5.0]]]
).unwrap();

// Point 0: x=2, y=3 → "6"
// Point 1: x=SKIP, y=5 → "5*x" (symbolic result)
```

### Return Types: `EvalResult`

Results preserve input type:

```rust
pub enum EvalResult {
    Expr(Expr),     // When input was Expr
    String(String), // When input was string
}
```

### Direct Function: `evaluate_parallel`

For programmatic use without macro:

```rust
use symb_anafis::parallel::{evaluate_parallel, ExprInput, VarInput, Value};

let results = evaluate_parallel(
    vec![ExprInput::from("x^2")],
    vec![vec![VarInput::from("x")]],
    vec![vec![vec![Value::Num(1.0), Value::Num(2.0)]]]
)?;
```

### High-Performance Numeric Evaluation: `eval_f64`

For pure numeric workloads (f64 inputs → f64 outputs), `eval_f64` offers maximum performance using chunked parallel SIMD evaluation. This is the recommended approach for evaluating expressions across large datasets.

> [!TIP]
> `eval_f64` is ideal for scientific computing, data analysis, and any scenario where you need to evaluate expressions at thousands or millions of points.

#### Function Signature

```rust
pub fn eval_f64<V: ToParamName + Sync>(
    exprs: &[&Expr],
    var_names: &[&[V]],
    data: &[&[&[f64]]],
) -> Result<Vec<Vec<f64>>, DiffError>
```

| Parameter   | Type           | Description                                             |
| ----------- | -------------- | ------------------------------------------------------- |
| `exprs`     | `&[&Expr]`     | Slice of expressions to evaluate                        |
| `var_names` | `&[&[V]]`      | Variable names for each expression (strings or symbols) |
| `data`      | `&[&[&[f64]]]` | Columnar data: `data[expr_idx][var_idx][point_idx]`     |

**Returns:** `Vec<Vec<f64>>` where `result[expr_idx][point_idx]` is the evaluation result.

#### Basic Usage

```rust
use symb_anafis::{eval_f64, symb};

let x = symb("x");
let expr = x.pow(2.0);

let x_data = vec![1.0, 2.0, 3.0, 4.0];
// Data layout: [expr_idx][var_idx][column_data]
let results = eval_f64(
    &[&expr],
    &[&["x"]],
    &[&[&x_data[..]]]
).unwrap();
// results[0] = [1.0, 4.0, 9.0, 16.0]
```

#### Multi-Variable Expressions

```rust
use symb_anafis::{eval_f64, parse};
use std::collections::HashSet;

let expr = parse("x^2 + y", &HashSet::new(), &HashSet::new(), None).unwrap();

let x_data = [1.0, 2.0, 3.0];
let y_data = [0.5, 0.5, 0.5];

let results = eval_f64(
    &[&expr],
    &[&["x", "y"]],
    &[&[&x_data[..], &y_data[..]]]
).unwrap();
// results[0] = [1.5, 4.5, 9.5]  (1²+0.5, 2²+0.5, 3²+0.5)
```

#### Multiple Expressions

Evaluate multiple expressions in parallel, each with its own variables and data:

```rust
use symb_anafis::{eval_f64, parse};
use std::collections::HashSet;

let expr1 = parse("x + 1", &HashSet::new(), &HashSet::new(), None).unwrap();
let expr2 = parse("y * 2", &HashSet::new(), &HashSet::new(), None).unwrap();

let x_data = [1.0, 2.0, 3.0];
let y_data = [10.0, 20.0, 30.0];

let results = eval_f64(
    &[&expr1, &expr2],
    &[&["x"], &["y"]],
    &[&[&x_data[..]], &[&y_data[..]]]
).unwrap();

// results[0] = [2.0, 3.0, 4.0]    (x + 1)
// results[1] = [20.0, 40.0, 60.0] (y * 2)
```

#### Performance Characteristics

`eval_f64` is optimized for high-throughput evaluation:

| Feature                         | Benefit                                                      |
| ------------------------------- | ------------------------------------------------------------ |
| **Chunked Parallelism**         | Splits work into 256-point chunks for optimal load balancing |
| **SIMD Vectorization**          | Uses f64x4 SIMD operations (processes 4 values at a time)    |
| **Cache Locality**              | Chunk-based processing improves L1 cache hit rates           |
| **Compiled Bytecode**           | Expressions are compiled to avoid tree traversal overhead    |
| **Parallel Across Expressions** | Multiple expressions evaluated concurrently with Rayon       |

> [!NOTE]
> For small datasets (&lt;256 points), `eval_f64` automatically uses sequential evaluation to avoid thread overhead.

#### Error Handling

```rust
use symb_anafis::{eval_f64, DiffError};

// Mismatched lengths will return an error
let result = eval_f64(&exprs, &var_names, &data);

match result {
    Ok(values) => { /* use values */ }
    Err(DiffError::InvalidSyntax { msg, .. }) => {
        println!("Input error: {}", msg);
    }
    Err(e) => println!("Other error: {:?}", e),
}
```

#### Python API

```python
import symb_anafis
from symb_anafis import parse, eval_f64

# Single expression
expr = parse("x^2 + sin(x)")
x_data = [0.0, 1.0, 2.0, 3.0]
results = eval_f64([expr], [["x"]], [[[x_data]]])
# results[0] = [0.0, 1.8414..., 4.9092..., 9.1411...]

# Multiple expressions with different variables
expr1 = parse("x + y")
expr2 = parse("x * y")
x_data = [1.0, 2.0, 3.0]
y_data = [10.0, 20.0, 30.0]

results = eval_f64(
    [expr1, expr2],
    [["x", "y"], ["x", "y"]],
    [[[x_data, y_data]], [[x_data, y_data]]]
)
# results[0] = [11.0, 22.0, 33.0]  (x + y)
# results[1] = [10.0, 40.0, 90.0]  (x * y)
```

> [!IMPORTANT]
> The Python binding releases the GIL during evaluation, allowing true parallel execution in multi-threaded Python programs.

---

## Compilation & Performance

For tight loops or massive repeated evaluation, use the `CompiledEvaluator`. It compiles the expression tree into a flat bytecode that is interpreted efficiently, avoiding tree traversal overhead and enabling SIMD optimizations.

### Common Subexpression Elimination (CSE)

The compiler automatically detects and caches repeated subexpressions using 64-bit structural hashing. This provides up to **28% faster evaluation** for expressions with repeated patterns.

```rust
// Expression: sin(x)² + sin(x) - repeated subexpression sin(x)
let expr = x.sin().pow(2.0) + x.sin();
let compiled = expr.compile()?;

// CSE automatically:
// 1. Detects sin(x) appears twice (same structural hash)
// 2. Generates: StoreCached(0) after first sin(x)
// 3. Uses: LoadCached(0) for second occurrence
// Result: sin(x) computed once, reused from cache
```

| CSE Bytecode Instructions | Description                           |
| ------------------------- | ------------------------------------- |
| `StoreCached(slot)`       | Pop value from stack, store in cache  |
| `LoadCached(slot)`        | Push cached value onto stack          |

> [!NOTE]
> CSE is automatic and requires no configuration. The cache is pre-allocated based on detected subexpressions during compilation.

### `CompiledEvaluator`

```rust
use symb_anafis::{Expr, symb};

// Create an expression
let expr = symb("x").sin() * symb("x").pow(2.0);

// Compile with auto-detected variables
let compiled = expr.compile()?;

// Or compile with explicit parameter order (strings or symbols)
let compiled = expr.compile_with_params(&["x"])?;
let compiled = expr.compile_with_params(&[&x])?;  // Also works with symbols!

// Evaluate repeatedly (much faster than expr.evaluate)
let result = compiled.evaluate(&[0.5]); // Result at x=0.5
```

**Methods:**

| Method                                                | Description                                       |
| ----------------------------------------------------- | ------------------------------------------------- |
| `CompiledEvaluator::compile(&expr, &params, context)` | Compile with explicit params (strings or symbols) |
| `CompiledEvaluator::compile_auto(&expr, context)`     | Compile, auto-detecting variables                 |
| `expr.compile()`                                      | Convenience method, auto-detects variables        |
| `expr.compile_with_params(&params)`                   | Convenience method with explicit params           |
| `evaluate(&values)`                                   | Evaluate at a single point                        |
| `eval_batch(&columns, &mut output)`                   | Batch evaluate (SIMD optimized)                   |

### Using Symbols or Strings

You can pass either strings or symbols to `compile`:

```rust
use symb_anafis::{symb, CompiledEvaluator};

let x = symb("x");
let y = symb("y");
let expr = x.pow(2.0) + y;

// Using strings
let c1 = CompiledEvaluator::compile(&expr, &["x", "y"], None)?;

// Using symbols (preferred - faster lookup)
let c2 = CompiledEvaluator::compile(&expr, &[&x, &y], None)?;
```

### Using `Context` with Compiler

To use custom functions within compiled expressions, register them in a `Context` with a body definition.

```rust
use symb_anafis::{Context, UserFunction, CompiledEvaluator, Expr, symb};

// 1. Create a context with a function body
let ctx = Context::new()
    .with_function("my_sq", UserFunction::new(1..=1)
        .body(|args| args[0].pow(2.0))); 

// 2. Create expression using the function
let x = symb("x");
let expr = Expr::func("my_sq", x.to_expr());

// 3. Compile with context (using symbols)
let compiled = CompiledEvaluator::compile(&expr, &[&x], Some(&ctx))?;

let result = compiled.evaluate(&[3.0]); // 9.0
```

### Python API

Python bindings provide a high-performance `CompiledEvaluator` class that releases the GIL during heavy computations, enabling true parallelism.

```python
from symb_anafis import CompiledEvaluator, Context, parse

# 1. Basic Compilation
expr = parse("x^2 + sin(x)")
# compile(expr, params_list)
compiled = CompiledEvaluator(expr, ["x"])

val = 2.0
result = compiled.evaluate([val])
# Batch evaluation (SIMD optimized)
results = compiled.eval_batch([[0.0, 1.0, 2.0]])

# 2. Compilation with Custom Functions
def my_sq(args): return args[0]**2

ctx = Context().with_function("my_sq", 1, my_sq, [])
expr_custom = parse("my_sq(x) + 5", custom_functions=["my_sq"])

# Pass context to constructor
compiled_ctx = CompiledEvaluator(expr_custom, ["x"], ctx)
res = compiled_ctx.evaluate([3.0])  # 14.0
```

---

## Built-in Functions

| Category                   | Functions                                                                     |
| -------------------------- | ----------------------------------------------------------------------------- |
| **Trig**                   | `sin`, `cos`, `tan`, `cot`, `sec`, `csc`                                      |
| **Inverse Trig**           | `asin`, `acos`, `atan`, `atan2`, `acot`, `asec`, `acsc`                       |
| **Hyperbolic**             | `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`                                |
| **Inverse Hyperbolic**     | `asinh`, `acosh`, `atanh`, `acoth`, `asech`, `acsch`                          |
| **Exp/Log**                | `exp`, `ln`, `log(b, x)`, `log10`, `log2`, `exp_polar`                        |
| **Roots**                  | `sqrt`, `cbrt`                                                                |
| **Error Functions**        | `erf`, `erfc`                                                                 |
| **Gamma Family**           | `gamma`, `digamma`, `trigamma`, `tetragamma`, `polygamma(n, x)`, `beta(a, b)` |
| **Zeta**                   | `zeta`, `zeta_deriv(n, s)`                                                    |
| **Bessel**                 | `besselj(n, x)`, `bessely(n, x)`, `besseli(n, x)`, `besselk(n, x)`            |
| **Elliptic Integrals**     | `elliptic_k`, `elliptic_e`                                                    |
| **Orthogonal Polynomials** | `hermite(n, x)`, `assoc_legendre(l, m, x)`                                    |
| **Spherical Harmonics**    | `spherical_harmonic(l, m, θ, φ)`, `ynm(l, m, θ, φ)`                           |
| **Other**                  | `abs`, `signum`, `sinc`, `lambertw`, `floor`, `ceil`, `round`                 |

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

| Element            | Syntax                     | Example                |
| ------------------ | -------------------------- | ---------------------- |
| Variables          | Any identifier             | `x`, `y`, `sigma`      |
| Numbers            | Integer/decimal/scientific | `1`, `3.14`, `1e-5`    |
| Addition           | `+`                        | `x + 1`                |
| Subtraction        | `-`                        | `x - 1`                |
| Multiplication     | `*`                        | `x * y`                |
| Division           | `/`                        | `x / y`                |
| Power              | `^`                        | `x^2`                  |
| Function calls     | `name(args)`               | `sin(x)`, `log(10, x)` |
| Constants          | `pi`, `e`                  | Auto-recognized        |
| Implicit mult      | Adjacent terms             | `2x`, `(x+1)(x-1)`     |
| Partial derivative | `∂_f(x)/∂_x`               | Output notation        |

### Operator Precedence

| Precedence | Operators   | Associativity |
| ---------- | ----------- | ------------- |
| Highest    | `^` (power) | Right         |
|            | `*`, `/`    | Left          |
| Lowest     | `+`, `-`    | Left          |

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

**`DiffError` Variants:**

| Variant                                            | Description                                       |
| -------------------------------------------------- | ------------------------------------------------- |
| **Input Errors**                                   |                                                   |
| `EmptyFormula`                                     | Input was empty or whitespace only                |
| `InvalidSyntax { msg, span }`                      | General syntax error                              |
| `InvalidNumber { value, span }`                    | Could not parse number literal                    |
| `InvalidToken { token, span }`                     | Unrecognized token                                |
| `UnexpectedToken { expected, got, span }`          | Wrong token at position                           |
| `UnexpectedEndOfInput`                             | Input ended prematurely                           |
| `InvalidFunctionCall { name, expected, got }`      | Wrong number of arguments                         |
| **Semantic Errors**                                |                                                   |
| `VariableInBothFixedAndDiff { var }`               | Variable is both fixed and differentiation target |
| `NameCollision { name }`                           | Name used for both variable and function          |
| `UnsupportedOperation(String)`                     | Operation not supported                           |
| `AmbiguousSequence { sequence, suggestion, span }` | Ambiguous token sequence                          |
| **Safety Limits**                                  |                                                   |
| `MaxDepthExceeded`                                 | Expression exceeds max AST depth                  |
| `MaxNodesExceeded`                                 | Expression exceeds max node count                 |
| **Compilation Errors**                             |                                                   |
| `UnsupportedExpression(String)`                    | Unsupported construct for compilation             |
| `UnsupportedFunction(String)`                      | Function not supported in compiled mode           |
| `UnboundVariable(String)`                          | Variable not in parameter list                    |
| `StackOverflow { depth, limit }`                   | Expression requires too much stack                |
| **Batch Evaluation Errors**                        |                                                   |
| `EvalColumnMismatch { expected, got }`             | Column count doesn't match params                 |
| `EvalColumnLengthMismatch`                         | Column lengths differ                             |
| `EvalOutputTooSmall { needed, got }`               | Output buffer too small                           |
| **UserFunction Errors**                            |                                                   |
| `InvalidPartialIndex { index, max_arity }`         | Partial derivative index out of bounds            |

---