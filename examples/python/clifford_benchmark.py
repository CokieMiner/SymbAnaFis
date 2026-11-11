"""Clifford Attractor Benchmark and Quad-View Dashboard.

Compares numerical simulation of the 2D Clifford chaotic attractor
across symbolic backends (SymPy, SymEngine when available, and SymbAnaFis)
and renders an animated quad-view dashboard video.

Equations:
    x_{n+1} = sin(a * y) + c * cos(a * x)
    y_{n+1} = sin(b * x) + d * cos(b * y)
"""

from __future__ import annotations

import time
import warnings
from typing import Any, Callable

import matplotlib.animation as animation
import numpy as np

from bench_common import (
    create_dashboard_figure,
    plot_performance_panel,
    print_metric_summary,
    save_benchmark_animation,
)

warnings.filterwarnings("ignore")

# CONFIGURATION
N_PARTICLES = 1_000_000
N_STEPS = 50
INTERP_FRAMES = 20
A, B, C, D = -1.4, 1.6, 1.0, 0.7
GENERATE_VIDEO = True
REQUIRE_ALL_ENGINES = False


def setup_sympy() -> tuple[Callable[..., Any] | None, float]:
    try:
        import sympy as sp
        print("SymPy: Setting up...")
        t0 = time.perf_counter()

        x, y, a, b, c, d = sp.symbols("x y a b c d")
        eq_x = sp.sin(a * y) + c * sp.cos(a * x)
        eq_y = sp.sin(b * x) + d * sp.cos(b * y)

        subs = {a: A, b: B, c: C, d: D}
        eq_x = eq_x.subs(subs)
        eq_y = eq_y.subs(subs)

        func = sp.lambdify([x, y], [eq_x, eq_y], "numpy")
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            return func(x_arr, y_arr)

        return evaluate, setup_time
    except ImportError:
        print("   SymPy not available, skipping.")
        return None, 0.0


def setup_symengine() -> tuple[Callable[..., Any] | None, float]:
    try:
        import symengine as se
        print("SymEngine: Setting up...")
        t0 = time.perf_counter()

        syms = se.symbols("x y a b c d")
        x, y, a, b, c, d = syms[0], syms[1], syms[2], syms[3], syms[4], syms[5]
        eq_x = se.sin(a * y) + c * se.cos(a * x)
        eq_y = se.sin(b * x) + d * se.cos(b * y)

        subs = {a: A, b: B, c: C, d: D}
        eq_x = eq_x.subs(subs)
        eq_y = eq_y.subs(subs)

        func = se.Lambdify([x, y], [eq_x, eq_y], backend="llvm")
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms (LLVM backend)")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            inp = np.column_stack((x_arr, y_arr))
            out = func(inp)
            return out[:, 0], out[:, 1]

        return evaluate, setup_time
    except Exception as exc:
        print(f"   SymEngine not available or LLVM backend failed ({exc}), skipping.")
        return None, 0.0


def setup_symbanafis() -> tuple[Callable[..., Any] | None, float]:
    try:
        from symb_anafis import Context
        print("SymbAnaFis: Setting up...")
        t0 = time.perf_counter()

        ctx = Context()
        x, y = ctx.symbols("x", "y")
        e_x = ctx.parse(f"sin({A}*y) + {C}*cos({A}*x)")
        e_y = ctx.parse(f"sin({B}*x) + {D}*cos({B}*y)")

        prog_x = e_x.compile([x, y], backend="f64")
        prog_y = e_y.compile([x, y], backend="f64")

        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.3f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            cols = [x_arr, y_arr]
            rx = np.asarray(prog_x.evaluate_batch(cols), dtype=np.float64)
            ry = np.asarray(prog_y.evaluate_batch(cols), dtype=np.float64)
            return rx, ry

        return evaluate, setup_time
    except ImportError as e:
        print(f"   SymbAnaFis import error: {e}")
        return None, 0.0


def main() -> None:
    print("Clifford Attractor Benchmark")
    print(f"Particles: {N_PARTICLES:,} | Steps: {N_STEPS}")
    print("-" * 56)

    initial_x = np.random.uniform(-2, 2, N_PARTICLES)
    initial_y = np.random.uniform(-2, 2, N_PARTICLES)

    engines = [
        ("SymPy", setup_sympy, "#60a5fa"),
        ("SymEngine", setup_symengine, "#34d399"),
        ("SymbAnaFis", setup_symbanafis, "#fbbf24"),
    ]

    results: list[dict[str, Any]] = []
    vis_count = 100_000
    step_idx = np.linspace(0, N_PARTICLES - 1, vis_count, dtype=int)

    for name, setup, color in engines:
        fn, s_time = setup()
        if not fn:
            continue

        try:
            fn(initial_x[:100], initial_y[:100])
        except Exception:
            continue

        print(f"Running {name}...")
        x = initial_x.copy()
        y = initial_y.copy()

        history_x = np.zeros((N_STEPS, vis_count))
        history_y = np.zeros((N_STEPS, vis_count))

        start = time.perf_counter()
        for i in range(N_STEPS):
            history_x[i] = x[step_idx]
            history_y[i] = y[step_idx]
            x, y = fn(x, y)

        run_time = time.perf_counter() - start
        print(f"   Execution time: {run_time:.4f}s")

        results.append({
            "name": name,
            "prep_time": s_time,
            "run_time": run_time,
            "color": color,
            "hist_x": history_x,
            "hist_y": history_y,
        })

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
    fig, gs = create_dashboard_figure(f"Clifford Attractor: {N_PARTICLES:,} Particles")
    axes_map = {0: gs[0, 0], 1: gs[0, 1], 2: gs[1, 0]}
    subplots = []

    for i, res in enumerate(results[:3]):
        ax = fig.add_subplot(axes_map[i], facecolor="#0f172a")
        ax.set_xlim(-2.5, 2.5)
        ax.set_ylim(-2.5, 2.5)
        ax.axis("off")
        ax.set_title(res["name"], color=res["color"], fontsize=14, fontweight="bold")

        scat = ax.scatter([], [], s=0.2, c=res["color"], alpha=0.6)
        subplots.append({"scat": scat, "hx": res["hist_x"], "hy": res["hist_y"]})

    ax_perf = fig.add_subplot(gs[1, 1], facecolor="#1e293b")
    plot_performance_panel(ax_perf, results, title="Calculation Time")

    total_frames = (N_STEPS - 1) * INTERP_FRAMES

    def update(frame: int) -> list[Any]:
        curr_idx = frame // INTERP_FRAMES
        alpha = (frame % INTERP_FRAMES) / INTERP_FRAMES
        next_idx = curr_idx + 1
        if next_idx >= N_STEPS:
            next_idx = curr_idx
            alpha = 0.0

        for sub in subplots:
            x0, y0 = sub["hx"][curr_idx], sub["hy"][curr_idx]
            x1, y1 = sub["hx"][next_idx], sub["hy"][next_idx]
            x_vis = x0 * (1.0 - alpha) + x1 * alpha
            y_vis = y0 * (1.0 - alpha) + y1 * alpha
            sub["scat"].set_offsets(np.column_stack((x_vis, y_vis)))
            sub["scat"].set_alpha(0.1 + 0.8 * (curr_idx / N_STEPS))
        return []

    ani = animation.FuncAnimation(fig, update, frames=total_frames, interval=30, blit=False)
    save_benchmark_animation(ani, "clifford_quad.mp4", fps=30, dpi=100, cq=26)


if __name__ == "__main__":
    main()
