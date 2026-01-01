# SymbAnaFis Architecture

This document describes the internal architecture of SymbAnaFis, a high-performance symbolic mathematics library.

## Overview

```mermaid
flowchart TB
    subgraph "User API"
        A1["diff(formula, var)"]
        A2["simplify(formula)"]
        A3["parse(formula)"]
        A4["evaluate_str(formula, vars)"]
        A5["gradient_str / hessian_str / jacobian_str"]
    end

    subgraph "Builder Layer"
        B1["Diff::new()
        .domain_safe()
        .max_depth()
        .fixed_var()
        .user_fn()"]
        B2["Simplify::new()
        .domain_safe()
        .max_depth()"]
    end

    subgraph "Parser"
        P1["Lexer
        (lexer.rs)"]
        P2["Tokenizer
        (tokens.rs)"]
        P3["Pratt Parser
        (pratt.rs)"]
        P4["Implicit Mul
        (implicit_mul.rs)"]
    end

    subgraph "Core AST"
        C1["Expr"]
        C2["ExprKind
        Number | Symbol | Sum | Product
        Div | Pow | FunctionCall | Poly
        Derivative"]
        C3["InternedSymbol
        (Copy, O(1) compare)
        Used for symbols AND function names"]
    end

    subgraph "Differentiation Engine"
        D1["Sum Rule: (a+b)' = a'+b'"]
        D2["Product Rule: (ab)' = a'b + ab'"]
        D3["Chain Rule: f(g)' = f'(g)·g'"]
        D4["Power Rule: (a^b)' = a^b(b'ln(a) + ba'/a)"]
        D5["Custom Functions"]
    end

    subgraph "Simplification Engine"
        S1["Bottom-up traversal"]
        S2["Multi-pass until stable"]
        S3["Priority ordering"]
        S4["Hash-based change detection"]
        subgraph "Rules (120+)"
            R1["Numeric: 0+x→x, constant fold"]
            R2["Algebraic: factor, distribute"]
            R3["Trigonometric: sin²+cos²=1"]
            R4["Exponential: exp(ln(x))→x"]
        end
    end

    subgraph "Output"
        O1["String: 2*x + cos(x)"]
        O2["LaTeX: 2x + \\cos(x)"]
        O3["Unicode: 2x + cos(x)"]
    end

    A1 --> B1
    A2 --> B2
    A3 --> P1
    A4 --> C1
    A5 --> D1

    B1 --> P1
    B2 --> P1

    P1 --> P2 --> P3 --> P4 --> C1
    C1 --> C2
    C2 --> C3

    B1 --> D1
    D1 --> D2 --> D3 --> D4 --> D5
    D5 --> S1

    B2 --> S1
    S1 --> S2 --> S3 --> R1
    R1 --> R2 --> R3 --> R4

    R4 --> O1
    O1 --> O2
    O1 --> O3
```

---

## Differentiation Engine Architecture

The differentiation engine (`diff/engine.rs`) implements symbolic differentiation via recursive rule application.

```mermaid
flowchart TB
    subgraph "Entry Point"
        E1["Expr.derive(var, context: Option<&Context>)"]
        E2["Expr.derive_raw(var) - No simplification"]
    end

    subgraph "Pattern Matching on ExprKind"
        M1["Number(n)"]
        M2["Symbol(s)"]
        M3["FunctionCall{name, args}"]
        M4["Sum(terms)"]
        M5["Product(factors)"]
        M6["Div(u, v)"]
        M7["Pow(u, v)"]
        M8["Derivative{inner, var, order}"]
        M9["Poly(polynomial)"]
    end

    subgraph "Differentiation Rules"
        R1["d/dx[c] = 0"]
        R2["d/dx[x] = 1, d/dx[y] = 0"]
        R3["Chain Rule via Registry
        or Custom Derivatives"]
        R4["Sum Rule: Σ d/dx[term_i]"]
        R5["N-ary Product Rule:
        Σ (term_i' × Π_{j≠i} term_j)"]
        R6["Quotient Rule:
        (u'v - uv') / v²"]
        R7["Logarithmic Differentiation:
        u^v × (v'×ln(u) + v×u'/u)"]
        R8["Order Increment:
        ∂^n/∂x^n → ∂^(n+1)/∂x^(n+1)"]
        R9["Polynomial Derivative:
        Fast O(n) term-by-term"]
    end

    subgraph "Function Registry Lookup"
        F1["Lookup in global Registry
        (50+ built-in functions)"]
        F2["Check Context user_fn partials"]
        F3["Fallback: Symbolic ∂f/∂x"]
    end

    subgraph "Inline Optimizations"
        O1["0 + x → x"]
        O2["1 × x → x"]
        O3["0 × x → 0"]
        O4["Skip zero derivatives"]
    end

    E1 --> M1 & M2 & M3 & M4 & M5 & M6 & M7 & M8 & M9

    M1 --> R1
    M2 --> R2
    M3 --> F1
    F1 -->|"Built-in"| R3
    F1 -->|"Not Found"| F2
    F2 -->|"Has Partials"| R3
    F2 -->|"No Partials"| F3

    M4 --> R4
    M5 --> R5
    M6 --> R6
    M7 --> R7
    M8 --> R8
    M9 --> R9

    R1 & R2 & R3 & R4 & R5 & R6 & R7 & R8 & R9 --> O1 --> O2 --> O3 --> O4
    O4 --> Result["Expr (unsimplified)"]
```

### Key Differentation Features

| Feature                         | Implementation                                                             |
| ------------------------------- | -------------------------------------------------------------------------- |
| **Logarithmic Differentiation** | Handles `x^x`, `f^g` correctly                                             |
| **Custom Functions**            | `Diff::user_fn("f", UserFunction)` with partial derivatives and evaluation |
| **Multi-arg Partials**          | `CustomFn::partial(0, \|args                                               | ...)` for chain rule |
| **Fixed Variables**             | Variables in `fixed_vars` treated as constants (derivative = 0)            |
| **Inline Optimization**         | Prevents expression explosion during recursion                             |

--- 

## Simplification Engine Architecture

The simplification engine (`simplification/engine.rs`) uses a multi-pass, rule-based approach.

```mermaid
flowchart TB
    subgraph "Entry Point"
        E1["Simplifier::new()
        .with_domain_safe(bool)
        .with_max_iterations(n)
        .with_max_depth(n)"]
        E2["simplify(expr)"]
    end

    subgraph "Main Loop"
        L1["Wrap expr in Arc"]
        L2["apply_rules_bottom_up(expr, depth)"]
        L3{"Changed?"}
        L4{"Iterations < max?"}
        L5["Return result"]
    end

    subgraph "Bottom-Up Traversal"
        B1["Recurse into children first"]
        B2["Rebuild node if children changed"]
        B3["apply_rules_to_node(node)"]
        B4["Return simplified node"]
    end

    subgraph "Rule Application"
        RA1["Get rules for ExprKind"]
        RA2["Sort by priority (descending)"]
        RA3["For each rule:"]
        RA4{"Matches pattern?"}
        RA5{"domain_safe &&
        alters_domain?"}
        RA6["Apply transformation"]
        RA7["Mark changed, restart"]
    end

    subgraph "Rule Categories (Priority)"
        P1["85-95: Expansion
        (distribute, flatten)"]
        P2["70-84: Cancellation
        (x/x→1, x^0→1)"]
        P3["40-69: Consolidation
        (factor, combine)"]
        P4["1-39: Canonicalization
        (sort terms)"]
    end

    subgraph "Rule Registry"
        RR1["Numeric Rules (~20)"]
        RR2["Algebraic Rules (~40)"]
        RR3["Trigonometric Rules (~30)"]
        RR4["Hyperbolic Rules (~15)"]
        RR5["Exponential Rules (~10)"]
        RR6["Root Rules (~5)"]
    end

    E1 --> E2 --> L1 --> L2

    L2 --> B1 --> B2 --> B3 --> B4
    B4 --> L3
    L3 -->|"Yes"| L4
    L3 -->|"No"| L5
    L4 -->|"Yes"| L2
    L4 -->|"No"| L5

    B3 --> RA1 --> RA2 --> RA3 --> RA4
    RA4 -->|"No"| RA3
    RA4 -->|"Yes"| RA5
    RA5 -->|"Skip"| RA3
    RA5 -->|"Apply"| RA6 --> RA7

    RA1 -.-> RR1 & RR2 & RR3 & RR4 & RR5 & RR6
    RR1 & RR2 & RR3 & RR4 & RR5 & RR6 -.-> P1 & P2 & P3 & P4
```

### Rule Execution Flow

```mermaid
flowchart LR
    subgraph "Phase 1: Expand"
        E1["a×(b+c) → ab + ac"]
        E2["(a+b)² → a² + 2ab + b²"]
        E3["Flatten nested Sum/Product"]
    end

    subgraph "Phase 2: Cancel"
        C1["x/x → 1"]
        C2["x - x → 0"]
        C3["x^0 → 1"]
        C4["x^a/x^b → x^(a-b)"]
    end

    subgraph "Phase 3: Consolidate"
        N1["2x + 3x → 5x"]
        N2["x×x → x²"]
        N3["sin²+cos² → 1"]
        N4["exp(ln(x)) → x"]
    end

    subgraph "Phase 4: Canonicalize"
        S1["Sort Sum terms"]
        S2["Sort Product factors"]
        S3["Normalize coefficients"]
    end

    E1 & E2 & E3 --> C1 & C2 & C3 & C4 --> N1 & N2 & N3 & N4 --> S1 & S2 & S3
```

---

## Evaluation Engine Architecture

The evaluation engine provides both tree-walking and compiled bytecode evaluation.

```mermaid
flowchart TB
    subgraph "Entry Points"
        E1["expr.evaluate(&vars)
        Tree-walking interpreter"]
        E2["expr.compile(&var_names)
        → CompiledEvaluator"]
        E3["evaluator.evaluate(&values)
        Bytecode VM"]
    end

    subgraph "Tree-Walking Evaluator"
        TW1["Match on ExprKind"]
        TW2["Number → return value"]
        TW3["Symbol → lookup in vars HashMap"]
        TW4["Sum → evaluate all, sum"]
        TW5["Product → evaluate all, multiply"]
        TW6["FunctionCall → eval args, call fn"]
        TW7["Recurse for Div, Pow, etc."]
    end

    subgraph "Bytecode Compiler"
        C1["Traverse expression tree"]
        C2["Generate Instruction sequence"]
        C3["Map variable names → indices"]
        C4["Calculate max stack depth"]
        C5["Return CompiledEvaluator"]
    end

    subgraph "Bytecode Instructions"
        I1["LoadConst(f64)"]
        I2["LoadParam(index)"]
        I3["Add, Mul, Sub, Div, Pow"]
        I4["Sin, Cos, Tan, Exp, Ln..."]
        I5["Square, Cube, Recip (optimized)"]
        I6["CallExternal{id, arity}"]
    end

    subgraph "Stack-Based VM"
        VM1["Pre-allocate stack to max_depth"]
        VM2["For each instruction:"]
        VM3["LoadConst → push constant"]
        VM4["LoadParam → push param[i]"]
        VM5["BinOp → pop 2, push result"]
        VM6["UnaryOp → modify top"]
        VM7["Return stack[0]"]
    end

    E1 --> TW1
    TW1 --> TW2 & TW3 & TW4 & TW5 & TW6 & TW7

    E2 --> C1 --> C2 --> C3 --> C4 --> C5

    C2 -.-> I1 & I2 & I3 & I4 & I5 & I6

    E3 --> VM1 --> VM2
    VM2 --> VM3 & VM4 & VM5 & VM6 --> VM7
```

### Compiled vs Tree-Walking Performance

| Operation | Tree-Walking       | Compiled                           |
| --------- | ------------------ | ---------------------------------- |
| Setup     | None               | ~1-5 μs compile                    |
| Per-eval  | ~100-500 ns        | ~10-50 ns                          |
| Best for  | Single evaluation  | Batch evaluation (1000s of points) |
| Memory    | O(depth) recursion | Pre-allocated stack                |

---

## Parallel Evaluation Architecture

The parallel module (`bindings/parallel.rs`) provides Rayon-powered batch evaluation.

```mermaid
flowchart TB
    subgraph "Input Types"
        I1["ExprInput::Parsed(Expr)"]
        I2["ExprInput::String(formula)"]
        I3["VarInput (name, symbol_id)"]
        I4["Value::Num(f64)"]
        I5["Value::Expr(symbolic)"]
        I6["Value::Skip"]
    end

    subgraph "evaluate_parallel()"
        EP1["Parse string inputs (if needed)"]
        EP2["Compile each expression"]
        EP3["Rayon parallel iterator"]
        EP4["Evaluate at each point"]
        EP5["Collect results"]
    end

    subgraph "Parallelization Strategy"
        PS1["Each expression independent"]
        PS2["Each evaluation point independent"]
        PS3["Rayon work-stealing scheduler"]
        PS4["Batch size tuning"]
    end

    subgraph "Output Types"
        O1["EvalResult::Expr"]
        O2["EvalResult::String"]
        O3["Vec<Vec<f64>> for numeric"]
    end

    I1 & I2 --> EP1
    I3 & I4 & I5 & I6 --> EP3

    EP1 --> EP2 --> EP3 --> EP4 --> EP5

    EP3 -.-> PS1 & PS2 & PS3 & PS4

    EP5 --> O1 & O2 & O3
```

### Parallel Evaluation Flow

```mermaid
sequenceDiagram
    participant User
    participant evaluate_parallel
    participant Parser
    participant Compiler
    participant Rayon
    participant Worker1
    participant Worker2
    participant WorkerN

    User->>evaluate_parallel: exprs, vars, values[][]
    
    loop For each expr
        evaluate_parallel->>Parser: Parse (if string)
        Parser-->>evaluate_parallel: Expr
        evaluate_parallel->>Compiler: compile(Expr, vars)
        Compiler-->>evaluate_parallel: CompiledEvaluator
    end

    evaluate_parallel->>Rayon: par_iter(values)
    
    par
        Rayon->>Worker1: batch[0..n]
        Rayon->>Worker2: batch[n..m]
        Rayon->>WorkerN: batch[m..]
    end

    Worker1-->>Rayon: results[0..n]
    Worker2-->>Rayon: results[n..m]
    WorkerN-->>Rayon: results[m..]

    Rayon-->>evaluate_parallel: Vec<f64>
    evaluate_parallel-->>User: EvalResult
```

### Usage Example

```rust
use symb_anafis::bindings::parallel::*;

// Evaluate derivative at 1000 points
let expr = symb_anafis::diff("sin(x)^2", "x", None, None)?;
let vars = vec!["x".into()];
let values: Vec<Vec<f64>> = (0..1000)
    .map(|i| vec![i as f64 * 0.001])
    .collect();

let results = evaluate_parallel(
    vec![expr.into()],
    vec![vars],
    vec![values],
)?;
// results[0] contains 1000 f64 values
```

---

### Data Flow: `diff("x^2 + sin(x)", "x")`

```mermaid
flowchart LR
    subgraph Parse
        I[x^2 + sin x] --> L[Lexer]
        L --> T[Tokens]
        P[Pratt Parser]
        P --> AST[Sum - Pow - sin]
    end

    subgraph Differentiate
        AST --> SUM[Sum Rule]
        SUM --> POW[d/dx x² = 2x]
        SUM --> CHAIN[d/dx sin = cos]
        POW --> RESULT1[2x + cos x]
        CHAIN --> RESULT1
    end

    subgraph Simplify
        RESULT1 --> PASS1[Numeric rules]
        PASS1 --> PASS2[Algebraic rules]
        PASS2 --> PASS3[Canonicalize]
        PASS3 --> FINAL[2*x + cos x]
    end

    subgraph Output
        FINAL --> STR[String result]
    end
```

## Module Structure

```
src/
├── lib.rs                    # Public API: diff(), simplify(), re-exports (6KB)
├── core/                     # Core types and utilities
│   ├── mod.rs               # Re-exports
│   ├── expr.rs              # Expr, ExprKind - core expression types (61KB)
│   ├── symbol.rs            # InternedSymbol, Symbol - Copy symbols (58KB)
│   ├── known_symbols.rs     # Pre-interned symbols for O(1) lookup
│   ├── evaluator.rs         # CompiledEvaluator bytecode VM (103KB)
│   ├── unified_context.rs   # Context, UserFunction - isolated registries (19KB)
│   ├── display.rs           # to_string(), to_latex(), to_unicode() (34KB)
│   ├── error.rs             # DiffError, Span - error types (14KB)
│   ├── traits.rs            # MathScalar trait
│   ├── poly.rs              # Univariate polynomial type (24KB)
│   └── visitor.rs           # AST visitor pattern
│
├── parser/                   # String → Expr
│   ├── mod.rs               # parse() function
│   ├── lexer.rs             # Tokenizer (30KB)
│   ├── tokens.rs            # Token definitions (13KB)
│   ├── pratt.rs             # Pratt parsing algorithm (17KB)
│   └── implicit_mul.rs      # 2x → 2*x handling
│
├── api/                      # Builder layer
│   ├── mod.rs               # Re-exports
│   ├── builder.rs           # Diff, Simplify builders (fluent API) (14KB)
│   └── helpers.rs           # gradient, hessian, jacobian (8KB)
│
├── diff/                     # Differentiation engine
│   ├── mod.rs               # Module exports
│   └── engine.rs            # Core differentiation logic (23KB)
│
├── simplification/           # Rule-based simplification
│   ├── mod.rs               # simplify_expr() entry point
│   ├── engine.rs            # Multi-pass engine (13KB)
│   ├── helpers.rs           # Expression utilities (27KB)
│   ├── README.md            # Rule documentation
│   ├── patterns/            # Pattern matching
│   └── rules/               # 120+ rules organized by category
│       ├── mod.rs           # Rule trait, priority system, RuleRegistry (21KB)
│       ├── numeric/         # 0+x→x, constant folding
│       ├── algebraic/       # 9 files: factoring, power, terms, etc.
│       ├── trigonometric/   # 7 files: Pythagorean, double angle, etc.
│       ├── hyperbolic/      # 5 files: sinh/cosh identities
│       ├── exponential/     # exp/ln rules
│       └── root/            # sqrt/cbrt rules
│
├── uncertainty.rs            # Uncertainty propagation (GUM formula) (12KB)
│
├── functions/                # Built-in functions
│   ├── mod.rs               # Function registry
│   ├── definitions.rs       # 50+ function defs (55KB)
│   └── registry.rs          # Name → Function lookup
│
├── math/                     # Numeric implementations
│   ├── mod.rs               # Special function evaluations (gamma, bessel, etc.)
│   └── dual.rs              # Dual numbers for automatic differentiation
│
├── bindings/                 # Optional feature-gated bindings
│   ├── mod.rs               # Feature gates
│   ├── eval_f64.rs          # Fast f64 evaluation helper (6KB)
│   ├── parallel.rs          # (parallel feature) Rayon batch evaluation (25KB)
│   └── python.rs            # (python feature) PyO3 bindings (99KB)
│
└── tests/                    # 65 test files (+22 integration tests at root)
```

### Key File Sizes

| File                        | Size   | Purpose                                           |
| --------------------------- | ------ | ------------------------------------------------- |
| `core/evaluator.rs`         | 103 KB | Bytecode VM, CompiledEvaluator, all instructions  |
| `bindings/python.rs`        | 99 KB  | Full Python API bindings via PyO3                 |
| `core/expr.rs`              | 61 KB  | Expr, ExprKind, constructors, methods             |
| `core/symbol.rs`            | 58 KB  | InternedSymbol, Symbol, global registry           |
| `functions/definitions.rs`  | 55 KB  | All built-in function derivatives/evaluations     |
| `core/display.rs`           | 34 KB  | LaTeX/Unicode/String formatting                   |
| `parser/lexer.rs`           | 30 KB  | Tokenizer with edge case handling                 |
| `simplification/helpers.rs` | 27 KB  | Pattern matching utilities                        |
| `diff/engine.rs`            | 23 KB  | Core differentiation rules (chain, product, etc.) |

---

## Core Components

### 1. Expression AST (`core/expr.rs`)

The fundamental data structure is `Expr`, a tree-based AST using `Arc` for shared ownership.

```rust
pub struct Expr {
    pub id: u64,          // Structural hash for equality/caching
    pub hash: u64,        // Pre-computed hash for HashMap
    pub kind: ExprKind,
}

pub enum ExprKind {
    Number(f64),
    Symbol(InternedSymbol),
    
    // N-ary operations (flat, auto-sorted)
    Sum(Vec<Arc<Expr>>),      // a + b + c + ...
    Product(Vec<Arc<Expr>>),  // a * b * c * ...
    
    // Binary operations (non-associative)
    Div(Arc<Expr>, Arc<Expr>),
    Pow(Arc<Expr>, Arc<Expr>),
    
    FunctionCall { name: InternedSymbol, args: Vec<Arc<Expr>> },
    Derivative { inner: Arc<Expr>, var: String, order: u32 },
    Poly(Poly),               // Optimized polynomial representation
}
```

**Design decisions:**
- **`Arc<Expr>`** for sharing subexpressions (e.g., in `x + x`)
- **Binary operators** as separate variants (faster pattern matching than n-ary)
- **`Derivative`** variant for unevaluated partial derivatives

### 2. Symbol System (`symbol.rs`)

Symbols are **interned** for O(1) comparison and implement **`Copy`** for ergonomic usage.

```rust
/// An interned symbol stored in the global registry
#[derive(Debug, Clone)]
pub struct InternedSymbol {
    id: u64,
    name: Option<Arc<String>>,  // None for anonymous symbols
}

/// Copy-able handle to an interned symbol (just the ID)
#[derive(Copy, Clone)]
pub struct Symbol {
    id: u64,
}
```

**Key features:**
- `symb("x")` - Get or create symbol (idempotent)
- `symb_new("x")` - Create only (errors if exists)
- `symb_get("x")` - Get only (errors if not found)
- Registry is **global with RwLock** for thread-safe access
- **Function names** also use `InternedSymbol` for O(1) pattern matching

**Why Copy?** Enables natural operator usage: `x + x` works without `.clone()`.

### 2b. Unified Context (`unified_context.rs`)

The `Context` type provides **isolated symbol registries** and **user-defined functions** for advanced use cases.

```rust
use symb_anafis::{Context, UserFunction, Expr, Diff};

let ctx = Context::new()
    .with_symbol("x")
    .with_symbol("alpha")  // Multi-char symbol for parsing
    .with_function("f", UserFunction::new(1..=1)
        .body(|args| (*args[0]).clone().pow(2.0) + 1.0)
        .partial(0, |args| 2.0 * (*args[0]).clone()).expect("valid"));

let x = ctx.symb("x");
let derivative = Diff::new()
    .with_context(&ctx)
    .differentiate(&Expr::func("f", x), &x);
```

**Key features:**
- **Isolated registries**: Symbols created via `ctx.symb()` don't collide with global symbols
- **User-defined functions**: `UserFunction` with optional body and partial derivatives
- **Thread-safe**: Uses `Arc<RwLock>` internally for concurrent access

| Method                              | Purpose                                     |
| ----------------------------------- | ------------------------------------------- |
| `with_symbol(name)`                 | Register symbol for parsing hints           |
| `with_function(name, UserFunction)` | Register custom function with body/partials |
| `symb(name)`                        | Get or create symbol in this context        |
| `get_user_fn(name)`                 | Retrieve a registered function              |

### 3. Parser (`parser/`)

A **Pratt parser** (operator-precedence) with custom lexer supporting:

```
parser/
├── lexer.rs        # Tokenizes input (numbers, symbols, operators)
├── tokens.rs       # Token definitions
├── pratt.rs        # Pratt parsing algorithm
└── implicit_mul.rs # Handles implicit multiplication (2x, xy)
```

**Features:**
- Implicit multiplication: `2x` → `2 * x`, `xy` → `x * y`
- Scientific notation: `1e-5`, `2.5e3`
- Multi-character symbols: `sigma`, `alpha`
- Function calls: `sin(x)`, `besselj(0, x)`

**Parsing speed:** ~500-900 ns per expression (1.6-2.3x faster than Symbolica)

### 4. Differentiation (`diff/engine.rs`)

Implements standard calculus rules:

| Rule     | Implementation                          |
| -------- | --------------------------------------- |
| Constant | d/dx[c] = 0                             |
| Variable | d/dx[x] = 1                             |
| Sum      | d/dx[a + b] = a' + b'                   |
| Product  | d/dx[a · b] = a'b + ab'                 |
| Quotient | d/dx[a/b] = (a'b - ab')/b²              |
| Chain    | d/dx[f(g)] = f'(g) · g'                 |
| Power    | d/dx[a^b] = a^b · (b' ln(a) + b · a'/a) |

**Custom functions:**
```rust
Diff::new()
    .user_fn("f", UserFunction::new(1..=1)
        .partial(0, |args: &[Arc<Expr>]| {
            // ∂f/∂u = 2u
            Expr::number(2.0) * Expr::from(&args[0])
        })
        .expect("valid arg"))
```

### 5. Simplification Engine (`simplification/`)

A **multi-pass, rule-based** simplification system.

```
simplification/
├── engine.rs       # Bottom-up tree traversal
├── helpers.rs      # Expression matching utilities
└── rules/          # 120+ categorized rules
    ├── numeric/    # Constant folding, 0+x→x
    ├── algebraic/  # Factoring, distribution, powers
    ├── trigonometric/  # sin²+cos²=1, double angles
    ├── hyperbolic/     # sinh/cosh identities
    ├── exponential/    # exp(ln(x))→x, log rules
    └── root/           # sqrt/cbrt simplification
```

**Priority ordering:** Expand → Cancel → Compact

| Priority | Phase            | Purpose               |
| -------- | ---------------- | --------------------- |
| 85-95    | Expansion        | Distribute, flatten   |
| 70-84    | Cancellation     | x^0→1, x/x→1          |
| 40-69    | Consolidation    | Factor, combine terms |
| 1-39     | Canonicalization | Sort terms            |

**Domain-safe mode:** Skips rules that alter domains (e.g., `sqrt(x²) → x`).

### 6. Builder Pattern (`builder.rs`)

Fluent API for configuration:

```rust
Diff::new()
    .domain_safe(true)      // Preserve domains
    .max_depth(200)         // AST depth limit
    .max_nodes(50000)       // Node count limit
    .fixed_var(&symb("a"))  // Treat as constant
    .custom_fn("f")         // Register function
    .diff_str("a*f(x)", "x")
```

Both `Diff` and `Simplify` builders share similar configuration.

---

## Data Flow

### `diff("x^2 + sin(x)", "x")`

```
1. PARSE
   "x^2 + sin(x)" → Expr::Add(Pow(x,2), sin(x))

2. DIFFERENTIATE
   d/dx[x² + sin(x)]
   = d/dx[x²] + d/dx[sin(x)]
   = 2x + cos(x)

3. SIMPLIFY (automatic)
   Multi-pass rule application
   Result: "2*x + cos(x)"

4. FORMAT
   Expr → String: "2*x + cos(x)"
```

---

## Special Features

### Vector Calculus (`helpers.rs`)

```rust
gradient_str("x^2 + y^2", &["x", "y"])?;  // ["2*x", "2*y"]
hessian_str("x^2*y", &["x", "y"])?;       // [["2*y", "2*x"], ["2*x", "0"]]
jacobian_str(&["x+y", "x*y"], &["x", "y"])?;
```

### Uncertainty Propagation (`uncertainty.rs`)

Implements the GUM formula: σ_f = √(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))

```rust
uncertainty_propagation(&expr, &["x", "y"], Some(&cov_matrix))?;
```

### Parallel Evaluation (`parallel.rs`, requires `parallel` feature)

Batch evaluation using Rayon:

```rust
eval_parallel!(
    [expr1, expr2],
    [["x"], ["x", "y"]],
    [[[1.0, 2.0]], [[1.0, 2.0], [3.0, 4.0]]]
)
```

### Python Bindings (`python.rs`, requires `python` feature)

PyO3 bindings exposing:
- `diff()`, `simplify()`, `parse()`
- `PyExpr` with operator overloading
- `PyDiff`, `PySimplify` builders

---

## Built-in Functions (`functions/`)

50+ functions with differentiation and evaluation rules:

| Category   | Functions                               |
| ---------- | --------------------------------------- |
| Trig       | sin, cos, tan, cot, sec, csc + inverses |
| Hyperbolic | sinh, cosh, tanh + inverses             |
| Exp/Log    | exp, ln, log, log10, log2               |
| Special    | gamma, digamma, polygamma, beta         |
| Bessel     | besselj, bessely, besseli, besselk      |
| Error      | erf, erfc                               |
| Other      | zeta, LambertW, abs, signum             |

---

## Extension Points

1. **Custom functions:** `Diff::new().user_fn("f", UserFunction)` with partial derivatives and evaluation
2. **Custom evaluation:** `Diff::new().custom_eval("f", ...)`
3. **New simplification rules:** Add to `simplification/rules/`
4. **New functions:** Add to `functions/definitions.rs` and `registry.rs`
