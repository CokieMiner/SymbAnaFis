# SymbAnaFis — API

Public API reference for the active `symb_anafis` modules.

The examples use the version in this repository. Each Rust block is an independent executable program. Public types are imported directly from `symb_anafis`.

## Core

The core constructs canonical expressions, manages symbol and function identities, analyzes scalar properties, and applies substitutions and rewrite rules. [Parsing](#parsing) constructs these expressions from text. Symbolic differentiation uses `Expr::derivative`; [simplification](#simplification) uses reusable sequences of passes. [Numerical evaluation](#numerical-evaluation) supports direct evaluation and reusable scalar register programs.

### Contents

1. [Configuration and features](#1-configuration-and-features)
2. [Construction and operators](#2-construction-and-operators)
3. [Automatic normalization](#3-automatic-normalization)
4. [Numbers and constants](#4-numbers-and-constants)
5. [Symbols and contexts](#5-symbols-and-contexts)
6. [Mathematical functions](#6-mathematical-functions)
7. [User functions and partial derivatives](#7-user-functions-and-partial-derivatives)
8. [Expression inspection](#8-expression-inspection)
9. [DAG queries](#9-dag-queries)
10. [Properties and domain proofs](#10-properties-and-domain-proofs)
11. [Concrete substitution](#11-concrete-substitution)
12. [Simultaneous substitutions](#12-simultaneous-substitutions)
13. [Patterns and rewrite rules](#13-patterns-and-rewrite-rules)
14. [Predicates, guards, and search limits](#14-predicates-guards-and-search-limits)
15. [Callback replacements](#15-callback-replacements)
16. [Binomial expansion with a variable order](#16-binomial-expansion-with-a-variable-order)
17. [DAG transformation and folding](#17-dag-transformation-and-folding)
18. [Symbolic differentiation](#18-symbolic-differentiation)
19. [Formatting](#19-formatting)
20. [Errors, memory, and limits](#20-errors-memory-and-limits)

### 1. Configuration and features

To use this checkout in a local application:

```toml
[dependencies]
symb_anafis = { path = "../SymbAnaFis" }
```

| Feature | Behavior |
| --- | --- |
| No features | `no_std + alloc`; dynamically allocated collections and internal spin mutexes. |
| `std` | Enables `num_anafis/std` and blocking mutexes with recovery after poisoning. |
| `num-traits` | Implements `num_traits::Zero` and `num_traits::One` for `Expr`. |

The path is relative to the application's `Cargo.toml`. For an application using the standard library:

```toml
symb_anafis = { path = "../SymbAnaFis", features = ["std"] }
```

The `no_std` build requires a global allocator and pointer-width atomics, used by `Arc` and identity sequences. In the examples, `std::sync::Arc` can be replaced with `alloc::sync::Arc`; `Vec` and `vec!` come from `alloc` in those environments.

The `num-traits` feature is independent of `std`. The following example also requires a direct `num-traits` dependency in the application:

```toml
symb_anafis = { path = "../SymbAnaFis", features = ["num-traits"] }
num-traits = { version = "0.2", default-features = false }
```

```rust
use num_traits::{One, Zero};
use symb_anafis::Expr;

fn main() {
    let zero = Expr::zero();
    let one = Expr::one();
    assert!(zero.is_zero());
    assert!(one.is_one());
}
```

### 2. Construction and operators

`Expr` is an owning handle to an immutable node. Structurally identical expressions that are alive at the same time share one canonical node and its payloads, including expressions constructed by separate operations.

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;

    let expression = x.pow(2) + 2 * x * y + y.pow(2);
    let same = y.pow(2) + x * y * 2 + x.pow(2);
    assert_eq!(expression, same);

    let terms = vec![x.pow(2), 2 * x * y, y.pow(2)];
    assert_eq!(Expr::sum(terms), expression);
    assert_eq!(Expr::product(vec![x.into(), y.into()]), x * y);
    Ok(())
}
```

| Operation | Syntax |
| --- | --- |
| Literal | `Expr::number(3)`, `Expr::number(0.5)` |
| Local symbol | `ctx.symb("x")?` |
| Global symbol as an expression | `Expr::symbol("x")` |
| Sum, difference, product, quotient | `x + y`, `x - y`, `x * y`, `x / y` |
| Negation | `-x`, `-&expression` |
| Power | `x.pow(3)`, `expression.pow(exponent)` |
| Dynamically sized sum or product | `Expr::sum(terms)`, `Expr::product(factors)` |
| Empty identities | `Expr::sum(vec![])` is `0`; `Expr::product(vec![])` is `1`. |

Rust has no `**` operator; `^` does not represent exponentiation in this API.

`Symbol` is `Copy`: reusing `x` does not require `clone()`. `Expr` owns its node: use references in arithmetic, or `clone()` when retaining a handle passed by value. Cloning an `Expr` shares the node; it does not copy the tree.

```rust
use symb_anafis::Context;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let shared = x.sin() + 1;
    let expression = &shared * &shared;
    assert_eq!(expression, shared.pow(2));
    Ok(())
}
```

### 3. Automatic normalization

Operators and vector constructors use the same normalization kernels. Construction performs:

- Flattening and canonical ordering of sums and products.
- Arithmetic on finite constants and removal of exact neutral elements.
- Collection of equal terms, such as \(x^3+2x^3=3x^3\).
- Collection of positive integer powers, such as \(x^2x^3=x^5\).
- Collection of nested positive integer powers, such as \((x^2)^3=x^6\), when the combined exponent fits.
- Exact scalar division represented by multiplicative coefficients, such as \(x/2=(1/2)x\).
- Numeric coefficients outside shared symbolic quotients, such as \((2x)/(3y)=(2/3)(x/y)\).
- Representation of known negative real exponents through denominators.
- Private polynomial compaction of groups with repeated positive integer powers.

```rust
use symb_anafis::{Context, Expr, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let half = Number::from_int(1) / Number::from_int(2);

    assert_eq!(x.pow(3) + 2 * x.pow(3), 3 * x.pow(3));
    assert_eq!(x.pow(2) * x.pow(3), x.pow(5));
    assert_eq!(x.pow(2).pow(3), x.pow(6));
    assert_eq!(x + 0, x.to_expr());
    assert_eq!(x.pow(-2), Expr::number(1) / x.pow(2));
    assert_eq!(x / 2, Expr::number(half.clone()) * x);
    assert_eq!(x / (2 * y), (Expr::number(half.clone()) * x) / y);
    let quotient = x / y;
    let two_thirds = Number::from_int(2) / Number::from_int(3);
    assert_eq!((2 * x) / (3 * y), Expr::number(two_thirds) * &quotient);
    assert_eq!(2 * &quotient + 3 * &quotient, 5 * &quotient);
    assert_eq!(x.pow(half), x.sqrt());
    assert_eq!(Expr::e().pow(x), x.exp());
    assert_eq!(x.log(2), x.log2());
    assert_eq!((x * y).pow(2), x.pow(2) * y.pow(2));

    assert_ne!(x / x, Expr::number(1));
    assert_ne!((x + y).pow(2), x.pow(2) + 2 * x * y + y.pow(2));
    Ok(())
}
```

The chosen form of exact \(x^{1/2}\) is `sqrt(x)`. `exp(x)` represents \(e^x\); `log(e, x)`, `log(2, x)`, and `log(10, x)` use `ln`, `log2`, and `log10`. `exp_neg(x)` uses `exp(-x)`. A float exponent retains its approximate representation.

Normalization preserves domain exclusions. In general, \(x/x\), \(x^0\), and \(0f(x)\) are retained without a sufficient proof. Construction also does not expand \((x+y)^n\) automatically or apply arbitrary trigonometric identities.

Polynomial storage is private. A polynomial remains an expression inspected as a sum; constant terms belong to the outer sum. Users do not choose between `Sum` and `Poly`.

Exact quotients use \((aA)/(bB)=(a/b)(A/B)\), with the numeric coefficient outside the shared symbolic quotient. This includes rational and complex scalars, chained divisions, and coefficients within denominator products when the divisor is nonzero and the coefficient can be computed within backend capacity. Different numeric multiples share the same quotient node. Unit coefficients and unit denominators are omitted.

Only direct multiplicative coefficients are extracted. Numeric literals remain inside sums, exponents, and function arguments, and reciprocals may retain the numerator one. Symbolic denominator factors remain explicit. This normalization does not introduce float reciprocals. Zero denominators, approximate division, and unrepresentable exact coefficient quotients remain explicit; finite literal arithmetic still folds normally.

`Eq` and `Hash` for `Expr` use canonical identity in constant time. Structural equality is not a general test of mathematical equivalence: forms requiring expansion or an explicit identity may differ.

### 4. Numbers and constants

Literals use `num_anafis::Number`. `Number`, `Real`, `Complex`, and the `Numeric` trait are available through the `symb_anafis` facade.

| Value | Construction |
| --- | --- |
| Exact integer | `Expr::number(2)` |
| Exact rational | `Expr::number(Number::from_int(1) / Number::from_int(4))` |
| Approximate float | `Expr::number(0.25)` |
| Scalar complex number | `Expr::number(Number::from_complex(re, im))` |
| Exact imaginary unit | `Expr::i()` |
| Mathematical constant \(\pi\) | `Expr::pi()` |
| Mathematical constant \(e\) | `Expr::e()` |

`i`, `pi`, and `e` are dedicated constants, independent of symbol names. Scalar complex multiplication is commutative; there is no public Clifford algebra support.

```rust
use symb_anafis::{Expr, ExprView, Number};

fn main() {
    let quarter = Number::from_int(1) / Number::from_int(4);
    let half = Number::from_int(1) / Number::from_int(2);
    let z = Number::from_complex(Number::from_int(3), Number::from_int(4));

    assert_eq!(Expr::number(quarter).sqrt(), Expr::number(half));
    assert_eq!(Expr::number(z).abs(), Expr::number(5));
    assert_eq!(Expr::i() * Expr::i(), Expr::number(-1));
    assert!(matches!(Expr::pi().view(), ExprView::Pi));
    assert_ne!(Expr::pi(), Expr::symbol("pi"));
    assert_ne!(Expr::number(2), Expr::number(2.0));
    assert_ne!(Expr::number(0.0), Expr::number(-0.0));
}
```

Literal identity preserves representation: integers and floats remain distinct, as do float zeros with different signs and NaNs with different payloads. This differs from the numeric equality provided by the backend.

#### Evaluation during construction

Exact inputs are evaluated only when the core recognizes a supported exact result. Approximate inputs are evaluated when the backend produces a finite result.

```rust
use symb_anafis::{Expr, ExprView};

fn main() {
    assert_eq!(Expr::number(4).sqrt(), Expr::number(2));
    assert_eq!(Expr::number(0).sin(), Expr::number(0));
    assert_eq!(Expr::number(4.0).sqrt(), Expr::number(2.0));
    assert!(matches!(Expr::number(1).sin().view(), ExprView::Function { .. }));
    assert!(Expr::number(1.0).sin().as_number().is_some());
    assert!(matches!(Expr::number(0).ln().view(), ExprView::Function { .. }));
}
```

Thus, integer `sin(1)` stays symbolic; `sin(1.0)` can be approximated immediately. Perfect rational roots, including recognized exact complex results, can be reduced. The absence of folding does not prove that an exact result is impossible.

`as_number()` returns `Option<&Number>`. `is_zero_num()`, `is_one_num()`, and `is_neg_one_num()` query numeric identities of literal nodes. The dedicated `Expr::i()` node is inspected through `ExprView::ImaginaryUnit`.

### 5. Symbols and contexts

#### Local identities

`Context` is a namespace for symbols and user functions. The same name resolves to the same identity while its binding exists in that context. Independent contexts can contain distinct symbols with the same name.

```rust
use symb_anafis::Context;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let first = Context::new();
    let second = Context::new();
    let x = first.symb("x")?;

    assert_eq!(first.symb("x")?, x);
    assert_ne!(second.symb("x")?, x);
    assert_eq!(first.get_symbol("x"), Some(x));
    assert_eq!(first.symbol_from_id(x.id())?, x);
    assert_eq!(x.name().as_deref(), Some("x"));

    assert!(first.remove_symbol("x"));
    assert_eq!(x + 1, x.to_expr() + 1);
    assert_ne!(first.symb("x")?, x);
    Ok(())
}
```

`Symbol<'owner>` is `Copy` and cannot outlive its owner. `to_expr()` creates an owning handle that can outlive the context:

```rust
use symb_anafis::Context;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let expression = {
        let ctx = Context::new();
        let x = ctx.symb("x")?;
        x.sin() + x.pow(2)
    };
    let x = expression.symbols().into_iter()
        .find(|symbol| symbol.name().as_deref() == Some("x"))
        .ok_or("the expression must contain x")?;
    assert!(expression.contains_symbol(x));
    Ok(())
}
```

| `Context` method | Behavior |
| --- | --- |
| `new()`, `default()` | Creates an independent namespace. |
| `clone()` | Shares the namespace and its bindings. |
| `id()` | Returns the namespace identity. |
| `symb(name)` | Gets or creates a local symbol; returns `Result`. |
| `get_symbol(name)` | Queries without creating; returns `Option`. |
| `symbol_from_id(id)` | Validates that the identity belongs to this namespace. |
| `contains_symbol(name)` | Tests a name binding. |
| `symbol_names()`, `symbol_count()` | Lists sorted names or counts local bindings. |
| `remove_symbol(name)`, `clear_symbols()` | Removes bindings; retains issued identities. |
| `clear_functions()`, `clear_all()` | Removes function bindings, or both kinds of binding. |
| `is_empty()` | Tests for the absence of symbol and function bindings. |
| `reset(&mut self)` | Releases this owner's namespace and creates another. |

Removing names does not release metadata for issued symbols while the namespace owns it. Drop the context or use `reset()` to release that ownership. Other clones retain the previous namespace; surviving expressions retain the metadata they reference. Rust prevents `reset()` when a borrowed symbol will still be used.

```rust
use symb_anafis::Context;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let mut ctx = Context::new();
    let expression = ctx.symb("x")? + 1;
    let previous_id = ctx.id();
    ctx.reset();

    assert_ne!(ctx.id(), previous_id);
    assert!(ctx.is_empty());
    assert!(expression.contains_var("x"));
    Ok(())
}
```

#### Global symbols

| Operation | API |
| --- | --- |
| Get or create | `Symbol::intern(name)` |
| Create with a required free name | `Symbol::register(name) -> Result` |
| Query without creating | `Symbol::get(name) -> Result` |
| Create an anonymous identity | `Symbol::anon()` |
| Query a global identity | `Symbol::from_id(id) -> Result` |
| Query names | `Symbol::exists(name)`, `Symbol::names()`, `Symbol::count()` |
| Remove a binding by name | `Symbol::remove(name)` |
| Remove the handle's own binding | `symbol.unregister()` |
| Read metadata | `symbol.name()`, `symbol.id()` |
| Get an owning expression | `symbol.to_expr()` |

`Expr::symbol(name)` directly creates or obtains a global symbol expression. Global identities have permanent metadata to keep `Copy` handles valid. Removing a binding does not reclaim that metadata; rebinding allocates another identity. There is no bulk global clear. Use local contexts for temporary sets of symbols.

`Symbol::from_id` rejects local identities. `Context::symbol_from_id` rejects global IDs and IDs from other namespaces. A numeric ID cannot bypass the owner's lifetime boundary.

### 6. Mathematical functions

The following methods exist on `Expr`, `Symbol`, and `Pattern`. Pattern arguments can contain captures. Expression calls use the folding and canonical forms described above.

| Family | Methods |
| --- | --- |
| Trigonometric | `sin()`, `cos()`, `tan()`, `cot()`, `sec()`, `csc()` |
| Inverse trigonometric | `asin()`, `acos()`, `atan()`, `acot()`, `asec()`, `acsc()`, `atan2(x)` |
| Hyperbolic | `sinh()`, `cosh()`, `tanh()`, `coth()`, `sech()`, `csch()` |
| Inverse hyperbolic | `asinh()`, `acosh()`, `atanh()`, `acoth()`, `asech()`, `acsch()` |
| Exponential | `exp()`, `exp_m1()`, `exp_neg()` |
| Logarithmic | `ln()`, `ln_1p()`, `log(base)`, `log2()`, `log10()` |
| Roots | `sqrt()`, `cbrt()` |
| Error | `erf()`, `erfc()` |
| Gamma | `gamma()`, `lgamma()`, `digamma()`, `trigamma()`, `tetragamma()`, `polygamma(order)`, `beta(other)` |
| Zeta | `zeta()`, `zeta_deriv(order)` |
| Bessel | `besselj(order)`, `bessely(order)`, `besseli(order)`, `besselk(order)` |
| Elliptic integrals | `elliptic_k()`, `elliptic_e()` |
| Orthogonal polynomials | `hermite(degree)`, `assoc_legendre(degree, order)` |
| Spherical harmonics | `spherical_harmonic(degree, order, phi)` |
| Lambert W | `lambertw(branch)` |
| Magnitude and sign | `abs()`, `signum()`, `sinc()` |
| Rounding | `floor()`, `ceil()`, `round()`, `trunc()`, `fract()` |

The receiver follows these conventions:

| Rust syntax | Mathematical call |
| --- | --- |
| `x.log(b)` | \(\log_b(x)\) |
| `y.atan2(x)` | \(\operatorname{atan2}(y,x)\) |
| `x.besselj(n)` | \(J_n(x)\) |
| `x.polygamma(n)` | \(\psi^{(n)}(x)\) |
| `x.zeta_deriv(n)` | \(\zeta^{(n)}(x)\) |
| `x.hermite(n)` | \(H_n(x)\) |
| `x.assoc_legendre(l,m)` | \(P_l^m(x)\) |
| `theta.spherical_harmonic(l,m,phi)` | \(Y_l^m(\theta,\phi)\) |
| `z.lambertw(k)` | \(W_k(z)\) |

`lambertw` takes the branch explicitly; there are no separate branch methods or public `ynm` alias. Numerical evaluation and projections of discrete parameters follow the NumAnaFis backend; symbolic construction does not validate every mathematical domain.

#### Known function identities

`Function::builtin(name)` resolves only recognized names and aliases. It does not create unknown functions. `Function::call` validates the argument count and constructs a normalized expression.

```rust
use symb_anafis::{Context, Function, FunctionError};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let sin = Function::builtin("sin").ok_or("builtin sin is missing")?;

    assert_eq!(sin.arity(), 1..=1);
    assert_eq!(sin.as_str(), "sin");
    assert_eq!(sin.call(vec![x.into()])?, x.sin());
    assert!(matches!(sin.call(vec![]), Err(FunctionError::InvalidArity { .. })));
    assert!(Function::builtin("unknown_function").is_none());
    assert_eq!(Function::builtin("bessel_j"), Function::builtin("besselj"));
    Ok(())
}
```

Fluent methods pass the builtin identity directly. The internal enum and dispatch tables are not public API. Custom calls require a registered handle; there is no `Expr::call("name", ...)` or `Expr::func("name", ...)`.

### 7. User functions and partial derivatives

`UserFunction` describes an arity range, an optional body, and optional partial derivatives indexed by argument. `Context::register_function` turns that definition into an immutable identity.

The example defines \(q(x,y)=x^2+y^2\), with \(\partial q/\partial x=2x\) and \(\partial q/\partial y=2y\):

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, ExprView, FunctionError, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let definition = UserFunction::new(2..=2)
        .body(Arc::new(|args| Expr::sum(args.iter().map(|arg| arg.pow(2)).collect())))
        .partial(0, Arc::new(|args| 2 * &args[0]))?
        .partial(1, Arc::new(|args| 2 * &args[1]))?;
    let q = ctx.register_function("q", definition)?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let args = vec![x.into(), y.into()];
    let call = q.call(args.clone())?;

    assert!(matches!(call.view(), ExprView::Function { .. }));
    let definition = q.definition().ok_or("custom definition is missing")?;
    let body = definition.body_fn().ok_or("body is missing")?;
    let dx = definition.partial_fn(0).ok_or("partial derivative is missing")?;
    assert_eq!(body(&args), x.pow(2) + y.pow(2));
    assert_eq!(dx(&args), 2 * x);

    assert!(matches!(
        ctx.register_function("q", UserFunction::new(1..=1)),
        Err(FunctionError::DuplicateName(_))
    ));
    assert!(ctx.remove_function("q"));
    let replacement = ctx.register_function("q", UserFunction::new(1..=1))?;
    assert_ne!(q, replacement);
    assert_eq!(q.arity(), 2..=2);
    Ok(())
}
```

Constructing `q.call(...)` retains a symbolic call; it does not execute the body or calculate derivatives. Callbacks are accessible through the definition for explicit use or by engines built on the core.

Each callback has the contract `Fn(&[Expr]) -> Expr + Send + Sync`, shared through `Arc`. Its input is the complete argument list, including for partial derivatives. A caller invoking a callback obtained directly from `body_fn()` or `partial_fn()` must supply a valid arity; the indexed accesses above require the two arguments of `q`.

| Operation | API |
| --- | --- |
| Prepare a definition | `UserFunction::new(minimum..=maximum)` |
| Accept any arity | `UserFunction::default()` |
| Set the body | `definition.body(Arc::new(callback))` |
| Set a partial derivative | `definition.partial(index, Arc::new(callback))?` |
| Inspect a definition | `arity()`, `has_body()`, `has_partial(index)` |
| Share callbacks | `body_fn()`, `partial_fn(index)` |
| Register | `ctx.register_function(name, definition)?` |
| Query local registrations | `ctx.get_function(name)`, `ctx.has_function(name)` |
| List functions by name | `ctx.functions()` |
| Query whether bodies exist | `ctx.has_expandable_functions()` |
| Remove a binding | `ctx.remove_function(name)` |
| Read a registered definition | `function.definition()`; `None` for builtins. |

`partial` replaces a callback with the same index in the draft definition. This does not alter already registered functions. An index must exist in at least one accepted arity; for a variable-arity call, only a partial whose argument is present may be used.

Duplicate custom names in one context are errors, including builtin names and aliases. After removing a binding, the name may be registered with a new identity and arity. Existing handles and calls retain the previous definition, including after context destruction.

The core does not prove mathematical consistency between bodies and partial derivatives. That responsibility belongs to the definition's author.

### 8. Expression inspection

`Expr::view()` returns a borrowed view. Internal nodes cannot be constructed or modified through that view.

| `ExprView` variant | Contents |
| --- | --- |
| `Number(value)` | Reference to `Number`. |
| `ImaginaryUnit`, `Pi`, `Euler` | Dedicated mathematical constants. |
| `Symbol(symbol)` | Identity borrowed from the expression. |
| `Function { name, args }` | Function handle and ordered arguments. |
| `Sum(terms)` | Logical term iterator, including compacted monomials. |
| `Product(factors)` | Slice of canonical factors. |
| `Div(numerator, denominator)` | Numerator and denominator. |
| `Pow(base, exponent)` | Base and exponent. |
| `Derivative { inner, var, order }` | Unevaluated derivative. |

`ExprView` is non-exhaustive: external `match` expressions include a fallback arm. A polynomial's view is `Sum`, without a public `Poly` variant.

```rust
use symb_anafis::{Context, Expr, ExprView};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = x.pow(2) + x.pow(3) + 1;
    match expression.view() {
        ExprView::Sum(terms) => {
            assert_eq!(terms.len(), 3);
            assert_eq!(Expr::sum(terms.collect()), expression);
        }
        _ => return Err("expected a sum".into()),
    }

    let children = expression.children();
    assert_eq!(children.len(), 3);
    assert_eq!(Expr::sum(children.collect()), expression);
    assert_eq!(x.to_expr().as_symbol(), Some(x));
    assert_eq!(x.to_expr().symbol_name(), Some("x"));
    Ok(())
}
```

`children()` returns `ExprChildren`, a cloneable, exact-size, fused iterator. Returned children are owning `Expr` values. Counting does not materialize monomials; iteration may reconstruct them through interning.

A power's children are its base and exponent. A derivative's children include only the inner expression; its differentiation variable is metadata available through the view and symbol queries.

### 9. DAG queries

| `Expr` method | Result |
| --- | --- |
| `node_count()` | Number of distinct reachable DAG nodes, including the root; excludes payload buffers and registry allocations. |
| `max_depth()` | Longest stored path; the root has depth 1. |
| `symbols()` | Set of identities borrowed from the expression. |
| `variables()` | Set of names; identities with equal names are merged. |
| `variables_ordered()` | Unique names in lexicographic order. |
| `contains_symbol(symbol)` | Searches by identity, including a derivative's variable. |
| `contains_var(name)` | Searches by name across contexts. |
| `has_free_symbols(&excluded)` | Is there an identity outside the exclusion set? |
| `has_free_variables(&excluded)` | Is there a name outside the exclusion set? |

Anonymous symbols appear as `$id` in name collections. The `has_free_*` queries use caller-provided exclusions; they do not implement variable binding for integrals or other operators.

```rust
use symb_anafis::{Context, HashSet};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let first = Context::new();
    let second = Context::new();
    let x = first.symb("x")?;
    let other_x = second.symb("x")?;
    let expression = x + other_x;

    assert_eq!(expression.symbols().len(), 2);
    assert_eq!(expression.variables_ordered(), vec!["x"]);
    assert!(expression.contains_symbol(x));
    assert!(expression.has_free_symbols(&HashSet::from([x])));
    assert!(!expression.has_free_symbols(&HashSet::from([x, other_x])));
    assert!(expression.node_count() >= 3);
    Ok(())
}
```

Public `HashMap` and `HashSet` are the `hashbrown` types used by the core. Set iteration order is not a sorting contract. Node counts and depths describe storage, which can compact sums; they are not sizes of an expanded tree.

### 10. Properties and domain proofs

`expression.properties()` analyzes the DAG once and returns `Properties`. Each subsequent `get(Property)` reads its result without traversing the expression again.

| Property | Meaning |
| --- | --- |
| `Finite` | Defined finite complex value. |
| `Real` | Finite real value. |
| `Integer` | Real integer, independent of literal representation. |
| `Rational` | Real rational value. |
| `Zero` | Zero value. |
| `Nonzero` | Finite complex value unequal to zero. |
| `Positive`, `Negative` | Strict real sign. |
| `Nonnegative`, `Nonpositive` | Non-strict real sign. |

`Truth::True` means proved; `Truth::False` means refuted; `Truth::Unknown` means no conclusion. `Unknown` does not authorize a transformation requiring the property.

```rust
use symb_anafis::{Context, Property, Truth};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;

    assert_eq!(x.to_expr().properties().get(Property::Finite), Truth::True);
    assert_eq!(x.to_expr().properties().get(Property::Real), Truth::Unknown);
    assert_eq!(x.pow(2).properties().get(Property::Nonnegative), Truth::Unknown);
    assert_eq!(x.exp().properties().get(Property::Positive), Truth::Unknown);

    let facts = (x.abs() + 1).properties();
    assert_eq!(facts.get(Property::Real), Truth::True);
    assert_eq!(facts.get(Property::Positive), Truth::True);
    assert_eq!(x.abs().exp().properties().get(Property::Positive), Truth::True);
    Ok(())
}
```

Symbols represent finite complex scalars, without additional assumptions about realness, sign, integrality, or being nonzero. Consequently, \(x^2\) is not automatically proved nonnegative, nor \(\exp(x)\) positive. The API currently attaches no assumptions to symbols or contexts.

Properties describe mathematical values; they do not guarantee absence of overflow or rounding during numerical evaluation.

### 11. Concrete substitution

`Substitution::new(from, to)` defines a replacement of concrete expressions. `apply_all(&expression)` applies it throughout the expression with DAG memoization.

```rust
use symb_anafis::{Context, Expr, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let z = ctx.symb("z")?;
    let u = ctx.symb("u")?;
    let group = Substitution::new(x + y, u.into());

    assert_eq!(group.apply_all(&(x + y + z)), u + z);
    assert_eq!(group.apply_all(&(2 * x + 2 * y + z)), 2 * u + z);
    assert_eq!(Substitution::new(x.pow(2), u.into()).apply_all(&x.pow(5)), u.pow(2) * x);
    assert_eq!(Substitution::new(2 * x, u.into()).apply_all(&(3 * x)), (Expr::number(3) / 2) * u);
    assert_eq!(Substitution::new(2 * x, u.into()).apply_all(&x.to_expr()), (Expr::number(1) / 2) * u);
    Ok(())
}
```

A sum or product source can match a group of operands; the remainder survives without a `rest` capture. Integer or rational arithmetic sources can also consume implicit power and coefficient groups without expanding large multiplicities into vectors.

| Substitution | Input | Result |
| --- | --- | --- |
| \(x+y\to u\) | \(x+y+z\) | \(u+z\) |
| \(x+y\to u\) | \(2x+2y+z\) | \(2u+z\) |
| \(x^2\to u\) | \(x^5\) | \(u^2x\) |
| \(2x\to u\) | \(3x\) | \(\frac32u\) |
| \(2x\to u\) | \(x\) | \(\frac12u\) |
| \(x+y\to u\) | \(f(z)x+f(z)y\) | \(f(z)u\) |
| \(\sqrt{x}\to u\) | \(x^{3/2}\) | \(u^3\) |
| \(x^2\to u\) | \(1/x^4\) | \(1/u^2\) |

Matching respects symbol and function identities. It also finds monomials held in private polynomial storage.

Manifest common factors can be retained without dividing by a symbolic expression. The matcher does not invent division by a possibly zero symbolic value, introduce poles into a defined input, or extract arbitrary fractional roots. For example, \(\sqrt{x^2}\) is not identified with \(x\) for a general complex value.

A matching whole node takes precedence and its descendants are not traversed. Surviving original factors continue to be traversed; inserted destinations, generated coefficient quotients, and generated exponent counts do not become new sources in the same pass. `from()` and `to()` inspect the substitution.

#### Changes of variable

A change of variable supplies the chosen inverse \(x\to g(u)\). The caller chooses its domain and branch. The core applies the explicit replacement without requiring a proof that it preserves the original domain.

For \(u=x^2\), choosing \(x=\sqrt{u}\) gives \(x^5=u^{5/2}\). Choosing \(x=-\sqrt{u}\) gives \(x^5=-u^{5/2}\).

```rust
use symb_anafis::{Context, Number, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let u = ctx.symb("u")?;
    let exponent = Number::from_int(5) / Number::from_int(2);
    let input = x.pow(5) + x.sin();

    let inverse = u.sqrt();
    let positive = Substitution::new(x.into(), inverse.clone());
    let result = positive.apply_all(&input);
    assert_eq!(result, u.pow(exponent.clone()) + inverse.sin());
    assert!(!result.contains_symbol(x));

    let negative_inverse = -u.sqrt();
    let negative = Substitution::new(x.into(), negative_inverse.clone());
    assert_eq!(negative.apply_all(&input), -u.pow(exponent) + negative_inverse.sin());
    Ok(())
}
```

Every scalar occurrence of the source symbol is replaced, including function arguments, exponents, denominators, and compact polynomial bases. The source disappears from these positions when the destination does not contain that symbol identity. Equal names from another context denote different symbols and are preserved. Destinations are inserted once without further substitution.

The differentiation variable of an unevaluated `Derivative` is metadata and retains its identity. Changing differential operators or integration measures requires the corresponding calculus transformation.

#### Substituting only symbols

| `Expr` API | Selection |
| --- | --- |
| `substitute_symbol(symbol, &value)` | One exact identity. |
| `substitute_symbols(&map)` | Multiple identities, simultaneously. |
| `substitute(name, &value)` | All symbols with that name, including from different contexts. |

These methods replace symbol nodes. They do not change the differentiation-variable metadata of a `Derivative` node.

```rust
use symb_anafis::{Context, Expr, HashMap};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let expression = x.pow(2) + y;
    let values = HashMap::from([(x, Expr::number(3)), (y, Expr::number(4))]);

    assert_eq!(expression.substitute_symbols(&values), Expr::number(13));
    assert_eq!(expression.substitute_symbol(x, &y.to_expr()), y.pow(2) + y);
    Ok(())
}
```

Substituting values reconstructs and normalizes changed parents. Use [numerical evaluation](#numerical-evaluation) to compute scalar results without rebuilding the expression.

### 12. Simultaneous substitutions

`Substitutions::new(entries)` validates a collection of `(Expr, Expr)` pairs. All sources are matched against original nodes, before rebuilding their children.

```rust
use symb_anafis::{Context, Substitutions, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let c = ctx.symb("c")?;
    let pair = ctx.register_function("pair", UserFunction::new(2..=2))?;
    let f = ctx.register_function("f", UserFunction::new(1..=1))?;

    let swap = Substitutions::new([(x.into(), y.into()), (y.into(), x.into())])?;
    let input = pair.call(vec![x.into(), y.into()])?;
    assert_eq!(swap.apply_all(&input), pair.call(vec![y.into(), x.into()])?);

    let fx = f.call(vec![x.into()])?;
    let overlapping = Substitutions::new([(x.into(), y.into()), (fx.clone(), c.into())])?;
    assert_eq!(overlapping.apply_all(&fx), c.to_expr());
    assert_eq!(swap.len(), 2);
    assert!(!swap.is_empty());
    Ok(())
}
```

| Situation | Contract |
| --- | --- |
| \(x\to y\), \(y\to x\) | Simultaneous swap without cascading. |
| \(f(x)\to c\), \(x\to y\) | Original \(f(x)\) becomes \(c\) directly. |
| Repeated source, same canonical destination | Accepts one entry. |
| Repeated source, different destinations | `SubstitutionError::Conflict`. |
| Overlapping partial groups | Entry order determines priority; whole-node matches take precedence. |
| Destination contains its source | Terminates after one pass; does not revisit the destination. |
| No match | Preserves the canonical expression. |

`get(&source)` queries an exact entry; `iter()` yields distinct pairs in entry order; `len()` and `is_empty()` inspect the collection. An empty collection can also be constructed with `Substitutions::default()`.

### 13. Patterns and rewrite rules

`Pattern` describes a family of expressions. `RewriteRule` connects an input pattern to a destination constructed from the matched captures.

```rust
use symb_anafis::{Context, Pattern, RewriteRule, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let pair = ctx.register_function("pair", UserFunction::new(2..=2))?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(a.sin(), a.cos())?;
    let input = pair.call(vec![x.sin(), (x + y).sin()])?;

    assert_eq!(rule.apply_all(&input, None, None)?, pair.call(vec![x.cos(), (x + y).cos()])?);
    Ok(())
}
```

This rule requests \(\sin(a)\to\cos(a)\); it does not assert a mathematical identity. The core validates the structural operation, not equivalence between source and destination.

| Pattern construction | Semantics |
| --- | --- |
| `Pattern::capture("a", None)` | Named capture without a predicate. |
| `Pattern::capture("a", Some(predicate))` | Capture with a local condition. |
| `Pattern::exact(expression)` | Concrete expression without captures. |
| `Pattern::apply(function, arguments)?` | Known function with pattern arguments and validated arity. |
| `a + b`, `a * b`, `a / b`, `a.pow(n)` | Composition through normalized operators. |
| `Pattern::sum(terms)`, `Pattern::product(factors)` | Dynamically sized composition. |
| `a.sin()`, `a.log(base)`, other mathematical methods | The same fluent methods as expressions. |

A symbol or expression used inside a pattern denotes an exact value. A capture denotes an expression to discover. Captures with the same name require the same canonical value and accumulate their conditions, even when constructed separately. Different names may capture equal values. There is no anonymous `any` wildcard.

Pattern sums and products are also flattened and normalized. For example, two added `a` captures become a multiplicity requirement for `a`; they do not become independent variables.

#### Reusing and discarding captures

The destination may use a subset of source captures, duplicate them, or ignore them. A capture present only in the destination produces `RewriteError::UnboundCapture`. Exact expressions in the destination may introduce new concrete symbols.

```rust
use symb_anafis::{Context, Pattern, RewriteRule, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let pair = ctx.register_function("pair", UserFunction::new(2..=2))?;
    let a = Pattern::capture("a", None);
    let b = Pattern::capture("b", None);
    let source = Pattern::apply(pair.clone(), vec![a.clone(), b])?;
    let first = RewriteRule::new(source, a)?;

    assert_eq!(first.apply_all(&pair.call(vec![x.into(), y.into()])?, None, None)?, x.to_expr());
    Ok(())
}
```

#### Commutative groups and implicit factors

At a rule's root, sums and products can consume disjoint operand groups. Remaining operands are preserved automatically. Inside another pattern structure, a sum or product must cover the complete group; a capture may bind a nonempty group of operands.

```rust
use symb_anafis::{Context, Pattern, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let z = ctx.symb("z")?;
    let u = ctx.symb("u")?;
    let rule = RewriteRule::new(Pattern::exact(x + z), Pattern::exact(u))?;

    assert_eq!(rule.apply_all(&(x + y + z), None, None)?, u + y);

    let angle = Pattern::capture("angle", None);
    let identity = RewriteRule::new(angle.sin().pow(2) + angle.cos().pow(2), Pattern::exact(1))?;
    assert_eq!(identity.apply_all(&(x.sin().pow(2) + x.cos().pow(2) + y), None, None)?, y + 1);
    Ok(())
}
```

The matcher uses deterministic backtracking. It tries whole operands before counted blocks and grouped partitions. Integer powers, collected addends, and admissible rational groups do not need to be expanded into lists of copies.

Scalar coefficients can be inferred: `2*a` can match `x`, capturing `a = x/2`. Such a rule can match nearly every visited node, including literals. Use a function structure or a guard to restrict its scope when needed.

The branch, pole, and symbolic-factor restrictions described for `Substitution` also matter in rewriting. The matcher is not a prover of all algebraic equivalences.

#### Application order

`rule.apply_all(&expression, match_limit, guard)` returns `Result<Expr, RewriteError>`. It visits logical nodes bottom-up and matches rebuilt parents. Concrete simultaneous substitution selects original nodes before rebuilding children.

Inserted destinations are not traversed again in the same application. Every edge to a shared input node receives the same memoized result. One application is not automatic reduction to a fixed point; repeated rewriting requires an explicit caller policy.

### 14. Predicates, guards, and search limits

A predicate belongs to a capture and rejects candidates during search. A guard receives the complete set of captures and decides whether to accept that assignment. Rejection can make the matcher try another assignment.

| Callback | Logical type | Timing |
| --- | --- | --- |
| Capture predicate | `Fn(&Expr) -> bool + Send + Sync` | Before accepting a candidate capture. |
| Rule guard | `FnMut(&HashMap<&str, Expr>) -> bool` | After finding a complete assignment. |
| Dynamic destination | `Fn(&HashMap<&str, Expr>) -> Expr + Send + Sync` | After guard acceptance. |

Predicates and dynamic destinations must be pure and stable for equal inputs. The matcher may evaluate predicates repeatedly and reuse destinations for counted groups. Guards may run multiple times; external effects are not rolled back if the operation fails.

#### An identity requiring a realness proof

For real values, \(\sqrt{a^2}=|a|\). This conclusion does not hold for an arbitrary complex value. The predicate below requires the proof before applying the rule:

```rust
use std::sync::Arc;
use symb_anafis::{Context, Pattern, Property, RewriteRule, Truth};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", Some(Arc::new(|node| {
        node.properties().get(Property::Real) == Truth::True
    })));
    let rule = RewriteRule::new(a.pow(2).sqrt(), a.abs())?;
    let real_base = x.abs() + 1;

    assert_eq!(rule.apply_all(&real_base.pow(2).sqrt(), None, None)?, real_base.abs());
    let unknown = x.pow(2).sqrt();
    assert_eq!(rule.apply_all(&unknown, None, None)?, unknown);
    Ok(())
}
```

#### Guarding a complete assignment

A guard can check relationships between captures or consult external proofs. Here, \(\sqrt{a^2}\to a\) is allowed only when `a` is proved nonnegative:

```rust
use symb_anafis::{Context, Expr, HashMap, Pattern, Property, RewriteRule, Truth};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(a.pow(2).sqrt(), a)?;
    let mut guard = |captures: &HashMap<&str, Expr>| {
        captures.get("a").is_some_and(|value| {
            value.properties().get(Property::Nonnegative) == Truth::True
        })
    };

    assert_eq!(rule.apply_all(&x.abs().pow(2).sqrt(), Some(10_000), Some(&mut guard))?, x.abs());
    let input = x.pow(2).sqrt();
    assert_eq!(rule.apply_all(&input, None, Some(&mut guard))?, input);
    Ok(())
}
```

#### Work limit

`match_limit = None` allows unbounded candidate search. `Some(n)` limits commutative attempts across the entire pass. Complete ordered matches consume no attempts; the limit is not a count of nodes, visits, or replacements.

If another attempt is needed after exhaustion, the result is `RewriteError::MatchLimitExceeded { limit }`. The input expression remains valid and no partially transformed output is published. The limit does not bound callback execution time, input memory, or generated destination size.

### 15. Callback replacements

`RewriteRule::new` accepts a destination `Pattern` directly, or `Replacement::Callback(Arc::new(...))` to construct the destination from captures.

The callback below expands \(\sin(2a)=2\sin(a)\cos(a)\). This mechanism is useful when the destination depends on calculations or structures generated in Rust:

```rust
use std::sync::Arc;
use symb_anafis::{Context, Pattern, Replacement, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(
        (2_i32 * a).sin(),
        Replacement::Callback(Arc::new(|captures| {
            let angle = &captures["a"];
            2 * angle.sin() * angle.cos()
        })),
    )?;

    assert_eq!(rule.apply_all(&(2 * x).sin(), None, None)?, 2 * x.sin() * x.cos());
    Ok(())
}
```

The `captures["a"]` access relies on the name declared in the input pattern; the matcher supplies that capture to an accepted callback. Unlike a destination `Pattern`, a callback has no static validation of the capture names it accesses.

A counted assignment may reuse a single destination for multiple occurrences. Capture-free callbacks are evaluated lazily and, when needed, once per application. Do not use a callback to count occurrences or generate distinct identities for each edge.

### 16. Binomial expansion with a variable order

One rule can construct the \(n+1\) terms of

$$
(a+b)^n=\sum_{k=0}^{n}\binom{n}{k}a^{n-k}b^k.
$$

The example accepts literal integer exponents from 2 to 60. Coefficients are calculated using

$$
c_0=1,\qquad c_{k+1}=c_k\frac{n-k}{k+1}.
$$

This range keeps coefficients and intermediate products within the current backend's native 64-bit integer. It is not a matcher limit. Zero-exponent factors are omitted explicitly, avoiding constructions such as \(a^0\) at the endpoints.

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, Number, Pattern, Replacement, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let a = Pattern::capture("a", None);
    let b = Pattern::capture("b", None);
    let n = Pattern::capture("n", Some(Arc::new(|expression| {
        expression.as_number().is_some_and(|value| {
            value.is_int() && (2..=60).contains(&value.to_int())
        })
    })));

    let expand = RewriteRule::new(
        (a + b).pow(n),
        Replacement::Callback(Arc::new(|captures| {
            let left = &captures["a"];
            let right = &captures["b"];
            let exponent = &captures["n"];
            let Some(value) = exponent.as_number() else {
                return (left + right).pow(exponent);
            };
            let degree = value.to_int();
            let mut coefficient = Number::from_int(1);
            let mut terms = Vec::new();

            for k in 0..=degree {
                let mut factors = vec![Expr::number(coefficient.clone())];
                if degree > k {
                    factors.push(left.pow(degree - k));
                }
                if k > 0 {
                    factors.push(right.pow(k));
                }
                terms.push(Expr::product(factors));

                if k < degree {
                    coefficient = coefficient * Number::from_int(degree - k)
                        / Number::from_int(k + 1);
                }
            }
            Expr::sum(terms)
        })),
    )?;

    let result = expand.apply_all(&(x + y).pow(4), None, None)?;
    let expected = x.pow(4) + 4 * x.pow(3) * y
        + 6 * x.pow(2) * y.pow(2) + 4 * x * y.pow(3) + y.pow(4);
    assert_eq!(result, expected);

    for degree in 2..=60 {
        let expanded = expand.apply_all(&(x + y).pow(degree), None, None)?;
        let at_one = expanded.substitute_symbol(x, &Expr::number(1))
            .substitute_symbol(y, &Expr::number(1));
        assert_eq!(expanded.children().len(), usize::try_from(degree + 1)?);
        assert_eq!(at_one, Expr::number(1_i64 << degree));
    }
    Ok(())
}
```

For \(n=2\), the rule generates \(a^2+2ab+b^2\); for \(n=3\), it generates \(a^3+3a^2b+3ab^2+b^3\). The bases may be expressions such as `x.sin()` and `y.exp()` instead of individual symbols.

Expansion is explicit: constructing `(x + y).pow(n)` does not run it. When a sum has more than two terms, captures can represent groups; one pass does not re-expand newly generated powers of those groups. A normalized negative exponent can have its positive denominator power expanded during traversal.

The same mechanism can generate finite sums, recurrences, and truncated series. A general Taylor series additionally requires a way to obtain derivatives; the core does not yet provide that engine.

### 17. DAG transformation and folding

`Expr::map` transforms logical nodes in children-before-parent order:

```text
map(|rebuilt_node| -> Result<Option<Expr>, Error> { ... })
```

`Ok(None)` retains the rebuilt node; `Ok(Some(expr))` replaces it; `Err(error)` interrupts the operation. The callback sees the node after its children are reconstructed. Each distinct original logical node is visited once; inserted destinations are not revisited.

```rust
use core::convert::Infallible;
use symb_anafis::{Context, ExprView};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let target = x.sin();
    let expression = target.pow(2) + target.pow(3);
    let mut visits = 0;
    let result = expression.map(|node| -> Result<_, Infallible> {
        if node == &target {
            visits += 1;
            Ok(Some(x.cos()))
        } else {
            Ok(None)
        }
    })?;

    assert_eq!(visits, 1);
    assert_eq!(result, x.cos().pow(2) + x.cos().pow(3));
    let powers = result.fold(0, |count, node| {
        count + usize::from(matches!(node.view(), ExprView::Pow(_, _)))
    });
    assert_eq!(powers, 2);
    Ok(())
}
```

`fold(initial, callback)` reduces distinct logical nodes in one children-before-parent pass. Compacted monomials appear as expressions; internal additive storage groups do not introduce extra visits. Neither `map` nor `fold` requires knowledge of private polynomial storage.

Use `map` for programmatic transformations of rebuilt nodes, `RewriteRule` for matching with captures, and `Substitution` for selecting concrete sources in the original expression.

### 18. Symbolic differentiation

`Expr::derivative(inner, variable, order)` evaluates

$$
\frac{\partial^{\mathrm{order}}}{\partial \mathrm{variable}^{\mathrm{order}}}
\mathrm{inner}.
$$

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let derivative = Expr::derivative(x.sin(), x, 2);

    assert_eq!(derivative, -x.sin());
    assert_eq!(Expr::derivative(x.sin(), x, 0), x.sin());
    assert_eq!(Expr::derivative(derivative, x, 3), Expr::derivative(x.sin(), x, 5));
    Ok(())
}
```

Order zero returns the inner expression. Supported derivatives evaluate immediately, using symbol identities rather than display names. Other symbols are held constant. The engine visits each shared logical node once per order, including monomials in compact polynomial storage. Higher orders use repeated differentiation; periodic derivative states are skipped with constant additional storage.

The result uses the ordinary DAG constructors for constant folding and term collection. Differentiation does not run a separate simplification pass and has no simplification option.

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = ctx.parse("sin(x^2) + x^3")?;
    let derivative = Expr::derivative(expression, x, 1);
    assert_eq!(derivative, 2 * x * x.pow(2).cos() + 3 * x.pow(2));
    assert_eq!(Expr::derivative(x.sin(), x, u32::MAX), x.cos() * -1);
    Ok(())
}
```

Rules cover sums, products, quotients, powers, elementary analytic functions, error and gamma functions, zeta derivatives, Bessel argument derivatives, beta, logarithms with variable bases, and Lambert W argument derivatives. Products use a balanced product rule without dividing by their factors. The Lambert W slope uses

$$
W_k'(z)=\frac{\exp(-W_k(z))}{1+W_k(z)},
$$

which also evaluates to one at the principal branch's regular point `z = 0`. Bessel derivatives use [adjacent-order recurrences](https://dlmf.nist.gov/10.6), with the corresponding [modified Bessel recurrences](https://dlmf.nist.gov/10.29).

Analytic formulas apply where their principal-branch factors are regular. The returned expression does not store the input's domain restrictions or extend formulas through removable singularities; evaluations at exceptional parameter values can require limits. Generic complex absolute values and nonsmooth rounding functions remain unevaluated. Formulas for sinc and complete elliptic integrals require a proved nonzero argument to avoid introducing a removable pole at zero. Polygamma and zeta derivative orders, and Hermite degrees, require supported exact nonnegative integer literals. Differentiating a varying Bessel order, a varying Lambert branch, an associated Legendre function, or a spherical harmonic's polar angle remains symbolic; fixed-degree and fixed-order spherical harmonics support azimuth derivatives.

Custom calls use their registered partials and the multivariate chain rule. The engine checks all needed partials before executing any callback for that call. Constant arguments need no partial, and function bodies are not expanded implicitly.

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let f = ctx.register_function("f", UserFunction::new(1..=1)
        .partial(0, Arc::new(|args| 2 * &args[0]))?)?;
    let expression = f.call(vec![x.sin()])?;
    assert_eq!(Expr::derivative(expression, x, 1), 2 * x.sin() * x.cos());
    Ok(())
}
```

An unsupported derivative remains an owning `ExprView::Derivative` node. Missing custom partials preserve the whole call's derivative. Substitution, rewriting, and mapping retry differentiation when they change that node's inner expression.

```rust
use symb_anafis::{Context, Expr, ExprView, Substitution, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let f = ctx.register_function("f", UserFunction::new(1..=1))?;
    let call = f.call(vec![x.to_expr()])?;
    let deferred = Expr::derivative(call.clone(), x, 2);
    assert!(matches!(deferred.view(), ExprView::Derivative { order: 2, .. }));
    let result = Substitution::new(call, x.sin()).apply_all(&deferred);
    assert_eq!(result, -x.sin());
    Ok(())
}
```

Adjacent unevaluated derivatives in the same symbol identity combine their orders when the sum fits `u32`; larger totals stay nested. Different differentiation variables do not commute automatically. Stored orders are positive and retain their variable metadata. Callback execution and numeric reconstruction can propagate the callbacks' or numeric backend's panics.

#### Multivariable calculus

`expression.gradient(&variables)`, `Expr::jacobian(&outputs, &variables)`, and
`expression.hessian(&variables)` return owning vectors in the requested coordinate
order:

$$
g_i=\partial_{x_i}f,\qquad
J_{ij}=\partial_{x_j}f_i,\qquad
H_{ij}=\partial_{x_j}(\partial_{x_i}f).
$$

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new();
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let model = x.pow(2) + x * y;
    assert_eq!(model.gradient(&[x, y]), vec![2 * x + y, x.to_expr()]);
    assert_eq!(model.hessian(&[x, y]), vec![
        vec![Expr::number(2), Expr::number(1)],
        vec![Expr::number(1), Expr::number(0)],
    ]);
    assert_eq!(Expr::jacobian(&[model, x * y], &[x, y]), vec![
        vec![2 * x + y, x.to_expr()],
        vec![y.to_expr(), x.to_expr()],
    ]);
    Ok(())
}
```

Jacobians store outputs as rows and coordinates as columns. Each column reuses one
memo and traversal workspace across all output roots, including shared subtrees.
Hessians compute both mixed derivative orders; they do not assume that unresolved
partials commute. The scalar derivative's principal-branch and regularity
conditions apply to every component.

Repeated coordinates repeat components. A symbol with the same name from another
context remains a different coordinate. Empty coordinates produce an empty
gradient or Hessian and one empty Jacobian row per output; no outputs produce no
rows. Results retain symbol and function metadata after context destruction.
There are no string-specific wrappers: parse with `Context::parse`, use scoped
symbols, and format the resulting expressions when needed. Apply a
`SimplificationSequence` explicitly to choose additional identities.

### 19. Formatting

| API | Format |
| --- | --- |
| `Display`, `to_string()`, `format!("{expression}")` | Mathematical text. |
| `to_latex()` | LaTeX. |
| `to_unicode()` | Unicode mathematical text. |
| `Debug` for `Expr` | Mathematical representation of the expression. |

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = (x.pow(2) + Expr::pi()) / (x + 1);

    println!("{expression}");
    println!("{}", expression.to_latex());
    println!("{}", expression.to_unicode());
    Ok(())
}
```

Formatting presents mathematical structure; it is not a serialization format for identities, IEEE representations, or namespaces. Distinct symbols with equal names can produce identical text. Parsing formatted text therefore does not guarantee identity round-tripping.

### 20. Errors, memory, and limits

#### Explicit core errors

| Type | Situations |
| --- | --- |
| `SymbolError` | Duplicate or missing global name, invalid ID for the owner, symbol ID exhaustion. |
| `FunctionError` | Duplicate or reserved name, empty arity range, invalid call arity, invalid partial index. |
| `RewriteError` | Capture absent from the input pattern, or search limit exhaustion. |
| `SubstitutionError` | One concrete source with different canonical destinations. |
| `SimplifyError` | A simplification input or candidate exceeds its node, depth, expansion, polynomial, or rewrite bound. |
| `EvalError` | Unsupported numerical inputs or bodies, invalid compiled dataflow, or incompatible parameter and batch shapes. |
| `UncertaintyError` | Covariance shape or symmetry violations, repeated coordinates, or disproved finite real propagation conditions. |

All implement `core::error::Error` and `Display`. Public error enums are non-exhaustive. `DiffError` and `Span` also exist in the facade, but do not represent additional core operations described here.

#### Boundaries that can panic

- `Context::symb` returns an error when another ID cannot be allocated. `Symbol::intern`, `Symbol::anon`, and `Expr::symbol` return directly and can panic on ID exhaustion. `Context::new` and `reset` can panic on context ID exhaustion.
- Numeric operations and reconstruction can propagate rational backend capacity panics. Native integer arithmetic follows the backend's overflow policy; this core does not provide arbitrary precision.
- Predicates, guards, bodies, partial derivatives, and replacement callbacks belong to the caller; their panics propagate when executed.
- Allocation uses the program's allocator. The API does not convert all allocation failures into `Result`.

There is no universal promise of panic-free execution or a fixed memory bound.

#### Ownership and memory growth

Expressions own nodes shared through `Arc`; the interner holds weak references. Expression destruction dismantles the DAG iteratively, and nodes release ownership of their contents after their final owner disappears. Weak-entry reclamation and retained table capacity do not imply immediately returning all memory to the operating system.

Global symbol metadata is permanent. Local contexts retain issued identities until their namespace loses its final owner; `clear_symbols()` removes names, while `reset()` or destruction allows ownership to be released. Live expressions retain the metadata they use.

Registered functions and callbacks also have shared ownership. A callback capturing a clone of the context that registers it can create an `Arc` cycle; use weak references or remove that cycle's bindings before abandoning external owners.

Memory usage depends on live nodes, symbols, definitions, table capacities, matching search, and generated destinations. The library imposes no global quotas. `match_limit` controls matching attempts, not expansion size; a resource policy should also bound inputs and callback work.

Without `std`, a spin mutex requires its lock owner to be able to continue running. Reentering registry operations from an interrupt that suspended that owner is unsupported. With `std`, synchronization uses blocking mutexes.

## Simplification

`SimplificationSequence` is an ordered, reusable program over the existing expression and rewrite APIs. Construction already folds constants, collects like terms, normalizes exact coefficients, and orders operands. A sequence selects additional identities and the desired mathematical form.
Builtin rule plans compile during sequence construction and are shared by clones,
so the same sequence can process many expressions without recompiling patterns.

### Presets

```rust
use symb_anafis::{Context, Expr, SimplificationSequence};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = ctx.parse("sin(x)^2 + cos(x)^2")?;
    let compact = SimplificationSequence::compact().simplify(&expression)?;
    assert_eq!(compact.expression(), &Expr::number(1));

    let expanded = SimplificationSequence::expanded().simplify(&(x + 1).pow(2))?;
    assert_eq!(expanded.into_expression(), x.pow(2) + 2 * x + 1);
    let y = ctx.symb("y")?;
    let factored = SimplificationSequence::factored()
        .simplify(&(x.pow(2) + x * y))?;
    assert_eq!(factored.into_expression(), x * (x + y));
    Ok(())
}
```

| Constructor | Desired form |
| --- | --- |
| `new()` | Empty sequence, compact objective. |
| `compact()` / `default()` | Scalar identities and factoring, minimizing logical size. |
| `expanded()` | Identities and bounded distributivity, minimizing remaining distributable sites before size. |
| `factored()` | Identities and factoring, minimizing composite terms under sums before size. |
| `compilation()` | Identities, native numerical forms, and factoring, minimizing estimated operation cost over the shared DAG before size. |

Logical size counts repeated edges even when nodes are shared. A composite term is an integer power or a product with multiple nonnumeric factors, including integer powers behind scalar coefficients. These are heuristics for choosing among visited forms; they do not certify a globally minimal expression or complete polynomial factorization.

### Custom sequences

Append builtin passes, existing `RewriteRule` values, or mapping callbacks with `then`. Inspect and clone `steps()` to extend or reorder a preset.

```rust
use symb_anafis::{Context, Expr, Pattern, RewriteRule, SimplificationSequence, SimplificationStep};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", None);
    let identity = RewriteRule::new(
        a.sin().pow(2) + a.cos().pow(2), Pattern::exact(1),
    )?;
    let sequence = SimplificationSequence::new()
        .then(identity)
        .then(SimplificationStep::Factor);
    assert_eq!(sequence.simplify(&(x.sin().pow(2) + x.cos().pow(2)))?.into_expression(), Expr::number(1));
    Ok(())
}
```

Builtin steps are `Algebraic`, `Roots`, `Exponential`, `Trigonometric`, `Hyperbolic`, `Expand`, `Factor`, and `Compilation`. Algebraic includes bounded, exact multivariate polynomial GCD cancellation with verified quotients and nonzero-factor guards. `Rewrite(rule)` uses the core matcher and its capture predicates. `Map(Arc::new(callback))` applies a pure `Fn(&Expr) -> Option<Expr>` bottom-up to each distinct logical node, through `Expr::map`. Inserted nodes are not revisited within that pass. User rules and callbacks retain the caller's responsibility for mathematical equivalence.

Registered function bodies are also available through ordinary mapping:

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, ExprView, SimplificationSequence, SimplificationStep, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let f = ctx.register_function("double", UserFunction::new(1..=1)
        .body(Arc::new(|args| Expr::sum(args.to_vec()) * 2)))?;
    let x = ctx.symb("x")?;
    let expand_bodies = SimplificationStep::Map(Arc::new(|node| {
        let ExprView::Function { name, args } = node.view() else { return None; };
        name.definition()?.body_fn().map(|body| body(args))
    }));
    let sequence = SimplificationSequence::new().then(expand_bodies)
        .score_with(|node| node.fold(0_usize, |count, part| count.saturating_add(
            usize::from(matches!(part.view(), ExprView::Function { name, .. } if name.definition().is_some()))
        )));
    assert_eq!(sequence.simplify(&f.call(vec![x.to_expr()])?)?.into_expression(), 2 * x);
    Ok(())
}
```

### Compilation preparation

The compilation preset prepares expressions for numerical lowering. It selects native forms such as `exp_m1`, `ln_1p`, and `erfc`, and factors common polynomial terms into nested arithmetic. The `Compilation` step can also be used alone in a custom sequence.

```rust
use symb_anafis::{Context, SimplificationSequence, SimplificationStep, SimplificationTarget};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let preset = SimplificationSequence::compilation();
    assert_eq!(preset.simplify(&(x.exp() - 1))?.into_expression(), x.exp_m1());
    assert_eq!(preset.simplify(&(x + 1).ln())?.into_expression(), x.ln_1p());
    assert_eq!(preset.simplify(&(1 - x.erf()))?.into_expression(), x.erfc());

    let custom = SimplificationSequence::new()
        .then(SimplificationStep::Compilation)
        .target(SimplificationTarget::Compilation);
    assert_eq!(custom.simplify(&(x.exp() - 1))?.into_expression(), x.exp_m1());
    Ok(())
}
```

The pass also selects `cbrt` for exact cube-root powers of proved positive real inputs and `sinc` for `sin(x)/x` only when `x` is proved nonzero. Inverse logarithm/exponential compositions retain their realness and nonzero guards. These transformations preserve principal branches and pole exclusions.

The compilation objective sums estimated operations once per distinct logical DAG node, so existing shared function results count once. Addition and multiplication cost one per binary operation, division costs four, and positive integer powers use the multiplication count for binary exponentiation. General powers cost 24; simple scalar functions cost one, square roots four, cube roots eight, other function calls 16, and retained derivatives 32. Literal and symbol leaves cost zero. These portable weights are a heuristic; a backend can provide its own `score_with` function.

### Targets, scores, and limits

`target(SimplificationTarget::Compact | Expanded | Factored | Compilation)` changes candidate selection and clears a custom scorer. It leaves the steps unchanged. `score_with(|expression| expression.node_count())` supplies a pure primary score; logical size breaks ties. Equal complete scores select the latest candidate.

Every step receives the preceding step's output. Temporary growth remains available to later cancellation. The input and every completed step compete for the returned result. `Simplification` exposes `expression()`, `into_expression()`, `iterations()`, and `termination()`; the termination reason is `FixedPoint`, `Cycle`, or `IterationLimit`. A selected earlier candidate need not itself be a fixed point.

| Setting | Default | Scope |
| --- | --- | --- |
| `max_iterations` | 16 | Complete sweeps; zero returns the input without a sweep. |
| `max_nodes` | 100,000 | Distinct stored nodes in input and completed candidates. |
| `max_depth` | 512 | Stored depth, counting the root. |
| `match_limit` | `Some(100_000)` | Candidate attempts per core rewrite; `None` is unrestricted. |
| `expansion_limit` | 4,096 | Cumulative Cartesian terms generated in one expansion pass. |
| `polynomial_limit` | 100,000 | Shared sparse conversion, term-product, and GCD remainder work per algebraic pass. |

On 16-bit targets, the 100,000-unit defaults are capped at `usize::MAX`.

An exceeded node, depth, expansion, polynomial, or rewrite bound returns `SimplifyError` and discards partial output. Callback panics and numeric capacity panics propagate. Callback allocations and work remain caller-controlled.

### Domain modes

`DomainSafety::Preserving` is the default for every preset. Builtin identities use conservative `Properties` proofs and preserve principal branches and poles. Thus an unconstrained `x/x` remains a quotient, `ln(exp(x))` needs realness, and `sqrt(x^2)` becomes `abs(x)` only for a proved real input. Targets and custom scores do not relax these guards.

`domain_safety(DomainSafety::AssumptionDependent)` also permits explicitly tagged identities with caller-supplied nonzero, realness, positivity, and principal-range hypotheses. For example, cancellation assumes the removed factor is nonzero; `asin(sin(x)) = x` assumes a real input in `[-pi/2, pi/2]`. A tested false condition is rejected. This mode does not prove the caller's assumptions and can change the represented domain. Custom rules and mapping callbacks keep their caller-owned contract in both modes.

```rust
use symb_anafis::{DomainSafety, Expr, SimplificationSequence};

fn main() -> Result<(), symb_anafis::SimplifyError> {
    let x = Expr::symbol("assumed_nonzero_x");
    let preserving = SimplificationSequence::compact();
    assert_eq!(preserving.simplify(&(&x / &x))?.into_expression(), &x / &x);
    // The caller supplies x != 0 for this cancellation.
    let assumed = preserving.domain_safety(DomainSafety::AssumptionDependent);
    assert_eq!(assumed.simplify(&(&x / &x))?.into_expression(), Expr::number(1));
    Ok(())
}
```

The [complete rule reference](../src/simplification/RULES.md) lists the formulas, conditions for both modes, preset membership, and the destination of every registered legacy rule. The [subsystem guide](../src/simplification/README.md) describes traversal, costs, and limits.

## Parsing

### Expressions and scoped names

`Context::parse(&str) -> Result<Expr, ParseError>` constructs expressions through the same normalized kernels as arithmetic operators. Unknown variable names create symbols in that context. Repeated parsing uses its existing bindings; the returned expression can outlive the context.

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let expression = ctx.parse("(x + y)^4 + 2x sin(y)")?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    assert_eq!(expression, (x + y).pow(4) + 2 * x * y.sin());

    assert_eq!(ctx.parse("1/4")?, Expr::number(1) / 4);
    assert_eq!(ctx.parse("sqrt(1/4)")?, Expr::number(1) / 2);
    assert_eq!(ctx.parse("2.0")?, Expr::number(2.0));
    assert_ne!(ctx.parse("2.0")?, Expr::number(2));
    Ok(())
}
```

| Syntax | Meaning |
| --- | --- |
| `42`, `1_024`, `-9223372036854775808` | Exact `i64` literals. Out-of-range integers return an error. |
| `2.0`, `.25`, `1.5e-3` | Finite `f64` literals; approximate representation is retained. |
| `pi`, `π`, `e`, `i` | Exact mathematical constants, unless a symbol of that name already exists in the context. |
| `+`, `-`, `*`, `/` | Scalar arithmetic with unary signs. |
| `^`, `**` | Right-associative powers: `x^y^z` means `x^(y^z)`. |
| `-x^2`, `x^-2` | `-(x^2)` and `x^(-2)`, respectively. |
| `2x`, `x(y+1)`, `(x+1)(y+1)` | Implicit multiplication with the same precedence as `*`. |
| `velocity`, `x2`, `xy` | Whole symbol names; `xy` does not split into `x*y`. |
| `sin(x)`, `log(2,x)`, `besselj(1,x)` | Known functions with the registry's argument order and folding rules. |

Names start with a letter or underscore and continue with letters, digits, or underscores. Unicode letters and whitespace are accepted; `−`, `×`, `·`, and `÷` are operator aliases. Adjacent numeric literals such as `1 2` are rejected. Function calls require parentheses.

### Registered custom functions

The parser resolves builtin names and aliases through `Function::builtin`, and custom names through the supplied context. It validates call arity and retains the registered identity. Parsing leaves custom calls symbolic; it does not execute bodies or partial callbacks.

```rust
use symb_anafis::{Context, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let response = ctx.register_function("response", UserFunction::new(2..=2))?;
    let parsed = ctx.parse("response(x, sin(x))")?;
    let x = ctx.symb("x")?;
    assert_eq!(parsed, response.call(vec![x.into(), x.sin()])?);
    assert!(ctx.parse("response(x)").is_err());
    Ok(())
}
```

An unknown `f(x)` denotes multiplication by a symbol `f`; it never creates an unregistered function identity. Register `f` before parsing a call. Variables and functions occupy separate namespaces: a known function followed by `(` is a call, even if a variable shares its name.

### Diagnostics and limits

`ParseError` implements `Display` and `core::error::Error`. Its variants distinguish invalid characters, invalid literals, missing operands, unexpected tokens, unclosed parentheses, function errors, and symbol allocation errors. Each located diagnostic carries a half-open UTF-8 byte `Span`.

```rust
use symb_anafis::{Context, ParseError, Span};

fn main() {
    let ctx = Context::new();
    assert_eq!(ctx.parse("x +"), Err(ParseError::ExpectedExpression {
        span: Span::new(3, 3),
    }));
    assert_eq!(ctx.parse("(x"), Err(ParseError::UnclosedParenthesis {
        span: Span::new(0, 1),
    }));
}
```

The parser consumes the complete input and requires balanced parentheses. Symbols created before an error remain in the context. Parsing uses heap-backed operator and group frames; flat additive and multiplicative chains are passed to n-ary constructors together. Memory usage scales with source structure and the resulting DAG. Core numeric capacity panics and allocator failures retain their existing contracts.

The grammar covers scalar arithmetic and known function calls. Differential notation such as `d^2 f(x)/dx^2`, equations, matrices, and binders are not part of this grammar; construct unevaluated derivatives through `Expr::derivative`.

## Uncertainty propagation

`Uncertainty::propagate(&model, &variables, &covariance)` constructs a real,
first-order uncertainty model. With sensitivity coefficients
\(g_i=\partial_{x_i}f\), its combined variance and standard uncertainty are

$$
u_f^2=g^T\Sigma g
=\sum_i g_i^2\Sigma_{ii}+2\sum_{i<j}g_i g_j\Sigma_{ij},\qquad
u_f=\sqrt{u_f^2}.
$$

These are the local linear propagation formulas in
[JCGM 100, section 5](https://www.iso.org/sites/JCGM/GUM/JCGM100/C045315e-html/C045315e_FILES/MAIN_C045315e/05_e.html).
The caller supplies real coordinates, a differentiable model at the point of
use, a finite real positive semidefinite covariance matrix, and the adequacy of
the linear approximation. The API rejects disproved scalar conditions but does
not prove positive semidefiniteness, regularity, or unknown symbolic assumptions.
It does not implement nonlinear or Monte Carlo propagation.

```rust
use symb_anafis::{Context, CovarianceMatrix, Expr, Uncertainty};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new();
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let covariance = CovarianceMatrix::new(vec![
        vec![Expr::number(4), Expr::number(1)],
        vec![Expr::number(1), Expr::number(9)],
    ])?;
    let result = Uncertainty::propagate(&(x + 2 * y), &[x, y], &covariance)?;
    assert_eq!(result.sensitivities(), &[Expr::number(1), Expr::number(2)]);
    assert_eq!(result.variance(), &Expr::number(44));
    assert_eq!(result.standard_deviation(), Expr::number(44).sqrt());
    assert_eq!(result.relative(), Expr::number(44).sqrt() / (x + 2 * y).abs());
    Ok(())
}
```

`CovarianceMatrix::new` accepts `Vec<Vec<Expr>>` and checks square shape and exact
symmetry. Symbolic pairs must have canonical identity; numeric pairs may have
equal values with different literal representations. There is no epsilon
tolerance. Numeric representations remain intact, and propagation retains
approximation supplied by either triangle. `get(row, column)` borrows an entry or
returns `None`; `dimension()` returns the coordinate count.

For independent coordinates, `CovarianceMatrix::diagonal` accepts **variances**,
not standard deviations, and stores only a linear number of entries. Numeric and
symbolic entries use the same `Expr` type:

```rust
use symb_anafis::{Context, CovarianceMatrix, Expr, Uncertainty};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new();
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let sigma_x = context.symb("sigma_x")?;
    let sigma_y = context.symb("sigma_y")?;
    let covariance = CovarianceMatrix::diagonal(vec![sigma_x.pow(2), sigma_y.pow(2)])?;
    let result = Uncertainty::propagate(&(x + 2 * y), &[x, y], &covariance)?;
    assert_eq!(result.variance(), &(sigma_x.pow(2) + 4 * sigma_y.pow(2)));
    Ok(())
}
```

Here the caller supplies real standard deviations. Constructing a covariance
never generates global symbols or infers independence from variable names.
Covariance order must match a list of distinct symbol identities; the same name
from another context denotes another coordinate. Other model symbols are held
fixed. Results own their model and derivative metadata.

`expression()` and `sensitivities()` expose the retained model and gradient.
`variance()` borrows the combined variance; `standard_deviation()` constructs its
square root; `relative()` constructs \(u_f/|f|\), preserving the exclusion
\(f=0\). A zero covariance never silently cancels an unproved sensitivity pole.
An empty coordinate list requires an empty covariance and produces zero variance.
Differentiation and propagation use ordinary constructors without running an
additional simplification sequence. A caller can simplify or compile the
returned expressions through the existing APIs.

The [Python API](PYTHON.md) exposes this model through `CovarianceMatrix` and
`Uncertainty` classes, alongside the other native subsystems.

## Numerical evaluation

### Direct evaluation

`Expr::evaluate` computes a scalar result from symbol–value bindings. It visits shared computed nodes once per call and uses an explicit stack for deep subgraphs. It does not compile a register program or substitute numeric expressions into the DAG.

```rust
use symb_anafis::{Context, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let expression = (x.pow(2) + y) / 3;

    let exact = expression.evaluate([
        (x, Number::from_int(2)),
        (y, Number::from_int(1)),
    ])?;
    assert_eq!(exact, Number::from_int(5) / Number::from_int(3));

    let approximate = expression.evaluate([(x, 2.0), (y, 1.0)])?;
    assert!((approximate - 5.0 / 3.0).abs() < 1.0e-15);
    Ok(())
}
```

Bindings use symbol identity. Symbols with the same display name in different contexts are distinct. Binding order is unrestricted; duplicate bindings return `EvalError::DuplicateParameter`. Unused bindings are permitted. Every symbol needed by the expression or an expanded custom body must be bound.

The selected scalar determines function domains. `Number` can promote supported real inputs to complex results, while `Real` and `f64` retain their real-domain behavior:

```rust
use symb_anafis::{Context, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let root = x.to_expr().sqrt();
    let value = root.evaluate([(x, Number::from_int(-4))])?;
    assert_eq!(value, Number::from_complex(Number::zero(), Number::from_int(2)));
    assert!(root.evaluate([(x, -4.0)])?.is_nan());
    Ok(())
}
```

For a constant without bindings, specify the scalar explicitly: `expression.evaluate::<Number>([])`.

### Compiled scalar programs

`Evaluator::<Value>::compile(&expression, &parameters)` builds a reusable register program. Parameters have an explicit order, including unused parameters. Repeated subexpressions share instructions, constants are initialized once, and temporary registers are reused after their last read.

```rust
use symb_anafis::{Context, Evaluator, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = ctx.parse("x^3 + 2*x + 1")?;

    let mut native = Evaluator::<f64>::compile(&expression, &[x])?;
    assert_eq!(native.evaluate(&[2.0])?, 13.0);
    assert_eq!(native.evaluate(&[3.0])?, 34.0);

    let mut exact = Evaluator::<Number>::compile(&expression, &[x])?;
    assert_eq!(exact.evaluate(&[Number::from_int(2)])?, Number::from_int(13));
    Ok(())
}
```

`evaluate` requires `&mut self` because the evaluator reuses its workspace. Cloning shares immutable instructions and copies an independent workspace. A program retains parameter metadata and remains usable after its context and input expression are dropped. `parameters()` returns borrowed symbol handles in input order; `instruction_count()` and `workspace_size()` report program instructions and initialized scalar slots.

Execution does not construct DAG nodes, resolve function names, or execute custom symbolic bodies. The scalar backend can allocate internally, for example for multiprecision values.

### Inspecting compiled instructions

`disassemble()` returns a borrowed display of the compiled program. It lists parameter identities, constant values, physical register operands, builtin names, and the result register. Formatting neither evaluates the program nor changes its workspace. The display itself does not allocate; `format!` or `to_string()` can collect the text when needed.

```rust
use symb_anafis::{Context, Evaluator};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let evaluator = Evaluator::<f64>::compile(&(x.sin() + 1), &[x])?;
    let listing = format!("{}", evaluator.disassemble());
    assert!(listing.contains("sin r0"));
    assert!(listing.contains("return r"));
    println!("{listing}");
    Ok(())
}
```

An instruction may overwrite a register after its last read. Listings describe the allocated numerical plan, including polynomial lowering and fused operations, rather than the original expression tree. Current input values and temporary contents are omitted. The text layout is not a serialization contract.

When sine and cosine share one compiled argument, their evaluations form a two-output `sin_cos` instruction. The same applies to `sinh_cosh`. The scalar backend owns shared numerical work and the precision of both components. The expression DAG retains its separate function nodes.

```rust
use symb_anafis::{Context, Evaluator};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = x.sin() + x.cos() + x.sinh() + x.cosh();
    let evaluator = Evaluator::<f64>::compile(&expression, &[x])?;
    let listing = format!("{}", evaluator.disassemble());
    assert!(listing.contains("sin_cos"));
    assert!(listing.contains("sinh_cosh"));
    Ok(())
}
```

### Scalar batches

`evaluate_batch` accepts one column per parameter and fills caller-owned output. Every column must have exactly the output length. Shape validation completes before any output is written. Execution is scalar.

```rust
use symb_anafis::{Context, Evaluator};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let mut evaluator = Evaluator::<f64>::compile(&(x.pow(2) + y), &[x, y])?;
    let xs = [1.0, 2.0, 3.0];
    let ys = [10.0, 20.0, 30.0];
    let mut output = [0.0; 3];
    evaluator.evaluate_batch(&[&xs, &ys], &mut output)?;
    assert_eq!(output, [11.0, 24.0, 39.0]);
    Ok(())
}
```

Empty batches are accepted. A zero-parameter program accepts no columns and repeats its result for every output element.

### Custom bodies

Custom function bodies construct symbolic expressions. Compilation expands each distinct call once, then lowers its result. Direct evaluation expands bodies during that evaluation and memoizes shared calls. Neither path treats a partial derivative callback as a numerical body.

```rust
use std::sync::Arc;
use symb_anafis::{Context, Evaluator, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let energy = ctx.register_function(
        "energy",
        UserFunction::new(2..=2).body(Arc::new(|args| {
            let [mass, velocity] = args else { return 0.into() };
            mass * velocity.pow(2) / 2
        })),
    )?;
    let m = ctx.symb("m")?;
    let v = ctx.symb("v")?;
    let expression = energy.call(vec![m.into(), v.into()])?;
    let mut evaluator = Evaluator::<f64>::compile(&expression, &[m, v])?;
    assert_eq!(evaluator.evaluate(&[4.0, 3.0])?, 18.0);
    Ok(())
}
```

Bodies must terminate and obey their declared dependencies. Recursive body definitions return `EvalError::RecursiveFunction`; missing bodies return `EvalError::MissingBody`. An unevaluated derivative returns `EvalError::UnevaluatedDerivative`. No numerical differentiation or unknown function implementation is inferred.

### Backends and floating arithmetic

| Scalar | Behavior |
| --- | --- |
| `Number` | Backend integer and rational arithmetic, floating components, and supported complex promotion. |
| `Real` | Backend real arithmetic and real function domains. |
| `Complex` | Backend complex scalar arithmetic and function domains. |
| `f64` | Native scalar arithmetic and NumAnaFis real function kernels. Non-real stored literals are rejected. |
| Custom `EvaluationValue` | A commutative `Numeric` implementation with a fallible conversion from `Number`. |

`EvaluationValue::from_number` supplies literal conversion; `Numeric` supplies arithmetic and mathematical functions. Unsupported literal domains or capacities return `None`. Compatible generic code can select a backend without changing the expression:

```rust
use symb_anafis::{Context, EvalError, EvaluationValue, Evaluator, Expr, Number, Real, Symbol};

fn evaluate_with<T: EvaluationValue>(
    expression: &Expr,
    symbol: Symbol<'_>,
    input: T,
) -> Result<T, EvalError> {
    Evaluator::<T>::compile(expression, &[symbol])?.evaluate(&[input])
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = x.pow(2) + 1;
    assert_eq!(evaluate_with(&expression, x, Number::from_int(3))?, Number::from_int(10));
    assert_eq!(evaluate_with(&expression, x, Real::from_int(3))?, Real::from_int(10));
    Ok(())
}
```

Generic programs retain the stored arithmetic grouping and backend power operations. The native `f64` compiler permits algebraic floating optimizations: positive integer multiplication chains, sparse Horner evaluation, bounded dense Estrin evaluation, shared reciprocals, and FMA when efficiently supported. These can change rounding, intermediate overflow or underflow, signed zeros, and nonfinite results relative to direct evaluation. Direct `f64` evaluation follows the stored DAG without these compiler rewrites. Neither path promises a user-selected mathematical simplification sequence.

Literal conversion and precision belong to the numerical backend. Converting an existing `f64` literal to a higher-precision type cannot recover digits already rounded away.

### Errors and resource use

Compilation validates symbol bindings, supported constants, body availability, register counts, and the completed instruction dataflow. Inputs are checked once per row or batch. Register instructions and their operand storage are private and immutable after compilation.

`EvalError` distinguishes duplicate parameters, unbound symbols, unsupported literals or constants, missing or recursive bodies, unevaluated derivatives, capacity failures, invalid compiled dataflow, and input-shape errors. Nonfinite numerical results follow the selected backend rather than becoming symbolic errors. User callback panics and backend panics propagate; a batch panic can leave completed output rows written.

The direct path allocates traversal and memoization storage per call. A compiled evaluator retains instructions, pooled operands, parameter metadata, and a fixed-length register workspace. Compilation and memory use scale with the stored DAG, custom body expansions, and generated arithmetic instructions; custom callbacks have no execution or allocation bound.
