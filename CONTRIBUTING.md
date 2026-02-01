# Contributing to symb_anafis

## Adding a New Mathematical Function

Adding a new function requires updating multiple locations. Here's the complete checklist for the current modular architecture (v0.7.0+):

### Required (Core Functionality)

| #   | Location                                 | File                           | Module |
| --- | ---------------------------------------- | ------------------------------ | ------ |
| 1   | `FunctionDefinition` (eval + derivative) | `src/functions/definitions.rs` | - |
| 2   | `Instruction` enum variant               | `src/evaluator/instruction.rs` | - |
| 3   | `process_instruction!` macro             | `src/evaluator/execution.rs`   | - |
| 4   | `single_fast_path!` macro                | `src/evaluator/execution.rs`   | - |
| 5   | `batch_fast_path!` macro                 | `src/evaluator/simd.rs`        | - |
| 6   | `simd_batch_fast_path!` macro            | `src/evaluator/simd.rs`        | - |
| 7   | `exec_simd_slow_instruction()`           | `src/evaluator/simd.rs`        | - |
| 8   | Compiler name→instruction match          | `src/evaluator/compiler.rs`    | - |

### Optional (API Ergonomics)

| #   | Location                           | File                              | Module |
| --- | ---------------------------------- | --------------------------------- | ------ |
| 9   | `PyExpr` method (Python binding)   | `src/bindings/python/expr.rs`     | - |
| 10  | `PyDual` method (Python binding)   | `src/bindings/python/dual.rs`     | - |
| 11  | `PySymbol` method (Python binding) | `src/bindings/python/symbol.rs`   | - |
| 12  | `ArcExprExt` trait method          | `src/core/symbol/math_methods.rs` | - |
| 13  | `Dual<T>` method (auto-diff)       | `src/math/dual.rs`               | - |
| 14  | Known symbol constant              | `src/core/known_symbols.rs`       | - |
| 15  | LaTeX formatting (if special)      | `src/core/display.rs`             | - |

> **Note**: Display/LaTeX works automatically! Unknown functions display as `myfunc(x)` in terminal and `\text{myfunc}(x)` in LaTeX. Only edit `display.rs` if you need special notation (e.g., `\sqrt{x}` or `J_n(x)`).

### Testing

Add tests in `src/tests/eval_func_tests.rs` covering:
- Basic numeric evaluation
- Edge cases and domain errors
- Compiled evaluation
- SIMD batch evaluation (`cargo test --features parallel`)

### Example: Adding `myfunc(x)`

1. **definitions.rs**: Add `FunctionDefinition` with `eval` and `derivative`
2. **instruction.rs**: Add `Instruction::MyFunc` enum variant
3. **execution.rs**: Add dispatch in `process_instruction!` and `single_fast_path!` macros
4. **simd.rs**: Add dispatch in `batch_fast_path!` and `simd_batch_fast_path!` macros + `exec_simd_slow_instruction()`
5. **compiler.rs**: Add `("myfunc", 1) => Instruction::MyFunc` in compiler match
6. **expr.rs**: Add `fn myfunc(&self) -> Self` to `PyExpr`
7. **dual.rs**: Add `fn myfunc(&self) -> Self` to `PyDual`
8. **symbol.rs**: Add `fn myfunc(&self) -> Self` to `PySymbol`
9. **math_methods.rs**: Add to `ArcExprExt` trait
10. **dual.rs**: Add `fn myfunc(self) -> Self` with derivative to `Dual<T>`
11. **known_symbols.rs**: Add `pub const MYFUNC: u64 = intern_id("myfunc");` and update `KnownSymbols` struct

### Running Tests

```bash
cargo test --features parallel
cargo test eval_func_tests
```

---

## Adding a Simplification Rule

Simplification rules require only **2-3 locations** (simpler than functions!):

| #   | Location                                     | What to do                                  |
| --- | -------------------------------------------- | ------------------------------------------- |
| 1   | `src/simplification/rules/<category>/*.rs`   | Define rule with `rule_arc!` macro          |
| 2   | `src/simplification/rules/<category>/mod.rs` | Export module + register in `get_*_rules()` |
| 3   | `src/tests/*_tests.rs`                       | Add tests                                   |

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
