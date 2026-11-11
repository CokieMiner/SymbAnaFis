# SymbAnaFis - Parsing API

Reference for mathematical expression parsing, grammar rules, scopes, custom functions, and diagnostics in `symb_anafis`.

[← Back to API Index](../API.md)

---

### Contents

1. [Expressions and scoped names](#1-expressions-and-scoped-names)
2. [Grammar and operator syntax](#2-grammar-and-operator-syntax)
3. [Registered custom functions](#3-registered-custom-functions)
4. [Diagnostics and limits](#4-diagnostics-and-limits)

---

### 1. Expressions and scoped names

`Context::parse(&str) -> Result<Expr, ParseError>` constructs expressions through the same normalized kernels as arithmetic operators. Unknown variable names create symbols in that context. Repeated parsing uses its existing bindings; the returned expression can outlive the context.

```rust
use symb_anafis::{Context, Expr};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let expression = ctx.parse("(x + y)^4 + 2x sin(y)")?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    assert_eq!(expression, (x + y).pow(4) + 2 * x * y.sin());

    assert_eq!(ctx.parse("1/4")?, Expr::number(1) / 4);
    assert_eq!(ctx.parse("sqrt(1/4)")?, Expr::number(1) / 2);
    assert_eq!(ctx.parse("2.0")?, Expr::number(2.0));
    assert_ne!(ctx.parse("2.0")?, Expr::number(2));
    Ok(())
}
```

---

### 2. Grammar and operator syntax

| Syntax | Meaning |
| --- | --- |
| `42`, `1_024`, `-9223372036854775808` | Exact `i64` literals. Out-of-range integers return an error. |
| `2.0`, `.25`, `1.5e-3` | Finite `f64` literals; approximate representation is retained. |
| `pi`, `π`, `e`, `i` | Exact mathematical constants, unless a symbol of that name already exists in the context. |
| `+`, `-`, `*`, `/` | Scalar arithmetic with unary signs. |
| `^`, `**` | Right-associative powers: `x^y^z` means `x^(y^z)`. |
| `-x^2`, `x^-2` | `-(x^2)` and `x^(-2)`, respectively. |
| `2x`, `x(y+1)`, `(x+1)(y+1)` | Implicit multiplication with the same precedence as `*`. |
| `velocity`, `x2`, `xy` | Whole symbol names; `xy` does not split into `x*y`. |
| `sin(x)`, `log(2,x)`, `besselj(1,x)` | Known functions with the registry's argument order and folding rules. |

Names start with a letter or underscore and continue with letters, digits, or underscores. Unicode letters and whitespace are accepted; `−`, `×`, `·`, and `÷` are operator aliases. Adjacent numeric literals such as `1 2` are rejected. Function calls require parentheses.

---

### 3. Registered custom functions

The parser resolves builtin names and aliases through `Function::builtin`, and custom names through the supplied context. It validates call arity and retains the registered identity. Parsing leaves custom calls symbolic; it does not execute bodies or partial callbacks.

```rust
use symb_anafis::{Context, UserFunction};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let response = ctx.register_function("response", UserFunction::new(2..=2))?;
    let parsed = ctx.parse("response(x, sin(x))")?;
    let x = ctx.symb("x")?;
    assert_eq!(parsed, response.call(vec![x.to_expr(), x.sin()])?);
    assert!(ctx.parse("response(x)").is_err());
    Ok(())
}
```

An unknown `f(x)` denotes multiplication by a symbol `f`; it never creates an unregistered function identity. Register `f` before parsing a call. Variables and functions occupy separate namespaces: a known function followed by `(` is a call, even if a variable shares its name.

---

### 4. Diagnostics and limits

`ParseError` implements `Display` and `core::error::Error`. Its variants distinguish invalid characters, invalid literals, missing operands, unexpected tokens, unclosed parentheses, function errors, and symbol allocation errors. Each located diagnostic carries a half-open UTF-8 byte `Span`. A `Span` displays as a location suffix with one-based byte positions, ` at position 3` or ` at positions 3-5`, and an empty span displays as nothing.

```rust
use symb_anafis::{Context, ParseError, Span};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    assert_eq!(ctx.parse("x +"), Err(ParseError::ExpectedExpression {
        span: Span::new(3, 3),
    }));
    assert_eq!(ctx.parse("(x"), Err(ParseError::UnclosedParenthesis {
        span: Span::new(0, 1),
    }));
    Ok(())
}
```

The parser consumes the complete input and requires balanced parentheses. Symbols created before an error remain in the context. Parsing uses heap-backed operator and group frames; flat additive and multiplicative chains are passed to n-ary constructors together. Memory usage scales with source structure and the resulting DAG.

The grammar covers scalar arithmetic and known function calls. Differential notation such as `d^2 f(x)/dx^2`, equations, matrices, and binders are not part of this grammar; construct unevaluated derivatives through `Expr::derivative`.
