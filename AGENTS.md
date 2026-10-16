# SymbAnaFis - Project Guidelines

This document specifies the architecture, invariants, memory safety contracts, performance discipline, module organization, and verification procedures for the `SymbAnaFis` Rust crate (`symb_anafis` in Cargo).

The current legacy implementation is archived under `old/` as behavioral reference. Active subsystems are re-engineered directly under `src/` following these guidelines.

---

## 1. Priorities & System Architecture

`SymbAnaFis` is a high-performance symbolic mathematics library in Rust designed for exact algebraic manipulation, term rewriting, symbolic differentiation, and numerical code generation.

### Core Types & Representation Contracts

- **`Expr`**: A one-pointer owning handle (`Option<Arc<Node>>` utilizing the null-pointer niche) over a private, immutable node enum.
  - Every node is constructed via library-wide, collision-safe weak interning. Identical live subtrees share exactly one allocation.
  - Structural equality (`Eq`, `PartialEq`) and hashing (`Hash`) are $O(1)$ canonical pointer comparisons.
  - Specialized multivariate polynomials (`Node::Poly`) remain private. Public views expose logical sums with lazy child iterators; public mapping and folding visit logical monomials once without introducing visits for internal additive storage groups.
  - Operators, mathematical methods, and n-ary vector constructors share normalized kernels. Keep consuming arithmetic kernels private rather than exposing duplicate technical constructors.
- **Numeric Literals (`NumericValue`)**: Representation-preserving DAG identity over `num_anafis::Number`.
  - Retains exact integers, reduced rationals, native IEEE floats, and scalar complex components.
  - Structural literal identity is strictly distinguished from numeric equality (e.g. `2.0` float vs `2` integer).
  - Exact components stay exact (rationals are irreducible with strictly positive denominators); approximate floats stay approximate (no auto-promotion to exact tiers).
- **`Symbol<'owner>`**: A validated native-width `Copy` identifier with a zero-sized owner lifetime.
  - Global handles (`Symbol<'static>`) have permanent metadata in the sharded directory.
  - Local handles borrow a `Context` or an owning `Expr`; the compiler enforces that they cannot outlive that owner.
  - Name unbinding preserves identities within the owner's lifetime; rebinding creates a fresh unique identity. Raw IDs cannot bypass the owner boundary.
- **`Function`**: An opaque identity containing a static `BuiltinFunction` enum or an owning canonical custom name.
  - Builtin functions pass enum identities directly; arity, derivatives, and exact folding dispatch to static definitions without intern locks or hash-map lookups.
  - Custom functions register unique, sparse, sorted partial derivatives by argument index. Duplicate registration returns an error without mutating existing definitions.
  - Registered arities are immutable. Unknown strings cannot create function identities or AST calls.
- **`Pattern` / `RewriteRule`**: Structural pattern matching with capture groups and memoized replacement across every edge to a shared input node.
  - **Associative / Commutative Matching**: Root sum and product patterns match disjoint commutative operand groups with backtracking and transactional undos (`Undo::Popped`, `Undo::Capture`). Unmatched operands survive automatically. Nested groups require complete coverage.
  - **Bounded Search**: `match_limit: Option<usize>` strictly bounds candidate permutations across the entire transformation pass, returning `RewriteError::MatchLimitExceeded` on exhaustion without publishing corrupted partial output.
  - **Guards**: `RewriteGuard` callbacks allow callers to enforce semantic conditions on captured bindings (e.g., domain proofs, sign checks, integrality) before accepting an assignment.
  - **Counted Multiplicities**: Power matching consumes positive integer powers and collected integer addends without expanding exponents into repeated handles.
- **`Substitution` / `Substitutions`**: Concrete expression replacement by canonical identity, including logical polynomial monomials.
  - Exact whole matches take priority over partial groups. Entries apply simultaneously without revisiting destination subtrees in the same pass.
  - Duplicate sources accept identical destinations and reject conflicting destinations.
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

- **`src/collections/`**: Private `no_std` collection facades (`HashMap`, `HashSet`) backed by `hashbrown` and deterministic hashing (`foldhash`).
- **`src/core/api/`**: Public borrowed views (`ExprView`) and inspectors.
- **`src/core/logic/`**: Type owners, canonical construction, weak interning, symbol/function registries, traversal, rewriting, substitutions, and memory reclamation.
- **`src/errors/`**: Subsystem diagnostic owners (`core.rs`, `diff.rs`, `parser.rs`).
- **`src/diff/`**: Symbolic differentiation engine, chain rule, product rule, quotient rule, and sparse partial callbacks. *(In staging)*
- **`src/parser/`**: Pratt text-to-AST parser, precedence climbing, and exact source `Span` diagnostics. *(In staging)*
- **`src/simplification/`**: Term canonicalization, like-term collection, constant folding, and identity rules powered by `RewriteRule` and multivariate polynomial normal forms. *(In staging)*
- **`src/evaluator/`**: Numerical evaluation pipelines, bytecode virtual machines, and vectorized execution backends. *(In staging)*
- **`src/functions/`**: Built-in mathematical functions registry and algebraic domain properties. *(In staging)*

---

## 2. Correctness, Invariants & Contracts

- **Representation Invariants**:
  - **DAG Canonicity**: Every live identical node shares exactly one interned allocation. Hashing and equality checks must remain $O(1)$ pointer operations.
  - **Polynomial Normal Form**: Monomial bases are sorted in canonical structural order; exponents are positive `u32` integers; coefficients share finite numeric literal handles; degree-zero terms belong to the outer sum.
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

- **SAFETY Proofs**: Every `unsafe` block requires an immediately preceding `// SAFETY:` comment proving the exact obligations: bounds, initialization, aliasing, alignment, lifetimes, capacity, or target prerequisites. (Unsafe code is strictly quarantined to synchronization primitives like `mutex.rs`).
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
  - Public visitors (`map_logical`, `fold_monomials`) must visit shared subtrees predictably without generating redundant visits for internal polynomial representations.
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
  - Tests reside in dedicated `tests.rs` or `tests/` directories located directly beside the files they test.
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
    python3 -m unittest discover -s tools/audit/tests
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
