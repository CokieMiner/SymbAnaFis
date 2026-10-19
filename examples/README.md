# Rust Examples

Run an example from the repository root:

```sh
cargo run --example quickstart
cargo run --example view_api_demo
cargo run --example instruction_dump
cargo run --example applications
```

| Example | Operations |
| --- | --- |
| [quickstart.rs](quickstart.rs) | Scoped parsing, differentiation, explicit simplification, exact `Number` evaluation, native `f64` evaluation, scalar batches, and formatting. |
| [view_api_demo.rs](view_api_demo.rs) | Borrowed views, logical polynomial terms, shared-subexpression traversal, and retained derivatives of registered opaque functions. |
| [instruction_dump.rs](instruction_dump.rs) | Derivatives of nine scientific formulas, explicit compilation simplification, register allocation, and bytecode disassembly. |
| [applications.rs](applications.rs) | Kinematics, independent length uncertainties propagated to a volume, and a polar-coordinate Jacobian. |

Each executable uses a scoped `Context`. Unknown parser variables belong to that context; custom functions require registration. Exact literals use integers or rational expressions. The native evaluator can reassociate arithmetic and fuse multiply-add operations, so its floating-point results need not be bitwise identical to a direct DAG evaluation.

The examples are standard-library executables. The library uses its default `no_std + alloc` configuration unless a feature is selected explicitly. Add `--features std` to select blocking registry mutexes and NumAnaFis standard-library integration.

`instruction_dump` always prints the original derivative's program. It limits each simplifier rewrite pass to 10,000 candidate attempts. If simplification exceeds a resource bound, it reports the error and continues to the next formula without publishing a partial simplified result.

Scientific models state their real-coordinate and regularity assumptions in the source. Their units are supplied by the caller. Uncertainty propagation is a first-order approximation, and diagonal covariance entries are variances.

See the [API guide](../docs/API.md) for custom functions, guarded rewrites, simultaneous substitutions, and callback-generated binomial expansions.
