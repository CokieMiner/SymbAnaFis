# Python API

The Python package exposes classes over the native Rust core. Operations belong
to `Context`, `Expr`, `Function`, `Pattern`, `RewriteRule`,
`SimplificationSequence`, `Evaluator`, and the uncertainty types. The package
does not export standalone mathematical functions.

## Build and verification

From the repository root, with the same sibling NumAnaFis checkout used by the
Rust crate:

```sh
python -m venv .venv
source .venv/bin/activate
python -m pip install maturin
maturin develop --release
python -m unittest discover -s python/tests
```

`pyproject.toml` selects `bindings/python/Cargo.toml`. The extension is
`symb_anafis._native`; the public classes and type annotations live in
`python/symb_anafis`. Python requires 3.9 or later. The Rust crate's default
`no_std + alloc` build has no Python dependency. Binding development additionally
uses `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

## Expressions and scoped symbols

```python
from fractions import Fraction
from symb_anafis import Context, Expr, Number, Property, Truth

context = Context()
x, y = context.symbols("x", "y")
model = context.parse("x^2 + x*y")
assert model == x**2 + x*y
assert model.gradient([x, y]) == [2*x+y, x]
assert model.hessian([x, y]) == [[Expr(2), Expr(1)], [Expr(1), Expr(0)]]
assert Expr.jacobian([model, x*y], [x, y]) == [[2*x+y, x], [y, x]]
assert x.properties[Property.REAL] is Truth.UNKNOWN
assert x.abs().properties[Property.NONNEGATIVE] is Truth.TRUE
```

`Context.symb(name)` gets or creates one local identity. `Context.symbols(*names)`
returns a tuple in the requested order. The `context.symbol_names` property
returns a `tuple[str, ...]` of currently bound symbol names.
`Context.get_symbol(name)`, `Context.remove_symbol(name)`,
`Context.clear_symbols()`, and `Context.reset()` manage bindings; existing
symbols, expressions, and function handles keep owning their metadata. Equal
names in different contexts remain different identities. `Symbol(name)` and
`Expr("formula")` use `Context.default()` unless a context is supplied explicitly.

Arithmetic accepts `Expr`, `Symbol`, `Number`, integers, floats, complex values,
and `fractions.Fraction`. Strings supplied to `Expr` are parsed. `Expr.sum` and
`Expr.product` batch n-ary construction. `Expr.pi()`, `Expr.e()`, and `Expr.i()`
construct constants independently of symbol names.

Equality and hashing describe canonical **Expr structure**, including literal
representation: `Expr(2) != Expr(2.0)`, and `Expr(2) != 2`. Compare with `Expr(2)`
or inspect a literal's `.value`. Symbolic expressions and unknown `Truth` values
cannot be converted implicitly to `bool`.

`Expr.derivative(symbol, order=1)` and multivariable methods use the Rust
engine's regular principal-branch formulas. Unsupported derivatives stay
symbolic. Jacobian rows are outputs and columns are coordinates; Hessians retain
mixed operator order. Repeated calculus coordinates repeat components. No extra
simplifier runs implicitly.

`kind`, `args`, `symbols`, `properties`, `node_count`, `max_depth`, and `view`
inspect logical structure. Views own their children and derivative metadata.
`to_latex()`, `to_unicode()`, and notebook `_repr_latex_()` format expressions.
Formatting does not serialize scoped identities.

## Numeric representations

```python
exact = Number(Fraction(7, 13))
assert exact.real == Fraction(7, 13) and exact.is_exact
components = Number.complex(Fraction(1, 3), Fraction(2, 5))
assert components.real == Fraction(1, 3)
assert components.imag == Fraction(2, 5)
assert components.is_exact
assert not Number(2.0).is_exact
```

`Number` retains NumAnaFis's native signed 64-bit integer and reduced rational
representations, with positive unsigned 64-bit denominators. Oversized Python
integer inputs raise `OverflowError`. Floats stay approximate; complex
components can stay exact. Booleans are rejected as numeric input.

`Number.real` and `.imag` return Python integers, fractions, or floats.
`float(number)`, `complex(number)`, and conversion of nonreal values through
`to_python()` explicitly request approximation. Arithmetic follows NumAnaFis
domain and capacity rules. Numeric equality compares represented values; Number
is unhashable. Compatible NumPy scalar types are accepted through Python's
numeric protocols without making NumPy a package dependency.

## Custom functions

```python
square = context.register_function("square", 1, body=lambda value: value**2)
assert square.arity == (1, 1)
assert square(x).derivative(x) == 2*x
assert square(x).evaluate({x: 3}) == 9

product = context.register_function("product", 2,
    body=lambda first, second: first*second,
    partials={0: lambda first, second: second, 1: lambda first, second: first})
assert product(y, x).derivative(x) == y
```

Builders execute once with positional symbolic parameters. Their expressions
become owning native templates; later differentiation and evaluation do not call
Python builders. Omitting `partials` derives supported partials from a supplied
body; unsupported partials remain symbolic with the caller's differentiation
variable. Passing an explicit sparse dictionary, including `{}`, preserves that
partial coverage. Without a body, calls can remain symbolic but cannot be
evaluated numerically.

Bodies and explicit partial templates cannot contain unresolved differential
operators. Scalar argument substitution does not transform differentiation
variables, so registration rejects such templates with `ValueError`.

Plain expression templates retain their existing symbol identities. Supply
`parameters=[a, b, ...]` to make those symbols the function arguments; other
symbols stay fixed. Arguments are replaced simultaneously, so swaps do not
cascade. Registration fixes arity and rejects duplicate or builtin names.
`Function.builtin(name)` resolves a known builtin; unknown names do not create
identities. Function objects also accept Pattern arguments.

The `function.arity` property returns `(min_arity, max_arity)`, with inclusive
bounds; a fixed unary function returns `(1, 1)`. The `context.functions` property
returns a `tuple[Function, ...]` of currently registered custom functions.
`context.get_function(name)` returns a registered function or `None`.
`context.remove_function(name)` removes its binding and returns whether it
existed; existing function handles and expressions retain their definitions.

## Replacements and rewrite rules

```python
from symb_anafis import Pattern, RewriteRule, Substitution, Substitutions

assert Substitutions({x: y, y: x}).apply(x**2+y) == y**2+x
assert Substitution(x+y, y).apply((x+y)**2) == y**2

angle = Pattern.capture("angle")
identity = RewriteRule(angle.sin()**2 + angle.cos()**2, 1)
assert identity.apply(x.sin()**2 + x.cos()**2 + y) == 1+y

rewrite = RewriteRule(angle.sin(), lambda captures: captures["angle"].cos())
assert rewrite.apply(x.sin()) == x.cos()
```

`Substitutions` accepts a mapping or iterable of pairs and applies entries
simultaneously without revisiting destinations. Conflicting duplicate sources
raise `ValueError`. `Expr.substitute(source, destination)` applies one entry.

Patterns support arithmetic and the same mathematical methods as Expr.
`Pattern.capture(name, properties=[Property.REAL, ...])` requires conservative
proofs before accepting a candidate; unknown facts fail the condition.
`Pattern.exact(expr)` binds a concrete identity. Unmatched root operands survive.

`RewriteRule.apply(expr, match_limit=100_000, guard=None)` accepts a Python guard
over captured expressions. `None` removes the candidate search bound. The bound
counts candidate permutations; an already ordered complete match can need no
permutation attempts. Replacement callbacks run after guard acceptance.
Rules, guards, replacement callbacks, and custom identities must be pure and
mathematically valid under the caller's assumptions.

`Expr.map(callback)` visits rebuilt logical nodes once, with `None` retaining a
node. `Expr.fold(initial, callback)` reduces shared nodes in postorder. Generated
destinations are not revisited in the same pass. Python exceptions propagate;
provisional transformed expressions are never published after callback failure.
Callback side effects cannot be rolled back.

## Simplification sequences

```python
from symb_anafis import (
    DomainSafety, SimplificationSequence, SimplificationStep,
)

compact = SimplificationSequence.compact()
assert compact.simplify(x.sin()**2+x.cos()**2).expression == Expr(1)
assert SimplificationSequence.expanded().simplify((x+1)**2).expression == x**2+2*x+1
assert SimplificationSequence.compilation().simplify(x.exp()-1).expression == x.exp_m1()

custom = (SimplificationSequence()
          .then(identity)
          .then(SimplificationStep.FACTOR)
          .score_with(lambda expression: expression.node_count))
assert custom.simplify(x.sin()**2+x.cos()**2).expression == Expr(1)
```

Presets are `compact`, `expanded`, `factored`, and `compilation`. An ordinary
constructor starts with no steps. `then` accepts a builtin step, a RewriteRule,
or a mapping callback. Builders return independent sequences; `.steps` exposes
an immutable tuple that can seed another sequence. Native builtin/template plans
are cached across applications.

`target`, `domain_safety`, `score_with`, `max_iterations`, `max_nodes`,
`max_depth`, `match_limit`, `expansion_limit`, and `polynomial_limit` select the
same contracts as Rust. `DomainSafety.PRESERVING` is the default;
`ASSUMPTION_DEPENDENT` permits tagged identities whose hypotheses belong to the
caller. Custom rules and maps always retain caller-owned validity.

`simplify` returns a frozen `Simplification(expression, termination, iterations)`.
It selects the best visited candidate, not necessarily the last pass's output
or a global minimum. `Expr.simplify(sequence=None)` returns only the selected
expression and reuses the default compact plan when omitted.

## Numerical programs and batches

```python
from symb_anafis import Evaluator

program = Evaluator(x+2*y, [x, y])
assert program.evaluate([3, Fraction(1, 2)]) == 4
assert program.evaluate_batch([[1, 2], [3, 4]]) == [7, 10]
floating = Evaluator(x.sin(), [x], backend="f64")
assert floating.evaluate([0.0]) == 0.0
assert Evaluator(7, []).evaluate_batch([], rows=3) == [7, 7, 7]
```

`Expr.evaluate(bindings=(), *, backend="number")` performs direct evaluation
with a mapping or iterable of `(Symbol, value)` pairs.
`Expr.compile(parameters=None, *, backend="number")` constructs an Evaluator.
In both methods, `backend` must be passed by keyword. Omitted parameters use the
model's symbol identities in ID order. Explicit ordering is preserved.

The `number` backend returns Number objects and preserves exact/complex tiers;
`f64` returns Python floats and uses the native compiler's FMA/reassociation
policy. Rounding and intermediate overflow may differ from direct evaluation.

Batch input contains one column per parameter, including unused parameters.
Shapes are checked before execution. Inputs are copied into native values and
results are returned as lists; NumPy arrays can supply iterable columns. Batch
execution is scalar. `rows` specifies the output count for zero-parameter
programs. `parameters`, `instruction_count`, `workspace_size`, and `disassemble()`
inspect the compiled program. Native work releases the GIL. Use `copy()` for
independent workspaces when evaluating concurrently; concurrent mutation of one
workspace is rejected by the native borrow boundary.

## Uncertainty propagation

```python
from symb_anafis import CovarianceMatrix, Uncertainty

covariance = CovarianceMatrix([[4, 1], [1, 9]])
uncertainty = Uncertainty.propagate(x+2*y, [x, y], covariance)
assert uncertainty.variance == Expr(44)
assert uncertainty.sensitivities == (Expr(1), Expr(2))
assert uncertainty.standard_deviation() == Expr(44).sqrt()
```

For real coordinates, the combined first-order variance is
\(u_f^2=\nabla f^T\Sigma\nabla f\). `CovarianceMatrix.diagonal(variances)` stores
independent **variances**, not standard deviations, in linear space. Dense
matrices check shape and exact symmetry without epsilon tolerances.

Unknown scalar hypotheses, positive semidefiniteness, regular differentiability,
and the adequacy of the linear approximation belong to the caller.
`Uncertainty.expression`, `.sensitivities`, and `.variance` retain owning handles;
`standard_deviation()` constructs the square root and `relative()` divides it by
the absolute model. Poles and the relative expression's zero exclusion survive.

## Diagnostics

`ParseError`, `RewriteError`, `SimplifyError`, `EvaluationError`, and
`UncertaintyError` inherit ValueError. Parse errors expose `.source` and `.span`:
a half-open UTF-8 **byte** range, or `None` when no location applies. Invalid
numeric types raise TypeError and oversized integer inputs raise OverflowError.
Registry, arity, and conflicting substitution errors raise ValueError. Original
Python callback exceptions retain their type and instance, with fresh error
bookkeeping per invocation. Callback references participate in Python GC.
