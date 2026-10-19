# AnaFis Roadmap

This roadmap separates the SymbAnaFis symbolic engine from the numerical, statistical, and application projects in the AnaFis ecosystem. Numerical capabilities already supplied by NumAnaFis and MpAnaFis are reused through their owning crates.

## Planning Baseline

This roadmap assumes that all existing SymbAnaFis subsystems have been reintegrated with the expression DAG and NumAnaFis backend: parsing, differentiation, simplification, bytecode evaluation, uncertainty propagation, and Python bindings.

Checked tasks describe that assumed starting point. Unchecked tasks describe improvements and new capabilities beyond it. Current API availability is documented separately in the [API reference](API.md).

## Numerical Foundations and Ownership

| Crate | Available implementation | SymbAnaFis relationship |
| --- | --- | --- |
| [NumAnaFis](https://github.com/CokieMiner/NumAnafis) | `Number`, `Real`, and `Complex`; native integer/rational arithmetic; approximate scalars; elementary and special functions; exact supported roots and arithmetic cells. | Active numeric dependency for literals, coefficients, and constructor evaluation. |
| NumAnaFis native kernels | `F32Ext`, `F64Ext`, and complex-pair functions; precision comparison tooling. | Reuse numeric kernels and their validation work; do not duplicate special-function approximations in SymbAnaFis. |
| NumAnaFis optional Clifford layer | `GeneratorSet`, `Metric`, `CliffordNumber`, `FastClifford`, and the `Cga` catalog. | Available numerically; symbolic Clifford and tensor support remain separate future work. Scalar complex multiplication remains commutative. |
| [MpAnaFis](https://github.com/CokieMiner/MpAnafis) | `MpInt` and `MpUint`; arbitrary-precision integer arithmetic; GCD, roots, factorials, modular arithmetic, and number theory; bounded or unlimited integer precision. | Available as an ecosystem integer foundation; not integrated into NumAnaFis `Number` or SymbAnaFis literals. |

The current `Number` uses native `i64` integers, reduced rationals with native components, and `f64` approximate values. Integer arithmetic follows the backend's wrapping policy; rational capacity failures can panic. Using `Number` does not make symbolic arithmetic arbitrary precision or remove floating-point rounding.

MpAnaFis currently exports integer types, not a public arbitrary-precision rational or floating-point type. Its integer precision bounds are not a floating-point precision API. Multiprecision integer integration, multiprecision rationals, and multiprecision approximate functions are distinct milestones.

| Crate | Current feature contract |
| --- | --- |
| SymbAnaFis | Default `no_std + alloc`; optional `std` and expression `num-traits`. |
| NumAnaFis | Default `std`; native `no_std` without an allocator when `clifford` is disabled; optional `clifford` and `python`. There is currently no `num-traits` feature to forward. |
| MpAnaFis | Default `no_std + alloc`; optional `std`, `num-traits`, and `rayon`, with `rayon` enabling `std`. |

### Numeric Integration Work

- [ ] Design the optional MpAnaFis-backed integer path in NumAnaFis, including native-to-multiprecision promotion and conversions.
- [ ] Define reduced multiprecision rational storage and arithmetic using the integer backend, with explicit denominator and capacity contracts.
- [ ] Keep numeric ownership in NumAnaFis; consume the resulting `Number` contract in SymbAnaFis rather than adding a second integer or fraction implementation.
- [ ] Revalidate symbolic literal identity, ordering, hashing, folding, and polynomial coefficients after numeric representation changes.
- [ ] Treat arbitrary-precision floating-point storage, rounding, and transcendental kernels as a separate design; do not mark them complete from integer support alone.
- [ ] Keep native arithmetic fast and preserve exact versus approximate representation across promotion.
- [ ] Check division remainders and root perfectness before treating integer quotient or floor-root operations as exact symbolic results.
- [ ] Reuse NumAnaFis accuracy tooling for domains, signed zeros, branch cuts, and special-function limits. Function availability and accuracy validation are separate statuses.

## Part I — SymbAnaFis

### Expression Core Baseline

- [x] Immutable, library-wide weak-interned expression DAG with collision-safe node sharing.
- [x] One-pointer `Expr` handles with constant-time canonical equality and hashing.
- [x] NumAnaFis `Number` literals and coefficients with representation-preserving identity.
- [x] Automatic sum/product normalization, like-term collection, and positive integer factor collection.
- [x] Private compact polynomial storage with public logical sum views and traversal.
- [x] Dedicated imaginary-unit, pi, and Euler constant nodes.
- [x] Scoped local symbols, isolated contexts, and permanent global symbol identities.
- [x] Opaque builtin identities and registered custom functions with immutable arities.
- [x] Custom symbolic bodies and sparse partial-derivative callbacks.
- [x] Concrete and simultaneous substitutions, including arithmetic groups and implicit multiplicities.
- [x] Associative and commutative pattern matching, repeated captures, predicates, guards, and bounded candidate search.
- [x] Template and callback replacements, including generated finite expansions.
- [x] Conservative finite-scalar property analysis with three-valued proof results.
- [x] Logical views, mapping, folding, symbol queries, and expression formatting.
- [x] Unevaluated derivative nodes.
- [x] `no_std + alloc` by default, with optional `std` synchronization and expression `num-traits` support.
- [x] Property-based tests for core transformations and deterministic contract tests.
- [x] Core API reference with executable examples.

Automatic normalization belongs to construction. Mathematical identity selection, expansion, factorization, and assumption-dependent simplification belong to explicit higher-level operations.

### Symbolic Engine Baseline

- [x] Pratt parsing with implicit multiplication and source diagnostics.
- [x] Symbolic differentiation with builtin formulas and custom partial callbacks.
- [x] Algebraic simplification using normalized expressions and symbolic identities.
- [x] Compiled register-based bytecode evaluation and SIMD execution.
- [x] Instruction scheduling, linear-scan register allocation, GVN, and late FMA fusion.
- [x] Bytecode disassembly and performance statistics.
- [x] Uncertainty propagation and dual-number interfaces.
- [x] Python bindings for the symbolic engine and evaluator.
- [x] Singularity analysis under the selected domain policy.

### Development Priorities

| Priority | Goal | Scope |
| --- | --- | --- |
| High | Multiprecision numeric integration | Reuse MpAnaFis through NumAnaFis, following the numeric work above. Keep integer, rational, and approximate precision milestones separate. |
| High | Domain-aware algebra | Extend assumptions and proofs, make simplification goals explicit, and control transformation growth. |
| High | Evaluation performance and optional JIT | Improve the existing VM, compare JIT candidates against it, and retain portable execution. |
| Medium | Series, limits, integration, and solving | Build scoped mathematical operations with stated domains and verifiable results. |
| Medium | Extended function coverage | Add numerical kernels in NumAnaFis and symbolic rules in SymbAnaFis. |
| Longer term | Matrices, tensors, noncommutative algebra, and application layers | Design the required algebraic representations and reuse existing numerical components. |

#### Small-Scope Extensions

The following slices reuse the scalar DAG without changing its node layout, symbol ownership, or interner. Each requires bounded construction, explicit unsupported cases, and tests for its stated mathematical contract.

| Candidate | Existing machinery | Initial scope and acceptance |
| --- | --- | --- |
| Finite Taylor/Maclaurin polynomial | Memoized differentiation, concrete substitution, exact coefficients, and normalized sums. | Construct \(T_n(x)=\sum_{k=0}^{n} f^{(k)}(a)(x-a)^k/k!\) for a bounded order at a regular expansion point. Require defined coefficients and representable arithmetic; report unsupported derivatives or singular coefficients. Verify derivatives through order \(n\) at the center. This does not establish convergence or a remainder bound. |
| Polynomial antiderivative | Symbol-identity dependency queries, logical monomials, and normalized arithmetic. | Integrate polynomials in one selected symbol with coefficients independent of that symbol: \(c x^k\mapsto c x^{k+1}/(k+1)\), for nonnegative integer \(k\). Return a primitive with zero integration constant; reject unsupported terms and capacity failures. Verify by differentiation. |
| Finite Chebyshev T/U polynomials | Exact arithmetic and bounded polynomial construction. | Generate nonnegative integer degrees using \(P_{n+1}(x)=2xP_n(x)-P_{n-1}(x)\), with \(T_0=U_0=1\), \(T_1=x\), and \(U_1=2x\). Bound degree, work, and output size. These are generated polynomials; general function nodes require NumAnaFis kernels. |
| Explicit denominator diagnostics | Memoized logical traversal, `Div` views, and conservative nonzero proofs. | Collect denominator expressions whose nonzero condition is required, distinguishing disproved and unresolved conditions. Preserve exclusions. This is a structural report, not a complete pole finder or removable-singularity classifier. |

Some roadmap areas already have reusable public operations:

- [`SimplificationSequence`](../src/simplification/api/sequence.rs) supplies compact, expanded, factored, and compilation objectives. Its passes have iteration, node, depth, matching, expansion, and polynomial-work budgets. Further work should extend these contracts rather than introduce another simplification API.
- [`Expr::jacobian`](../src/core/logic/expression/calculus.rs) already differentiates coordinate maps and shares derivative work across output roots. A polar-coordinate example is available in [applications](../examples/applications.rs); a compiled evaluator with several outputs remains a separate extension.
- The Python expression wrapper supplies [`_repr_latex_`](../python/symb_anafis/_expression.py) for Jupyter display. LaTeX input parsing remains separate work.
- [Runnable examples](../examples/README.md) cover parser-to-expression evaluation, DAG inspection, scientific derivatives, and first-order uncertainty propagation.

#### Domain-Aware Algebra and Targeted Simplification

- [ ] Design an assumptions environment without silently changing immutable expression identity or overloading namespace management.
- [ ] Extend current property analysis with requested proofs and function-domain facts.
- [ ] Extend the simplifier with explicit expansion, collection, factorization, and cancellation goals with stated scope.
- [ ] Reuse core normalization and rewriting rather than duplicating constant folding or like-term collection.
- [ ] Define termination, output growth, and search budgets for repeated transformations.
- [ ] Identify zero denominators, poles, and removable singularities while preserving original exclusions.
- [ ] Keep continuous extension and limit evaluation explicit. For example, replacing \(\sin(x)/x\) with a value at zero changes the original domain.

Acceptance: each transformation states its domain and goal, returns a predictable result or failure, and has a policy for resource exhaustion.

#### Evaluation Performance and Optional JIT

- [ ] Reduce compilation and execution work by exploiting shared DAG nodes and measured register-liveness patterns.
- [ ] Improve batch and SIMD execution using NumAnaFis kernels and representative numerical workloads.
- [ ] Extend gradients, Jacobians, and Hessians to share intermediate calculations across outputs.
- [ ] Make exact-versus-approximate execution and FMA/reassociation policies explicit across backends.
- [ ] Evaluate JIT backend candidates, including `symjit`, against the existing bytecode VM.
- [ ] Measure compile latency, execution throughput, memory, and break-even workload sizes.
- [ ] Specify supported numeric tiers, custom function calls, targets, and feature gates before choosing a backend.
- [ ] Keep JIT optional and retain the portable evaluator for unsupported configurations.
- [ ] Expose new evaluation backends and symbolic operations through the Python API as their contracts stabilize.

Native scalar VM work can be divided into small, measurable changes:

| Candidate | Current execution | Contract and priority |
| --- | --- | --- |
| Native negation, subtraction, and signed FMA | `-x` multiplies by a stored `-1`. `x-y` uses multiply/add, or one FMA when enabled. Products with a negative coefficient can prevent a complete signed FMA fusion. | High priority. Recognize signs in the existing normalized representation, update instruction validation and disassembly, and compare instruction counts, register demand, compile time, and execution time. |
| Reciprocal of an approximate constant | Exact `x/2` already becomes a multiplication by `1/2`. Approximate `x/2.0` retains division. Repeated symbolic denominators already share a reciprocal in native plans. | Small compiler extension for the native floating contract. Validate constant and reciprocal ranges; exclude zero and nonfinite cases. Multiplication by a rounded reciprocal can change rounding, so retain backend division semantics for generic plans. |
| Constant exponent specialization | Exact half powers become square roots; exact negative integer powers use reciprocals and multiplication chains. Approximate constant exponents retain `Power`, whose NumAnaFis kernel already has shortcuts for `1/2`, `2`, and `-1`. | Separate VM dispatch reduction from numerical kernel changes. Use exact constant comparisons, not epsilon matching. A shortcut such as `x*sqrt(x)` for `x^1.5` requires explicit rounding and exceptional-value validation; NumAnaFis owns stronger kernel accuracy guarantees. |

Instruction latency ratios do not establish whole-expression speedups. These candidates require workload measurements, including compilation latency, and preserve the generic `Number` backend's operation semantics.

### Further Symbolic Capabilities

These are scoped backlogs. A heading does not promise a general solver for every mathematical case.

| Area | Planned work | Dependencies and limits |
| --- | --- | --- |
| Series | Taylor/Maclaurin and Laurent series; power-series arithmetic and composition; asymptotic expansions. | Differentiation or explicit recurrence formulas; truncation, expansion point, and remainder conventions. A generated finite sum alone is not a general series engine. |
| Limits | Symbolic limits, indeterminate forms, limits at infinity, and multivariable limits. Conditional evaluation of an explicitly requested continuous extension can be considered after its limit is established. | Domain and branch analysis, series where applicable, and explicit applicability checks for L'Hôpital's rule. A limit does not redefine function evaluation at a singular point. |
| Algebraic solving | Linear systems, variable isolation, analytical polynomial formulas through degree four, and transcendental solution patterns. | Solution domains, exceptional coefficients, multiplicities, conditional results, and integer solution parameters. |
| Differential equations | Selected symbolic methods for ODEs; separate research tracks for PDEs, DAEs, DDEs, IDEs, SDEs, and fractional equations. | Equation representation, supported classes, and verifiable solution conditions; avoid one undifferentiated general-solver milestone. |
| Integration | Polynomial integration, table rules, and a scoped heuristic Risch approach. | Domain and branch handling; numerical integration and uncertainty-aware fallbacks belong to the numerical layer and must be selected explicitly. |
| Integral transforms | Laplace, Fourier, and Mellin rules. | Transform conventions, convergence regions, and conditions on inputs. |
| Units | Symbolic dimension checking for length, time, mass, and derived dimensions. | Unit metadata and the numerical/metrology representation. |
| Coordinates | Coordinate mappings and Jacobians. | Differentiation and explicit coordinate domains. |
| Symmetry | Lie symmetry analysis and invariant reduction for supported equation classes. | Differentiation and equation representations. |
| Matrices and tensors | Shape checking, element-wise operations, matrix differentiation, and Einstein notation. | Decide between typed containers and AST extensions. Matrix or Clifford multiplication cannot reuse commutative scalar multiplication unchanged. No public `Expr::Matrix` design is selected here. |
| Expression analysis | Poles and zeros, asymptotic behavior, parity, and periodicity. | Extend existing structural metrics, singularity analysis, and conservative properties. |
| Input/output | LaTeX parsing and Jupyter display integration. | Build on the parser, Python bindings, and existing text, Unicode, and LaTeX output. |

### Function Coverage Backlog

Numerical implementations and their accuracy belong to NumAnaFis. SymbAnaFis owns symbolic identities, arities, derivative formulas, domain rules, and backend wiring. MpAnaFis integer primitives should be reused for combinatorial work where applicable; its factorial implementation is not yet a symbolic `factorial` node.

| Family | Candidate functions |
| --- | --- |
| Integer combinatorics | Factorial, double factorial, and subfactorial. |
| Exponential and related integrals | `ei`, `li`, `shi`, `chi`, `si`, and `ci`. |
| Hypergeometric and general functions | `hyp1f1`, `hyp2f1`, Meijer G, and Fox H. |
| Error and oscillatory integrals | `erfi`, Fresnel S/C, and Dawson. |
| Gamma and beta extensions | Upper/lower incomplete gamma, incomplete beta, and multivariate gamma. |
| Orthogonal polynomials | Laguerre, Chebyshev T/U, and Jacobi. |
| Zeta-related functions | Polylogarithm, Dirichlet eta, Lerch phi, and Hurwitz zeta. |
| Airy and Struve | Airy Ai/Bi and Struve H/L. |
| Elliptic extensions | Incomplete F/E/Pi, complete Pi, and Jacobi sn/cn/dn. |
| Other special functions | Kelvin ber/bei and Mathieu c/s. |

## Part II — AnaFis Numerical and Application Ecosystem

### Scientific Direction

AnaFis targets uncertainty-aware experimental analysis. Classical statistical and metrological methods form the baseline; neural estimators, variational inference, and physics-informed models are research extensions with separate validation requirements.

Uncertainty-aware contracts belong to numerical and application APIs. They do not require every exact scalar operation or symbolic constructor to allocate an uncertainty tensor. Applicability to GUM/JCGM workflows is a validation goal, not a current blanket compliance claim.

### Shared Numerical and Metrology Types

Proposed `core-anafis` responsibilities:

- [ ] Define measured values, standard uncertainties, covariance/correlation, and distribution information needed by downstream methods.
- [ ] Design `UncertaintyTensor<T>` and a `Measurable` contract around explicit shapes, units, and correlation assumptions.
- [ ] Add Type A and Type B uncertainty input workflows.
- [ ] Evaluate a `Measurable` derive macro and Arrow/ndarray interoperability after the data model is stable.
- [ ] Reuse NumAnaFis scalar values and MpAnaFis-backed capabilities as available; keep statistical meaning distinct from numeric precision.
- [ ] Specify and validate propagation methods and reporting requirements before making standards-compliance claims.

### Ecosystem Baseline

The ecosystem planning baseline also includes the existing numerical and application capabilities:

| Project | Baseline capability | Further work |
| --- | --- | --- |
| `odr_anafis` / proposed `regression-anafis` | Basic ODR with known input/output uncertainties; symbolic Jacobians and Hessian-based parameter uncertainty. | Extend constrained fitting, model selection, and explicit covariance/model assumptions. |
| Proposed `geo-anafis` | Curve fitting with symbolic gradients. | Extend implicit and parametric geometry workflows. |
| `anafis-tauri` | Tauri GUI setup and CSV/SQLite/Excel import. | Add analysis orchestration, interactive plots, and streaming. |

### Statistical Projects

The items below extend the baseline or introduce new projects; project names do not imply published crates.

| Project | Planned work |
| --- | --- |
| `power-anafis` | Grid-based power simulation, GP surrogates for power curves, Bayesian optimization, and power/cost tradeoffs. |
| `correlation-anafis` | KSG mutual information, error-aware estimators, conditional/partial information, and MINE/InfoNCE research for high-dimensional inputs. |
| `exploration-anafis` | PPCA via EM, heteroscedastic noise from measurement uncertainty, loading confidence intervals, and varimax/promax rotations. |
| `outliers-anafis` | MCD baselines, uncertainty-weighted Mahalanobis distances, iterative refinement, and multivariate visualization. |
| `imputation-anafis` | GPR with RBF and noise kernels, known observation uncertainties, covariance-aware batch imputation, and multiple imputation. |
| `signal-anafis` | DWT thresholding, CWT ridge detection, SureShrink, physics-informed smoothing, and asymmetric least-squares baseline correction. |
| `regression-anafis` | ODR extensions, constrained fitting, SINDy, and model selection through AIC/BIC/cross-validation. |
| `hypothesis-anafis` | Laplace marginal-likelihood approximation, bridge sampling, nuisance-parameter marginalization, and Savage-Dickey methods with stated assumptions. |

### Optimization, Learning, and Physics Projects

| Project | Planned work |
| --- | --- |
| `opt-anafis` | SGD/momentum, Adam/AdaGrad, Newton methods using symbolic Hessians, L-BFGS, and constrained optimization. |
| `kan-anafis` | B-spline KAN layers, training, spline-to-symbolic conversion, pruning, and physics-informed constraints. |
| `symreg-anafis` | DAG-aware expression genomes, crossover/mutation, complexity-aware fitness, Pareto selection, and numeric constant optimization such as CMA-ES. |
| `pinn-anafis` | A selected neural backend, with Burn as a candidate; PDE residual construction from symbolic expressions, boundary conditions, inverse problems, and uncertainty-quantification experiments. |
| `phys-anafis` | RK4/adaptive integration, forward/adjoint sensitivities, Euler-Lagrange derivation, Hamiltonian helpers, and Noether-related workflows. |
| `geo-anafis` | Implicit surfaces, normals/curvature, symbolic SDF ray marching, parametric geometry, curve fitting, and CSG. Reuse NumAnaFis Clifford/CGA numerics where appropriate; symbolic noncommutative algebra remains a separate decision. |

Each project needs a defined baseline, numerical error model, representative data, and acceptance measurements before selecting advanced variants.

### Applications and Delivery

| Area | Planned work |
| --- | --- |
| `reporting-anafis` | Markdown/LaTeX reports, uncertainty tables, plots, and Jupyter notebook export. |
| `anafis-tauri` | Analysis workflow orchestration, interactive error-bar plots, and sensor-data streaming. |
| WebAssembly | Verify allocator/atomic/backend requirements for selected targets and build an interactive expression demo. |
| GPU evaluation | Evaluate a separate large-batch backend only after portable evaluation and workload measurements justify it. No point-count threshold or GPU API is selected in advance. |

### Documentation and Demonstrations

- [x] Core API guide with examples of construction, substitutions, rewriting, proofs, callbacks, and binomial expansion.
- [ ] Parser-to-expression and custom-function differentiation tutorials.
- [ ] Numerical backend and precision-selection guide after multiprecision integration has a public contract.
- [ ] Symbolic regression: discovering physical laws from data.
- [ ] Physics-informed modeling: neural ODE and PDE workflows.
- [ ] Evaluator/JIT comparison using a validated numerical example such as a heat-equation workflow.
- [ ] Uncertainty propagation for laboratory reports.
- [ ] ODR fitting with error bars and explicit measurement assumptions.
