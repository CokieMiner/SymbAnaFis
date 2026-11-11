"""Aizawa Attractor Benchmark and 3D Flow Dashboard.

Simulates 3D particle trajectories flowing through the chaotic Aizawa attractor,
comparing evaluation speed across SymPy, SymEngine (LLVM), Symbolica (SymJIT),
and SymbAnaFis (Native JIT + Rayon).

System:
    dx/dt = (z - B) * x - D * y
    dy/dt = D * x + (z - B) * y
    dz/dt = C + A * z - z^3 / 3 - (x^2 + y^2) * (1 + E * z) + F * z * x^3
"""

from __future__ import annotations

import argparse
import sys
import time
import warnings
from pathlib import Path
from typing import Any, Callable

import matplotlib.animation as animation
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
N_PARTICLES = 500_000
N_STEPS = 600
DT = 0.01
GENERATE_VIDEO = True
REQUIRE_ALL_ENGINES = False

# Aizawa parameters
A, B, C, D, E_PARAM, F = 0.95, 0.7, 0.6, 3.5, 0.25, 0.1


def setup_sympy() -> tuple[Callable[..., Any] | None, float]:
    try:
        import sympy as sp

        print("SymPy: Setting up...")
        t0 = time.perf_counter()

        x, y, z = sp.symbols("x y z")
        dx = (z - B) * x - D * y
        dy = D * x + (z - B) * y
        dz = C + A * z - z**3 / 3 - (x**2 + y**2) * (1 + E_PARAM * z) + F * z * x**3

        func = sp.lambdify([x, y, z], [dx, dy, dz], "numpy", cse=True)
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms")

        def evaluate(
            x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray
        ) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
            result = func(x_arr, y_arr, z_arr)
            return result[0], result[1], result[2]

        return evaluate, setup_time
    except ImportError:
        print("   SymPy not available, skipping.")
        return None, 0.0


def setup_symengine() -> tuple[Callable[..., Any] | None, float]:
    try:
        import symengine as se

        print("SymEngine: Setting up...")
        t0 = time.perf_counter()

        x, y, z = se.symbols("x y z")
        dx = (z - B) * x - D * y
        dy = D * x + (z - B) * y
        dz = C + A * z - z**3 / 3 - (x**2 + y**2) * (1 + E_PARAM * z) + F * z * x**3

        func = se.Lambdify([x, y, z], [dx, dy, dz], backend="llvm")
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms (LLVM)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(
            x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray
        ) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 3), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = x_arr
            inp[:, 1] = y_arr
            inp[:, 2] = z_arr
            out = func(inp)
            return out[:, 0], out[:, 1], out[:, 2]

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

        x, y, z = S("x"), S("y"), S("z")
        ex_dx = E(f"(z - {B}) * x - {D} * y")
        ex_dy = E(f"{D} * x + (z - {B}) * y")
        ex_dz = E(f"{C} + {A}*z - z^3/3 - (x^2 + y^2) * (1 + {E_PARAM}*z) + {F} * z * x^3")

        ev = Expression.evaluator_multiple([ex_dx, ex_dy, ex_dz], [x, y, z], jit_compile=True)
        ev.evaluate(np.zeros((1, 3), dtype=np.float64))
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms (SymJIT)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(
            x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray
        ) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 3), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = x_arr
            inp[:, 1] = y_arr
            inp[:, 2] = z_arr
            out = ev.evaluate(inp)
            return out[:, 0], out[:, 1], out[:, 2]

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
        x, y, z = ctx.symbols("x", "y", "z")
        opt = SimplificationSequence.compilation()

        ex_dx = opt.simplify((z - B) * x - D * y).expression
        ex_dy = opt.simplify(D * x + (z - B) * y).expression
        ex_dz = opt.simplify(
            C + A * z - z**3 / 3 - (x**2 + y**2) * (1 + E_PARAM * z) + F * z * x**3
        ).expression

        prog = Evaluator([ex_dx, ex_dy, ex_dz], [x, y, z], backend="f64", native=True)

        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.3f} ms ({mode})")

        out_cache: list[tuple[np.ndarray, np.ndarray, np.ndarray] | None] = [None]

        def evaluate(
            x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray
        ) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
            n = x_arr.shape[0]
            outs = out_cache[0]
            if outs is None or outs[0].shape[0] != n:
                outs = (
                    np.empty(n, dtype=np.float64),
                    np.empty(n, dtype=np.float64),
                    np.empty(n, dtype=np.float64),
                )
                out_cache[0] = outs
            prog.evaluate_batch([x_arr, y_arr, z_arr], out=outs, parallel=parallel)
            return outs

        return evaluate, setup_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0


def main() -> None:
    parser = argparse.ArgumentParser(description="Aizawa Attractor Benchmark")
    parser.add_argument("--no-video", action="store_true", help="Skip MP4 dashboard generation")
    parser.add_argument("--particles", type=int, default=N_PARTICLES)
    parser.add_argument("--steps", type=int, default=N_STEPS)
    args = parser.parse_args()

    n_particles, n_steps = args.particles, args.steps
    generate_video = GENERATE_VIDEO and not args.no_video

    print("Aizawa Attractor Benchmark")
    print(f"Particles: {n_particles:,} | Steps: {n_steps} | dt: {DT}")
    print("-" * 56)

    rng = np.random.default_rng(42)
    x0 = rng.normal(0.0, 0.1, n_particles)
    y0 = rng.normal(0.0, 0.1, n_particles)
    z0 = rng.normal(0.0, 0.5, n_particles)

    engines = [
        ("SymPy", setup_sympy, "#60a5fa", True),
        ("SymEngine (LLVM)", setup_symengine, "#34d399", True),
        ("Symbolica (JIT)", setup_symbolica, "#c084fc", True),
        ("SymbAnaFis (JIT 1T)", lambda: setup_symbanafis(parallel=False), "#f97316", False),
        ("SymbAnaFis (JIT+Par)", lambda: setup_symbanafis(parallel=True), "#fbbf24", True),
    ]

    results: list[dict[str, Any]] = []
    vis_count = min(10_000, n_particles)
    vis_idx = np.linspace(0, n_particles - 1, vis_count, dtype=int)
    tmp = np.empty(n_particles, dtype=np.float64)

    for name, setup, color, show_panel in engines:
        fn, s_time = setup()
        if not fn:
            continue

        fn(x0[:128], y0[:128], z0[:128])

        print(f"Running {name}...")
        x, y, z = x0.copy(), y0.copy(), z0.copy()

        record = generate_video and show_panel
        hx = np.empty((n_steps, vis_count), dtype=np.float64) if record else None
        hy = np.empty((n_steps, vis_count), dtype=np.float64) if record else None
        hz = np.empty((n_steps, vis_count), dtype=np.float64) if record else None

        t0 = time.perf_counter()
        dt = DT
        for i in range(n_steps):
            if hx is not None and hy is not None and hz is not None:
                hx[i], hy[i], hz[i] = x[vis_idx], y[vis_idx], z[vis_idx]
            dx, dy, dz = fn(x, y, z)
            np.multiply(dx, dt, out=tmp)
            x += tmp
            np.multiply(dy, dt, out=tmp)
            y += tmp
            np.multiply(dz, dt, out=tmp)
            z += tmp

        run_time = time.perf_counter() - t0
        print(f"   Execution time: {run_time:.4f}s")

        results.append({
            "name": name,
            "prep_time": s_time,
            "run_time": run_time,
            "color": color,
            "show_panel": show_panel,
            "hx": hx,
            "hy": hy,
            "hz": hz,
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
    print("\nGenerating Multi-Engine 3D Dashboard Video...")
    fig, sim_slots, perf_slot = create_dashboard_figure(
        f"Aizawa Attractor: {n_particles:,} Particles", n_engines=len(sim_results)
    )
    subplots = []

    for slot, res in zip(sim_slots, sim_results):
        ax = fig.add_subplot(slot, projection="3d", facecolor="#0f172a")
        ax.set_facecolor("#0f172a")
        ax.set_xlim(-2, 2)
        ax.set_ylim(-2, 2)
        ax.set_zlim(-1, 3)
        ax.set_axis_off()
        ax.set_title(res["name"], color=res["color"], fontsize=13, fontweight="bold")

        scat = ax.scatter([], [], [], s=1, c=res["color"], alpha=0.15, depthshade=False)
        subplots.append({"scat": scat, "hx": res["hx"], "hy": res["hy"], "hz": res["hz"]})

    ax_perf = fig.add_subplot(perf_slot, facecolor="#1e293b")
    plot_performance_panel(ax_perf, results, title="Simulation Time")

    def update(frame: int) -> list[Any]:
        frame_idx = min(frame, n_steps - 1)
        for sub in subplots:
            sub["scat"]._offsets3d = (
                sub["hx"][frame_idx],
                sub["hy"][frame_idx],
                sub["hz"][frame_idx],
            )
        return []

    ani = animation.FuncAnimation(fig, update, frames=n_steps, interval=40, blit=False)
    save_benchmark_animation(ani, "aizawa_quad.mp4", fps=30, dpi=100, cq=24)


if __name__ == "__main__":
    main()
