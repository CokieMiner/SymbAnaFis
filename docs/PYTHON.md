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
assert model.laplacian([x, y]) == Expr(2)
assert model.partial_derivatives([(x, 1), (y, 1)]) == Expr(1)
assert model.directional_derivative({x: 3, y: 4}) == 3*(2*x+y) + 4*x
assert model.jacobian([x, y]) == [[2*x+y, x]]
assert Expr.jacobian([model, x*y], [x, y]) == [[2*x+y, x], [y, x]]
assert (x.sin()**2 + x.cos()**2).compact() == Expr(1)
assert ((x + 1)**2).expand() == x**2 + 2*x + 1
assert (x**2 + x*y).factor() == x * (x + y)
assert "x" in model and x in model
assert model.variables == ("x", "y")
assert not model.is_zero and not model.is_constant
assert x.properties[Property.REAL] is Truth.UNKNOWN
assert x.abs().properties[Property.NONNEGATIVE] is Truth.TRUE
x.set_properties([Property.POSITIVE])
assert (x**2).sqrt().compact() == x
x.set_properties([])
```

`Context.symb(name)` (or alias `Context.symbol(name)`) gets or creates one local identity. `Context.anon()`
allocates a distinct anonymous symbol in that namespace. `Context.symbols(*names)`
returns a tuple in the requested order. `context.symbol_names`, `context.symbol_count`,
and `context.is_empty` inspect current bindings.
`Context.get_symbol(name)`, `Context.remove_symbol(name)`,
`Context.clear_symbols()`, `Context.clear_functions()`, `Context.clear_all()`, and
`Context.reset()` manage bindings; existing
symbols, expressions, and function handles keep owning their metadata. Equal
names in different contexts remain different identities. `Symbol(name)` and
`Expr("formula")` use `Context.default()` unless a context is supplied explicitly.

`Symbol.set_properties(properties=())` replaces the closed domain assumptions on a
symbol identity (passing an empty iterable resets to the default finite complex
domain) and invalidates cached expression properties and simplification masks.
Contradictory property sets raise `ValueError`.

Arithmetic accepts `Expr`, `Symbol`, `Number`, integers, floats, complex values,
and `fractions.Fraction`. Strings supplied to `Expr` are parsed. `Expr.sum` and
`Expr.product` batch n-ary construction. `Expr.pi()`, `Expr.e()`, and `Expr.i()`
construct constants independently of symbol names.

Equality and hashing describe canonical **Expr structure**, including literal
representation: `Expr(2) != Expr(2.0)`, and `Expr(2) != 2`. Compare with `Expr(2)`
or inspect a literal's `.value`. Symbolic expressions and unknown `Truth` values
cannot be converted implicitly to `bool`.

`Expr.derivative(symbol, order=1)`, `partial_derivatives(orders)`,
`directional_derivative(direction)`, `gradient(variables)`, `jacobian(...)`,
`hessian(variables)`, and `laplacian(variables)` use the Rust
engine's regular principal-branch formulas. `partial_derivatives` accepts an
iterable of `(Symbol, order)` pairs or a `{Symbol: order}` mapping;
`directional_derivative` accepts `(Symbol, component)` pairs or a mapping.
Unsupported derivatives stay
symbolic. Jacobian rows are outputs and columns are coordinates; Hessians retain
mixed operator order. Repeated calculus coordinates repeat components. No extra
simplifier runs implicitly.

`kind`, `args`, `symbols`, `variables`, `functions`, `is_constant`, `is_zero`,
`is_one`, `is_neg_one`, `is_exact_zero`, `is_exact_one`, `properties`,
`node_count`, `max_depth`, and `view` inspect logical structure and contents.
`is_exact_zero` and `is_exact_one` test only exact integer `0` and `1` (excluding
`0.0`, `-0.0`, and `1.0`), whereas `is_zero`, `is_one`, and `is_neg_one` match
across exact and floating-point literal tiers.
Membership testing (`"x" in expr`, `x in expr`, or `fn in expr`, also available as
`contains_var(name)`, `contains_symbol(symbol)`, and `contains_function(function)`)
checks whether a variable name, symbol identity, or function identity occurs in the
expression; `has_free_symbols(excluded=())` and `has_free_variables(excluded=())`
check for occurrences outside an exclusion set. Views own their
children and derivative metadata.
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

`Number.real` and `.imag` return Python integers, fractions, or floats;
`is_exact`, `is_finite`, and `is_real` query scalar predicates.
`float(number)`, `complex(number)`, and conversion of nonreal values through
`to_python()` explicitly request approximation. Arithmetic follows NumAnaFis
domain and capacity rules. Numeric equality compares represented values; Number
is unhashable. Compatible NumPy scalar types are accepted through Python's
numeric protocols without making NumPy a package dependency.

## Custom functions

```python
square = context.register_function(
    "square", 1, body=lambda value: value**2, returns=[Property.REAL, Property.NONNEGATIVE]
)
assert square.arity == (1, 1)
assert square.properties[Property.NONNEGATIVE] is Truth.TRUE
assert square(x).derivative(x) == 2*x
assert square(x).evaluate({x: 3}) == 9

variadic = context.register_function("variadic", (1, 3))
assert variadic.arity == (1, 3)

product = context.register_function("product", 2,
    body=lambda first, second: first*second,
    partials={0: lambda first, second: second, 1: lambda first, second: first})
assert product(y, x).derivative(x) == y
```

`arity` accepts a fixed nonnegative integer or an inclusive `(min_arity, max_arity)`
tuple (variable arities require `body=None` and `partials=None`). Passing
`returns=[Property.POSITIVE, ...]` attaches conservative return-domain properties
to the registered definition, readable via `function.properties` (`Properties` for
custom functions, `None` for builtins). `Function` handles are hashable and can be
used in sets and dictionaries.

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
cascade. Registration rejects duplicate or builtin names.
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

subs = Substitutions({x: y, y: x})
assert subs.apply(x**2+y) == y**2+x
assert subs.get(x) == y and not subs.is_empty
assert Substitution(x+y, y).apply((x+y)**2) == y**2

angle = Pattern.capture("angle")
identity = RewriteRule(angle.sin()**2 + angle.cos()**2, 1)
assert identity.apply(x.sin()**2 + x.cos()**2 + y) == 1+y
assert len(identity.required_functions) == 2

rewrite = RewriteRule(angle.sin(), lambda captures: captures["angle"].cos())
assert rewrite.apply(x.sin()) == x.cos()
```

`Substitutions` accepts a mapping or iterable of pairs and applies entries
simultaneously without revisiting destinations. Conflicting duplicate sources
raise `ValueError`. `get(source)`, `items()`, `is_empty`, `len(...)`, `bool(...)`,
and iteration inspect entries; `Substitution` exposes `.source` and `.destination`.
`Expr.substitute(source, destination)` applies one entry.

Patterns support arithmetic, `Pattern.sum(values)`, `Pattern.product(values)`, and
the same mathematical methods as Expr.
`Pattern.capture(name, properties=[Property.REAL, ...])` requires conservative
proofs before accepting a candidate; unknown facts fail the condition.
`Pattern.exact(expr)` binds a concrete identity. Unmatched root operands survive.

`RewriteRule.apply(expr, match_limit=100_000, guard=None)` accepts a Python guard
over captured expressions. `None` removes the candidate search bound. The bound
counts candidate permutations; an already ordered complete match can need no
permutation attempts. `RewriteRule.required_functions` returns the tuple of
`Function` identities required by the rule's pattern. Replacement callbacks run after guard acceptance.
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
same contracts and defaults as Rust; `None` is unbounded. `DomainSafety.PRESERVING` is the default;
`ASSUMPTION_DEPENDENT` permits tagged identities whose hypotheses belong to the
caller. Custom rules and maps always retain caller-owned validity.

`simplify` returns a frozen `Simplification(expression, termination, iterations)`.
It selects the best visited candidate, not necessarily the last pass's output
or a global minimum. Convenience methods `Expr.compact()`, `Expr.expand()`, and
`Expr.factor()` run the respective canonical presets directly without instantiating
a sequence manually.

## Numerical programs and batches

```python
from symb_anafis import Evaluator

program = Evaluator(x+2*y, [x, y])
assert program.evaluate([3, Fraction(1, 2)]) == 4
assert program.evaluate_batch([[1, 2], [3, 4]]) == [7, 10]

floating = Evaluator(x.sin(), [x], backend="f64", native=True)
assert floating.evaluate([0.0]) == 0.0
assert floating.native_bytes is not None
assert floating.to_wasm_bytes().startswith(b"\0asm\x01\0\0\0")

single = Evaluator(x**2 + 1, [x], backend="f32")
assert single.evaluate([3.0]) == 10.0

complex_prog = Evaluator(x**2 + 1, [x], backend="complex128")
assert complex_prog.evaluate([1j]) == 0j

assert Evaluator(7, []).evaluate_batch([], rows=3) == [7, 7, 7]

system = Evaluator([x.sin() + y, x.cos() - y], [x, y], backend="f64", native=True)
assert system.output_count == 2
assert system.evaluate([0.0, 2.0]) == (2.0, -1.0)
```

`Expr.evaluate(bindings=(), *, backend="number")` performs direct evaluation
with a mapping or iterable of `(Symbol, value)` pairs.
`Evaluator(expression, parameters=None, *, backend="number", native=False)`
compiles either a single expression or a `list`/`tuple` of expressions into a
reusable program; for a sequence of expressions (`output_count == len(expressions)`),
CSE, constants, reciprocals, and paired `sin`/`cos` calls are shared across all
outputs, `evaluate(...)` returns a `tuple` of scalars, `evaluate_batch(...)`
returns a `tuple` of arrays/lists, and `out=(buf0, buf1, ...)` writes all
outputs zero-copy in one parallel dispatch. `backend` and `native` must be
passed by keyword. Omitted parameters use the model's symbol identities in
deterministic order. Explicit ordering is preserved.

Supported backends are:
- `"number"`: returns `Number` objects preserving exact integer/rational/complex tiers.
- `"f64"` / `"f32"`: IEEE-754 floating-point VM/JIT/WASM returning Python `float`
  (or NumPy `float64`/`float32` arrays for buffer batches), using the native
  compiler's FMA/reassociation policy. Rounding and intermediate overflow may differ
  from direct evaluation.
- `"complex128"` / `"complex64"`: fixed-layout complex VM/JIT returning Python
  `complex` (or NumPy `complex128`/`complex64` arrays for buffer batches).

Calling `evaluator.with_native()` (or passing `native=True` to `Evaluator`)
upgrades `"f64"`, `"f32"`, `"complex128"`, and `"complex64"` programs to
position-independent `W^X` host machine code (`x86_64`, `aarch64`, `riscv64`),
with lazy AVX2/AVX-512 SIMD batch kernels on `x86_64`.
`evaluator.native_bytes` returns the compiled scalar machine-code `bytes` (or
`None` for VM-only programs). `evaluator.to_wasm_bytes()` emits a standalone
WebAssembly (`.wasm`) binary module for `"f64"` and `"f32"` programs.

Batch input contains one column per parameter, including unused parameters.
Shapes are checked before execution. For all four native backends (`"f64"`,
`"f32"`, `"complex128"`, and `"complex64"`), C-contiguous matching-dtype
PEP 3118 buffer inputs (including NumPy `float64`, `float32`, `complex128`, and
`complex64` arrays) are borrowed **zero-copy** without intermediate Python
objects; non-contiguous views (such as `arr[::2]` or `points.T`) and convertible
numeric dtypes (such as `int64` or `float32` into `"f64"`) are converted via
`np.ascontiguousarray`. Complex NumPy arrays/scalars are rejected on real
backends (`TypeError`) rather than discarding imaginary parts, and boolean arrays
or scalars are rejected across all backends.

When any input is a buffer or NumPy scalar and NumPy is installed,
`evaluate_batch` returns a matching `np.ndarray` (lists otherwise). N-dimensional
arrays (such as `np.meshgrid` grids, including `sparse=True` broadcastable grids)
and mixed scalar/array inputs (e.g. `evaluate_batch([xs, 2.0])`) preserve the
broadcast shape on output, while all-1D mismatched column lengths continue to
raise `EvaluationError`. Passing a writable C-contiguous `out=` buffer writes
directly into caller-owned memory for 1D or matching-shape N-D outputs (read-only
buffers and shape mismatches raise `ValueError`; in-place aliasing such as `out=x`
is detected and staged safely; `out=` on `"number"` raises `TypeError`).
`evaluate_batch(..., parallel=True)` dispatches rows across the native Rayon
thread pool. `rows` specifies the output count for zero-parameter programs. `parameters`, `backend`, `instruction_count`,
`workspace_size`, and `disassemble()` inspect the compiled program. Native work
releases the GIL. Use `copy()` for independent workspaces when evaluating
concurrently; concurrent mutation of one workspace is rejected by the native borrow
boundary.

## NumPy integration

NumPy is strictly optional (`pip install symb_anafis[numpy]`), and integrates at
both the symbolic and numerical layers:

- **Symbolic ufuncs and array arithmetic**: `Expr`, `Symbol`, and `Pattern`
  implement `__array_ufunc__` and Python's symmetric `NotImplemented` arithmetic
  protocol, so `x + np.array([1.0, 2.0])`, `np.array([1.0, 2.0]) + x`,
  `np.sin(x)`, `np.log(x)` (mapping to `x.ln()`), `np.arcsin(x)` (`x.asin()`),
  `np.arctan2(y, x)`, `np.expm1(x)`, `np.log1p(x)`, `np.sign(x)`, and `np.rint(x)`
  construct symbolic expressions or broadcast `object` arrays cleanly.
- **NumPy scalars**: `Number`, `Expr`, and `Evaluator` accept NumPy integer,
  floating-point, and complex scalars while rejecting `np.bool_`.
- **Zero-copy numerical batches**: `"f64"`, `"f32"`, `"complex128"`, and
  `"complex64"` borrow matching NumPy arrays zero-copy on both input and `out=`,
  supporting N-D grids (`meshgrid`), scalar broadcasting, native JIT/SIMD, and
  Rayon parallel evaluation.

## Uncertainty propagation

```python
from symb_anafis import CovarianceMatrix, Uncertainty

covariance = CovarianceMatrix([[4, 1], [1, 9]])
uncertainty = Uncertainty.propagate(x+2*y, [x, y], covariance)
assert uncertainty.variance == Expr(44)
assert uncertainty.sensitivities == (Expr(1), Expr(2))
assert uncertainty.standard_deviation() == Expr(44).sqrt()

factor = CovarianceMatrix.from_factor([[2, 0], [1, 3]])
assert uncertainty.recontract(factor) == factor.contract(uncertainty.sensitivities)
```

For real coordinates, the combined first-order variance is
$u_f^2 = \nabla f^T \Sigma \nabla f$. `CovarianceMatrix.diagonal(variances)` stores
independent **variances**, not standard deviations, in linear space;
`CovarianceMatrix.symbolic_diagonal(context, variables)` and
`CovarianceMatrix.symbolic_correlated(context, variables)` build symbolic
independent ($\sigma_i^2$) and correlated ($\rho_{ij}\sigma_i\sigma_j$)
covariances using scoped symbols in `context`;
`CovarianceMatrix.from_factor(factor)` constructs $\Sigma = L L^T$ from a symbolic
factor matrix; `dimension`, `get(row, column)`, and `contract(sensitivities)`
inspect and contract stored entries. Dense matrices check shape and exact symmetry
without epsilon tolerances.

Unknown scalar hypotheses, positive semidefiniteness, regular differentiability,
and the adequacy of the linear approximation belong to the caller.
`Uncertainty.expression`, `.sensitivities`, and `.variance` retain owning handles;
`recontract(covariance)` reuses the computed sensitivities with another covariance
matrix; `standard_deviation()` constructs the square root and `relative()` divides
it by the absolute model. Poles and the relative expression's zero exclusion survive.

## Diagnostics

`ParseError`, `RewriteError`, `SimplifyError`, `EvaluationError`, and
`UncertaintyError` inherit ValueError. Parse errors expose `.source` and `.span`:
a half-open UTF-8 **byte** range, or `None` when no location applies. Invalid
numeric types raise TypeError and oversized integer inputs raise OverflowError.
Registry, arity, and conflicting substitution errors raise ValueError. Original
Python callback exceptions retain their type and instance, with fresh error
bookkeeping per invocation. Callback references participate in Python GC.
