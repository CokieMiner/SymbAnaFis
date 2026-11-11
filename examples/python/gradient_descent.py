"""Gradient Descent Benchmark and 3D Avalanche Dashboard.

Simulates an ensemble of particles rolling down a complex terrain surface
using gradient descent with adaptive step sizing: step = lr / (1 + ||grad||).

Compares symbolic differentiation, native compilation, and vectorized batch
evaluation across SymPy, SymEngine (when available), and SymbAnaFis.

Terrain:
    h(x, y) = sin(x)*cos(y) + 0.1*(x^2 + y^2)
              - 0.2*cos(1.5*x - 1.0)*sin(2.0*y)
              + 0.2*erf(x/2.0)*erf(y/2.0)
"""

from __future__ import annotations

import time
import warnings
from typing import Any, Callable

import matplotlib.animation as animation
import numpy as np

from bench_common import (
    create_dashboard_figure,
    plot_stacked_performance_panel,
    print_metric_summary,
    save_benchmark_animation,
)

warnings.filterwarnings("ignore")

# CONFIGURATION
# Default scale: 50,000 particles across 300 steps
N_BALLS = 50_000
STEPS = 300
LR = 0.05
LIMIT = 8.0
GENERATE_VIDEO = True
REQUIRE_ALL_ENGINES = False

EXPR_STR = (
    "sin(x) * cos(y) + 0.1 * (x^2 + y^2) "
    "- 0.2 * cos(1.5 * x - 1.0) * sin(2.0 * y) "
    "+ 0.2 * erf(x / 2.0) * erf(y / 2.0)"
)


def terrain_z(x: np.ndarray, y: np.ndarray) -> np.ndarray:
    return (
        np.sin(x) * np.cos(y)
        + 0.1 * (x**2 + y**2)
        - 0.2 * np.cos(1.5 * x - 1.0) * np.sin(2.0 * y)
        + 0.2 * erf(x / 2.0) * erf(y / 2.0)
    )


def setup_sympy() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        import scipy  # noqa: F401
        import sympy as sp
        print("SymPy: Setting up...")
        t0 = time.perf_counter()

        x, y = sp.symbols("x y")
        h = (
            sp.sin(x) * sp.cos(y)
            + 0.1 * (x**2 + y**2)
            - 0.2 * sp.cos(1.5 * x - 1.0) * sp.sin(2.0 * y)
            + 0.2 * sp.erf(x / 2.0) * sp.erf(y / 2.0)
        )

        print("   Differentiating...")
        diff_t0 = time.perf_counter()
        dh_dx = sp.diff(h, x)
        dh_dy = sp.diff(h, y)
        diff_time = time.perf_counter() - diff_t0

        print("   Compiling via lambdify (NumPy + SciPy)...")
        comp_t0 = time.perf_counter()
        grad_fn = sp.lambdify([x, y], [dh_dx, dh_dy], modules=["numpy", "scipy"], cse=True)
        comp_time = time.perf_counter() - comp_t0

        print(f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            dx, dy = grad_fn(x_arr, y_arr)
            return dx, dy

        return evaluate, diff_time, comp_time
    except ImportError:
        print("   SymPy not available, skipping.")
        return None, 0.0, 0.0


def setup_symengine() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        import symengine as se
        print("SymEngine: Setting up...")
        t0 = time.perf_counter()

        syms = se.symbols("x y")
        x, y = syms[0], syms[1]
        h = (
            se.sin(x) * se.cos(y)
            + 0.1 * (x**2 + y**2)
            - 0.2 * se.cos(1.5 * x - 1.0) * se.sin(2.0 * y)
            + 0.2 * se.erf(x / 2.0) * se.erf(y / 2.0)
        )

        print("   Differentiating...")
        diff_t0 = time.perf_counter()
        dh_dx = h.diff(x)
        dh_dy = h.diff(y)
        diff_time = time.perf_counter() - diff_t0

        print("   Compiling via Lambdify (LLVM backend)...")
        comp_t0 = time.perf_counter()
        grad_fn_inner = se.Lambdify([x, y], [dh_dx, dh_dy], backend="llvm")
        comp_time = time.perf_counter() - comp_t0

        print(f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            inp = np.column_stack((x_arr, y_arr))
            out = grad_fn_inner(inp)
            return out[:, 0], out[:, 1]

        return evaluate, diff_time, comp_time
    except Exception as exc:
        print(f"   SymEngine not available or LLVM backend failed ({exc}), skipping.")
        return None, 0.0, 0.0


def setup_symbanafis() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        from symb_anafis import Context
        print("SymbAnaFis: Setting up...")
        t0 = time.perf_counter()

        ctx = Context()
        x, y = ctx.symbols("x", "y")
        h = ctx.parse(EXPR_STR)

        print("   Differentiating...")
        diff_t0 = time.perf_counter()
        grad = h.gradient([x, y])
        diff_time = time.perf_counter() - diff_t0

        print("   Compiling native f64 plans...")
        comp_t0 = time.perf_counter()
        prog_dx = grad[0].compile([x, y], backend="f64")
        prog_dy = grad[1].compile([x, y], backend="f64")
        comp_time = time.perf_counter() - comp_t0

        print(f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            cols = [x_arr, y_arr]
            dx = np.asarray(prog_dx.evaluate_batch(cols), dtype=np.float64)
            dy = np.asarray(prog_dy.evaluate_batch(cols), dtype=np.float64)
            return dx, dy

        return evaluate, diff_time, comp_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0, 0.0


def simulate(eval_fn: Callable[..., Any], x0: np.ndarray, y0: np.ndarray) -> tuple[float, np.ndarray]:
    vis_count = 2000
    step = max(1, N_BALLS // vis_count)
    vis_indices = np.arange(0, N_BALLS, step)[:vis_count]

    history = np.zeros((STEPS, len(vis_indices), 2))
    x = x0.copy()
    y = y0.copy()

    t0 = time.perf_counter()
    for i in range(STEPS):
        history[i] = np.column_stack((x[vis_indices], y[vis_indices]))
        gx, gy = eval_fn(x, y)

        grad_norm = np.sqrt(gx**2 + gy**2)
        step_size = LR / (1.0 + grad_norm)

        x -= gx * step_size
        y -= gy * step_size

    run_time = time.perf_counter() - t0
    return run_time, history


def main() -> None:
    print("Gradient Descent Terrain Benchmark")
    print(f"Particles: {N_BALLS:,} | Steps: {STEPS} | Learning rate: {LR}")
    print("-" * 56)

    np.random.seed(42)
    n_a = N_BALLS // 2
    xa = np.random.normal(-2.5, 0.8, n_a)
    ya = np.random.normal(2.5, 0.8, n_a)
    n_b = N_BALLS - n_a
    xb = np.random.normal(2.5, 0.8, n_b)
    yb = np.random.normal(-2.5, 0.8, n_b)
    x0 = np.concatenate([xa, xb])
    y0 = np.concatenate([ya, yb])

    engines = [
        ("SymPy", setup_sympy, "#60a5fa"),
        ("SymEngine", setup_symengine, "#34d399"),
        ("SymbAnaFis", setup_symbanafis, "#fbbf24"),
    ]

    results: list[dict[str, Any]] = []

    for name, setup_fn, color in engines:
        try:
            eval_fn, dt, ct = setup_fn()
            if eval_fn is None:
                continue

            print(f"Warming up {name}...")
            eval_fn(x0[:100], y0[:100])

            print(f"Running simulation with {name}...")
            run_time, hist = simulate(eval_fn, x0, y0)
            print(f"   Execution time: {run_time:.4f}s")

            results.append({
                "name": name,
                "diff_time": dt,
                "comp_time": ct,
                "prep_time": dt + ct,
                "run_time": run_time,
                "history": hist,
                "color": color,
            })
        except Exception as exc:
            print(f"   {name} failed: {exc}")

    if not results:
        print("No engines completed successfully.")
        return

    if REQUIRE_ALL_ENGINES and len(results) != len(engines):
        raise RuntimeError("Not all engines completed. Aborting to keep benchmark fair.")

    print_metric_summary(results)

    if not GENERATE_VIDEO:
        print("Video generation disabled (GENERATE_VIDEO=False).")
        return

    print("\nGenerating Quad-View Dashboard Video...")
    fig, gs = create_dashboard_figure(f"Gradient Descent Benchmark: {N_BALLS:,} Particles")
    axes_map = {0: gs[0, 0], 1: gs[0, 1], 2: gs[1, 0]}
    subplots = []

    low, high = -LIMIT, LIMIT
    X = np.linspace(low, high, 80)
    Y = np.linspace(low, high, 80)
    XX, YY = np.meshgrid(X, Y)
    ZZ = terrain_z(XX, YY)

    for i, res in enumerate(results[:3]):
        pos = axes_map[i]
        ax = fig.add_subplot(pos, projection="3d", facecolor="#0f172a")
        ax.set_facecolor("#0f172a")

        ax.plot_surface(
            XX, YY, ZZ, cmap="viridis", alpha=0.3,
            rstride=4, cstride=4, linewidth=0, antialiased=False,
        )
        ax.plot_wireframe(XX, YY, ZZ, color="cyan", alpha=0.05, rstride=8, cstride=8)

        hist = res["history"]
        pts_x = hist[0][:, 0]
        pts_y = hist[0][:, 1]
        pts_z = terrain_z(pts_x, pts_y) + 0.1

        scat = ax.scatter(pts_x, pts_y, pts_z, c=res["color"], s=10, alpha=0.8, depthshade=False)

        ax.set_xlim(low, high)
        ax.set_ylim(low, high)
        ax.set_zlim(-3, 5)
        ax.set_axis_off()
        ax.view_init(elev=70, azim=-45)
        ax.set_title(res["name"], color=res["color"], fontsize=14, fontweight="bold")

        subplots.append({"ax": ax, "scat": scat, "hist": hist})

    ax_perf = fig.add_subplot(gs[1, 1], facecolor="#1e293b")
    plot_stacked_performance_panel(ax_perf, results, title="Performance Comparison")

    def update(frame: int) -> list[Any]:
        for sub in subplots:
            hist = sub["hist"]
            scat = sub["scat"]
            idx = min(frame, len(hist) - 1)

            hx = hist[idx][:, 0]
            hy = hist[idx][:, 1]
            hz = terrain_z(hx, hy) + 0.1
            scat._offsets3d = (hx, hy, hz)
            sub["ax"].view_init(elev=70, azim=-45 + frame * 0.1)

        return [s["scat"] for s in subplots]

    ani = animation.FuncAnimation(fig, update, frames=STEPS, interval=30, blit=False)
    save_benchmark_animation(ani, "gradient_descent_quad.mp4", fps=30, dpi=100, cq=24)


if __name__ == "__main__":
    main()
