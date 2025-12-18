# SymbAnaFis v0.3.1 - N-ary AST + Expression Flags Architecture

## Overview

This document outlines the implementation plan for a major performance refactor inspired by GiNaC, Symbolica, and Lisp/Maxima. The key architectural changes:

1. **N-ary Sum/Product** - Flat representation for sums and products
2. **Expression Flags** - Skip re-simplifying already-processed expressions
3. **On-demand Poly Detection** - Convert Sum to polynomial form for operations like GCD and division

### Current Performance (v0.3.1 - December 2025)
| Operation | vs SymPy | vs Symbolica |
|-----------|----------|--------------|
| Parsing | 17-21x faster ✓ | 1.2-1.9x faster ✓ |
| Differentiation (includes simplify) | Mixed | 14-49x **slower** (was 62x) |
| Simplification only | 4-43x faster ✓ | — |

**The bottleneck**: Memory allocations during diff/simplify (Arc clones). Structural hashing (v0.3.1) reduced this overhead significantly.

### Target Performance (v0.3.1)
- Polynomial simplification: **10x+ faster**
- Full diff+simplify pipeline: **within 10x of Symbolica** (currently ~18x for large exprs)

---

## Architecture

### Current: N-ary AST + Flags (Implemented)
```
x + y + z represented as:
    Sum([x, y, z])  // Flat, sorted, canonical

Expression flags:
    Expr { id, kind, flags: { is_simplified: true } }
```
**Advantages**:
- Flat structure: O(N) term combination
- Sorted on construction: canonical form
- Flags skip re-processing

### Next Step: Memory Pooling (v0.3.1)
- Replace `Arc<Expr>` with arena-allocated `ExprRef`.
- Remove atomic reference counting overhead.
- Improve cache locality.

---

## Implementation Plan

### Phase 1-5: N-ary AST & Basic Rules (Completed)
*See previous sections for details on Sum/Product migration.*

### Phase 6: On-Demand Poly Detection (In Progress)
*See `src/poly.rs` section below.*

### Phase 7: Allocation-Free Structural Hashing (Completed)

#### [MODIFY] `src/simplification/helpers.rs` (Completed)

Replaced string-based term signatures with `u64` structural hashing:
- **Commutative Hashing**: `wrapping_add` for Sum/Product (order-independent O(N)).
- **Interned IDs**: Use `InternedSymbol::id()` (O(1)) instead of string hashing.
- **Zero Allocation**: Removed all `Vec` and `String` allocations in hot path.

**Result**: 20-30% speedup on large expressions.

---

### Phase 7b: Inline Hashing (v0.3.1 - IMMEDIATE PRIORITY)

**Goal**: Eliminate `HashMap` equality check overhead (`__memcmp_evex_movbe` ~8%). Crucial prerequisite for efficient Phase 8.

**Optimization**:
- Store `hash: u64` directly in `Expr` struct.
- Compute hash once during construction (O(N)).
- Implement `PartialEq`/`Eq` to check hash first:
  ```rust
  impl PartialEq for Expr {
      fn eq(&self, other: &Self) -> bool {
          if self.hash != other.hash { return false; } // Fast reject
          self.kind == other.kind // Slow check only on collision
      }
  }
  ```
- **Benefit**: `HashMap` lookups become nearly instant key comparisons.

---

### Phase 7c: Context-Aware Parsing (Completed)

**Goal**: Enable isolated symbol scope for parallel parsing and advanced builder patterns.

**Implementation**:
- Introduced `SymbolContext` struct to hold thread-local/task-local symbol registries.
- Updated `parser::parse` to accept `Option<&SymbolContext>`.
- Modified `Diff` and `Simplify` builders to propagate context.
- Ensured thread safety for parallel operations (`Rayon`).

**Benefit**:
- Prevents symbol collisions in concurrent environments.
- Allows "local" variables that don't pollute the global symbol table.
- Essential for correct polynomial generator internals (Phase 8).


---

### Phase 8: Universal Polynomial Architecture (The "Arena" Revolution)

**Status**: Targets v0.4.0 (Prototype Branch)

**The "Nuclear Option"**: Moving from `Arc<Expr>` to `ExprId` (Arena allocation) offers massive cache locality and zero-atomic overhead wins, but risks API ergonomics.

#### Architectural Decision: The "Hybrid" Approach (Smart Handles)
To allow zero-overhead arenas while preserving `x + y` syntax:
1.  **Context**: A thread-local or passed `Context` struct holding the Arena.
2.  **Smart Handles**: `ExprRef` struct wrapping `(ExprId, &Context)`.
3.  **Global Registry Backing**: `Symbol` remains a `Copy` `u64` wrapper backed by a global registry for convenience (like `println!("{}", x)`), while Contexts provide isolation.

#### The "Gotcha": Context vs Global Collision
- `Expr::symbol("x")` (Global ID) != `ctx.symb("x")` (Context ID).
- **Rule**: When using a Context, ALWAYS create variables via `ctx` to avoid collisions. The parser has been updated (Phase 7c) to respect this rule.

#### Implementation Strategy
- **Stage 1 (v0.3.1)**: Inline Hashing (Phase 7b) and Context-Aware Parsing (Phase 7c).
- **Stage 2 (Technically v0.4.0)**: Create `ExprArena`, `ExprId`, and `ExprRef`.
    - Prototype in a separate branch to benchmark raw speedup (target: 3x).
    - If successful, merge as the new engine core.

#### Expected Benefit
- **Differentiation**: `d(P)/dx` becomes simple polynomial derivative over the ring.
- **Simplification**: Like terms combine automatically by reduction.
- **Parsing**: "Bump allocation" speed (instant).

---

## Second Bottleneck Analysis: Hashing Layout

**Observation**: `HashMap` lookups consume ~7-8% of runtime, specifically `__memcmp_evex_movbe` during collision resolution or equality checks.

**Optimization**:
- **Inline Hashing**: Store the hash *in* the `Expr` struct to avoid re-hashing during lookup.
- **Interned Keys**: For `HashMap<Expr, V>`, use `ExprId` as the key directly (perfect hashing equivalent) if we have unique cons-ing.

---

## Files Modified Summary

| File | Changes |
|------|---------|
| `src/ast.rs` | Add ExprFlags, Sum, Product; remove Add/Sub/Mul |
| `src/parser/pratt.rs` | Produce Sum/Product |
| `src/display.rs` | Display Sum/Product |
| `src/differentiation.rs` | Handle Sum/Product |
| `src/simplification/engine.rs` | Early termination, Sum/Product |
| `src/simplification/rules/mod.rs` | Add Sum/Product to ExprKind enum |
| `src/simplification/rules/numeric/` | Combine numbers in Sum/Product |
| `src/simplification/rules/algebraic/` | Combine like terms, powers |
| `src/simplification/helpers.rs` | **New**: Allocation-free get_term_hash |
| `src/poly.rs` | Add try_from_sum, to_sum |
| `src/visitor.rs` | Handle Sum/Product |
| `src/symbol.rs` | **New**: `SymbolContext` for isolated parsing scopes |
| `src/parser/` | Context-aware parsing signatures |
| `src/builder.rs` | Context propagation in Builder API |
| All test files | Update for new AST |

---

## Success Criteria

- [x] `cargo test` passes (all existing tests)
- [x] FunctionCall names interned for O(1) comparison (diff_full up to 67% faster)
- [ ] Polynomial simplification 10x+ faster
- [x] No regression in parsing, evaluation, display
- [x] Clean removal of binary Add/Sub/Mul variants
- [x] Reduce differentiation overhead (via structural hashing)
