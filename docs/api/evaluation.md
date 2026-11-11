# SymbAnaFis - Numerical Evaluation API

Reference for direct scalar evaluation, compiled register VM programs, native JIT compilation, standalone WebAssembly (`.wasm`) export, disassembly, batch evaluation, custom bodies, and numerical backends in `symb_anafis`.

[← Back to API Index](../API.md)

---

### Contents

1. [Direct evaluation](#1-direct-evaluation)
2. [Compiled scalar programs](#2-compiled-scalar-programs)
3. [Inspecting compiled instructions](#3-inspecting-compiled-instructions)
4. [Scalar batches](#4-scalar-batches)
5. [Native JIT compilation](#5-native-jit-compilation)
6. [Standalone WebAssembly export](#6-standalone-webassembly-export)
7. [Custom bodies](#7-custom-bodies)
8. [Backends and floating arithmetic](#8-backends-and-floating-arithmetic)
9. [Errors and resource use](#9-errors-and-resource-use)

---

### 1. Direct evaluation

`Expr::evaluate` computes a scalar result from symbol–value bindings. It visits shared computed nodes once per call and uses an explicit stack for deep subgraphs. It does not compile a register program or substitute numeric expressions into the DAG.

```rust
use symb_anafis::{Context, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let expression = (x.pow(2) + y) / 3;

    let exact = expression.evaluate([
        (x, Number::from_int(2)),
        (y, Number::from_int(1)),
    ])?;
    assert_eq!(exact, Number::from_int(5) / Number::from_int(3));

    let approximate = expression.evaluate([(x, 2.0), (y, 1.0)])?;
    assert!((approximate - 5.0 / 3.0).abs() < 1.0e-15);
    Ok(())
}
```

Bindings use symbol identity. Symbols with the same display name in different contexts are distinct. Binding order is unrestricted; duplicate bindings return `EvalError::DuplicateParameter`. Unused bindings are permitted. Every symbol needed by the expression or an expanded custom body must be bound.

The selected scalar determines function domains. `Number` can promote supported real inputs to complex results, while `Real`, `f64`, and `f32` retain their real-domain behavior:

```rust
use symb_anafis::{Context, Number};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let root = x.to_expr().sqrt();
    let value = root.evaluate([(x, Number::from_int(-4))])?;
    assert_eq!(value, Number::from_complex(Number::zero(), Number::from_int(2)));
    assert!(root.evaluate([(x, -4.0)])?.is_nan());
    Ok(())
}
```

For a constant without bindings, specify the scalar explicitly: `expression.evaluate::<Number>([])`.

---

### 2. Compiled scalar programs

`Program::<Value>::compile(&expression, &parameters)` builds an immutable reusable register program. Parameters have an explicit order, including unused parameters. Repeated subexpressions share instructions, constants are initialized once, and temporary registers are reused after their last read.

A `Program` contains no mutable execution state. It can be cloned cheaply, wrapped in `Arc`, and shared between threads. Each concurrent evaluation owns a `Workspace`, created via `Workspace::new()` (which binds lazily on first evaluation). A workspace can be reused across different programs (both VM and native-backed); evaluation reinitializes its constants and reuses the allocation when possible.

```rust
use symb_anafis::{Context, Program, Workspace};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let expression = ctx.parse("x^3 + 2*x + 1")?;
    let program = Program::<f64>::compile(&expression, &[x])?;

    let mut workspace = Workspace::new();
    assert_eq!(program.eval_with(&mut workspace, &[2.0])?, 13.0);
    assert_eq!(program.eval_with(&mut workspace, &[3.0])?, 34.0);
    assert_eq!(program.eval(&[4.0])?, 73.0);
    Ok(())
}
```

`Program` is the single reusable execution handle; `Workspace` owns all mutable execution state. Sequential callers reuse one workspace across evaluations, while concurrent callers create a distinct workspace for each thread. Cloning a program shares immutable instructions and any attached native kernel; cloning a workspace copies its registers and bound-program metadata handle.

A program retains parameter metadata and remains usable after its context and input expression are dropped. `parameters()` returns borrowed symbol handles in input order; `instruction_count()` and `workspace_size()` report program instructions and initialized scalar slots.

For systems of equations or vector fields (such as gradients, ODE right-hand sides, or coupled dynamics), `MultiProgram::<Value>::compile(&expressions, &parameters)` compiles multiple expressions into one joint register program sharing CSE, constants, shared reciprocals, and paired `sin`/`cos` + `sinh`/`cosh` evaluations across all outputs. It supports the same `Workspace`, `with_native()`, `to_wasm_bytes()` (WebAssembly multi-value return), `eval`/`eval_with`, `eval_batch`/`eval_batch_with`, and `eval_batch_par` execution paths:

```rust
use symb_anafis::{Context, MultiProgram};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let (x, y) = (ctx.symb("x")?, ctx.symb("y")?);
    let denom = x.pow(2) + y.pow(2) + 1;
    let system = MultiProgram::<f64>::compile(
        &[(x.sin() + y) / &denom, (x.cos() - y) / &denom],
        &[x, y],
    )?;
    let system = system.with_native().unwrap_or(system);
    let out = system.eval(&[0.0, 2.0])?;
    assert_eq!(out, vec![0.4, -0.2]);
    Ok(())
}
```

Execution does not construct DAG nodes, resolve function names, or execute custom symbolic bodies. The scalar backend can allocate internally, for example for multiprecision values.

---

### 3. Inspecting compiled instructions

`disassemble()` returns a borrowed display of the compiled program. It lists parameter identities, constant values, physical register operands, builtin names, and the result register. Formatting neither evaluates the program nor changes any workspace. The display itself does not allocate; `format!` or `to_string()` can collect the text when needed.

```rust
use symb_anafis::{Context, Program};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let program = Program::<f64>::compile(&(x.sin() + 1), &[x])?;
    let listing = format!("{}", program.disassemble());
    assert!(listing.contains("sin r0"));
    assert!(listing.contains("return r"));
    Ok(())
}
```

An instruction may overwrite a register after its last read. Listings describe the allocated numerical plan, including polynomial lowering and fused operations, rather than the original expression tree. Current input values and temporary contents are omitted. The text layout is not a serialization contract.

When sine and cosine share one compiled argument, their evaluations form a two-output `sin_cos` instruction. The same applies to `sinh_cosh`. The scalar backend owns shared numerical work and the precision of both components. The expression DAG retains its separate function nodes.

```rust
use symb_anafis::{Context, Program};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let expression = x.sin() + x.cos() + x.sinh() + x.cosh();
    let program = Program::<f64>::compile(&expression, &[x])?;
    let listing = format!("{}", program.disassemble());
    assert!(listing.contains("sin_cos"));
    assert!(listing.contains("sinh_cosh"));
    Ok(())
}
```

---

### 4. Scalar batches

`eval_batch` and `eval_batch_with` accept one column per parameter and fill caller-owned output (`eval_batch_with` reuses a caller-owned `Workspace`). Every column must have exactly the output length. Shape validation completes before any output is written.

```rust
use symb_anafis::{Context, Program, Workspace};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let program = Program::<f64>::compile(&(x.pow(2) + y), &[x, y])?;
    let mut workspace = Workspace::new();
    let xs = [1.0, 2.0, 3.0];
    let ys = [10.0, 20.0, 30.0];
    let mut output = [0.0; 3];
    program.eval_batch_with(&mut workspace, &[&xs, &ys], &mut output)?;
    assert_eq!(output, [11.0, 24.0, 39.0]);
    Ok(())
}
```

Empty batches are accepted. A zero-parameter program accepts no columns and repeats its result for every output element. Native floating VM programs execute multi-row batches in blocks of sixteen lanes; programs upgraded via `with_native()` lazily compile a SIMD batch kernel on supported CPUs (see [Native JIT compilation](#5-native-jit-compilation)).

With the `rayon` feature enabled, `eval_batch_par` partitions rows into work-stealing chunks across the active Rayon pool for any `Value: EvaluationValue + Send + Sync` (reusing one transient `Workspace` per worker thread, aligned to whole SIMD or VM blocks when wide enough).

---

### 5. Native JIT compilation

With the `native_jit` Cargo feature (which implies `std`), `Program::with_native(&self) -> Result<Program<Value>, EvalError>` lowers a validated program for any sealed `NativeScalar` (`f64`, `f32`, `Complex<f64>`, `Complex<f32>`) into position-independent host machine code mapped into strict read-execute (`W^X`) virtual memory.

Supported targets are `x86_64`, `aarch64`, and `riscv64` on Linux, macOS, and Windows. On `x86_64`, CPU features (`SSE4.1`, `AVX`, `AVX2`, `FMA`, `AVX-512`) are detected at runtime. Calling `with_native()` on a program that already carries a native kernel clones it without recompiling. `Program::native_bytes(&self) -> Option<&[u8]>` borrows the generated scalar machine-code bytes (`None` for VM-only programs).

```rust
use symb_anafis::{Context, Program, Workspace};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let program = Program::<f64>::compile(&(x.pow(2) + 2 * x + 1), &[x])?;

    // Upgrade to native JIT, or fall back seamlessly to the portable VM:
    let program = program.with_native().unwrap_or(program);
    let mut workspace = Workspace::new();
    assert_eq!(program.eval_with(&mut workspace, &[3.0])?, 16.0);
    Ok(())
}
```

`Program::with_native` always exists in the public API regardless of target or Cargo features. When `native_jit` is disabled, the target lacks a native backend, or the OS denies executable memory allocation, `with_native()` returns `Err(EvalError::NativeCompilationUnavailable)`. Without `native_jit`, the native slot inside `Program` compiles down to a zero-sized type with zero OS FFI and zero runtime dispatch overhead.

Batch calls (`eval_batch*`) on a native-backed real (`f64`/`f32`) `Program` lazily compile and cache a multi-lane SIMD batch kernel on first batch use (`AVX-512` 8× `f64` / 16× `f32` or `AVX2` 4× `f64` / 8× `f32` on `x86_64`), keeping scalar-only `with_native()` compilation latency minimal while accelerating multi-row batches across both pure arithmetic and transcendental functions.

---

### 6. Standalone WebAssembly export

`Program::to_wasm_bytes(&self) -> Result<Vec<u8>, EvalError>` emits a standalone WebAssembly (`.wasm`) binary module for any sealed `WasmScalar` (`f64`, `f32`). It is pure safe Rust with no Cargo feature gate, OS FFI, or target restriction.

```rust
use symb_anafis::{Context, Program};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let program = Program::<f64>::compile(&(x.pow(2) + 3 * y - 1), &[x, y])?;
    let wasm = program.to_wasm_bytes()?;
    assert_eq!(&wasm[..8], b"\0asm\x01\0\0\0");
    Ok(())
}
```

The emitted module exports `"eval"` with one floating-point parameter per compiled parameter and one return value: `(param f64|f32 ...) (result f64|f32)`. Compiled constants are embedded inline (`f64.const`/`f32.const`) and temporary registers live in WebAssembly locals, so no linear memory import is required.

Pure arithmetic (`+`, `-`, `*`, `/`, negation) and `abs`, `ceil`, `floor`, `trunc`, `sqrt` lower to inline WebAssembly instructions and emit zero imports. `round` is imported (rather than lowered to `f64.nearest`/`f32.nearest`) to preserve round-half-away-from-zero semantics. Every other builtin and non-integer `pow` is imported from module `"math"` under its canonical display name (`"pow"`, `"sin"`, `"exp_m1"`, `"atan2"`, `"besselj"`, `"lambertw"`, `"assoc_legendre"`, etc.) with arguments in **symbolic expression order** (for example, `pow(base, exp)`, `log(base, x)`, `atan2(y, x)`, `besselj(n, x)`, `assoc_legendre(degree, order, x)`). Because core WebAssembly 1.0 lacks fused multiply-add instructions, fused operations lower to separate multiply and add/subtract instructions and may differ by up to 1 ULP from host FMA execution.

---

### 7. Custom bodies

Custom function bodies construct symbolic expressions. Compilation expands each distinct call once, then lowers its result. Direct evaluation expands bodies during that evaluation and memoizes shared calls. Neither path treats a partial derivative callback as a numerical body.

```rust
use std::sync::Arc;
use symb_anafis::{Context, Program, UserFunction, Workspace};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let energy = ctx.register_function(
        "energy",
        UserFunction::new(2..=2).body(Arc::new(|args| {
            let [mass, velocity] = args else { return 0.into() };
            mass * velocity.pow(2) / 2
        })),
    )?;
    let m = ctx.symb("m")?;
    let v = ctx.symb("v")?;
    let expression = energy.call(vec![m.into(), v.into()])?;
    let program = Program::<f64>::compile(&expression, &[m, v])?;
    let mut workspace = Workspace::new();
    assert_eq!(program.eval_with(&mut workspace, &[4.0, 3.0])?, 18.0);
    Ok(())
}
```

Bodies must terminate and obey their declared dependencies. Recursive body definitions return `EvalError::RecursiveFunction`; missing bodies return `EvalError::MissingBody`. An unevaluated derivative returns `EvalError::UnevaluatedDerivative`. No numerical differentiation or unknown function implementation is inferred.

---

### 8. Backends and floating arithmetic

| Scalar | Traits | Behavior |
| --- | --- | --- |
| `Number` | `EvaluationValue` | Backend integer and rational arithmetic, floating components, and supported complex promotion. |
| `Real` | `EvaluationValue` | Backend real arithmetic and real function domains. |
| `Complex` | `EvaluationValue` | Backend multiprecision complex scalar arithmetic and function domains. |
| `f64`, `f32` | `EvaluationValue`, `NativeScalar`, `WasmScalar` | Hardware IEEE-754 real arithmetic, algebraic float optimizations, SIMD/FMA VM and JIT execution, and `.wasm` export. Non-real stored literals are rejected. |
| `Complex<f64>`, `Complex<f32>` | `EvaluationValue`, `NativeScalar` | Fixed-layout complex arithmetic via `NumAnaFis` compensated kernels, supporting VM and native JIT execution. |
| Custom `EvaluationValue` | `EvaluationValue` | A commutative `Numeric` implementation with a fallible conversion from `Number`. |

`EvaluationValue::from_number` supplies literal conversion; `Numeric` supplies arithmetic and mathematical functions. Unsupported literal domains or capacities return `None`. Compatible generic code can select a backend without changing the expression:

```rust
use symb_anafis::{
    Context, EvalError, EvaluationValue, Expr, Number, Program, Real, Symbol, Workspace,
};

fn evaluate_with<T: EvaluationValue>(
    expression: &Expr,
    symbol: Symbol<'_>,
    input: T,
) -> Result<T, EvalError> {
    let program = Program::<T>::compile(expression, &[symbol])?;
    let mut workspace = Workspace::new();
    program.eval_with(&mut workspace, &[input])
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let expression = x.pow(2) + 1;
    assert_eq!(evaluate_with(&expression, x, Number::from_int(3))?, Number::from_int(10));
    assert_eq!(evaluate_with(&expression, x, Real::from_int(3))?, Real::from_int(10));
    Ok(())
}
```

Generic programs retain the stored arithmetic grouping and backend power operations. The native `f64`/`f32` compiler permits algebraic floating optimizations: positive integer multiplication chains, sparse Horner evaluation, bounded dense Estrin evaluation, shared reciprocals, and FMA when efficiently supported (including runtime AVX2/FMA/AVX-512 VM dispatch under `std`). These can change rounding, intermediate overflow or underflow, signed zeros, and nonfinite results relative to direct evaluation. Direct `f64`/`f32` evaluation follows the stored DAG without these compiler rewrites. Neither path promises a user-selected mathematical simplification sequence.

Literal conversion and precision belong to the numerical backend. Converting an existing `f64` literal to a higher-precision type cannot recover digits already rounded away.

---

### 9. Errors and resource use

Compilation validates symbol bindings, supported constants, body availability, register counts, and the completed instruction dataflow. Inputs are checked once per row or batch. Register instructions and their operand storage are private and immutable after compilation.

`EvalError` distinguishes duplicate parameters, unbound symbols, unsupported literals or constants, missing or recursive bodies, unevaluated derivatives, capacity failures (`ProgramTooLarge`), invalid compiled dataflow (`InvalidProgram`), input-shape errors (`ParameterCount`, `OutputCount`, `OutputLength`, `BatchLength`), and unavailable native JIT compilation or denied executable memory (`NativeCompilationUnavailable`). Nonfinite numerical results follow the selected backend rather than becoming symbolic errors. User callback panics and VM backend panics propagate; a batch panic can leave completed output rows written. Native JIT kernels (`with_native`) cross an `extern "C"` call boundary, so any panic inside a backend transcendental helper aborts rather than unwinding.

The direct path allocates traversal and memoization storage per call. A compiled program and its workspace retain instructions, pooled operands, parameter metadata, and a fixed-length register file. A native-backed program additionally owns shared immutable `W^X` virtual memory pages that are unmapped when the last handle referencing that native kernel is dropped (idle workspaces retain only bytecode metadata and never pin executable pages). Compilation and memory use scale with the stored DAG, custom body expansions, and generated arithmetic instructions; custom callbacks have no execution or allocation bound.
