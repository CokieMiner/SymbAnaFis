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

### Phase 7b: Inline Hashing (v0.3.1 - Next)

**Goal**: Eliminate `HashMap` equality check overhead (`__memcmp_evex_movbe` ~8%).

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

### Phase 8: Memory Pooling (v0.3.1 Target)

**Goal**: Elimination of `malloc`/`free` overhead and atomic reference counting (currently ~15% of runtime).

#### Architecture Change: Arena-based AST
Shift from `Arc<Expr>` to `ExprId` handles:

```rust
pub struct ExprArena {
    nodes: Vec<Expr>,        // Contiguous memory
    syms: Interner<String>,  // Deduplicated strings
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(u32); // 4-byte handle
```

#### Refactoring Scope (High Impact)
1. **Core AST**: Modify `ExprKind` to store `ExprId` instead of `Arc<Expr>`.
2. **Rules**: Update all ~35 simplification rules to take `&ExprArena` context.
3. **Engine**: Manage arena lifetime and garbage collection (if needed).

#### Expected Benefit
- **Performance**: 2x-5x speedup in differentiation/simplification (cache locality + no allocation).
- **Memory**: ~40% reduction in memory usage (no Arc overhead, smaller struct).

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
| All test files | Update for new AST |

---

## Success Criteria

- [x] `cargo test` passes (all existing tests)
- [x] FunctionCall names interned for O(1) comparison (diff_full up to 67% faster)
- [ ] Polynomial simplification 10x+ faster
- [x] No regression in parsing, evaluation, display
- [x] Clean removal of binary Add/Sub/Mul variants
- [x] Reduce differentiation overhead (via structural hashing)
