---
description: How to add a new simplification rule to symb_anafis
---

# Adding a Simplification Rule

Simplification rules transform expressions into simpler forms. Unlike adding functions, rules only require **2-3 locations**.

## Step 1: Choose Category and Priority

Categories (in `src/simplification/rules/`):
- `algebraic/` - General algebraic rules (x-x=0, x*1=x)
- `exponential/` - exp/log rules
- `hyperbolic/` - sinh, cosh, tanh rules
- `numeric/` - Constant folding
- `root/` - sqrt, cbrt rules
- `trigonometric/` - sin, cos, tan rules

Priority ranges (higher = runs first):
- **85-95**: Special values and identities (sin(0)=0)
- **70-84**: Identity/cancellation rules (x/x=1)
- **40-69**: Consolidation rules (combine terms)
- **1-39**: Canonicalization (sort, normalize)

## Step 2: Define the Rule

Create or edit a file in the category folder (e.g., `trigonometric/basic.rs`):

```rust
use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;

// Use rule_arc! when returning Arc<Expr> directly (most common)
rule_arc!(
    MyNewRule,               // Struct name (PascalCase)
    "my_new_rule",           // Rule name (snake_case for logs)
    95,                      // Priority (higher = runs first)
    Trigonometric,           // Category
    &[ExprKind::Function],   // Expression kinds this applies to
    targets: &["sin"],       // Optional: specific function names
    |expr: &Expr, _context: &RuleContext| {
        // Pattern match and return Some(Arc::new(result)) if rule applies
        // Return None if rule doesn't apply
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.as_str() == "sin"
            && args.len() == 1
            && matches!(&args[0].kind, AstKind::Number(n) if *n == 0.0)
        {
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);
```

### Macro Variants

| Macro                    | Use when                                       |
| ------------------------ | ---------------------------------------------- |
| `rule_arc!`              | Returning `Option<Arc<Expr>>` directly         |
| `rule!`                  | Returning `Option<Expr>` (auto-wrapped in Arc) |
| `rule_with_helpers!`     | Need helper functions inside the rule          |
| `rule_with_helpers_arc!` | Helpers + Arc return                           |

### Options

- `targets: &["fn1", "fn2"]` - Only check for specific functions (performance)
- `alters_domain: true` - Mark if rule changes expression domain

## Step 3: Export from Category mod.rs

Edit the category's `mod.rs` (e.g., `trigonometric/mod.rs`):

```rust
// Add module if new file
pub mod my_rules;

// Add to get_*_rules() function
pub fn get_trigonometric_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        // ... existing rules ...
        Arc::new(my_rules::MyNewRule),  // ADD THIS
    ]
}
```

## Testing

Add tests in `src/tests/simplification_tests.rs`:

```rust
#[test]
fn test_my_new_rule() {
    let input = parse("sin(0)", ...).unwrap();
    let result = Simplify::new().simplify(&input).unwrap();
    assert_eq!(result.to_string(), "0");
}
```

Run tests:
```bash
cargo test simplification
cargo test trig_simplification
```

## Quick Reference

| Location                  | What to do                             |
| ------------------------- | -------------------------------------- |
| `rules/<category>/*.rs`   | Define rule with `rule_arc!` macro     |
| `rules/<category>/mod.rs` | Export module + add to `get_*_rules()` |
| `tests/*_tests.rs`        | Add tests                              |

## ExprKind Reference

```rust
ExprKind::Number    // Constants (3.14)
ExprKind::Symbol    // Variables (x, pi)
ExprKind::Sum       // a + b + c
ExprKind::Product   // a * b * c
ExprKind::Div       // a / b
ExprKind::Pow       // a ^ b
ExprKind::Function  // sin(x), exp(x)
ExprKind::Poly      // Polynomial representation
```

## Helper Functions

Available in `simplification::helpers`:
- `is_zero(expr)`, `is_one(expr)`, `is_pi(expr)`
- `get_numeric_value(expr) -> Option<f64>`
- `approx_eq(a, b) -> bool`
- `is_negative(expr) -> bool`
