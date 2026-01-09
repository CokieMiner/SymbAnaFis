# Contributing to symb_anafis

## Adding a New Mathematical Function

Adding a new function requires updating multiple locations. Here's the complete checklist:

### Required (Core Functionality)

| #   | Location                                 | File                           | ~Line |
| --- | ---------------------------------------- | ------------------------------ | ----- |
| 1   | `FunctionDefinition` (eval + derivative) | `src/functions/definitions.rs` | 26    |
| 2   | `Instruction` enum variant               | `src/core/evaluator.rs`        | 88    |
| 3   | `process_instruction!` macro             | `src/core/evaluator.rs`        | 234   |
| 4   | `single_fast_path!` macro                | `src/core/evaluator.rs`        | 607   |
| 5   | `batch_fast_path!` macro                 | `src/core/evaluator.rs`        | 686   |
| 6   | `simd_batch_fast_path!` macro            | `src/core/evaluator.rs`        | 766   |
| 7   | `exec_simd_slow_instruction()`           | `src/core/evaluator.rs`        | 1312  |
| 8   | Compiler name→instruction match          | `src/core/evaluator.rs`        | 2286  |

### Optional (API Ergonomics)

| #   | Location                           | File                        | ~Line |
| --- | ---------------------------------- | --------------------------- | ----- |
| 9   | `PyExpr` method (Python binding)   | `src/bindings/python.rs`    | 432   |
| 10  | `PyDual` method (Python binding)   | `src/bindings/python.rs`    | 1159  |
| 11  | `PySymbol` method (Python binding) | `src/bindings/python.rs`    | 2290  |
| 12  | `ArcExprExt` trait method          | `src/core/symbol.rs`        | 1070  |
| 13  | `Dual<T>` method (auto-diff)       | `src/math/dual.rs`          | 570   |
| 14  | Known symbol constant              | `src/core/known_symbols.rs` | -     |
| 15  | LaTeX formatting (if special)      | `src/core/display.rs`       | 388   |

> **Note**: Display/LaTeX works automatically! Unknown functions display as `myfunc(x)` in terminal and `\text{myfunc}(x)` in LaTeX. Only edit `display.rs` if you need special notation (e.g., `\sqrt{x}` or `J_n(x)`).

### Testing

Add tests in `src/tests/eval_func_tests.rs` covering:
- Basic numeric evaluation
- Edge cases and domain errors
- Compiled evaluation
- SIMD batch evaluation (`cargo test --features parallel`)

### Example: Adding `myfunc(x)`

1. **definitions.rs**: Add `FunctionDefinition` with `eval` and `derivative`
2. **evaluator.rs**: Add `Instruction::MyFunc` enum variant
3. **evaluator.rs**: Add dispatch in all 4 macros + slow path
4. **evaluator.rs**: Add `("myfunc", 1) => Instruction::MyFunc` in compiler
5. **python.rs**: Add `fn myfunc(&self) -> Self` to PyExpr, PyDual, PySymbol
6. **symbol.rs**: Add to `ArcExprExt` trait
7. **dual.rs**: Add `fn myfunc(self) -> Self` with derivative

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
    targets: &["sin"],
    |expr: &Expr, _ctx: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.as_str() == "sin"
            && matches!(&args[0].kind, AstKind::Number(n) if *n == 0.0)
        {
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);
```
