# SymbAnaFis - Substitutions & Rewriting API

Reference for concrete substitutions, simultaneous replacements, pattern matching, rewrite rules, predicates, guards, callback replacements, and DAG folding in `symb_anafis`.

[← Back to API Index](../API.md)

---

### Contents

1. [Concrete substitution](#1-concrete-substitution)
2. [Simultaneous substitutions](#2-simultaneous-substitutions)
3. [Patterns and rewrite rules](#3-patterns-and-rewrite-rules)
4. [Predicates, guards, and search limits](#4-predicates-guards-and-search-limits)
5. [Callback replacements](#5-callback-replacements)
6. [Binomial expansion with a variable order](#6-binomial-expansion-with-a-variable-order)
7. [DAG transformation and folding](#7-dag-transformation-and-folding)

---

### 1. Concrete substitution

`Substitution::new(from, to)` defines a replacement of concrete expressions. `apply_all(&expression)` applies it throughout the expression with DAG memoization.

```rust
use symb_anafis::{Context, Expr, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
| $x + y \to u$ | $x + y + z$ | $u + z$ |
| $x + y \to u$ | $2x + 2y + z$ | $2u + z$ |
| $x^2 \to u$ | $x^5$ | $u^2 x$ |
| $2x \to u$ | $3x$ | $\frac{3}{2}u$ |
| $2x \to u$ | $x$ | $\frac{1}{2}u$ |
| $x + y \to u$ | $f(z)x + f(z)y$ | $f(z)u$ |
| $\sqrt{x} \to u$ | $x^{3/2}$ | $u^3$ |
| $x^2 \to u$ | $1/x^4$ | $1/u^2$ |

Matching respects symbol and function identities. It also finds monomials held in private polynomial storage.

Manifest common factors can be retained without dividing by a symbolic expression. The matcher does not invent division by a possibly zero symbolic value, introduce poles into a defined input, or extract arbitrary fractional roots. For example, $\sqrt{x^2}$ is not identified with $x$ for a general complex value.

A matching whole node takes precedence and its descendants are not traversed. Surviving original factors continue to be traversed; inserted destinations, generated coefficient quotients, and generated exponent counts do not become new sources in the same pass. `from()` and `to()` inspect the substitution.

#### Changes of variable

A change of variable supplies the chosen inverse $x \to g(u)$. The caller chooses its domain and branch. The core applies the explicit replacement without requiring a proof that it preserves the original domain.

For $u = x^2$, choosing $x = \sqrt{u}$ gives $x^5 = u^{5/2}$. Choosing $x = -\sqrt{u}$ gives $x^5 = -u^{5/2}$.

```rust
use symb_anafis::{Context, Number, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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

#### Substituting symbols

A `Substitution` or `Substitutions` whose sources are symbols replaces exactly those identities. Symbol sources form no implicit groups, so every stored edge is rebuilt once. `Expr::substitute(name, &value)` instead selects every symbol with that name, including symbols from different contexts. Neither changes the differentiation-variable metadata of a `Derivative` node.

```rust
use symb_anafis::{Context, Expr, Substitution, Substitutions};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let expression = x.pow(2) + y;
    let values = Substitutions::new([(x.into(), Expr::number(3)), (y.into(), Expr::number(4))])?;

    assert_eq!(values.apply_all(&expression), Expr::number(13));
    assert_eq!(Substitution::new(x.into(), y.into()).apply_all(&expression), y.pow(2) + y);
    Ok(())
}
```

Substituting values reconstructs and normalizes changed parents. Use [numerical evaluation](evaluation.md) to compute scalar results without rebuilding the expression.

### 2. Simultaneous substitutions

`Substitutions::new(entries)` validates a collection of `(Expr, Expr)` pairs. All sources are matched against original nodes, before rebuilding their children.

```rust
use symb_anafis::{Context, Substitutions, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
| $x \to y$, $y \to x$ | Simultaneous swap without cascading. |
| $f(x) \to c$, $x \to y$ | Original $f(x)$ becomes $c$ directly. |
| Repeated source, same canonical destination | Accepts one entry. |
| Repeated source, different destinations | `SubstitutionError::Conflict`. |
| Overlapping partial groups | Entry order determines priority; whole-node matches take precedence. |
| Destination contains its source | Terminates after one pass; does not revisit the destination. |
| No match | Preserves the canonical expression. |

`get(&source)` queries an exact entry; `iter()` yields distinct pairs in entry order; `len()` and `is_empty()` inspect the collection. An empty collection can also be constructed with `Substitutions::default()`.

### 3. Patterns and rewrite rules

`Pattern` describes a family of expressions. `RewriteRule` connects an input pattern to a destination constructed from the matched captures.

```rust
use symb_anafis::{Context, Pattern, RewriteRule, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let pair = ctx.register_function("pair", UserFunction::new(2..=2))?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(a.sin(), a.cos())?;
    let input = pair.call(vec![x.sin(), (x + y).sin()])?;

    assert_eq!(rule.apply_all(&input, usize::MAX, None)?, pair.call(vec![x.cos(), (x + y).cos()])?);
    Ok(())
}
```

This rule requests $\sin(a) \to \cos(a)$; it does not assert a mathematical identity. The core validates the structural operation, not equivalence between source and destination.

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
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let pair = ctx.register_function("pair", UserFunction::new(2..=2))?;
    let a = Pattern::capture("a", None);
    let b = Pattern::capture("b", None);
    let source = Pattern::apply(pair.clone(), vec![a.clone(), b])?;
    let first = RewriteRule::new(source, a)?;

    assert_eq!(first.apply_all(&pair.call(vec![x.into(), y.into()])?, usize::MAX, None)?, x.to_expr());
    Ok(())
}
```

#### Commutative groups and implicit factors

At a rule's root, sums and products can consume disjoint operand groups. Remaining operands are preserved automatically. Inside another pattern structure, a sum or product must cover the complete group; a capture may bind a nonempty group of operands.

```rust
use symb_anafis::{Context, Pattern, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let z = ctx.symb("z")?;
    let u = ctx.symb("u")?;
    let rule = RewriteRule::new(Pattern::exact(x + z), Pattern::exact(u))?;

    assert_eq!(rule.apply_all(&(x + y + z), usize::MAX, None)?, u + y);

    let angle = Pattern::capture("angle", None);
    let identity = RewriteRule::new(angle.sin().pow(2) + angle.cos().pow(2), Pattern::exact(1))?;
    assert_eq!(identity.apply_all(&(x.sin().pow(2) + x.cos().pow(2) + y), usize::MAX, None)?, y + 1);
    Ok(())
}
```

The matcher uses deterministic backtracking. It tries whole operands before counted blocks and grouped partitions. Integer powers, collected addends, and admissible rational groups do not need to be expanded into lists of copies.

Scalar coefficients can be inferred: `2*a` can match `x`, capturing `a = x/2`. Such a rule can match nearly every visited node, including literals. Use a function structure or a guard to restrict its scope when needed.

The branch, pole, and symbolic-factor restrictions described for `Substitution` also matter in rewriting. The matcher is not a prover of all algebraic equivalences.

#### Application order

`rule.apply_all(&expression, match_limit, guard)` returns `Result<Expr, RewriteError>`. It visits logical nodes bottom-up and matches rebuilt parents. Concrete simultaneous substitution selects original nodes before rebuilding children.

Inserted destinations are not traversed again in the same application. Every edge to a shared input node receives the same memoized result. One application is not automatic reduction to a fixed point; repeated rewriting requires an explicit caller policy.

### 4. Predicates, guards, and search limits

A predicate belongs to a capture and rejects candidates during search. A guard receives the complete set of captures and decides whether to accept that assignment. Rejection can make the matcher try another assignment.

| Callback | Logical type | Timing |
| --- | --- | --- |
| Capture predicate | `Fn(&Expr) -> bool + Send + Sync` | Before accepting a candidate capture. |
| Rule guard | `FnMut(&HashMap<&str, Expr>) -> bool` | After finding a complete assignment. |
| Dynamic destination | `Fn(&HashMap<&str, Expr>) -> Expr + Send + Sync` | After guard acceptance. |

Predicates and dynamic destinations must be pure and stable for equal inputs. The matcher may evaluate predicates repeatedly and reuse destinations for counted groups. Guards may run multiple times; external effects are not rolled back if the operation fails.

#### An identity requiring a realness proof

For real values, $\sqrt{a^2} = |a|$. This conclusion does not hold for an arbitrary complex value. The predicate below requires the proof before applying the rule:

```rust
use std::sync::Arc;
use symb_anafis::{Context, Pattern, Property, RewriteRule, Truth};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", Some(Arc::new(|node| {
        node.properties().get(Property::Real) == Truth::True
    })));
    let rule = RewriteRule::new(a.pow(2).sqrt(), a.abs())?;
    let real_base = x.abs() + 1;

    assert_eq!(rule.apply_all(&real_base.pow(2).sqrt(), usize::MAX, None)?, real_base.abs());
    let unknown = x.pow(2).sqrt();
    assert_eq!(rule.apply_all(&unknown, usize::MAX, None)?, unknown);
    Ok(())
}
```

#### Guarding a complete assignment

A guard can check relationships between captures or consult external proofs. Here, $\sqrt{a^2} \to a$ is allowed only when `a` is proved nonnegative:

```rust
use symb_anafis::{Context, Expr, HashMap, Pattern, Property, RewriteRule, Truth};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(a.pow(2).sqrt(), a)?;
    let mut guard = |captures: &HashMap<&str, Expr>| {
        captures.get("a").is_some_and(|value| {
            value.properties().get(Property::Nonnegative) == Truth::True
        })
    };

    assert_eq!(rule.apply_all(&x.abs().pow(2).sqrt(), 10_000, Some(&mut guard))?, x.abs());
    let input = x.pow(2).sqrt();
    assert_eq!(rule.apply_all(&input, usize::MAX, Some(&mut guard))?, input);
    Ok(())
}
```

#### Work limit

`match_limit` limits commutative attempts across the entire pass; `usize::MAX` is unbounded. Complete ordered matches consume no attempts; the limit is not a count of nodes, visits, or replacements.

If another attempt is needed after exhaustion, the result is `RewriteError::MatchLimitExceeded { limit }`. The input expression remains valid and no partially transformed output is published. The limit does not bound callback execution time, input memory, or generated destination size.

### 5. Callback replacements

`RewriteRule::new` accepts a destination `Pattern` directly, or `Replacement::Callback(Arc::new(...))` to construct the destination from captures.

The callback below expands $\sin(2a) = 2\sin(a)\cos(a)$. This mechanism is useful when the destination depends on calculations or structures generated in Rust:

```rust
use std::sync::Arc;
use symb_anafis::{Context, Pattern, Replacement, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(
        (2_i32 * a).sin(),
        Replacement::Callback(Arc::new(|captures| {
            let angle = &captures["a"];
            2 * angle.sin() * angle.cos()
        })),
    )?;

    assert_eq!(rule.apply_all(&(2 * x).sin(), usize::MAX, None)?, 2 * x.sin() * x.cos());
    Ok(())
}
```

The `captures["a"]` access relies on the name declared in the input pattern; the matcher supplies that capture to an accepted callback. Unlike a destination `Pattern`, a callback has no static validation of the capture names it accesses.

A counted assignment may reuse a single destination for multiple occurrences. Capture-free callbacks are evaluated lazily and, when needed, once per application. Do not use a callback to count occurrences or generate distinct identities for each edge.

### 6. Binomial expansion with a variable order

One rule can construct the $n + 1$ terms of

$$
(a+b)^n=\sum_{k=0}^{n}\binom{n}{k}a^{n-k}b^k.
$$

The example accepts literal integer exponents from 2 to 60. Coefficients are calculated using

$$
c_0=1,\qquad c_{k+1}=c_k\frac{n-k}{k+1}.
$$

This range keeps coefficients and intermediate products within the current backend's native 64-bit integer. It is not a matcher limit. Zero-exponent factors are omitted explicitly, avoiding constructions such as $a^0$ at the endpoints.

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, Number, Pattern, Replacement, RewriteRule, Substitutions};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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

    let result = expand.apply_all(&(x + y).pow(4), usize::MAX, None)?;
    let expected = x.pow(4) + 4 * x.pow(3) * y
        + 6 * x.pow(2) * y.pow(2) + 4 * x * y.pow(3) + y.pow(4);
    assert_eq!(result, expected);

    for degree in 2..=60 {
        let expanded = expand.apply_all(&(x + y).pow(degree), usize::MAX, None)?;
        let at_one = Substitutions::new([(x.into(), Expr::number(1)), (y.into(), Expr::number(1))])?
            .apply_all(&expanded);
        assert_eq!(expanded.children().len(), usize::try_from(degree + 1)?);
        assert_eq!(at_one, Expr::number(1_i64 << degree));
    }
    Ok(())
}
```

For $n = 2$, the rule generates $a^2 + 2ab + b^2$; for $n = 3$, it generates $a^3 + 3a^2b + 3ab^2 + b^3$. The bases may be expressions such as `x.sin()` and `y.exp()` instead of individual symbols.

Expansion is explicit: constructing `(x + y).pow(n)` does not run it. When a sum has more than two terms, captures can represent groups; one pass does not re-expand newly generated powers of those groups. A normalized negative exponent can have its positive denominator power expanded during traversal.

The same mechanism can generate finite sums, recurrences, and truncated series. A general Taylor series additionally requires a way to obtain derivatives; the core does not yet provide that engine.

### 7. DAG transformation and folding

`Expr::map` transforms logical nodes in children-before-parent order:

```text
map(|rebuilt_node| -> Result<Option<Expr>, Error> { ... })
```

`Ok(None)` retains the rebuilt node; `Ok(Some(expr))` replaces it; `Err(error)` interrupts the operation. The callback sees the node after its children are reconstructed. Each distinct original logical node is visited once; inserted destinations are not revisited.

```rust
use core::convert::Infallible;
use symb_anafis::{Context, ExprView};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
