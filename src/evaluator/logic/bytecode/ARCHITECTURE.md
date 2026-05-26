# Bytecode Compiler Architecture (AnaFis Symbolic Engine)

This document describes the lifecycle of a mathematical expression from its AST to optimized bytecode executed by a register-based virtual machine.

---

## 1. Module Map

```
bytecode/
├── instruction.rs        ISA definition (define_isa! macro, 43-instruction set)
├── functions.rs          Builtin function table (FnOp: ~60 math functions)
├── mod.rs                Public re-exports
│
├── compile/              Compilation pipeline (Expr → bytecode)
│   ├── compiler.rs       VirGenerator orchestrator
│   ├── codegen/
│   │   ├── traverse.rs   Iterative postorder tree walk (stack-based, no recursion)
│   │   ├── expand.rs     User-function inlining
│   │   └── lower/        Per-ExprKind lowering to VIR
│   │       ├── sum.rs    Sum compilation (FMA detection, negated-term extraction)
│   │       ├── product.rs  Product compilation (constant folding)
│   │       ├── div.rs    Division compilation (RecipExpm1, Sinc detection)
│   │       ├── pow.rs    Power compilation (Horner/Estrin, half-integer exponents)
│   │       ├── func.rs   Function-call compilation (exp(-x²) patterns)
│   │       ├── misc.rs   Helper traversal, symbol dispatch
│   │       ├── exp_helpers.rs  Exponential pattern helpers
│   │       └── emit_helpers.rs  N-ary add/mul emission
│   ├── vir/              Virtual Intermediate Representation
│   │   ├── types.rs      VReg (Param/Const/Temp), VInstruction (31 variants)
│   │   ├── node.rs       NodeData, constant folding, const_from_map
│   │   ├── matcher.rs    Pattern matchers (negated_product_two_vregs, etc.)
│   │   └── registry.rs   FN_MAP: interned-function-name → FnOp lookup
│   ├── analysis/         VIR-level passes
│   │   ├── gvn.rs        Global Value Numbering + LVN + constant folding
│   │   ├── fusion.rs     optimize_div_to_recip, fuse_vir (pre-scheduling)
│   │   ├── schedule.rs   Greedy Sethi-Ullman scheduler (CSR graph, BinaryHeap)
│   │   └── dce.rs        eliminate_vir_dead_code (backward liveness)
│   ├── emit/             Physical lowering
│   │   ├── reg_alloc.rs  Linear-scan register allocator
│   │   └── assemble.rs   assemble_flat_bytecode: VInstruction → Box<[u32]>
│   └── optimize/         Physical-instruction passes
│       ├── pipeline.rs   optimize_instructions orchestrator
│       ├── power_chain.rs  reduce_strength + optimize_power_chains
│       ├── fusion.rs     Peephole fusion (FMA, inverse, exp, sin-cos)
│       ├── dce.rs        Dead code elimination + copy forwarding (path compression)
│       ├── compact.rs    Constant pool compaction + register re-indexing
│       └── helper.rs     ConstantPool, calculate_use_count, validate_program
│
└── execute/              Bytecode execution
    ├── engine/
    │   ├── macros.rs     dispatch_loop! macro (shared scalar/SIMD interpreter)
    │   ├── scalar.rs     VmEvaluator::evaluate, evaluate_heap, eval_points_into
    │   ├── simd.rs       eval_batch_simd (vertical f64x4, 4-wide lanes)
    │   ├── builtins.rs   eval_builtin1/2/3/4 (scalar), eval_builtinN_simd
    │   └── mod.rs
    └── drivers/
        ├── parallel.rs   evaluate_parallel, evaluate_parallel_with_hint (Rayon)
        ├── batch.rs      run_chunked_evaluator (SIMD chunked dispatch)
        └── mod.rs
```

---

## 2. Instruction Set

The ISA is defined by the `define_isa!` macro in `instruction.rs`. It produces 43 instructions with dense integer opcodes 0–42 — verified at compile time by `Instruction::OPCODE_COUNT` and the `isa_opcodes_are_dense` test. This density guarantees a single-indirect-branch jump table at dispatch time.

**Unified Memory Layout:** The physical register file is a contiguous `[Params | Constants | Temps]` array. Registers 0..param_count are input parameters. Registers param_count..param_count+const_count hold deduplicated constants (bulk-copied once before dispatch). The remaining slots are temporaries.

**Specialized opcodes** include:
| Family | Instructions |
|---|---|
| Arithmetic | `Add`, `Add3`, `Add4`, `AddN`, `Mul`, `Mul3`, `Mul4`, `MulN`, `Sub`, `Div` |
| FMA | `MulAdd`, `MulSub`, `NegMul`, `NegMulAdd`, `NegMulSub` |
| Powers | `Square`, `Cube`, `Pow4`, `Pow3_2`, `InvPow3_2`, `InvSqrt`, `InvSquare`, `InvCube`, `Recip`, `Powi` |
| Transcendental | `Sin`, `Cos`, `SinCos`, `AsinAcos`, `Exp`, `Ln`, `Sqrt` |
| Specialized | `RecipExpm1`, `ExpSqr`, `ExpSqrNeg` |
| Builtins | `Builtin1`-`Builtin4` (arbitrary FnOp via indirect call) |
| Data movement | `Copy`, `Neg`, `End` |

N-ary instructions (`AddN`, `MulN`) use a separate **arg_pool** buffer for their operand lists, accessed via `(start_idx, count)` indices. The 2/3/4-operand variants avoid this indirection by encoding operands inline in the bytecode stream.

**Flat bytecode** is `Box<[u32]>`. Each instruction is encoded as `[opcode, dest, operand...]` with width 2-7 u32s. The stream is terminated by a single `End` sentinel (0), avoiding bounds checks in the dispatch loop. Constants and parameters are stored in separate `Box<[f64]>` buffers and bulk-copied into the register file before evaluation begins.

---

## 3. Compilation Pipeline

Compilation has two phases: a **VIR phase** operating on `VInstruction` (virtual registers, n-ary ops, builtin dispatch), followed by register allocation and a **physical phase** on `Instruction` (physical registers, fixed arity).

### Phase 1: VIR (in `compiler.rs::into_parts`)

```
GVN → Div-to-Recip → Pre-scheduling Fusion → Greedy Schedule → VIR DCE → RegAlloc
```

| Pass | File | Description |
|---|---|---|
| **GVN** | `analysis/gvn.rs` | Global Value Numbering + constant folding + identity simplification + LVN dedup. The key loop sets `dest` to a sentinel for hash-lookup, avoiding a clone on cache hits. |
| **Div-to-Recip** | `analysis/fusion.rs` | Identifies redundant divisions sharing a denominator; replaces with a single `Recip` + multiplications. |
| **Pre-scheduling Fusion** | `analysis/fusion.rs` | Light VIR fusion (`Mul+Add→MulAdd`, `Neg+Mul→NegMul`) so the scheduler treats fused units atomically. |
| **Greedy Schedule** | `analysis/schedule.rs` | Sethi-Ullman-inspired topological sort. Builds a CSR dependency DAG, prioritizes instructions that kill many registers. Uses a `BinaryHeap` for O(n log n) extraction. |
| **VIR DCE** | `analysis/dce.rs` | Backward liveness DCE. Eliminates instructions whose destination temp is never read. |
| **RegAlloc** | `emit/reg_alloc.rs` | Linear-scan register allocation. Maps unbounded `VReg::Temp` to physical slots via a free-list stack. Aggressively recycles slots using precomputed death-heads (linked-list bucket sort by last-use index). |

### Phase 2: Physical (in `pipeline.rs::optimize_instructions`)

```
Strength → Power → DCE → Fusion(loop) → DCE → Compact
```

| Pass | File | Description |
|---|---|---|
| **Strength Reduction** | `optimize/power_chain.rs` | `x/2.0 → x*0.5`, `x² → Square(x)`, etc. |
| **Power Chain** | `optimize/power_chain.rs` | `x³ = x² * x`, `x⁴ = (x²)²` — reuses previous power results. |
| **DCE** | `optimize/dce.rs` | Dead code elimination + copy forwarding with path compression. Resolves multi-hop forwarding chains (`T5→T4→T3→Param`) to immutable roots. |
| **Fusion loop** | `optimize/fusion.rs` | Peephole FMA detection (`[Mul,Add]→MulAdd`, `[Mul,Sub]→MulSub`, reversed `NegMulAdd`), inverse fusion (`[Sqrt,Recip]→InvSqrt`), exponential fusion (`[Neg,Exp]→ExpNeg`), power fusion (`[Square,Mul]→Cube`), sin-cos pair fusion. Iterates until convergence (bounded at 32). |
| **Compact** | `optimize/compact.rs` | Removes unused constants, shifts remaining constants and all temporaries down to create a dense minimal workspace. |

### Codegen: Lowering (`codegen/lower/`)

The lowering pass walks the postorder-traversed AST and emits `VInstruction` sequences directly:

- **FMA by construction:** `sum.rs` detects `a*b ± c` patterns and emits `MulAdd`/`MulSub`/`NegMulAdd`/`NegMulSub` directly, without relying on later peephole fusion.
- **Negated-term extraction:** `try_extract_negated_product` strips a `-1` factor from products, converting `a - b*c` → `MulSub` or `a + -(b*c)` → `Sub`.
- **Polynomial codegen:** `pow.rs` uses Horner's method for degree < 4, sparse Estrin's scheme for degree ≥ 4.
- **Recursion-free traversal:** `traverse.rs` uses an explicit stack with pointer-based deduplication (`FxHashSet<*const Expr>`), handling DAG-sharing correctly without recursion.

---

## 4. Execution Engine

### Dispatch Loop (`macros.rs`)

The `dispatch_loop!` macro is shared between scalar and SIMD engines via `$mode:tt` dispatch. It generates a `loop { match opcode { ... } }` over the flat `Box<[u32]>` bytecode using raw pointer arithmetic:

- `let opcode = *pc; pc = pc.add(1);` — no bounds check, terminated by `End` sentinel (opcode 0)
- Dense integer match (0-42) → LLVM jump table → O(1) single indirect branch
- Register access: `*($regs.add(idx))` via raw pointer offset
- Builtins dispatched via `$b1`/`$b2`/`$b3`/`$b4` function pointers

### Scalar Engine (`scalar.rs`)

Stack allocation staircase for the register file:
```
workspace ≤ 64  → MaybeUninit<[f64; 64]>   on stack  (512 bytes, L1)
workspace ≤ 128 → MaybeUninit<[f64; 128]>  on stack  (1 KB)
workspace ≤ 256 → MaybeUninit<[f64; 256]>  on stack  (2 KB)
workspace > 256 → thread_local! heap Vec<f64>          (reused, RefCell-guarded)
```

Constants are `copy_nonoverlapping`'d once before the dispatch loop. Parameters are written per evaluation. The dispatch loop then executes over the flat bytecode.

### SIMD Engine (`simd.rs`)

Uses `wide::f64x4` (4 lanes, 256-bit). Strategy: **vertical vectorization** — the same bytecode instruction is applied to 4 data points simultaneously. The register file holds `f64x4` instead of `f64`.

- Input columns loaded via contiguous `f64x4::from([f64; 4])` casts
- Output stored via `copy_from_slice(&res)` in 4-point chunks
- Tail handling (0-3 remaining points) falls back to scalar `eval_points_into`
- 17 of ~42 builtins have native `f64x4` SIMD paths (Sin, Cos, Tan, Exp, Abs, Asin, Atan, Sinh, Cosh, Tanh, etc.); remaining 25 use lane-by-lane `arr.map(f64::fn)` fallback
- Binary builtins (Bessel*, Polygamma, Beta, Hermite, etc.) are lane-by-lane with zero SIMD speedup

### Parallel Execution (`drivers/`)

`run_chunked_evaluator` splits the output into 256-point chunks and dispatches via Rayon:

- Each chunk is processed by `eval_batch` → `eval_batch_simd`
- `try_for_each_init` creates one SIMD workspace + column-slice buffer per Rayon job (~thread count), not per chunk
- The `evaluate_parallel` API provides a clean macro (`eval_parallel!`) and typed result handling (`EvalResult`)
- `evaluate_parallel_with_hint` accepts pre-computed numeric hints from Python bindings to skip per-point type checks

---

## 5. Key Design Decisions

| Decision | Rationale |
|---|---|
| Dense 0..42 opcodes | Guarantees jump table dispatch (verified by test) |
| N-ary AddN/MulN + specialized Add3/Add4/Mul3/Mul4 | Avoid arg_pool indirection for small N; support large fan-in for flat DAGs |
| Stack staircase (64→128→256→heap) | Zero-cost for common expressions; defers heap allocation |
| Vertical SIMD (f64x4) | Maximizes throughput; no gather/scatter needed |
| Linear-scan register allocator | O(n) vs graph-coloring O(n²); sufficient for single-basic-block VIR |
| Flat u32 bytecode | Dense encoding, sequential access, hardware prefetch |
| GVN before scheduling | Deduplicates shared sub-expressions before live-range decisions |
| Path-compression copy forwarding | O(n) resolution of arbitrary-length forwarding chains (replaced capped 4-iteration loop) |
| Constants bulk-copied once | No constant-pool lookups in the dispatch loop |
| Re-entrancy guard via RefCell::try_borrow_mut | Prevents UB from recursive evaluation without per-call allocation |
