---
description: How to add a new mathematical function to symb_anafis
---

# Adding a New Mathematical Function

This checklist covers all locations that need to be updated when adding a new function (e.g., `myfunc`).

## Required Locations (Core Functionality)

### 1. Function Definition (Symbolic Differentiation)

**File:** `src/functions/definitions.rs` (~line 26)

Add a `FunctionDefinition` in `all_definitions()`:

```rust
FunctionDefinition {
    name: "myfunc",
    arity: 1..=1,  // or 2..=2 for two-arg functions
    eval: |args| Some(/* numeric evaluation */),
    derivative: |args, arg_primes| {
        // Symbolic derivative using chain rule
    },
},
```

### 2. Bytecode Instruction Enum

**File:** `src/core/evaluator.rs` (~line 88-182)

Add variant to `enum Instruction`:
```rust
MyFunc,  // Add in appropriate category
```

### 3. Instruction Dispatch (6 locations in evaluator.rs)

#### 3a. `process_instruction!` macro (~line 234)
#### 3b. `single_fast_path!` macro (~line 607)
#### 3c. `batch_fast_path!` macro (~line 686)
#### 3d. `simd_batch_fast_path!` macro (~line 766)
#### 3e. `exec_simd_slow_instruction()` function (~line 1312)

Add dispatch for your new instruction. For simple functions, add to fast path:
```rust
Instruction::MyFunc => {
    let top = stack_top_mut!($stack);
    *top = /* compute result */;
}
```

For SIMD, add per-lane execution:
```rust
Instruction::MyFunc => {
    let top = $stack.last_mut().expect("Stack must not be empty");
    let arr = top.to_array();
    *top = f64x4::new([myfunc(arr[0]), myfunc(arr[1]), myfunc(arr[2]), myfunc(arr[3])]);
}
```

### 4. Compiler Name-to-Instruction Mapping

**File:** `src/core/evaluator.rs` (~line 2286-2400)

Add to `match (func_name, args.len())`:
```rust
("myfunc", 1) => Instruction::MyFunc,
// For 2-arg:
("myfunc", 2) => { self.pop(); Instruction::MyFunc }
```

---

## Optional Locations (API Ergonomics)

### 5. Python Bindings (3 types)

**File:** `src/bindings/python.rs`

- **PyExpr** methods (~line 432)
- **PyDual** methods (~line 1159)  
- **PySymbol** methods (~line 2290)

```rust
fn myfunc(&self) -> Self { Self(self.0.clone().myfunc()) }
```

### 6. Rust API: ArcExprExt Trait

**File:** `src/core/symbol.rs` (~line 1070-1183)

Add method for ergonomic `Arc<Expr>` usage:
```rust
fn myfunc(&self) -> Expr {
    Expr::func("myfunc", Expr::unwrap_arc(Arc::clone(self)))
}
```

### 7. Dual Number Methods (Automatic Differentiation)

**File:** `src/math/dual.rs` (~line 570+)

If supporting Dual numbers for AD:
```rust
pub fn myfunc(self) -> Self {
    Self::new(myfunc(self.val), /* derivative */ * self.eps)
}
```

### 8. Known Symbols (If Referenced in Derivatives)

**File:** `src/core/known_symbols.rs`

```rust
pub(crate) static MYFUNC: LazyLock<InternedSymbol> = LazyLock::new(|| symb_interned("myfunc"));
```

---

## Testing

Add tests in `src/tests/eval_func_tests.rs`:
- Basic evaluation
- Edge cases (domain errors, special values)
- Compiled evaluation
- SIMD batch evaluation (if parallel feature)

---

## Quick Reference Table

| What                         | File                       | Line  |
| ---------------------------- | -------------------------- | ----- |
| `FunctionDefinition`         | `functions/definitions.rs` | ~26   |
| `Instruction` enum           | `core/evaluator.rs`        | ~88   |
| `process_instruction!`       | `core/evaluator.rs`        | ~234  |
| `single_fast_path!`          | `core/evaluator.rs`        | ~607  |
| `batch_fast_path!`           | `core/evaluator.rs`        | ~686  |
| `simd_batch_fast_path!`      | `core/evaluator.rs`        | ~766  |
| `exec_simd_slow_instruction` | `core/evaluator.rs`        | ~1312 |
| Compiler match               | `core/evaluator.rs`        | ~2286 |
| `PyExpr` methods             | `bindings/python.rs`       | ~432  |
| `PyDual` methods             | `bindings/python.rs`       | ~1159 |
| `PySymbol` methods           | `bindings/python.rs`       | ~2290 |
| `ArcExprExt` trait           | `core/symbol.rs`           | ~1070 |
| `Dual<T>` methods            | `math/dual.rs`             | ~570  |
| Known symbols                | `core/known_symbols.rs`    | -     |
| LaTeX (optional)             | `core/display.rs`          | ~388  |

## Note: Display/LaTeX Works Automatically

You do NOT need to update `display.rs` for most functions!

**Automatic fallback:**
- **Terminal**: `myfunc(x)` 
- **LaTeX**: `\text{myfunc}\left(x\right)`

**Only edit `display.rs` if you need special notation like:**
- `sqrt` → `\sqrt{x}`
- `besselj` → `J_n(x)` (subscript notation)
- `gamma` → `\Gamma` (capital Greek)
