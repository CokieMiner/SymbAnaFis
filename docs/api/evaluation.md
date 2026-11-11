# SymbAnaFis - Numerical Evaluation API

Reference for direct scalar evaluation, compiled register VM programs, disassembly, batch evaluation, custom bodies, and numerical backends in `symb_anafis`.

[← Back to API Index](../API.md)

---

### Contents

1. [Direct evaluation](#1-direct-evaluation)
2. [Compiled scalar programs](#2-compiled-scalar-programs)
3. [Inspecting compiled instructions](#3-inspecting-compiled-instructions)
4. [Scalar batches](#4-scalar-batches)
5. [Custom bodies](#5-custom-bodies)
6. [Backends and floating arithmetic](#6-backends-and-floating-arithmetic)
7. [Errors and resource use](#7-errors-and-resource-use)

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

The selected scalar determines function domains. `Number` can promote supported real inputs to complex results, while `Real` and `f64` retain their real-domain behavior:

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

A `Program` contains no mutable execution state. It can be cloned cheaply, wrapped in `Arc`, and shared between threads. Each concurrent evaluation owns a `Workspace`; `Program::workspace()` creates one, and `Workspace::new()` creates an unbound workspace that binds on first evaluation. A workspace can be reused across different programs; evaluation reinitializes its constants and reuses the allocation when possible.

```rust
use symb_anafis::{Context, Program, Workspace};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let expression = ctx.parse("x^3 + 2*x + 1")?;
    let program = Program::<f64>::compile(&expression, &[x])?;

    let mut workspace = Workspace::new();
    assert_eq!(program.evaluate(&mut workspace, &[2.0])?, 13.0);
    assert_eq!(program.evaluate(&mut workspace, &[3.0])?, 34.0);
    Ok(())
}
```

`Program` is the only reusable execution object; `Workspace` owns all mutable execution state. Sequential callers reuse one workspace across evaluations, while concurrent callers create a distinct workspace for each thread. Cloning a program shares immutable instructions; cloning a workspace copies its registers and bound-program handle.

A program retains parameter metadata and remains usable after its context and input expression are dropped. `parameters()` returns borrowed symbol handles in input order; `instruction_count()` and `workspace_size()` report program instructions and initialized scalar slots.

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

`evaluate_batch` accepts one column per parameter and fills caller-owned output. Every column must have exactly the output length. Shape validation completes before any output is written. Execution is scalar.

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
    program.evaluate_batch(&mut workspace, &[&xs, &ys], &mut output)?;
    assert_eq!(output, [11.0, 24.0, 39.0]);
    Ok(())
}
```

Empty batches are accepted. A zero-parameter program accepts no columns and repeats its result for every output element.

---

### 5. Custom bodies

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
    assert_eq!(program.evaluate(&mut workspace, &[4.0, 3.0])?, 18.0);
    Ok(())
}
```

Bodies must terminate and obey their declared dependencies. Recursive body definitions return `EvalError::RecursiveFunction`; missing bodies return `EvalError::MissingBody`. An unevaluated derivative returns `EvalError::UnevaluatedDerivative`. No numerical differentiation or unknown function implementation is inferred.

---

### 6. Backends and floating arithmetic

| Scalar | Behavior |
| --- | --- |
| `Number` | Backend integer and rational arithmetic, floating components, and supported complex promotion. |
| `Real` | Backend real arithmetic and real function domains. |
| `Complex` | Backend complex scalar arithmetic and function domains. |
| `f64` | Native scalar arithmetic and NumAnaFis real function kernels. Non-real stored literals are rejected. |
| Custom `EvaluationValue` | A commutative `Numeric` implementation with a fallible conversion from `Number`. |

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
    program.evaluate(&mut workspace, &[input])
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

Generic programs retain the stored arithmetic grouping and backend power operations. The native `f64` compiler permits algebraic floating optimizations: positive integer multiplication chains, sparse Horner evaluation, bounded dense Estrin evaluation, shared reciprocals, and FMA when efficiently supported. These can change rounding, intermediate overflow or underflow, signed zeros, and nonfinite results relative to direct evaluation. Direct `f64` evaluation follows the stored DAG without these compiler rewrites. Neither path promises a user-selected mathematical simplification sequence.

Literal conversion and precision belong to the numerical backend. Converting an existing `f64` literal to a higher-precision type cannot recover digits already rounded away.

---

### 7. Errors and resource use

Compilation validates symbol bindings, supported constants, body availability, register counts, and the completed instruction dataflow. Inputs are checked once per row or batch. Register instructions and their operand storage are private and immutable after compilation.

`EvalError` distinguishes duplicate parameters, unbound symbols, unsupported literals or constants, missing or recursive bodies, unevaluated derivatives, capacity failures, invalid compiled dataflow, and input-shape errors. Nonfinite numerical results follow the selected backend rather than becoming symbolic errors. User callback panics and backend panics propagate; a batch panic can leave completed output rows written.

The direct path allocates traversal and memoization storage per call. A compiled program and its workspace retain instructions, pooled operands, parameter metadata, and a fixed-length register file. Compilation and memory use scale with the stored DAG, custom body expansions, and generated arithmetic instructions; custom callbacks have no execution or allocation bound.
