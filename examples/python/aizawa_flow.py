"""Aizawa Attractor Benchmark and 3D Flow Dashboard.

Simulates 3D particle trajectories flowing through the chaotic Aizawa attractor,
comparing evaluation speed across SymPy, SymEngine (when available), and SymbAnaFis.

System:
    dx/dt = (z - B) * x - D * y
    dy/dt = D * x + (z - B) * y
    dz/dt = C + A * z - z^3 / 3 - (x^2 + y^2) * (1 + E * z) + F * z * x^3
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
N_PARTICLES = 500_000
N_STEPS = 600
DT = 0.01
GENERATE_VIDEO = True
REQUIRE_ALL_ENGINES = False

# Aizawa parameters
A, B, C, D, E, F = 0.95, 0.7, 0.6, 3.5, 0.25, 0.1


def setup_sympy() -> tuple[Callable[..., Any] | None, float]:
    try:
        import sympy as sp
        print("SymPy: Setting up...")
        t0 = time.perf_counter()

        x, y, z = sp.symbols("x y z")
        dx = (z - B) * x - D * y
        dy = D * x + (z - B) * y
        dz = C + A * z - z**3 / 3 - (x**2 + y**2) * (1 + E * z) + F * z * x**3

        func = sp.lambdify([x, y, z], [dx, dy, dz], "numpy", cse=True)
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
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

        syms = se.symbols("x y z")
        x, y, z = syms[0], syms[1], syms[2]
        dx = (z - B) * x - D * y
        dy = D * x + (z - B) * y
        dz = C + A * z - z**3 / 3 - (x**2 + y**2) * (1 + E * z) + F * z * x**3

        func = se.Lambdify([x, y, z], [dx, dy, dz], backend="llvm")
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms (LLVM backend)")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
            inp = np.column_stack((x_arr, y_arr, z_arr))
            out = func(inp)
            return out[:, 0], out[:, 1], out[:, 2]

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
        x, y, z = ctx.symbols("x", "y", "z")

        ex_dx = ctx.parse(f"(z - {B}) * x - {D} * y")
        ex_dy = ctx.parse(f"{D} * x + (z - {B}) * y")
        ex_dz = ctx.parse(f"{C} + {A}*z - z^3/3 - (x^2 + y^2) * (1 + {E}*z) + {F} * z * x^3")

        prog_dx = ex_dx.compile([x, y, z], backend="f64")
        prog_dy = ex_dy.compile([x, y, z], backend="f64")
        prog_dz = ex_dz.compile([x, y, z], backend="f64")

        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms")

        def evaluate(x_arr: np.ndarray, y_arr: np.ndarray, z_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
            cols = [x_arr, y_arr, z_arr]
            rx = np.asarray(prog_dx.evaluate_batch(cols), dtype=np.float64)
            ry = np.asarray(prog_dy.evaluate_batch(cols), dtype=np.float64)
            rz = np.asarray(prog_dz.evaluate_batch(cols), dtype=np.float64)
            return rx, ry, rz

        return evaluate, setup_time
    except ImportError as e:
        print(f"   SymbAnaFis import error: {e}")
        return None, 0.0


def main() -> None:
    print("Aizawa Attractor Benchmark")
    print(f"Particles: {N_PARTICLES:,} | Steps: {N_STEPS} | dt: {DT}")
    print("-" * 56)

    x0 = np.random.normal(0.0, 0.1, N_PARTICLES)
    y0 = np.random.normal(0.0, 0.1, N_PARTICLES)
    z0 = np.random.normal(0.0, 0.5, N_PARTICLES)

    engines = [
        ("SymPy", setup_sympy, "#60a5fa"),
        ("SymEngine", setup_symengine, "#34d399"),
        ("SymbAnaFis", setup_symbanafis, "#fbbf24"),
    ]

    results: list[dict[str, Any]] = []
    vis_count = 10_000
    vis_idx = np.linspace(0, N_PARTICLES - 1, vis_count, dtype=int)

    for name, setup, color in engines:
        fn, s_time = setup()
        if not fn:
            continue

        try:
            fn(x0[:100], y0[:100], z0[:100])
        except Exception:
            continue

        print(f"Running {name}...")
        x, y, z = x0.copy(), y0.copy(), z0.copy()

        hx = np.zeros((N_STEPS, vis_count))
        hy = np.zeros((N_STEPS, vis_count))
        hz = np.zeros((N_STEPS, vis_count))

        t0 = time.perf_counter()
        dt = DT
        for i in range(N_STEPS):
            hx[i], hy[i], hz[i] = x[vis_idx], y[vis_idx], z[vis_idx]
            dx, dy, dz = fn(x, y, z)
            x += dx * dt
            y += dy * dt
            z += dz * dt

        run_time = time.perf_counter() - t0
        print(f"   Execution time: {run_time:.4f}s")

        results.append({
            "name": name,
            "prep_time": s_time,
            "run_time": run_time,
            "color": color,
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

    if not GENERATE_VIDEO:
        print("Video generation disabled (GENERATE_VIDEO=False).")
        return

    print("\nGenerating Quad-View Dashboard Video...")
    fig, gs = create_dashboard_figure(f"Aizawa Attractor: {N_PARTICLES:,} Particles")
    axes_map = {0: gs[0, 0], 1: gs[0, 1], 2: gs[1, 0]}
    subplots = []

    for i, res in enumerate(results[:3]):
        ax = fig.add_subplot(axes_map[i], projection="3d", facecolor="#0f172a")
        ax.set_facecolor("#0f172a")
        ax.set_xlim(-2, 2)
        ax.set_ylim(-2, 2)
        ax.set_zlim(-1, 3)
        ax.set_axis_off()
        ax.set_title(res["name"], color=res["color"], fontsize=14, fontweight="bold")

        scat = ax.scatter([], [], [], s=1, c=res["color"], alpha=0.1)
        subplots.append({"scat": scat, "hx": res["hx"], "hy": res["hy"], "hz": res["hz"]})

    ax_perf = fig.add_subplot(gs[1, 1], facecolor="#1e293b")
    plot_performance_panel(ax_perf, results, title="Performance")

    def update(frame: int) -> list[Any]:
        frame_idx = min(frame, N_STEPS - 1)
        for sub in subplots:
            sub_x = sub["hx"][frame_idx]
            sub_y = sub["hy"][frame_idx]
            sub_z = sub["hz"][frame_idx]
            sub["scat"]._offsets3d = (sub_x, sub_y, sub_z)
        return []

    ani = animation.FuncAnimation(fig, update, frames=N_STEPS, interval=40, blit=False)
    save_benchmark_animation(ani, "aizawa_quad.mp4", fps=30, dpi=100, cq=26)


if __name__ == "__main__":
    main()
