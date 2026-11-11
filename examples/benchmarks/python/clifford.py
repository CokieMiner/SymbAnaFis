"""Clifford Attractor Benchmark and Multi-Engine Dashboard.

Compares numerical simulation of the 2D Clifford chaotic attractor across
SymPy, SymEngine (LLVM), Symbolica (SymJIT), and SymbAnaFis (Native JIT + Rayon),
and renders a fast histogram-rasterized MP4 dashboard video.

Equations:
    x_{n+1} = sin(a * y) + c * cos(a * x)
    y_{n+1} = sin(b * x) + d * cos(b * y)
"""

from __future__ import annotations

import argparse
import sys
import time
import warnings
from pathlib import Path
from typing import Any, Callable

import matplotlib.animation as animation
from matplotlib.colors import LinearSegmentedColormap
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bench_common import (
    create_dashboard_figure,
    load_symbolica_license,
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
HIST_BINS = 420
AXIS_RANGE = (-2.5, 2.5)


def setup_sympy() -> tuple[Callable[..., Any] | None, float]:
    try:
        import sympy as sp

        print("SymPy: Setting up...")
        t0 = time.perf_counter()

        x, y, a, b, c, d = sp.symbols("x y a b c d")
        subs = {a: A, b: B, c: C, d: D}
        eq_x = (sp.sin(a * y) + c * sp.cos(a * x)).subs(subs)
        eq_y = (sp.sin(b * x) + d * sp.cos(b * y)).subs(subs)

        func = sp.lambdify([x, y], [eq_x, eq_y], "numpy", cse=True)
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms")

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

        x, y, a, b, c, d = se.symbols("x y a b c d")
        subs = {a: A, b: B, c: C, d: D}
        eq_x = (se.sin(a * y) + c * se.cos(a * x)).subs(subs)
        eq_y = (se.sin(b * x) + d * se.cos(b * y)).subs(subs)

        func = se.Lambdify([x, y], [eq_x, eq_y], backend="llvm")
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms (LLVM)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 2), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = x_arr
            inp[:, 1] = y_arr
            out = func(inp)
            return out[:, 0], out[:, 1]

        return evaluate, setup_time
    except Exception as exc:
        print(f"   SymEngine not available ({exc}), skipping.")
        return None, 0.0


def setup_symbolica() -> tuple[Callable[..., Any] | None, float]:
    try:
        from symbolica import E, Expression, S

        print("Symbolica: Setting up...")
        load_symbolica_license()
        t0 = time.perf_counter()

        x, y = S("x"), S("y")
        eq_x = E(f"sin({A}*y) + {C}*cos({A}*x)")
        eq_y = E(f"sin({B}*x) + {D}*cos({B}*y)")

        ev = Expression.evaluator_multiple([eq_x, eq_y], [x, y], jit_compile=True)
        # Trigger lazy SymJIT compilation during setup so it is fairly timed in prep_time.
        ev.evaluate(np.zeros((1, 2), dtype=np.float64))
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms (SymJIT)")

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

        return evaluate, setup_time
    except Exception as exc:
        print(f"   Symbolica not available ({exc}), skipping.")
        return None, 0.0


def setup_symbanafis(parallel: bool = True) -> tuple[Callable[..., Any] | None, float]:
    try:
        from symb_anafis import Context, Evaluator, SimplificationSequence

        mode = "Multi-Output Native JIT + Rayon" if parallel else "Multi-Output Native JIT (1T)"
        print(f"SymbAnaFis ({'Par' if parallel else '1T'}): Setting up...")
        t0 = time.perf_counter()

        ctx = Context()
        x, y = ctx.symbols("x", "y")
        opt = SimplificationSequence.compilation()
        e_x = opt.simplify((A * y).sin() + C * (A * x).cos()).expression
        e_y = opt.simplify((B * x).sin() + D * (B * y).cos()).expression

        prog = Evaluator([e_x, e_y], [x, y], backend="f64", native=True)
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.3f} ms ({mode})")

        # Ping-pong double buffers eliminate all per-step allocations cleanly.
        buf_cache: list[tuple[tuple[np.ndarray, np.ndarray], tuple[np.ndarray, np.ndarray]] | None] = [
            None
        ]
        flip = [0]

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            bufs = buf_cache[0]
            if bufs is None or bufs[0][0].shape[0] != n:
                bufs = (
                    (np.empty(n, dtype=np.float64), np.empty(n, dtype=np.float64)),
                    (np.empty(n, dtype=np.float64), np.empty(n, dtype=np.float64)),
                )
                buf_cache[0] = bufs
            slot = flip[0]
            flip[0] = 1 - slot
            out_pair = bufs[slot]
            prog.evaluate_batch([x_arr, y_arr], out=out_pair, parallel=parallel)
            return out_pair

        return evaluate, setup_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0


def _dark_glow_cmap(name: str, deep: str, mid: str, bright: str) -> LinearSegmentedColormap:
    """Create a dark-background density colormap matching the dashboard slate theme."""
    return LinearSegmentedColormap.from_list(name, ["#0b1120", deep, mid, bright])


def _rasterize_density(x: np.ndarray, y: np.ndarray) -> np.ndarray:
    hist, _, _ = np.histogram2d(
        y, x, bins=HIST_BINS, range=[AXIS_RANGE, AXIS_RANGE]
    )
    return np.log1p(hist)


def main() -> None:
    parser = argparse.ArgumentParser(description="Clifford Attractor Benchmark")
    parser.add_argument("--no-video", action="store_true", help="Skip MP4 dashboard generation")
    parser.add_argument("--particles", type=int, default=N_PARTICLES)
    parser.add_argument("--steps", type=int, default=N_STEPS)
    args = parser.parse_args()

    n_particles, n_steps = args.particles, args.steps
    generate_video = GENERATE_VIDEO and not args.no_video

    print("Clifford Attractor Benchmark")
    print(f"Particles: {n_particles:,} | Steps: {n_steps}")
    print("-" * 56)

    rng = np.random.default_rng(42)
    initial_x = rng.uniform(-2.0, 2.0, n_particles)
    initial_y = rng.uniform(-2.0, 2.0, n_particles)

    engines = [
        ("SymPy", setup_sympy, "#60a5fa", _dark_glow_cmap("sympy_dark", "#1d4ed8", "#60a5fa", "#e0f2fe"), True),
        ("SymEngine (LLVM)", setup_symengine, "#34d399", _dark_glow_cmap("symengine_dark", "#047857", "#34d399", "#d1fae5"), True),
        ("Symbolica (JIT)", setup_symbolica, "#c084fc", _dark_glow_cmap("symbolica_dark", "#6d28d9", "#c084fc", "#f3e8ff"), True),
        ("SymbAnaFis (JIT 1T)", lambda: setup_symbanafis(parallel=False), "#f97316", None, False),
        ("SymbAnaFis (JIT+Par)", lambda: setup_symbanafis(parallel=True), "#fbbf24", _dark_glow_cmap("symbanafis_dark", "#b45309", "#fbbf24", "#fef9c3"), True),
    ]

    results: list[dict[str, Any]] = []
    vis_count = min(100_000, n_particles)
    step_idx = np.linspace(0, n_particles - 1, vis_count, dtype=int)

    for name, setup, color, cmap, show_panel in engines:
        fn, s_time = setup()
        if not fn:
            continue

        # Warm up lazy SIMD kernels before timing.
        fn(initial_x[:128], initial_y[:128])

        print(f"Running {name}...")
        x = initial_x.copy()
        y = initial_y.copy()

        record = generate_video and show_panel
        history_x = np.empty((n_steps, vis_count), dtype=np.float64) if record else None
        history_y = np.empty((n_steps, vis_count), dtype=np.float64) if record else None

        start = time.perf_counter()
        for i in range(n_steps):
            if history_x is not None and history_y is not None:
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
            "cmap": cmap,
            "show_panel": show_panel,
            "hist_x": history_x,
            "hist_y": history_y,
        })

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
    print("\nGenerating Multi-Engine Dashboard Video (fast 2D density raster)...")
    fig, sim_slots, perf_slot = create_dashboard_figure(
        f"Clifford Attractor: {n_particles:,} Particles", n_engines=len(sim_results)
    )
    subplots = []

    for slot, res in zip(sim_slots, sim_results):
        ax = fig.add_subplot(slot, facecolor="#0b1120")
        ax.set_xticks([])
        ax.set_yticks([])
        for spine in ax.spines.values():
            spine.set_color("#1e293b")
            spine.set_linewidth(1.2)
        ax.set_title(res["name"], color=res["color"], fontsize=13, fontweight="bold", pad=8)
        init_img = _rasterize_density(res["hist_x"][0], res["hist_y"][0])
        im = ax.imshow(
            init_img,
            origin="lower",
            extent=[AXIS_RANGE[0], AXIS_RANGE[1], AXIS_RANGE[0], AXIS_RANGE[1]],
            cmap=res["cmap"],
            interpolation="bilinear",
            vmin=0.0,
            vmax=max(float(init_img.max()), 3.5),
        )
        subplots.append({"im": im, "hx": res["hist_x"], "hy": res["hist_y"]})

    ax_perf = fig.add_subplot(perf_slot, facecolor="#1e293b")
    plot_performance_panel(ax_perf, results, title="Simulation Time")

    total_frames = max(1, (n_steps - 1) * INTERP_FRAMES)

    def update(frame: int) -> list[Any]:
        curr_idx = min(frame // INTERP_FRAMES, n_steps - 1)
        alpha = (frame % INTERP_FRAMES) / INTERP_FRAMES
        next_idx = min(curr_idx + 1, n_steps - 1)
        artists = []
        for sub in subplots:
            x0, y0 = sub["hx"][curr_idx], sub["hy"][curr_idx]
            x1, y1 = sub["hx"][next_idx], sub["hy"][next_idx]
            x_vis = x0 + alpha * (x1 - x0)
            y_vis = y0 + alpha * (y1 - y0)
            sub["im"].set_data(_rasterize_density(x_vis, y_vis))
            artists.append(sub["im"])
        return artists

    ani = animation.FuncAnimation(fig, update, frames=total_frames, interval=30, blit=True)
    save_benchmark_animation(ani, "clifford_quad.mp4", fps=30, dpi=100, cq=24)


if __name__ == "__main__":
    main()
