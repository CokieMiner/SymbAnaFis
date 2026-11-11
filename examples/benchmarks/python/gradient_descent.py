"""Gradient Descent Benchmark and 3D Avalanche Dashboard.

Simulates an ensemble of particles rolling down a complex terrain surface
using gradient descent with adaptive step sizing: step = lr / (1 + ||grad||).

Compares symbolic differentiation, native compilation, and vectorized batch
evaluation across SymPy, SymEngine (LLVM), Symbolica (SymJIT), and SymbAnaFis
(Native JIT + Rayon).

Terrain:
    h(x, y) = sin(x)*cos(y) + 0.1*(x^2 + y^2)
              - 0.2*cos(1.5*x - 1.0)*sin(2.0*y)
              + 0.2*erf(x/2.0)*erf(y/2.0)
"""

from __future__ import annotations

import argparse
import math
import sys
import time
import warnings
from pathlib import Path
from typing import Any, Callable

import matplotlib.animation as animation
import numpy as np

try:
    from scipy.special import erf
except ImportError:
    erf = np.vectorize(math.erf)

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bench_common import (
    create_dashboard_figure,
    load_symbolica_license,
    plot_stacked_performance_panel,
    print_metric_summary,
    save_benchmark_animation,
)

warnings.filterwarnings("ignore")

# CONFIGURATION
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
        x, y = sp.symbols("x y")
        h = (
            sp.sin(x) * sp.cos(y)
            + 0.1 * (x**2 + y**2)
            - 0.2 * sp.cos(1.5 * x - 1.0) * sp.sin(2.0 * y)
            + 0.2 * sp.erf(x / 2.0) * sp.erf(y / 2.0)
        )

        diff_t0 = time.perf_counter()
        dh_dx, dh_dy = sp.diff(h, x), sp.diff(h, y)
        diff_time = time.perf_counter() - diff_t0

        comp_t0 = time.perf_counter()
        grad_fn = sp.lambdify([x, y], [dh_dx, dh_dy], modules=["numpy", "scipy"], cse=True)
        comp_time = time.perf_counter() - comp_t0
        print(f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            return grad_fn(x_arr, y_arr)

        return evaluate, diff_time, comp_time
    except ImportError:
        print("   SymPy not available, skipping.")
        return None, 0.0, 0.0


def setup_symengine() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        import symengine as se

        print("SymEngine: Setting up...")
        x, y = se.symbols("x y")
        h = (
            se.sin(x) * se.cos(y)
            + 0.1 * (x**2 + y**2)
            - 0.2 * se.cos(1.5 * x - 1.0) * se.sin(2.0 * y)
            + 0.2 * se.erf(x / 2.0) * se.erf(y / 2.0)
        )

        diff_t0 = time.perf_counter()
        dh_dx, dh_dy = h.diff(x), h.diff(y)
        diff_time = time.perf_counter() - diff_t0

        comp_t0 = time.perf_counter()
        grad_fn = se.Lambdify([x, y], [dh_dx, dh_dy], backend="llvm")
        comp_time = time.perf_counter() - comp_t0
        print(f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms (LLVM)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 2), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = x_arr
            inp[:, 1] = y_arr
            out = grad_fn(inp)
            return out[:, 0], out[:, 1]

        return evaluate, diff_time, comp_time
    except Exception as exc:
        print(f"   SymEngine not available ({exc}), skipping.")
        return None, 0.0, 0.0


def setup_symbolica() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        from symbolica import E, Expression, S

        print("Symbolica: Setting up...")
        load_symbolica_license()

        x, y = S("x"), S("y")
        h = E(EXPR_STR)

        diff_t0 = time.perf_counter()
        dh_dx, dh_dy = h.derivative(x), h.derivative(y)
        diff_time = time.perf_counter() - diff_t0

        comp_t0 = time.perf_counter()
        ev = Expression.evaluator_multiple([dh_dx, dh_dy], [x, y], jit_compile=True)
        ev.evaluate(np.zeros((1, 2), dtype=np.float64))
        comp_time = time.perf_counter() - comp_t0
        print(f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms (SymJIT)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 2), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = x_arr
            inp[:, 1] = y_arr
            out = ev.evaluate(inp)
            return out[:, 0], out[:, 1]

        return evaluate, diff_time, comp_time
    except Exception as exc:
        print(f"   Symbolica not available ({exc}), skipping.")
        return None, 0.0, 0.0


def setup_symbanafis(parallel: bool = True) -> tuple[Callable[..., Any] | None, float, float]:
    try:
        from symb_anafis import Context, Evaluator, SimplificationSequence

        mode = "Multi-Output Native JIT + Rayon" if parallel else "Multi-Output Native JIT (1T)"
        print(f"SymbAnaFis ({'Par' if parallel else '1T'}): Setting up...")
        ctx = Context()
        x, y = ctx.symbols("x", "y")
        h = (
            x.sin() * y.cos()
            + 0.1 * (x**2 + y**2)
            - 0.2 * (1.5 * x - 1.0).cos() * (2.0 * y).sin()
            + 0.2 * (x / 2.0).erf() * (y / 2.0).erf()
        )

        diff_t0 = time.perf_counter()
        grad = h.gradient([x, y])
        diff_time = time.perf_counter() - diff_t0

        comp_t0 = time.perf_counter()
        opt = SimplificationSequence.compilation()
        dx_expr = opt.simplify(grad[0]).expression
        dy_expr = opt.simplify(grad[1]).expression
        prog = Evaluator([dx_expr, dy_expr], [x, y], backend="f64", native=True)
        comp_time = time.perf_counter() - comp_t0
        print(
            f"   Diff: {diff_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms ({mode})"
        )

        out_cache: list[tuple[np.ndarray, np.ndarray] | None] = [None]

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            outs = out_cache[0]
            if outs is None or outs[0].shape[0] != n:
                outs = (np.empty(n, dtype=np.float64), np.empty(n, dtype=np.float64))
                out_cache[0] = outs
            prog.evaluate_batch([x_arr, y_arr], out=outs, parallel=parallel)
            return outs

        return evaluate, diff_time, comp_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0, 0.0


def simulate(
    eval_fn: Callable[..., Any],
    x0: np.ndarray,
    y0: np.ndarray,
    steps: int,
    record_history: bool,
) -> tuple[float, np.ndarray | None]:
    n_balls = x0.shape[0]
    vis_count = min(2000, n_balls)
    vis_indices = np.linspace(0, n_balls - 1, vis_count, dtype=int)

    history = np.empty((steps, vis_count, 2), dtype=np.float64) if record_history else None
    x = x0.copy()
    y = y0.copy()

    grad_norm = np.empty(n_balls, dtype=np.float64)
    step_size = np.empty(n_balls, dtype=np.float64)
    tmp = np.empty(n_balls, dtype=np.float64)

    t0 = time.perf_counter()
    for i in range(steps):
        if history is not None:
            history[i, :, 0] = x[vis_indices]
            history[i, :, 1] = y[vis_indices]
        gx, gy = eval_fn(x, y)

        np.hypot(gx, gy, out=grad_norm)
        grad_norm += 1.0
        np.divide(LR, grad_norm, out=step_size)

        np.multiply(gx, step_size, out=tmp)
        x -= tmp
        np.multiply(gy, step_size, out=tmp)
        y -= tmp

    return time.perf_counter() - t0, history


def main() -> None:
    parser = argparse.ArgumentParser(description="Gradient Descent Terrain Benchmark")
    parser.add_argument("--no-video", action="store_true", help="Skip MP4 dashboard generation")
    parser.add_argument("--balls", type=int, default=N_BALLS)
    parser.add_argument("--steps", type=int, default=STEPS)
    args = parser.parse_args()

    n_balls, steps = args.balls, args.steps
    generate_video = GENERATE_VIDEO and not args.no_video

    print("Gradient Descent Terrain Benchmark")
    print(f"Particles: {n_balls:,} | Steps: {steps} | Learning rate: {LR}")
    print("-" * 56)

    rng = np.random.default_rng(42)
    n_a = n_balls // 2
    n_b = n_balls - n_a
    x0 = np.concatenate([rng.normal(-2.5, 0.8, n_a), rng.normal(2.5, 0.8, n_b)])
    y0 = np.concatenate([rng.normal(2.5, 0.8, n_a), rng.normal(-2.5, 0.8, n_b)])

    engines = [
        ("SymPy", setup_sympy, "#60a5fa", True),
        ("SymEngine (LLVM)", setup_symengine, "#34d399", True),
        ("Symbolica (JIT)", setup_symbolica, "#c084fc", True),
        ("SymbAnaFis (JIT 1T)", lambda: setup_symbanafis(parallel=False), "#f97316", False),
        ("SymbAnaFis (JIT+Par)", lambda: setup_symbanafis(parallel=True), "#fbbf24", True),
    ]

    results: list[dict[str, Any]] = []
    for name, setup_fn, color, show_panel in engines:
        try:
            eval_fn, dt, ct = setup_fn()
            if eval_fn is None:
                continue

            eval_fn(x0[:128], y0[:128])

            print(f"Running simulation with {name}...")
            run_time, hist = simulate(eval_fn, x0, y0, steps, generate_video and show_panel)
            print(f"   Execution time: {run_time:.4f}s")

            results.append({
                "name": name,
                "diff_time": dt,
                "comp_time": ct,
                "prep_time": dt + ct,
                "run_time": run_time,
                "history": hist,
                "color": color,
                "show_panel": show_panel,
            })
        except Exception as exc:
            print(f"   {name} failed: {exc}")

    if not results:
        print("No engines completed successfully.")
        return

    if REQUIRE_ALL_ENGINES and len(results) != len(engines):
        raise RuntimeError("Not all engines completed. Aborting to keep benchmark fair.")

    print_metric_summary(results)

    if not generate_video:
        print("Video generation disabled.")
        return

    sim_results = [r for r in results if r["show_panel"]]
    print("\nGenerating Multi-Engine 3D Dashboard Video...")
    fig, sim_slots, perf_slot = create_dashboard_figure(
        f"Gradient Descent Benchmark: {n_balls:,} Particles", n_engines=len(sim_results)
    )
    subplots = []

    grid = np.linspace(-LIMIT, LIMIT, 50)
    XX, YY = np.meshgrid(grid, grid)
    ZZ = terrain_z(XX, YY)

    for slot, res in zip(sim_slots, sim_results):
        ax = fig.add_subplot(slot, projection="3d", facecolor="#0f172a")
        ax.set_facecolor("#0f172a")
        ax.plot_surface(
            XX,
            YY,
            ZZ,
            cmap="viridis",
            alpha=0.32,
            rstride=2,
            cstride=2,
            linewidth=0.25,
            edgecolor=(0.0, 1.0, 1.0, 0.08),
            antialiased=False,
        )

        hist = res["history"]
        pts_x = hist[0, :, 0]
        pts_y = hist[0, :, 1]
        pts_z = terrain_z(pts_x, pts_y) + 0.1

        scat = ax.scatter(pts_x, pts_y, pts_z, c=res["color"], s=10, alpha=0.8, depthshade=False)

        ax.set_xlim(-LIMIT, LIMIT)
        ax.set_ylim(-LIMIT, LIMIT)
        ax.set_zlim(-3, 5)
        ax.set_axis_off()
        ax.view_init(elev=70, azim=-45)
        ax.set_title(res["name"], color=res["color"], fontsize=13, fontweight="bold")
        subplots.append({"ax": ax, "scat": scat, "hist": hist})

    ax_perf = fig.add_subplot(perf_slot, facecolor="#1e293b")
    plot_stacked_performance_panel(ax_perf, results, title="Prep + Run Time")

    def update(frame: int) -> list[Any]:
        idx = min(frame, steps - 1)
        for sub in subplots:
            hx = sub["hist"][idx, :, 0]
            hy = sub["hist"][idx, :, 1]
            hz = terrain_z(hx, hy) + 0.1
            sub["scat"]._offsets3d = (hx, hy, hz)
            sub["ax"].view_init(elev=70, azim=-45 + frame * 0.1)
        return [s["scat"] for s in subplots]

    ani = animation.FuncAnimation(fig, update, frames=steps, interval=30, blit=False)
    save_benchmark_animation(ani, "gradient_descent_quad.mp4", fps=30, dpi=100, cq=24)


if __name__ == "__main__":
    main()
