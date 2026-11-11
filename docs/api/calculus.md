# SymbAnaFis - Calculus & Uncertainty API

Reference for symbolic differentiation, gradients, Jacobians, Hessians, and first-order uncertainty propagation in `symb_anafis`.

[← Back to API Index](../API.md)

---

### Contents

1. [Symbolic differentiation](#1-symbolic-differentiation)
2. [Multivariable calculus](#2-multivariable-calculus)
3. [Uncertainty propagation](#3-uncertainty-propagation)

---

### 1. Symbolic differentiation

`Expr::derivative(inner, variable, order)` evaluates

$$
\frac{\partial^{\mathrm{order}}}{\partial \mathrm{variable}^{\mathrm{order}}}
\mathrm{inner}.
$$

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
    let ctx = Context::new()?;
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

Analytic formulas apply where their principal-branch factors are regular. The returned expression does not store the input's domain restrictions or extend formulas through removable singularities; evaluations at exceptional parameter values can require limits. Generic complex absolute values and nonsmooth rounding functions remain unevaluated. Formulas for sinc and complete elliptic integrals require a proved nonzero argument to avoid introducing a removable pole at zero. Polygamma and zeta derivative orders, and Hermite degrees, require supported exact nonnegative integer literals. Differentiating a varying Bessel order, a varying Lambert branch, or an associated Legendre function remains symbolic.

Custom calls use their registered partials and the multivariate chain rule. The engine checks all needed partials before executing any callback for that call. Constant arguments need no partial, and function bodies are not expanded implicitly.

```rust
use std::sync::Arc;
use symb_anafis::{Context, Expr, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
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
    let ctx = Context::new()?;
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

### 2. Multivariable calculus

`expression.gradient(&variables)`, `outputs.jacobian(&variables)` (via the [`Jacobian`](#multivariable-calculus) trait),
`Expr::jacobian(&outputs, &variables)`, and `expression.hessian(&variables)` return owning vectors in the requested coordinate
order:

$$
g_i=\partial_{x_i}f,\qquad
J_{ij}=\partial_{x_j}f_i,\qquad
H_{ij}=\partial_{x_j}(\partial_{x_i}f).
$$

```rust
use symb_anafis::{Context, Expr, Jacobian};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let model = x.pow(2) + x * y;
    assert_eq!(model.gradient(&[x, y]), vec![2 * x + y, x.to_expr()]);
    assert_eq!(model.hessian(&[x, y]), vec![
        vec![Expr::number(2), Expr::number(1)],
        vec![Expr::number(1), Expr::number(0)],
    ]);

    // Using the Jacobian trait on arrays, slices, or vectors:
    let expected_j = vec![
        vec![2 * x + y, x.to_expr()],
        vec![y.to_expr(), x.to_expr()],
    ];
    assert_eq!([model, x * y].jacobian(&[x, y]), expected_j);

    // Or via the associated function Expr::jacobian:
    let outputs = vec![x.pow(2) + x * y, x * y];
    assert_eq!(Expr::jacobian(&outputs, &[x, y]), expected_j);
    Ok(())
}
```

Jacobians store outputs as rows and coordinates as columns. All outputs are
numbered once, so each column differentiates every distinct node, shared subtrees
included, once; a gradient numbers its expression once for all coordinates.
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

### 3. Uncertainty propagation

`Uncertainty::propagate(&model, &variables, &covariance)` constructs a real,
first-order uncertainty model. With sensitivity coefficients
$g_i = \partial_{x_i}f$, its combined variance and standard uncertainty are

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
    let context = Context::new()?;
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
    let context = Context::new()?;
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
square root; `relative()` constructs $u_f/|f|$, preserving the exclusion
$f = 0$. A zero covariance never silently cancels an unproved sensitivity pole.
An empty coordinate list requires an empty covariance and produces zero variance.
Differentiation and propagation use ordinary constructors without running an
additional simplification sequence. A caller can simplify or compile the
returned expressions through the existing APIs.
