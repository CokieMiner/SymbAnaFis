# SymbAnaFis - Simplification API

Reference for simplification presets, custom sequences, targets, scoring, limits, and domain safety modes in `symb_anafis`.

[← Back to API Index](../API.md)

---

### Contents

1. [Presets and forms](#1-presets-and-forms)
2. [Custom sequences and steps](#2-custom-sequences-and-steps)
3. [Compilation preparation](#3-compilation-preparation)
4. [Targets, scores, and limits](#4-targets-scores-and-limits)
5. [Domain modes](#5-domain-modes)

---

### 1. Presets and forms

`Expr` provides direct, canonical methods for the standard simplification presets:
- `expr.compact()?`: minimizes logical size using algebraic identities and factoring.
- `expr.expand()?`: applies bounded distributive expansions.
- `expr.factor()?`: extracts common factors across polynomial terms.

For multi-step pipelines, custom scoring, iteration inspection, or execution limits, `SimplificationSequence` provides an ordered, reusable program over the transformation engine. Builtin rule plans compile during sequence construction and are shared by clones, so the same sequence can process many expressions without recompiling patterns.

```rust
use symb_anafis::{Context, Expr, SimplificationSequence};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let expression = ctx.parse("sin(x)^2 + cos(x)^2")?;

    // 1. Direct convenience methods on Expr:
    assert_eq!(expression.compact()?, Expr::number(1));
    assert_eq!((x + 1).pow(2).expand()?, x.pow(2) + 2 * x + 1);
    assert_eq!((x.pow(2) + x * y).factor()?, x * (x + y));

    // 2. Full sequence presets with metadata:
    let compact = SimplificationSequence::compact().simplify(&expression)?;
    assert_eq!(compact.expression(), &Expr::number(1));
    assert_eq!(compact.into_expression(), Expr::number(1));

    let expanded = SimplificationSequence::expanded().simplify(&(x + 1).pow(2))?;
    assert_eq!(expanded.into_expression(), x.pow(2) + 2 * x + 1);

    let factored = SimplificationSequence::factored().simplify(&(x.pow(2) + x * y))?;
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

---

### 2. Custom sequences and steps

Append builtin passes, existing `RewriteRule` values, or mapping callbacks with `then`. Inspect and clone `steps()` to extend or reorder a preset.

```rust
use symb_anafis::{Context, Expr, Pattern, RewriteRule, SimplificationSequence, SimplificationStep};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
    let ctx = Context::new()?;
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

---

### 3. Compilation preparation

The compilation preset prepares expressions for numerical lowering. It selects native forms such as `exp_m1`, `ln_1p`, and `erfc`, and factors common polynomial terms into nested arithmetic. The `Compilation` step can also be used alone in a custom sequence.

```rust
use symb_anafis::{Context, SimplificationSequence, SimplificationStep, SimplificationTarget};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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

The compilation objective sums estimated operations once per distinct logical DAG node, so existing shared function results count once. Addition and multiplication cost one per binary operation, division costs four, and positive integer powers use the multiplication count for binary exponentiation. General powers cost 24; simple scalar functions (`abs`, `floor`, `ceil`, `round`, `trunc`, `fract`, `signum`) cost one, square roots four, cube roots eight, other function calls 16, and retained derivatives 32. Literal and symbol leaves cost zero. These portable weights are a heuristic; a backend can provide its own `score_with` function.

---

### 4. Targets, scores, and limits

`target(SimplificationTarget::Compact | Expanded | Factored | Compilation)` changes candidate selection and clears a custom scorer. It leaves the steps unchanged. `score_with(|expression| expression.node_count())` supplies a pure primary score; logical size breaks ties. Equal complete scores select the latest candidate.

Every step receives the preceding step's output. Temporary growth remains available to later cancellation. The input and every completed step compete for the returned result. `Simplification` exposes `expression()`, `into_expression()`, `iterations()`, and `termination()`; the termination reason is `FixedPoint`, `Cycle`, or `IterationLimit`. A selected earlier candidate need not itself be a fixed point.

| Setting | Default | Scope |
| --- | --- | --- |
| `max_iterations` | 16 | Complete sweeps; zero returns the input without a sweep. |
| `max_nodes` | `usize::MAX` (unbounded) | Distinct stored nodes in input, intermediate stage outputs, and completed candidates. |
| `max_depth` | `usize::MAX` (unbounded) | Stored depth, counting the root (validated alongside `max_nodes`). |
| `match_limit` | `10_000` | Candidate attempts per rule in each core rewrite pass; `usize::MAX` is unbounded. |
| `expansion_limit` | `usize::MAX` (unbounded) | Cumulative Cartesian terms generated in one expansion pass. |
| `polynomial_limit` | `usize::MAX` (unbounded) | Shared sparse conversion, term-product, and GCD remainder work per algebraic pass. |

An exceeded node, depth, expansion, polynomial, or `Rewrite` match bound returns `SimplifyError` and discards partial output. A builtin rule that exhausts `match_limit` is dropped and its step's pass restarts without it. Callback panics propagate. Callback allocations and work remain caller-controlled.

---

### 5. Domain modes

`DomainSafety::Preserving` is the default for every preset. Builtin identities use conservative `Properties` proofs and preserve principal branches and poles. Thus an unconstrained `x/x` remains a quotient, `ln(exp(x))` needs realness, and `sqrt(x^2)` becomes `abs(x)` only for a proved real input. Targets and custom scores do not relax these guards.

`domain_safety(DomainSafety::AssumptionDependent)` also permits explicitly tagged identities with caller-supplied nonzero, realness, positivity, and principal-range hypotheses. For example, cancellation assumes the removed factor is nonzero; `asin(sin(x)) = x` assumes a real input in `[-pi/2, pi/2]`. A tested false condition is rejected. This mode does not prove the caller's assumptions and can change the represented domain. Custom rules and mapping callbacks keep their caller-owned contract in both modes.

```rust
use symb_anafis::{DomainSafety, Expr, SimplificationSequence};

fn main() -> Result<(), symb_anafis::SimplifyError> {
    let x = Expr::symbol("assumed_nonzero_x").expect("valid symbol");
    let preserving = SimplificationSequence::compact();
    assert_eq!(preserving.simplify(&(&x / &x))?.into_expression(), &x / &x);
    // The caller supplies x != 0 for this cancellation.
    let assumed = preserving.domain_safety(DomainSafety::AssumptionDependent);
    assert_eq!(assumed.simplify(&(&x / &x))?.into_expression(), Expr::number(1));
    Ok(())
}
```

The [rule reference](../../src/simplification/RULES.md) lists the formulas, conditions for both modes, and preset step membership. The [subsystem guide](../../src/simplification/README.md) describes traversal, costs, and limits.
