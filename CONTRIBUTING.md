# Contributing to symb_anafis

## 🏛️ Architectural Principles

To maintain absolute modularity and maintainability, SymbAnaFis strictly adheres to a **3-tier Boundary-First Architecture** for every major subsystem directory:

1.  **`mod.rs` (Boundary Manager)**: MUST remain **Zero-Logic**. Strictly declarations (`pub mod logic;`) and declarative re-exports.
2.  **`api.rs` (Public Gatekeeper)**: User-facing surface builders (`Diff`, `Simplify`) or top-level macro loads.
3.  **`logic/` (Core Workspace)**: ALL operational algorithms, visitors, rules, and engines live inside this isolated subdirectory workspace.

---

## Adding a New Mathematical Function

Adding a new function requires updating multiple locations. Here is the complete checklist for the consolidated architecture:

### Required (Core Functionality)

| #   | Location                                 | Path                                              |
| --- | ---------------------------------------- | ------------------------------------------------- |
| 1   | `FunctionDefinition` (eval + derivative) | `src/functions/logic/definitions/<category>.rs`   |
| 2   | `Instruction` enum variant               | `src/evaluator/logic/instruction.rs`              |
| 3   | `process_instruction!` macro             | `src/evaluator/logic/execute/scalar.rs`           |
| 4   | `single_fast_path!` macro                | `src/evaluator/logic/execute/scalar.rs`           |
| 5   | `batch_fast_path!` macro                 | `src/evaluator/logic/execute/simd.rs`             |
| 6   | `simd_batch_fast_path!` macro            | `src/evaluator/logic/execute/simd.rs`             |
| 7   | `exec_simd_slow_instruction()`           | `src/evaluator/logic/execute/simd.rs`             |
| 8   | Compiler name→instruction match          | `src/evaluator/logic/compile/compiler.rs`         |

### Optional (API Ergonomics)

| #   | Location                           | Path                                               |
| --- | ---------------------------------- | -------------------------------------------------- |
| 9   | `PyExpr` method (Python binding)   | `src/bindings/python/expr.rs` (or adapters)       |
| 10  | `ArcExprExt` trait method          | `src/core/logic/symbol/math_methods.rs`           |
| 11  | `Dual<T>` method (auto-diff)       | `src/core/logic/math_methods.rs`                   |
| 12  | Known symbol constant              | `src/core/logic/known_symbols.rs`                  |

> **Note**: Display/LaTeX works automatically! Unknown functions display as `myfunc(x)` in terminal and `\text{myfunc}(x)` in LaTeX. Only edit `display.rs` if you need special notation (e.g., `\sqrt{x}`).

### Example: Adding `myfunc(x)`

1. **definitions/<category>.rs**: Add `FunctionDefinition` with `eval` and `derivative`
2. **instruction.rs**: Add `Instruction::MyFunc` enum variant
3. **execute/scalar.rs**: Add dispatch in `process_instruction!` and `single_fast_path!` macros
4. **execute/simd.rs**: Add dispatch in `batch_fast_path!` and `simd_batch_fast_path!` macros + `exec_simd_slow_instruction()`
5. **compile/compiler.rs**: Add `("myfunc", 1) => Instruction::MyFunc` in compiler match
6. **known_symbols.rs**: Add `pub const MYFUNC: u64 = intern_id("myfunc");`

---

## Adding a Simplification Rule

Rules require updates in rule registries located inside isolated logics:

| #   | Location                                     | What to do                                  |
| --- | -------------------------------------------- | ------------------------------------------- |
| 1   | `src/simplification/logic/rules/<category>/*.rs` | Define rule with `rule_arc!` macro          |
| 2   | `src/simplification/logic/rules/<category>/mod.rs` | Export module + register in `get_*_rules()` |
| 3   | `src/simplification/logic/tests.rs` (or local) | Add tests                                   |

### Categories

| Category         | Rules for                      |
| ---------------- | ------------------------------ |
| `algebraic/`     | General algebra (x-x=0, x*1=x) |
| `exponential/`   | exp/log rules                  |
| `hyperbolic/`    | sinh, cosh, tanh               |
| `numeric/`       | Constant folding               |
| `root/`          | sqrt, cbrt                     |
| `trigonometric/` | sin, cos, tan                  |

### Priority Ranges

- **85-95**: Special values (sin(0)=0)
- **70-84**: Cancellation (x/x=1)  
- **40-69**: Consolidation (combine terms)
- **1-39**: Canonicalization

### Example Rule

```rust
rule_arc!(
    SinZeroRule,
    "sin_zero",
    95,
    Trigonometric,
    &[ExprKind::Function],
    targets: &[KS.sin],
    |expr: &Expr, _ctx: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sin
            && args.len() == 1
            && matches!(&args[0].kind, AstKind::Number(n) if *n == 0.0)
        {
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);
```
