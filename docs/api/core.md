# SymbAnaFis - Core Expression API

Reference for expressions, canonical normalization, numbers, symbols, mathematical functions, properties, formatting, and memory management in `symb_anafis`.

[← Back to API Index](../API.md)

---

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
11. [Formatting](#11-formatting)
12. [Errors, memory, and limits](#12-errors-memory-and-limits)

---

### 1. Configuration and features

Add `symb_anafis` to your `Cargo.toml`:

```toml
[dependencies]
symb_anafis = "0.9"
```

| Feature | Behavior |
| --- | --- |
| No features | `no_std + alloc`; dynamically allocated collections and internal spin mutexes. |
| `std` | Enables `num_anafis/std` and blocking mutexes with recovery after poisoning. |
| `num-traits` | Implements `num_traits::Zero` and `num_traits::One` for `Expr`. |

For an application using the standard library:

```toml
[dependencies]
symb_anafis = { version = "0.9", features = ["std"] }
```

The `no_std` build requires a global allocator and pointer-width atomics, used by `Arc` and identity sequences. In the examples, `std::sync::Arc` can be replaced with `alloc::sync::Arc`; `Vec` and `vec!` come from `alloc` in those environments.

The `num-traits` feature is independent of `std`. The following example also requires a direct `num-traits` dependency in the application:

```toml
[dependencies]
symb_anafis = { version = "0.9", features = ["num-traits"] }
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
    let ctx = Context::new()?;
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
    let ctx = Context::new()?;
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
- Collection of equal terms, such as $x^3 + 2x^3 = 3x^3$.
- Collection of positive integer powers, such as $x^2 x^3 = x^5$.
- Collection of nested positive integer powers, such as $(x^2)^3 = x^6$, when the combined exponent fits.
- Exact scalar division represented by multiplicative coefficients, such as $x/2 = (1/2)x$.
- Numeric coefficients outside shared symbolic quotients, such as $(2x)/(3y) = (2/3)(x/y)$.
- Representation of known negative real exponents through denominators.
- Private polynomial compaction of groups with repeated positive integer powers.

```rust
use symb_anafis::{Context, Expr, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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

The chosen form of exact $x^{1/2}$ is `sqrt(x)`. `exp(x)` represents $e^x$; `log(e, x)`, `log(2, x)`, and `log(10, x)` use `ln`, `log2`, and `log10`. `exp_neg(x)` uses `exp(-x)`. A float exponent retains its approximate representation.

Normalization preserves domain exclusions. In general, $x/x$, $x^0$, and $0f(x)$ are retained without a sufficient proof. Construction also does not expand $(x+y)^n$ automatically or apply arbitrary trigonometric identities.

Polynomial storage is private. A polynomial remains an expression inspected as a sum; constant terms belong to the outer sum. Users do not choose between `Sum` and `Poly`.

Exact quotients use $(aA)/(bB) = (a/b)(A/B)$, with the numeric coefficient outside the shared symbolic quotient. This includes rational and complex scalars, chained divisions, and coefficients within denominator products when the divisor is nonzero and the coefficient can be computed within backend capacity. Different numeric multiples share the same quotient node. Unit coefficients and unit denominators are omitted.

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
| Mathematical constant $\pi$ | `Expr::pi()` |
| Mathematical constant $e$ | `Expr::e()` |

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

Literal identity preserves representation: integers and floats remain distinct, as do float zeros with different signs and NaNs with different payloads. This differs from the numeric equality provided by the backend. Normalization omits only the exact identities `0` and `1`: `1.0*x` and `x + 0.0` keep their approximate literal, a substitution matched with an approximate scale keeps that scale, and rendering shows an approximate unit coefficient (`1*x`).

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
    let first = Context::new()?;
    let second = Context::new()?;
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
        let ctx = Context::new()?;
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
| `symbol_names()`, `symbol_count()` | Lists sorted names or counts local bindings. |
| `remove_symbol(name)`, `clear_symbols()` | Removes bindings; retains issued identities. |
| `clear_functions()`, `clear_all()` | Removes function bindings, or both kinds of binding. |
| `is_empty()` | Tests for the absence of symbol and function bindings. |
| `reset(&mut self)` | Releases this owner's namespace and creates another. |

Removing names does not release metadata for issued symbols while the namespace owns it. Drop the context or use `reset()` to release that ownership. Other clones retain the previous namespace; surviving expressions retain the metadata they reference. Rust prevents `reset()` when a borrowed symbol will still be used.

```rust
use symb_anafis::Context;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let mut ctx = Context::new()?;
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
| Query names | `Symbol::names()`, `Symbol::count()` |
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
| Lambert W | `lambertw(branch)` |
| Magnitude and sign | `abs()`, `signum()`, `sinc()` |
| Rounding | `floor()`, `ceil()`, `round()`, `trunc()`, `fract()` |

The receiver follows these conventions:

| Rust syntax | Mathematical call |
| --- | --- |
| `x.log(b)` | $\log_b(x)$ |
| `y.atan2(x)` | $\operatorname{atan2}(y, x)$ |
| `x.besselj(n)` | $J_n(x)$ |
| `x.polygamma(n)` | $\psi^{(n)}(x)$ |
| `x.zeta_deriv(n)` | $\zeta^{(n)}(x)$ |
| `x.hermite(n)` | $H_n(x)$ |
| `x.assoc_legendre(l,m)` | $P_l^m(x)$ |
| `z.lambertw(k)` | $W_k(z)$ |

`lambertw` takes the branch explicitly; there are no separate branch methods or public `ynm` alias. Numerical evaluation and projections of discrete parameters follow the NumAnaFis backend; symbolic construction does not validate every mathematical domain.

#### Known function identities

`Function::builtin(name)` resolves only recognized names and aliases. It does not create unknown functions. `Function::call` validates the argument count and constructs a normalized expression.

```rust
use symb_anafis::{Context, Function, FunctionError};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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

The example defines $q(x, y) = x^2 + y^2$, with $\partial q/\partial x = 2x$ and $\partial q/\partial y = 2y$:

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, ExprView, FunctionError, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
| Inspect a definition | `arity()` |
| Borrow callbacks | `body_fn()`, `partial_fn(index)`, `None` when absent |
| Register | `ctx.register_function(name, definition)?` |
| Query local registrations | `ctx.get_function(name)` |
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
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let expression = x.pow(2) + x.pow(3) + 1;
    match expression.view() {
        ExprView::Sum(terms) => {
            assert_eq!(terms.len(), 3);
            assert_eq!(Expr::sum(terms.cloned().collect()), expression);
        }
        _ => return Err("expected a sum".into()),
    }

    let children = expression.children();
    assert_eq!(children.len(), 3);
    assert_eq!(Expr::sum(children.cloned().collect()), expression);
    assert_eq!(x.to_expr().as_symbol(), Some(x));
    assert_eq!(x.to_expr().symbol_name(), Some("x"));
    Ok(())
}
```

`children()` returns `ExprChildren`, a cloneable, exact-size, fused iterator over borrowed `&Expr` children; `.cloned()` gives owning handles. Counting does not materialize monomials. The terms of compact polynomial storage are built once, when first iterated, and are then kept with the expression.

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
    let first = Context::new()?;
    let second = Context::new()?;
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
    let ctx = Context::new()?;
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

Symbols represent finite complex scalars, without additional assumptions about realness, sign, integrality, or being nonzero. Consequently, $x^2$ is not automatically proved nonnegative, nor $\exp(x)$ positive. The API currently attaches no assumptions to symbols or contexts.

Properties describe mathematical values; they do not guarantee absence of overflow or rounding during numerical evaluation.

### 11. Formatting

| API | Format |
| --- | --- |
| `Display`, `to_string()`, `format!("{expression}")` | Mathematical text. |
| `to_latex()` | LaTeX. |
| `to_unicode()` | Unicode mathematical text. |
| `Debug` for `Expr` | Mathematical representation of the expression. |

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let expression = (x.pow(2) + Expr::pi()) / (x + 1);

    println!("{expression}");
    println!("{}", expression.to_latex());
    println!("{}", expression.to_unicode());
    Ok(())
}
```

Formatting presents mathematical structure; it is not a serialization format for identities, IEEE representations, or namespaces. Distinct symbols with equal names can produce identical text. Parsing formatted text therefore does not guarantee identity round-tripping.

### 12. Errors, memory, and limits

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

All implement `core::error::Error` and `Display`. Public error enums are non-exhaustive. Parsing diagnostics are described in [Parsing](parsing.md); `Span` records their half-open UTF-8 byte ranges.

#### Boundaries that can panic

- Symbol and context constructors (`Context::new`, `Context::reset`, `Context::symb`, `Symbol::intern`, `Symbol::anon`, `Expr::symbol`) return `SymbolError::IdExhausted` when no identity can be allocated.
- Literal folding uses checked arithmetic: an exact result beyond native capacity keeps its operation symbolic. This core does not provide arbitrary precision.
- Predicates, guards, bodies, partial derivatives, and replacement callbacks belong to the caller; their panics propagate when executed.
- Allocation uses the program's allocator. The API does not convert all allocation failures into `Result`.

There is no universal promise of panic-free execution or a fixed memory bound.

#### Ownership and memory growth

Expressions own nodes shared through `Arc`; the interner holds weak references. Expression destruction dismantles the DAG iteratively, and nodes release ownership of their contents after their final owner disappears. Weak-entry reclamation and retained table capacity do not imply immediately returning all memory to the operating system.

Global symbol metadata is permanent. Local contexts retain issued identities until their namespace loses its final owner; `clear_symbols()` removes names, while `reset()` or destruction allows ownership to be released. Live expressions retain the metadata they use.

Registered functions and callbacks also have shared ownership. A callback capturing a clone of the context that registers it can create an `Arc` cycle; use weak references or remove that cycle's bindings before abandoning external owners.

Memory usage depends on live nodes, symbols, definitions, table capacities, matching search, and generated destinations. The library imposes no global quotas. `match_limit` controls matching attempts, not expansion size; a resource policy should also bound inputs and callback work.

Without `std`, a spin mutex requires its lock owner to be able to continue running. Reentering registry operations from an interrupt that suspended that owner is unsupported. With `std`, synchronization uses blocking mutexes.
