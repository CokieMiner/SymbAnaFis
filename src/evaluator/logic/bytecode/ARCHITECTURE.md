# Bytecode Compiler Architecture (AnaFis Symbolic Engine)

This document describes the lifecycle of a mathematical expression (`Expr`) from its abstract syntax tree (AST) to a high-performance, register-based bytecode executed by a specialized virtual machine. The pipeline is designed for maximum throughput, low latency, and minimal memory overhead.

---

## 1. The Compilation Pipeline (`compile/`)

The compilation process transforms a high-level symbolic tree into a linear sequence of optimized instructions.

### A. The Orchestrator: `compiler.rs`
The `VirGenerator` manages the compilation state, including virtual register allocation and instruction generation. It lowers the AST into **Virtual IR (VIR)**, a pseudo-assembly format that supports unbounded temporary registers.

### B. Tree Management and Codegen (`compile/codegen/`)
*   **`traverse.rs`**: Implements a **recursion-free, iterative post-order traversal**. By using an explicit stack, it can handle expressions with millions of nodes (e.g., deep nested divisions) without risking a stack overflow.
*   **`expand.rs`**: Handles function inlining and macro expansion before lowering to VIR.
*   **`lower/`**: Transcribes `ExprKind` nodes into low-level `VInstruction` sequences.
    *   **Dense-by-Construction**: The lowering pass actively emits specialized fused instructions (e.g., `MulAdd`, `NegMulAdd`) where possible.
    *   **Polynomial Optimization**: Implements **Horner's Method** (for degree < 4) and **Sparse Estrin's Scheme** (for degree >= 4) to minimize instruction count for polynomial evaluations.

### C. Virtual Intermediate Representation (`compile/vir/`)
*   **`types.rs`**: Defines `VReg` (Virtual Register) types: `Param`, `Const`, and `Temp`. Supports N-ary operations through `Vec<VReg>` operand lists.
*   **`node.rs`**: Performs compile-time **Constant Folding** and expression collapsing (e.g., `sin(0)` -> `0.0`).

### D. Analytics (`compile/analysis/`)
*   **`gvn.rs`**: Implements **Global Value Numbering** combined with **Commutative Normalization**. It ensures that algebraically equivalent sub-expressions (e.g., `a+b` and `b+a`) are deduplicated into a single value number.

### E. Optimization Passes (`compile/optimize/`)
The pipeline follows a **Transform > Clean > Fuse > Polish** strategy:
`GVN > Strength > Power > Schedule > DCE > Fusion > DCE > Compact`.

*   **`schedule.rs` (Instruction Scheduler)**: A critical performance pass that reorders instructions to **minimize peak register pressure**.
    *   **Heuristic**: Uses a Sethi-Ullman-inspired greedy topological sort.
    *   **Weights**: Prioritizes instructions that "kill" the most active registers (reducing live ranges).
    *   **Representation**: Uses a high-performance **Compressed Sparse Row (CSR)** graph layout to represent the dependency DAG with zero-allocation overhead during construction.
*   **`fusion.rs` (Peephole Optimizer)**: Fuses instructions into specialized opcodes.
    *   **N-ary Fusion**: Detects patterns like `Mul(A, B) + C + D` and fuses them into native `Add3(Mul(A, B), C, D)` or `Add4` variants.
    *   **FMA Extraction**: Actively extracts Fused-Multiply-Add (`MulAdd`) and Fused-Multiply-Subtract patterns.
*   **`power_chain.rs`**: Optimizes power sequences (e.g., $x^2, x^3, x^4$) by reusing previous results (e.g., $x^3 = x^2 \times x$).
*   **`strength_reduction.rs`**: Replaces expensive operations with cheaper alternatives (e.g., `x / 2.0` -> `x * 0.5`, `x^2` -> `Square(x)`).
*   **`dce.rs` (Dead Code Elimination)**: Removes redundant instructions and performs copy-forwarding to simplify the DAG for fusion.
*   **`compact.rs`**: Performs register and constant re-indexing to eliminate holes created by optimization passes, improving L1 cache locality for the register file.

### F. Physical Emission (`compile/emit/`)
*   **`reg_alloc.rs`**: Maps unbounded virtual registers to a fixed set of physical slots using a **Linear Scan Register Allocation** algorithm. It tracks liveness to aggressively reuse slots, keeping the required "workspace" size minimal.

---

## 2. Virtual Machine and Execution (`execute/`)

The execution engine is designed for zero-overhead dispatch and cache-friendly data access.

### 2.1 The Dispatch Loop (`engine/`)
The VM uses a **Register-Based Architecture** with a dense, sequential opcode set.
*   **Jump Tables**: Opcodes are grouped logically (Add-family, Mul-family, etc.) and assigned sequential indices (0-41). This allows the compiler to generate a high-speed $O(1)$ jump table for the main loop.
*   **Specialized Opcodes**: To avoid the overhead of generic N-ary loops, the engine provides native implementations for `Add3`, `Add4`, `Mul3`, and `Mul4`. These fetch operands directly from the instruction stream without indirection.
*   **Unsafe Optimization**: Uses `unsafe` pointer arithmetic and `.get_unchecked()` to bypass bounds checks, relying on the compiler's mathematical proof of register safety.

### 2.2 Memory Management
*   **Thread-Local Stack**: For small expressions, the register "workspace" is allocated on the **CPU Thread Stack** using `MaybeUninit` to avoid zeroing overhead.
*   **Global Fallback**: Large expressions automatically spill to a pre-allocated heap buffer if the stack limit is exceeded.

### 2.3 Vectorization and Parallelism
*   **SIMD Engine (`simd.rs`)**: Uses `wide::f64x4` (or `f64x8`) to evaluate 4-8 data points in parallel per cycle.
*   **Work-Stealing Parallelism (`parallel.rs`)**: For large datasets, the engine uses **Rayon** to distribute chunks of data across all available CPU cores. Each core executes its own SIMD-optimized instance of the VM, achieving massive throughput.

### 2.4 Performance Summary
The combination of **GVN deduplication**, **Register Pressure Scheduling**, **Specialized Opcode Fusion**, and **SIMD Execution** allows AnaFis to match or exceed the performance of native-compiled code for complex symbolic expressions.
