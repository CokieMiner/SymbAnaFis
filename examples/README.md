# SymbAnaFis Examples & Benchmarks

Runnable demonstrations for Rust and Python workflows, plus head-to-head Rust and Python benchmark suites.

```text
examples/
├── rust/                  # Core Rust examples (run via `cargo run --example ...`)
├── python/                # Standalone Python examples
└── benchmarks/
    ├── rust/              # Rust benchmarks (SymbAnaFis vs Symbolica: VM & Native JIT)
    ├── python/            # Python ensemble benchmarks (SymbAnaFis vs SymPy & SymEngine)
    └── data/              # Large benchmark expressions
```

---

## Rust Examples (`examples/rust/`)

Run any Rust example from the repository root (pass `--all-features` to enable native JIT):

```sh
cargo run --all-features --example quickstart
cargo run --example view_api_demo
cargo run --example instruction_dump
cargo run --example applications
cargo run --example rewriting
```

| Example | Source | Description |
| --- | --- | --- |
| **Quickstart** | [`rust/quickstart.rs`](rust/quickstart.rs) | Parsing, differentiation, canonical `.compact()` simplification, symbol domain properties, exact `Number` and `f64` VM/JIT evaluation, batch runs, `.wasm` export, and LaTeX/Unicode output. |
| **View API Demo** | [`rust/view_api_demo.rs`](rust/view_api_demo.rs) | Inspecting expressions with borrowed `ExprView`s, postorder DAG folding, polynomial term iteration, and retained derivatives. |
| **Instruction Dump** | [`rust/instruction_dump.rs`](rust/instruction_dump.rs) | Derivatives across nine scientific models (Planck, Maxwell-Boltzmann, Lennard-Jones, etc.), register VM compilation, and bytecode disassembly. |
| **Applications** | [`rust/applications.rs`](rust/applications.rs) | Kinematics, volume uncertainty propagation with diagonal covariance, and polar coordinate Jacobians via the `Jacobian` trait. |
| **Rewriting** | [`rust/rewriting.rs`](rust/rewriting.rs) | Concrete `Substitution`, `UserFunction` registration, `Pattern`/`RewriteRule` matching, and `SimplificationSequence` with `DomainSafety`. |

---

## Python Examples (`examples/python/`)

After compiling the extension with `maturin develop --release` in a virtual environment:

```sh
python examples/python/quickstart.py
python examples/python/numpy_interop.py   # requires optional NumPy (`symb_anafis[numpy]`)
```

| Script | Source | Description |
| --- | --- | --- |
| **Quickstart** | [`python/quickstart.py`](python/quickstart.py) | Zero-dependency Python tutorial: calculus (gradient, Hessian, Laplacian, Jacobian), canonical simplification, symbol properties, exact/native JIT evaluation, and `.wasm` export. |
| **NumPy Interop** | [`python/numpy_interop.py`](python/numpy_interop.py) | Symbolic NumPy ufuncs (`__array_ufunc__`), zero-copy 2D `meshgrid` evaluation with JIT + Rayon, scalar broadcasting with `out=`, and zero-copy `complex128` buffers. |

---

## Benchmarks (`examples/benchmarks/`)

### Rust Benchmarks (`examples/benchmarks/rust/`)

Head-to-head comparison of `SymbAnaFis` (compilation simplification + bytecode VM + native JIT) against Symbolica (bytecode VM + SymJIT native JIT) on the 4.6M-character expression in `data/big_expr.txt` (parsing, compilation, VM vs VM evaluation, Native JIT vs Native JIT evaluation, and differentiation):

```sh
cargo run --manifest-path examples/benchmarks/Cargo.toml --release --bin symbolica_comparison
```

### Python Benchmarks (`examples/benchmarks/python/`)

Large-ensemble simulations comparing `SymbAnaFis` (compilation simplification + native JIT + Rayon parallel + zero-copy `out=` buffers) against SymPy, SymEngine (LLVM), and Symbolica (SymJIT), with adaptive 3- and 4-engine Matplotlib/FFmpeg MP4 dashboards written to `examples/videos/` (pass `--no-video` for headless timing):

```sh
python examples/benchmarks/python/clifford.py --no-video
python examples/benchmarks/python/aizawa_flow.py --no-video
python examples/benchmarks/python/double_pendulum.py --no-video
python examples/benchmarks/python/gradient_descent.py --no-video
python examples/benchmarks/python/robot_arm_uncertainty.py --no-video
```

| Script | Source | Description |
| --- | --- | --- |
| **Clifford Attractor** | [`benchmarks/python/clifford.py`](benchmarks/python/clifford.py) | 2D Clifford chaotic attractor across 1M particles (ping-pong double-buffered zero-copy evaluation + fast 2D density rasterization). |
| **Aizawa Flow** | [`benchmarks/python/aizawa_flow.py`](benchmarks/python/aizawa_flow.py) | 3D Aizawa chaotic flow simulation for 500k particles with in-place Euler integration. |
| **Double Pendulum** | [`benchmarks/python/double_pendulum.py`](benchmarks/python/double_pendulum.py) | Symbolic Lagrangian mechanics ($L = T - V$), Cramer's rule mass-matrix inversion, RK4 integration, and vectorized `LineCollection` rendering. |
| **Gradient Descent** | [`benchmarks/python/gradient_descent.py`](benchmarks/python/gradient_descent.py) | 3D avalanche ensemble rolling down a symbolic `erf`/trig potential surface using exact gradients. |
| **3-DOF Robot Arm IK & Uncertainty** | [`benchmarks/python/robot_arm_uncertainty.py`](benchmarks/python/robot_arm_uncertainty.py) | Symbolic $2\times 3$ Jacobian + correlated first-order covariance propagation ($J\Sigma_\theta J^T$, 11 joint compiled outputs), Damped Least-Squares IK across 50k arms, and live $2\sigma$ uncertainty ellipses. |
| **Shared Helpers** | [`benchmarks/python/bench_common.py`](benchmarks/python/bench_common.py), [`benchmarks/python/video_writer.py`](benchmarks/python/video_writer.py) | Adaptive 3/4-engine dashboards, Symbolica `.env` license loader, speedup tables, and validated NVENC/libx264 `yuv420p` MP4 export. |
