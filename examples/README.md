# SymbAnaFis Examples

Runnable demonstrations for Rust and Python workflows, scientific modeling, DAG inspection, and bytecode listings.

---

## Rust Examples (`examples/rust/`)

Run any Rust example directly with Cargo from the repository root:

```sh
cargo run --example quickstart
cargo run --example view_api_demo
cargo run --example instruction_dump
cargo run --example applications
```

| Example | Source | Description |
| --- | --- | --- |
| **Quickstart** | [`rust/quickstart.rs`](rust/quickstart.rs) | Expression creation, differentiation, canonical `.compact()` simplification, exact `Number` and `f64` evaluation, batch evaluation, and LaTeX/Unicode export. |
| **View API Demo** | [`rust/view_api_demo.rs`](rust/view_api_demo.rs) | Inspecting expressions with borrowed `ExprView`s, postorder DAG folding, polynomial term iteration, and retained derivatives. |
| **Instruction Dump** | [`rust/instruction_dump.rs`](rust/instruction_dump.rs) | Derivatives across nine scientific models (Planck, Maxwell-Boltzmann, Lennard-Jones, etc.), register VM compilation, and bytecode disassembly. |
| **Applications** | [`rust/applications.rs`](rust/applications.rs) | Real-world kinematics, volume uncertainty propagation with diagonal covariance, and polar coordinate Jacobians via the `.jacobian` trait. |

---

## Python Examples (`examples/python/`)

After compiling the extension with `maturin develop --release` in a virtual environment:

```sh
python examples/python/quickstart.py
```

| Script | Source | Description |
| --- | --- | --- |
| **Quickstart** | [`python/quickstart.py`](python/quickstart.py) | Pythonic symbolic workflows: differentiation, gradients, Hessians, multi-output Jacobians, canonical simplification (`.compact()`, `.expand()`, `.factor()`), and compiled evaluation. |
| **Clifford Benchmark** | [`python/clifford_benchmark.py`](python/clifford_benchmark.py) | 2D Clifford attractor simulation, performance comparison against SymPy and SymEngine, and animated Quad-View video rendering. |
| **Aizawa Flow** | [`python/aizawa_flow.py`](python/aizawa_flow.py) | 3D Aizawa chaotic flow simulation for large particle ensembles, comparing evaluation throughput. |
| **Double Pendulum** | [`python/double_pendulum_benchmark.py`](python/double_pendulum_benchmark.py) | Full symbolic Lagrangian mechanics derivation ($L = T - V$), symbolic mass matrix inversion via Cramer's rule, and parallel RK4 integration. |
| **Gradient Descent** | [`python/gradient_descent.py`](python/gradient_descent.py) | High-dimensional ensemble particle simulation rolling down a complex potential surface using adaptive step sizing and exact gradients. |
| **Benchmark Utilities** | [`python/bench_common.py`](python/bench_common.py) | Shared utilities for dark-themed Quad-View dashboards, metric formatting, performance bar panels, and animation export. |
| **Video Writer** | [`python/video_writer.py`](python/video_writer.py) | Matplotlib animation exporter with GPU NVENC hardware acceleration and automatic CPU fallback. |

---

## Benchmarks (`examples/benchmarks/`)

Independent benchmarks comparing `symb_anafis` against other symbolic mathematics engines (such as Symbolica) on large-scale expressions:

```sh
cargo run --manifest-path examples/benchmarks/Cargo.toml --bin expr_with_4649517_caracters-comparison
```
