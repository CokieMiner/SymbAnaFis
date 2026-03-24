# Module Architecture and Encapsulation Guidelines

## 1. Directory Structure Template
Every major subsystem must follow this exact folder hierarchy:

```text
src/[module_name]/
├── mod.rs               # Boundary Manager
├── api.rs               # Public API surface (or api_user.rs / api_crate.rs)
└── logic/               # Implementation Details
    ├── mod.rs           # Logic Registry
    └── [feature].rs     # Feature implementations
```

## 2. Layer Governance

### 2.1 Boundary Manager (`mod.rs`)
The file position is at the root level of the module or submodule.
- **Responsibility**: Declaring the subdirectories and connecting the API endpoint to upper tiers.
- **Rules**:
  - Must not declare any operational algorithms.
  - Must not declare data structures meant for processing.
  - Must use `pub use api::*;` to expose the API.
  - Must never use `pub use logic::*;`.

### 2.2 API Decider (`api.rs`)
Acts as the single gatekeeper determining module visibility.
- **Responsibility**: Declaring items meant for other components to consume.
- **Rules**:
  - Contains traits, builders, or safe handles.
  - Connects types from `logic/` securely.
  - Translate items from the logic layer for the consumer layer using safe wrappers.

### 2.3 Logic Core (`logic/`)
Contains all algorithms and direct implementations.
- **Responsibility**: Functional computations.
- **Rules**:
  - All items inside must be non-public to the outside of the module.
  - Use `pub(super)` for sharing items with the local `api.rs`.
  - Use `pub(crate)` for system-wide registries (e.g., dispatchers, instruction selection).
  - Standard `pub` is forbidden inside this folder for methods or types meant only for internal processing.

## 3. Test Placement
- **In-line `#[test]` modules inside logic source files are forbidden.**
- Tests must be placed in a dedicated `test.rs` or `tests.rs` file inside the module block.
- If there are multiple test components, they should be grouped logically in a single file or a structured test directory.
- Heavy integration or comprehensive execution tests should reside in the global `tests/` root directory.
