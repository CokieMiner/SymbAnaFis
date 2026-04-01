# SymbAnaFis — Module Organization & Encapsulation Guide

> This document is the **single source of truth** for module structure, visibility rules,
> and re-export discipline in the SymbAnaFis codebase.

---

## Table of Contents

- [SymbAnaFis — Module Organization \& Encapsulation Guide](#symbanafis--module-organization--encapsulation-guide)
  - [Table of Contents](#table-of-contents)
  - [1. Core Philosophy](#1-core-philosophy)
  - [2. Directory Structure Template](#2-directory-structure-template)
  - [3. The Three-Layer Model](#3-the-three-layer-model)
    - [3.1 Boundary Manager (`mod.rs`)](#31-boundary-manager-modrs)
    - [3.2 API Decider (`api.rs`)](#32-api-decider-apirs)
    - [3.3 Logic Core (`logic/`)](#33-logic-core-logic)
  - [4. The Staircase Rule — Strict Re-Export Chain](#4-the-staircase-rule--strict-re-export-chain)
    - [4.1 The Chain](#41-the-chain)
    - [4.2 Visual Representation](#42-visual-representation)
    - [4.3 Strict Constraints](#43-strict-constraints)
  - [5. Special Case: `core/` Top-Level Split](#5-special-case-core-top-level-split)
  - [6. Visibility Reference](#6-visibility-reference)
  - [7. Import Standards](#7-import-standards)
    - [7.1 Cross-Module Imports (External)](#71-cross-module-imports-external)
    - [7.2 Internal Imports (Same Component)](#72-internal-imports-same-component)
    - [7.3 Encapsulation ("Blocking")](#73-encapsulation-blocking)
  - [8. Test Placement](#8-test-placement)
  - [9. Layer Separation Policy](#9-layer-separation-policy)
  - [10. Canonical Code Example](#10-canonical-code-example)
  - [11. Forbidden Patterns (Never Do This)](#11-forbidden-patterns-never-do-this)
  - [12. Required Patterns (Always Do This)](#12-required-patterns-always-do-this)
  - [13. Common Failure Modes \& Fixes](#13-common-failure-modes--fixes)
    - [Problem: "unresolved import" errors](#problem-unresolved-import-errors)
    - [Problem: `pub(in crate::module)` item fails to re-export as `pub`](#problem-pubin-cratemodule-item-fails-to-re-export-as-pub)
    - [Problem: Making `logic/` or `logic/mod.rs` `pub`](#problem-making-logic-or-logicmodrs-pub)
    - [Problem: Skipping the staircase with `pub(crate)` in `logic/`](#problem-skipping-the-staircase-with-pubcrate-in-logic)

---

## 1. Core Philosophy

SymbAnaFis enforces **strict layered encapsulation**: implementation details are fully
hidden behind a controlled public surface. Visibility only travels **one step at a time**
up the module hierarchy via re-exports. No layer may skip over another.

This ensures:
- Consumers can never depend on internal types or paths.
- Refactoring internals never breaks downstream code.
- The API surface is explicit, reviewable, and minimal.

---

## 2. Directory Structure Template

Every subsystem follows this exact folder layout:

```text
src/[module_name]/
├── mod.rs               # Boundary Manager — declares submodules, re-exports API
├── api.rs               # API Decider — what the module exposes to the world
└── logic/               # Logic Core — all implementation details
    ├── mod.rs           # Logic Registry — aggregates feature files
    └── [feature].rs     # Feature implementations
```

**Optional split** (only for `core/` top-level):

```text
src/core/
├── mod.rs               # Boundary Manager
├── api_user.rs          # Public API (items for external consumers)
├── api_crate.rs         # Crate-internal API (items for sibling modules)
└── [submodule]/         # Each submodule still follows the template above
```

---

## 3. The Three-Layer Model

### 3.1 Boundary Manager (`mod.rs`)

**Position**: Root of the module (or submodule).

**Responsibilities**:
- Declare all child submodules (`mod logic;`, `mod api;`).
- Re-export the API surface to the parent tier via `pub use api::*;`.

**Rules**:
- MUST NOT declare any algorithms or data structures.
- MUST NOT use `pub use logic::*;` — the logic layer is never surfaced directly.
- All `mod` declarations for internal modules must be **private** (no `pub mod`).
- The ONLY allowed `pub use` is `pub use api::*;` (or `pub use api_user::*; pub use api_crate::*;` for `core/`).

### 3.2 API Decider (`api.rs`)

**Position**: Sibling of `mod.rs`, at the root of the module.

**Responsibilities**:
- Determine exactly what escapes the module boundary.
- Wrap, rename, or selectively re-export items from `logic/`.
- Define whether an item is `pub` (external) or `pub(crate)` (internal).

**Rules**:
- Reaches `logic/` via `use super::logic;` or `use super::logic::SomeItem;`.
- May define new `pub` types (wrappers, newtypes, builders) over internal types.
- Is allowed to use `pub(crate)` for items that must cross module lines inside the crate.
- Never declares algorithmic logic — only wiring and surface decisions.

### 3.3 Logic Core (`logic/`)

**Position**: Private subdirectory inside the module.

**Responsibilities**:
- All concrete implementations, algorithms, and internal data structures.

**Rules**:
- `mod logic;` in `mod.rs` is **always private** (bare `mod`, never `pub mod`).
- Feature files expose items **only to `logic/mod.rs`** with `pub(super)`.
- `logic/mod.rs` re-exports items **only to `api.rs`** (its parent) with `pub(super)`.
- Plain `pub` on any item inside `logic/` is **forbidden** if that item is only meant for internal use.
- `pub(crate)` is allowed inside `logic/` only for **system-wide registry items**
  (e.g., dispatchers, instruction selection) that must be visible anywhere in the crate.

---

## 4. The Staircase Rule — Strict Re-Export Chain

### 4.1 The Chain

```
logic/[feature].rs  →(pub(super))→  logic/mod.rs   [Logic Registry]
logic/mod.rs        →(pub(super))→  api.rs          [API Decider]
api.rs              →(pub / pub(crate))→  [consumed by mod.rs]
mod.rs              →(pub use api::*)→  crate/parent surface
```

Each arrow represents **exactly one hop**. No step may be skipped.

### 4.2 Visual Representation

```
                    ┌─────────────────────────────────────────────┐
                    │             lib.rs / crate root             │
                    │                                             │
                    │  mod core;  mod parser;  mod diff; ...      │
                    │  pub use core::*;    pub use diff::*;       │
                    └─────────────────┬───────────────────────────┘
                                      │ pub use api::*
                    ┌─────────────────┴───────────────────────────┐
                    │         [module]/mod.rs  (Boundary Manager) │
                    │                                             │
                    │  mod logic;       ← PRIVATE, never pub      │
                    │  mod api;         ← PRIVATE                 │
                    │  pub use api::*;  ← THE ONLY pub use        │
                    └─────────────────┬───────────────────────────┘
                                      │ pub(super)
                    ┌─────────────────┴───────────────────────────┐
                    │        [module]/api.rs  (API Decider)       │
                    │                                             │
                    │  use super::logic;                          │
                    │  pub struct PublicFoo(logic::InternalFoo);  │
                    │  pub fn factory() -> PublicFoo { ... }      │
                    │  pub(crate) type CrateAlias = ...;          │
                    └─────────────────┬───────────────────────────┘
                                      │ pub(super)
                    ┌─────────────────┴───────────────────────────┐
                    │       [module]/logic/mod.rs (Logic Registry)│
                    │                                             │
                    │  mod feature_a;                             │
                    │  pub(super) use feature_a::InternalFoo;     │
                    └─────────────────┬───────────────────────────┘
                                      │ pub(super)
                    ┌─────────────────┴───────────────────────────┐
                    │    [module]/logic/feature_a.rs              │
                    │                                             │
                    │  pub(super) struct InternalFoo { ... }      │
                    │  pub(super) fn compute() -> InternalFoo { } │
                    └─────────────────────────────────────────────┘
```

### 4.3 Strict Constraints

1. **Private modules**: `mod.rs` must declare `mod logic;` and `mod api;` as bare private
   (no `pub` prefix).
2. **Surface purity**: `mod.rs` ONLY does `pub use api::*;`. It never references `logic::` directly.
3. **No skipping**: `logic/mod.rs` must NOT re-export as `pub(crate)` unless the item genuinely
   needs crate-wide visibility. Use `pub(super)` for anything local to the module pair.
4. **Internal visibility**: Items inside `logic/` use `pub(super)` (for registry → api hop) or
   `pub(in crate::my_module)` (for cross-sibling hop within the same parent module).
   Plain `pub` is forbidden on internal types.

---

## 5. Special Case: `core/` Top-Level Split

`src/core/mod.rs` is the **only location** where the `api_user` / `api_crate` split is
permitted. All interior submodules of `core` (`context`, `expr`, `symbol`, `helpers`)
use a single `api.rs` layer.

```text
src/core/
├── mod.rs          →  pub use api_user::*;
│                      pub use api_crate::*;
├── api_user.rs     →  items intended for external crate consumers
├── api_crate.rs    →  items needed by sibling top-level modules (e.g., diff, evaluator)
├── context/        →  single api.rs (not split)
├── expr/           →  single api.rs
├── symbol/         →  single api.rs
└── helpers/        →  single api.rs
```

The rationale: the core module is the foundational layer that other modules depend on
both publicly (users import `Expr`, `Symbol`) and internally (sibling modules import
`DiffContext`, `arc_number`, etc.). Splitting into two files makes this boundary explicit.

Any future interior submodule of `core` must still use a single `api.rs` — the split is
strictly reserved for the `core/` root.

---

## 6. Visibility Reference

| Visibility | Where to Use |
|-----------|-------------|
| `pub(super)` | Item in `logic/[feature].rs` → visible to `logic/mod.rs`; item in `logic/mod.rs` → visible to sibling `api.rs` |
| `pub(crate)` | Item needed anywhere inside the crate (cross-module sibling access); used in `api.rs` for crate-internal types |
| `pub(in crate::module)` | Item needs visibility scoped to a specific module subtree; use sparingly |
| `pub` | Item is part of the public external API; declared in `api.rs` / `api_user.rs` |
| *(bare, no modifier)* | Item is private to the current file/module |

> [!IMPORTANT]
> A `pub(in crate::module)` item **cannot** be directly re-exported as `pub` at the crate root.
> Use a `pub` wrapper function (for functions) or ensure the type itself is `pub` in `api.rs`.

---

## 7. Import Standards

### 7.1 Cross-Module Imports (External)

Use `crate::[module]::[Item]` for items that have been re-exported in the target module's API.

```rust
// ✅ Good — uses the public surface
use crate::core::Expr;
use crate::parser::parse;

// ❌ Bad — bypasses the staircase
use crate::core::expr::logic::constructors::base::make_expr;
```

Always import specific items at the top of the file; never use fully qualified paths inline:

```rust
// ✅ Good
use crate::parser::parse;
parse(input);

// ❌ Bad
crate::parser::parse(input);
```

### 7.2 Internal Imports (Same Component)

Inside a module, use `super` to navigate the local hierarchy:

```rust
// In api.rs, reaching into logic:
use super::logic;
use super::logic::SomeInternalType;
```

Prefer shallow internal imports (`super::foo`, `super::foo::Bar`) over deep chains.
Avoid `super::super::...` unless there is no clearer local path.

Import concrete symbols at the top of the file and call them directly.
Do not use inline-qualified paths such as `module::item(...)` when the item can be imported.

```rust
// ✅ Good
use super::float_ops::{float_div, float_from_int};
let out = float_div(&float_from_int(&num), &float_from_int(&den));

// ❌ Bad
let out = super::float_ops::float_div(
  &super::float_ops::float_from_int(&num),
  &super::float_ops::float_from_int(&den),
);
```

Backend selector files may use one local alias import for dispatch readability:

```rust
use super::f64_ops as backend;
```

This alias is the only recommended alias pattern in numeric backend dispatch modules.

### 7.3 Encapsulation ("Blocking")

Internal modules (`logic/`) **must be private** to their parent. Visibility should be
as restrictive as possible. If a restricted item must cross a boundary, use a wrapper
function or type in `api.rs` — never export `logic/` paths directly.

---

## 8. Test Placement

| Test Type | Location |
|-----------|----------|
| Unit tests for a single feature file | `logic/tests.rs` inside the same module |
| Integration / cross-module tests | `tests/` at the crate root |
| Inline `#[test]` modules inside logic files | **Forbidden** |

Tests must never be placed directly inside `logic/[feature].rs` as inline modules. Create
a dedicated `logic/tests.rs` file and include it with `mod tests;` in `logic/mod.rs`.

---

## 9. Layer Separation Policy

Interior submodules (e.g., `core/context`, `core/expr`) use a **single `api.rs`**.

The `api_user.rs` / `api_crate.rs` split is reserved **exclusively** for `src/core/mod.rs`.

No other module may introduce this split without explicit architectural discussion and
documentation here.

---

## 10. Canonical Code Example

```rust
// FILE: src/module/logic/feature.rs
pub(super) struct InternalData { /* ... */ }
pub(super) fn compute() -> InternalData { /* ... */ }

// FILE: src/module/logic/mod.rs
mod feature;
pub(super) use feature::InternalData;
pub(super) use feature::compute;

// FILE: src/module/api.rs
use super::logic;

pub struct PublicWrapper(pub(crate) logic::InternalData);

pub fn factory() -> PublicWrapper {
    PublicWrapper(logic::compute())
}

// FILE: src/module/mod.rs
mod logic;   // private
mod api;     // private

pub use api::*;

// FILE: src/other_module/consumer.rs
use crate::module::PublicWrapper;
use crate::module::factory;
// ❌ FORBIDDEN:
// use crate::module::logic::feature::InternalData;
```

---

## 11. Forbidden Patterns (Never Do This)

| ❌ Pattern | Why It's Wrong |
|-----------|----------------|
| `mod.rs` doing `pub use logic::*` | Skips the API Decider entirely |
| `mod.rs` declaring `pub mod logic` | Exposes the implementation namespace publicly |
| Logic files using plain `pub` on internal types | Leaks internals beyond the module |
| Consumers using `crate::module::logic::...` | Bypasses the API boundary |
| `use super::super::api_crate` | Wrong path; internal API isn't an ancestor |
| Inline `#[test]` inside `logic/[feature].rs` | Violates test placement rules |
| `api_user.rs` / `api_crate.rs` split inside submodules | Reserved for `core/` root only |

---

## 12. Required Patterns (Always Do This)

| ✅ Pattern | Description |
|-----------|-------------|
| `mod logic;` (bare) in `mod.rs` | Private — logic is NEVER `pub` |
| `mod api;` (bare) in `mod.rs` | Private — api controls its own `pub` items |
| `pub use api::*;` as the only pub use in `mod.rs` | Single, controlled export point |
| `pub(super)` inside `logic/` | Visible one step up — to `logic/mod.rs` only |
| `pub(super)` in `logic/mod.rs` | Visible one step up — to sibling `api.rs` only |
| `pub(crate)` for cross-module internals | Crate-wide visibility when genuinely needed |
| `use crate::module::Item` in consumers | Through public API only — no deep paths |

---

## 13. Common Failure Modes & Fixes

### Problem: "unresolved import" errors

Diagnose with this checklist:
1. **Is the path correct?** Check `api` vs `api_crate` naming.
2. **Is the item visible?** Source must use `pub(super)` to allow the upward hop.
3. **Is the chain complete?** `logic/mod.rs` must explicitly re-export the item to `api.rs`.

### Problem: `pub(in crate::module)` item fails to re-export as `pub`

`pub(in ...)` restricts re-exporting. The fix is to either:
- Make the **type** `pub` inside `api.rs` while keeping the **module path** private.
- Wrap the function in a `pub` function inside `api.rs`.

### Problem: Making `logic/` or `logic/mod.rs` `pub`

This breaks encapsulation. The staircase handles all visibility automatically —
`mod logic;` in `mod.rs` must always be bare (private).

### Problem: Skipping the staircase with `pub(crate)` in `logic/`

Only use `pub(crate)` for items that genuinely need crate-wide access
(e.g., a shared registry). For items that only need to reach the local `api.rs`,
use `pub(super)`.

---

> [!NOTE]
> The staircase must be **unbroken**. Each layer sees only the floor directly above it.
> If you break the chain anywhere, you lose the encapsulation guarantees this architecture provides.
