# SymbAnaFis - Project Guidelines

This document specifies the architecture, invariants, memory safety contracts, performance discipline, module organization, and verification procedures for the `SymbAnaFis` Rust crate (`symb_anafis` in Cargo).

The current legacy implementation is archived under `old/` as behavioral reference. Active subsystems are re-engineered directly under `src/` following these guidelines.

---

## 1. Priorities & System Architecture

`SymbAnaFis` is a high-performance symbolic mathematics library in Rust designed for exact algebraic manipulation, term rewriting, symbolic differentiation, and numerical code generation.

### Core Types & Representation Contracts

- **`Expr`**: A one-pointer owning handle (`Option<Arc<Node>>` utilizing the null-pointer niche) over a private, immutable node enum.
  - Every node is constructed via library-wide, collision-safe weak interning. Identical live subtrees share one canonical node and its payloads.
  - Numeric payloads are stored inline in their nodes; function arguments use `Vec<Expr>` storage. Interning allocates a canonical node only when no equal live node exists.
  - Structural equality (`Eq`, `PartialEq`) and hashing (`Hash`) are $O(1)$ canonical pointer comparisons.
  - Specialized multivariate polynomials (`Node::Poly`) remain private. Public views expose logical sums with lazy child iterators; public mapping and folding visit logical monomials once without introducing visits for internal additive storage groups.
  - Operators, mathematical methods, and n-ary vector constructors share normalized kernels. Keep consuming arithmetic kernels private rather than exposing duplicate technical constructors.
  - Canonical aliases use `sqrt(x)` for exact `x^(1/2)`, `exp(x)` for `e^x`, and unary logarithms for exact bases `e`, `2`, and `10`. Approximate exponents retain their numeric representation. Known negative real powers use denominators; products combine denominators without symbolic cancellation. Positive integer powers distribute over products and quotients while preserving every pole.
  - Nested positive integer powers combine by checked multiplication of exact native exponents. Zero, general fractional, and approximate inner powers retain their nesting; square-root aliases normalize separately. Exponent overflow preserves explicit powers.
  - Derivatives evaluate through a memoized iterative engine on regular principal-branch domains. Constructors perform ordinary normalization; differentiation never invokes a separate simplifier. Unsupported cases retain owning derivative nodes. Order zero returns the inner expression; repeated derivative states admit constant-space cycle detection. Stored orders are `NonZeroU32`; adjacent unevaluated derivatives with the same symbol identity combine by checked addition. Overflow retains nesting, and different variable identities do not commute.
  - Exact quotients normalize as `(a*A)/(b*B) = (a/b)*Div(A, B)` when the finite numeric coefficients have a nonzero divisor and a representable result. Coefficients stay outside the shared symbolic quotient; unit coefficients and unit denominators are omitted. Numeric literals inside sums, exponents, and function arguments remain in place; reciprocal numerators may be one. Zero denominators, approximate division, and capacity-limited coefficients remain explicit. This normalization does not introduce float reciprocals or cancel symbolic denominator factors without proof.
- **Numeric Literals (`NumericValue`)**: Representation-preserving DAG identity over `num_anafis::Number`.
  - Retains exact integers, reduced rationals, native IEEE floats, and scalar complex components.
  - Structural literal identity is strictly distinguished from numeric equality (e.g. `2.0` float vs `2` integer).
  - Exact components stay exact (rationals are irreducible with strictly positive denominators); approximate floats stay approximate (no auto-promotion to exact tiers).
- **`Symbol<'owner>`**: A validated native-width `Copy` identifier with a zero-sized owner lifetime.
  - Global handles (`Symbol<'static>`) have permanent metadata in the sharded directory.
  - Local handles borrow a `Context` or an owning `Expr`; the compiler enforces that they cannot outlive that owner.
  - Name unbinding preserves identities within the owner's lifetime; rebinding creates a fresh unique identity. Raw IDs cannot bypass the owner boundary.
  - `Context::symb` returns `Result<Symbol<'_>, SymbolError>`; ID exhaustion leaves existing bindings valid.
- **`Function`**: An opaque identity containing a static `BuiltinFunction` enum or a shared, immutable custom definition with its canonical name.
  - Builtin functions pass enum identities directly; arity, derivatives, and exact folding dispatch to static definitions without intern locks or hash-map lookups.
  - Custom definitions store sparse, sorted partial derivatives with unique argument indices. Setting a partial again replaces that callback in the draft; registered definitions remain immutable. Registering a duplicate function name returns an error without changing its existing binding or definition.
  - Registered arities are immutable. Unknown strings cannot create function identities or AST calls.
  - Custom handles and expression nodes retain bodies and partials after name removal or context destruction. Contexts store only name-to-handle bindings; callback access uses the borrowed `Function::definition` accessor without registry lookup. Equality and hashing depend on identity, never callbacks.
- **`Pattern` / `RewriteRule`**: Structural pattern matching with capture groups and memoized replacement across every edge to a shared input node.
  - **Associative / Commutative Matching**: Root sum and product patterns match disjoint commutative operand groups with backtracking and transactional undos (`Undo::Popped`, `Undo::Capture`). Unmatched operands survive automatically. Nested groups require complete coverage.
  - **Bounded Search**: `match_limit: Option<usize>` strictly bounds candidate permutations across the entire transformation pass, returning `RewriteError::MatchLimitExceeded` on exhaustion without publishing corrupted partial output.
  - **Guards**: Caller callbacks enforce semantic conditions on complete captured bindings before accepting an assignment. Local capture predicates reject candidates before continuing the search; repeated names retain all their conditions after normalization. Callback aliases remain private.
  - **Replacement**: A rule takes a compiled pattern or an owning, pure callback through the same constructor and application method. Replacement callbacks run only after an assignment and its guard are accepted. Counted matches reuse one result per assignment; capture-free callbacks are evaluated lazily once per application pass. Inserted destinations are not revisited.
  - **Counted Multiplicities**: Exact rational templates consume integral groups on an unchanged principal base. Integer outer powers may capture fractional blocks present in the input; reciprocal templates require denominator factors. Nested reciprocal exclusions stay opaque. Bare expressions do not infer fractional roots. Collected integer addends remain compressed.
- **`Properties` / `Property` / `Truth`**: Read-only, conservative scalar proofs computed in one stored-DAG traversal. Symbols range over finite complex values without additional assumptions. Unknown facts never authorize guarded transformations; nonfinite literals and retained poles cannot acquire finite-value proofs. Analysis does not mutate nodes or validate user-defined identities.
- **`Substitution` / `Substitutions`**: Concrete expression replacement by canonical identity, including logical polynomial monomials.
  - Exact whole matches take priority over partial groups. Entries apply simultaneously without revisiting destination subtrees in the same pass.
  - Duplicate sources accept identical destinations and reject conflicting destinations.
  - Changes of variable supply the explicit inverse `x -> g(u)`. Every scalar occurrence of `x` is replaced; no such occurrence remains when the destination does not contain `x`. Domain and branch choices belong to the caller. Differentiation-variable metadata is retained; transforming a differential operator requires a calculus transformation.
  - Manifest common symbolic factors are retained while additive groups are replaced. Extraction never divides by an unproved symbolic value.
  - Exact rational powers and nested integer powers admit integral counted decomposition only when it preserves principal branches and zero exclusions. Fractional outer powers do not expose powers inside their own base. Widened traversal counts are checked; numeric literal arithmetic belongs to `Number`.
  - Surviving original factors are traversed; generated coefficients and exponent counts are not new sources in the same pass.
- **Feature Contract**:
  - The default core is `no_std + alloc`, using `hashbrown` collections, const-initialized global state, and private non-poisoning spin mutexes.
  - Supported targets provide a global allocator and pointer-width atomics for `Arc` and registry sequences.
  - The `std` feature forwards `num_anafis/std` and selects blocking registry mutexes whose guards recover access after poisoning.

### Design Priorities (strictly ordered)

1. **Mathematical correctness and algebraic invariants**: A mathematically unsound transformation or invalid simplification is a critical bug. Performance never excuses an unproved transform.
2. **Memory safety and sound encapsulation**: Zero undefined behavior. Sound lifetimes prevent use-after-free, dangling symbol handles, and data races.
3. **Architecture portability**: Pointer-width-safe counts, indices, and capacities across 16-, 32-, and 64-bit targets. Native `no_std + alloc` support.
4. **Execution efficiency**: Reduce mathematical work, algorithmic complexity, memory traffic, DAG allocations, and traversal overhead before micro-optimizing.
5. **Ergonomic public APIs**: Idiomatic, composable Rust interfaces with explicit error boundaries.

### Subsystem Boundaries

- **`src/collections/`**: Private `no_std` collection facades (`HashMap`, `HashSet`) backed by `hashbrown` and `foldhash`, without operating-system entropy. Global interner indices use fixed seeds; collection iteration order is unspecified.
- **`src/core/api/`**: Public borrowed views (`ExprView`) and inspectors.
- **`src/core/logic/`**: Type owners, canonical construction, weak interning, symbol/function registries, traversal, rewriting, substitutions, and memory reclamation.
- **`src/errors/`**: Subsystem diagnostic owners (`core.rs`, `diff.rs`, `evaluator.rs`, `parser.rs`, `simplification.rs`, `uncertainty.rs`).
- **`src/diff/`**: Iterative, memoized symbolic differentiation through `Expr::derivative`, `gradient`, `jacobian`, and `hessian`; balanced product rules, regular principal-branch formulas, and sparse partial callbacks. Jacobian columns reuse completed partials and traversal storage across output roots. Hessians retain mixed operator order. Missing active partials and unsupported derivatives remain symbolic; no synthetic function identities or separate simplification pass are introduced.
- **`src/uncertainty/`**: Real first-order propagation via gradient covariance contraction. Covariance constructors validate shape, exact symmetry, and disproved scalar conditions; unknown hypotheses, positive semidefiniteness, and local linear-model validity belong to the caller. Diagonal storage is linear. Results retain sensitivity poles, symbol identities, and numeric representation tiers.
- **`src/parser/`**: Streaming UTF-8 lexer and iterative precedence parser using scoped context symbols, known function identities, and exact source `Span` diagnostics. Integer literals stay exact; decimal and scientific literals retain floating representation. Parentheses balance strictly, names stay whole, and flat sums/products use batched construction.
- **`src/simplification/`**: Reusable ordered simplification sequences with compact, expanded, factored, and compilation presets; custom rules, mapping callbacks, and scoring; shared compiled plans, cycle detection, and explicit growth/work limits. Builtin rules preserve poles and principal branches by default; explicitly selected assumption-dependent rules require caller-supplied hypotheses. Custom rule validity always belongs to the caller. Exact polynomial GCD cancellation verifies divisibility and requires a proved nonzero removed factor in preserving mode. Compilation preparation selects native numerical forms using estimated operations over shared DAG nodes. Canonical construction supplies constant folding and term collection. `src/simplification/RULES.md` specifies both domain modes and complete legacy rule coverage.
- **`src/evaluator/`**: Memoized direct evaluation and shared scalar register programs for native `f64` and commutative `Numeric` backends. Compilation expands registered bodies, removes unreachable work, compacts constants, and reuses temporary slots. Joint sine/cosine and hyperbolic sine/cosine instructions call backend paired methods and validate two distinct destinations. Native floating plans admit integer multiplication chains, sparse Horner, bounded dense Estrin, shared reciprocals, and efficient FMA; generic plans retain backend operations. Larger plans use Sethi–Ullman pressure scheduling. Borrowed disassembly exposes allocated instructions without numerical execution. SIMD and parallel execution are outside the active evaluator.
- Builtin identities, arities, folding, and domain properties belong to `src/core/logic/`. Symbolic derivative formulas belong to `src/diff/`; numerical function kernels belong to `NumAnaFis`.
- **`bindings/python/` and `python/symb_anafis/`**: A separate PyO3 extension crate and a class-based Python facade. The default Rust build does not depend on Python. Python symbols own `Expr` handles; validated coordinates borrow pinned owners only during native calls. Function builders compile once into owning symbolic templates. Callback errors are invocation-local and checked before publishing output. GC-tracked containers own separate Python references and implement traversal/clearing; never share a single `Arc<Py<_>>` reference between GC-tracked owners. Native numerical execution releases the GIL after converting inputs; evaluator copies retain independent workspaces. Public mathematical operations remain namespaced methods.

---

## 2. Correctness, Invariants & Contracts

- **Representation Invariants**:
  - **DAG Canonicity**: Every live identical node shares exactly one interned allocation. Hashing and equality checks must remain $O(1)$ pointer operations.
  - **Polynomial Normal Form**: Monomial bases are sorted in canonical structural order and own their shared expression handles directly, without a redundant base table; exponents are positive `u32` integers; coefficients share finite numeric literal handles; degree-zero terms belong to the outer sum.
  - **Numeric Soundness**: Rationals are strictly irreducible with positive denominators. Scalar floats retain bitwise representation, signed zeros, and NaN payloads without canonical unification.
  - **Symbol Lifetime Isolation**: Local symbol handles are bound by `'owner` to their context or expression. Raw IDs cannot be transmuted across owner boundaries.
  - **Function Arity Invariance**: Once registered, function arities cannot be modified. Builtin identities require zero locks or hash lookups for arity verification.
- **Transformation Invariants**:
  - **Non-Exhaustive Preservation**: Root sum/product rewrites preserve all unconsumed operands.
  - **Single-Pass Disjointness**: Replaced destination nodes are not matched again in the same rewrite pass.
  - **Transactional Rollback**: Backtracking during pattern matching rolls back variable captures and expanded nodes before exploring alternate candidates.
  - **Deterministic Bound Exhaustion**: Reaching a search work bound halts exploration immediately and returns `RewriteError::MatchLimitExceeded`; partial transformations are never published.
- **Precondition Validation**:
  - Public entry points validate arguments once. Internal hot-path kernels are infallible and must not propagate unreachable `Option` or `Result` types.
- **Arithmetic & Portability**:
  - Buffer sizing, counts, offsets, and capacities use checked arithmetic or a local bounds proof. Wrapping arithmetic is reserved for proved modular reductions or hash accumulation.
  - Primitive casts require range and pointer-width proofs ensuring identical semantics across 32-bit and 64-bit systems.
- **Production Completeness**:
  - Placeholder stubs (`todo!()`, `unimplemented!()`) and dead branches are forbidden in production code.

---

## 3. Memory Safety & Performance Discipline

- **SAFETY Proofs**: Every `unsafe` block requires an immediately preceding `// SAFETY:` comment proving the exact obligations: bounds, initialization, aliasing, alignment, lifetimes, capacity, or target prerequisites. Unsafe code is confined to synchronization primitives and the scalar register execution boundary. Register execution requires independent instruction validation before construction, immutable instructions, initialized fixed-length workspaces, and source reads completed before a destination slot is reused. Unchecked parameter copies and batch reads require complete input-shape validation first.
- **Lint Allowances**:
  - Every `allow` or `expect` attribute requires an explicit, descriptive `reason = "..."` and the narrowest useful scope.
  - Prefer `expect` for justified exceptions. Do not suppress invalid code or misplaced feature gates.
  - Blanket allowances (`allow(dead_code)`, `expect(dead_code)`) are forbidden. Dead-code warnings must remain visible during staging.
- **4-Stage Optimization Order**:
  1. **Mathematical Work Reduction**: Derive the minimal algebraic work needed for the transformation. Exploit symmetry, canonical normal forms, and short-circuit identities to eliminate unnecessary tree operations.
  2. **Algorithmic Complexity & Data Flow**: Eliminate redundant tree traversals, intermediate allocations, and un-memoized recursion. Maximize DAG node reuse through weak interning. Keep backtracking bounded.
  3. **Generated-Code Review**: Inspect Rust MIR, optimized LLVM IR, and emitted assembly for hot routines. Eliminate repeated bounds checks, redundant branches, heap spills, and unexpected dynamic dispatch.
  4. **Empirical Validation**: Benchmark against standard reference engines (Symbolica, SymEngine) using identical inputs, representative expression depths, and pinned environments.
- **Execution Flow**:
  - Enforce the chronological pipeline: `Validate once` $\to$ `Dispatch once` $\to$ `Establish invariants` $\to$ `Execute infallible kernel` $\to$ `Primitive leaf operations`.
- **Hot-Path Traversal Discipline**:
  - Never recursively traverse an expression without memoization or cycle protection.
  - Public visitors (`map`, `fold`) must visit shared subtrees predictably without generating redundant visits for internal polynomial representations.
- **Forwarding Functions**:
  - Place trivial argument adaptation and forwarding at the call site. Avoid duplicate API variants that merely toggle internal options. Keep shared algorithms, required traits, and accessors that preserve private ownership.

---

## 4. Module Organization & Visibility

- **`mod.rs` as Module Registry**: `mod.rs` serves exclusively as a structural module registry. It contains zero code (no structs, enums, functions, constants, or traits). All types reside in dedicated child files.
  Item ordering in `mod.rs`:
  1. `//!` module documentation
  2. Inner attributes
  3. `use super::{...}`
  4. `mod ...;`
  5. `pub use ...;`
  6. `#[cfg(test)] mod tests;` (last, only if tests exist)
- **Module Facades & Import Paths**:
  - Declare private children with `mod` and expose selected types with `pub use`; never use `pub mod`, including in `lib.rs`.
  - Define each subsystem's public surface in its `mod.rs`.
  - Import public subsystem types via `crate::subsystem::Type`. Do not introduce per-type files that only forward an existing type or duplicate export aliases.
  - Implementation files import parent dependencies via `super::Thing`, never `super::sibling::Thing`. Cross-subsystem access uses `crate::subsystem::Thing`; production imports must never bypass a facade.
  - Mathematical drivers and shared kernels belong to their owning type's namespace. Facades export types rather than standalone functions; differentiation uses `Differentiation` and numerical traversal uses `Traversal`.
- **Opaque Type Ownership**:
  - Public types own private fields. Implementations needing field access reside as descendants of the type owner.
  - Owners live in dedicated files (e.g. `expression.rs`) and declare implementation children directly in `expression/`. Do not introduce an intermediate folder that only forwards declarations.
  - A narrow, reasoned `clippy::self_named_module_files` expectation permits private ownership with this shallow layout. Do not use `#[path]` to relocate children.
- **File Sizing**:
  - Keep production files focused and normally at most 500 lines. Do not split or pad a file solely to meet an arbitrary line count.
- **Visibility Gates**:
  - Keep local implementation items private. Items shared across module boundaries use plain `pub`, with external reachability controlled strictly by facade reexports.
  - **`pub(crate)`, `pub(super)`, and `pub(in ...)` are strictly FORBIDDEN.**
- **Import Grouping**:
  - Group imports separated by single blank lines in this exact order:
    1. `core`/`std`
    2. `alloc`
    3. External crates (`hashbrown`, `num_anafis`, etc.)
    4. `crate::`
    5. `super::`
    6. `self::` / child modules
  - Follow `rustfmt` alphabetical ordering within each group. Wildcard imports are forbidden in production source.
- **Cfg Placement**:
  - Gate modules, imports, reexports, and Cargo targets consistently. Architecture-specific logic belongs in its designated boundary; pointer-width constraints remain where their APIs require them.

---

## 5. Source-File Structure & Technical Documentation

### Monotonic Call-Graph Ordering (The Data Cascade)
Source files follow downward execution order matching chronological data flow:
1. **Module Header & Declarations**: Module documentation (`//!`), attributes, imports, constants, and type definitions.
2. **Primary Driver / Facade Entry Point**: Inherent methods or driver functions that validate boundaries and initiate execution.
3. **Algorithmic Sequence**: Functions appear in chronological execution order.
4. **Specialized Helpers**: Functions called exclusively by intermediate stages.
5. **Infallible Leaves & Leaf Kernels**: Tree traversals, pattern matches, and leaf node constructors placed at the bottom of the file.

### Technical Documentation & Mathematical Formulations
- **Mathematical Identities**: Algebraic transformations, rewrite rules, and normal forms must be documented with explicit LaTeX formulas (`\(...\)` or `$$...$$`).
- **Rustdoc Standards**: Document API purpose, mathematical domain, panics, errors, and preconditions for every public item.
- **Scientific Prose**: Use direct, descriptive, concise scientific prose. Describe active behavior, assumptions, domains, and proofs.
- **No Historical Narratives**: Avoid historical bug narratives, change logs, and commented-out legacy code in production source.

---

## 6. Test Layout & Verification

- **Strict Test Separation**:
  - Production files contain zero test code.
  - Standalone core API suites reside in `src/core/logic/tests/`, grouped by subsystem and contract. Types with implementation children may keep their existing adjacent `tests/` directories.
  - Tests requiring private fields remain descendants of the type owner. Use its existing `tests/` directory; do not introduce a directory for an individual implementation file's tests.
  - Standalone private owners may retain adjacent test directories when descendant access is required to verify ownership, exhaustion, or synchronization invariants.
  - The only test declaration in a production `mod.rs` or type owner is `#[cfg(test)] mod tests;`. Test-only imports or reexports are forbidden in production registries.
- **Coverage Requirements**:
  - Active core transformations require property-based tests (`proptest`) with varied inputs, verifying mathematical equivalence, structural invariants, and ownership lifetimes.
  - Deterministic tests are reserved for boundary values, exact counterexamples, and discrete contract validations. Fixed examples must not be wrapped in inert generators.
- **Verification Commands**:
  - For active library development:
    ```text
    cargo check --lib
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --lib --all-features
    cargo test --doc --all-features
    ```
  - For structural and architectural compliance (when audit tools are enabled):
    ```text
    python3 tools/structure_audit.py
    python3 tools/import_audit.py
    python3 tools/check_allows.py
    ```

---

## 7. Scope Discipline & Refactoring Protocol

- **Scope Discipline**: Modifications must directly improve correctness, performance, or guideline compliance. Unrelated refactoring or aesthetic churn is prohibited.
- **Public API Stability**: Stable public APIs must not be altered without explicit mathematical or architectural justification.
- **Direct Source Editing**: Maintain Rust source code directly. Automated bulk regex/rewrite scripts are prohibited.
- **Incremental Refactoring Protocol**:
  - Legacy code archived under `old/` serves as an algorithmic reference and test oracle.
  - Subsystems (`diff`, `parser`, `simplification`, `evaluator`, `functions`) are re-engineered incrementally into `src/`.
  - Every newly migrated module must adhere to all guidelines from its initial commit: 0 Clippy warnings, 0 unjustified allowances, pure `mod.rs` registries, downward data cascade, and comprehensive property-based tests.
