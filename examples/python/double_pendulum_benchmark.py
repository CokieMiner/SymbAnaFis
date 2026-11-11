"""Double Pendulum Benchmark and Chaotic Dynamics Dashboard.

Derives the equations of motion for an ideal double pendulum symbolically
from the Lagrangian L = T - V, inverts the 2x2 mass matrix symbolically
via Cramer's rule, and integrates trajectories in parallel using RK4.

Compares analytical derivation and numerical evaluation speed across
SymPy, SymEngine (when available), and SymbAnaFis.
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
N_PENDULUMS = 50_000
N_STEPS = 600
DT = 0.01
AXIS_LIMIT = 2.5
GENERATE_VIDEO = True
REQUIRE_ALL_ENGINES = False

# Physical constants
G = 9.81
L1, L2 = 1.0, 1.0
M1, M2 = 1.0, 1.0
DAMPING = 0.9995

# Derivation cache across runs
_DERIVATION_CACHE: dict[str, Any] = {}


def derive_sympy() -> tuple[Any, Any, tuple[Any, ...]]:
    if "sympy" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["sympy"]

    import sympy as sp
    th1, th2, w1, w2 = sp.symbols("th1 th2 w1 w2")
    m1, m2, l1, l2, g = sp.symbols("m1 m2 L1 L2 g")

    x1 = l1 * sp.sin(th1)
    y1 = -l1 * sp.cos(th1)
    x2 = x1 + l2 * sp.sin(th2)
    y2 = y1 - l2 * sp.cos(th2)

    vx1 = l1 * w1 * sp.cos(th1)
    vy1 = l1 * w1 * sp.sin(th1)
    vx2 = vx1 + l2 * w2 * sp.cos(th2)
    vy2 = vy1 + l2 * w2 * sp.sin(th2)

    T = 0.5 * m1 * (vx1**2 + vy1**2) + 0.5 * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1 = sp.diff(L, w1)
    dL_dw2 = sp.diff(L, w2)
    dL_dth1 = sp.diff(L, th1)
    dL_dth2 = sp.diff(L, th2)

    M11 = sp.diff(dL_dw1, w1)
    M12 = sp.diff(dL_dw1, w2)
    M21 = sp.diff(dL_dw2, w1)
    M22 = sp.diff(dL_dw2, w2)

    dt_term1 = sp.diff(dL_dw1, th1) * w1 + sp.diff(dL_dw1, th2) * w2
    dt_term2 = sp.diff(dL_dw2, th1) * w1 + sp.diff(dL_dw2, th2) * w2

    F1 = dL_dth1 - dt_term1
    F2 = dL_dth2 - dt_term2

    det = M11 * M22 - M12 * M21
    a1 = (F1 * M22 - F2 * M12) / det
    a2 = (M11 * F2 - M21 * F1) / det

    subs = {m1: M1, m2: M2, l1: L1, l2: L2, g: G}
    a1_sub = a1.subs(subs)
    a2_sub = a2.subs(subs)

    result = (a1_sub, a2_sub, (th1, th2, w1, w2))
    _DERIVATION_CACHE["sympy"] = result
    return result


def derive_symengine() -> tuple[Any, Any, tuple[Any, ...]]:
    if "symengine" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["symengine"]

    import symengine as se
    syms1 = se.symbols("th1 th2 w1 w2")
    th1, th2, w1, w2 = syms1[0], syms1[1], syms1[2], syms1[3]
    syms2 = se.symbols("m1 m2 L1 L2 g")
    m1, m2, l1, l2, g = syms2[0], syms2[1], syms2[2], syms2[3], syms2[4]

    x1 = l1 * se.sin(th1)
    y1 = -l1 * se.cos(th1)
    x2 = x1 + l2 * se.sin(th2)
    y2 = y1 - l2 * se.cos(th2)

    vx1 = l1 * w1 * se.cos(th1)
    vy1 = l1 * w1 * se.sin(th1)
    vx2 = vx1 + l2 * w2 * se.cos(th2)
    vy2 = vy1 + l2 * w2 * se.sin(th2)

    T = 0.5 * m1 * (vx1**2 + vy1**2) + 0.5 * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1 = se.diff(L, w1)
    dL_dw2 = se.diff(L, w2)
    dL_dth1 = se.diff(L, th1)
    dL_dth2 = se.diff(L, th2)

    M11 = se.diff(dL_dw1, w1)
    M12 = se.diff(dL_dw1, w2)
    M21 = se.diff(dL_dw2, w1)
    M22 = se.diff(dL_dw2, w2)

    dt_term1 = se.diff(dL_dw1, th1) * w1 + se.diff(dL_dw1, th2) * w2
    dt_term2 = se.diff(dL_dw2, th1) * w1 + se.diff(dL_dw2, th2) * w2

    F1 = dL_dth1 - dt_term1
    F2 = dL_dth2 - dt_term2

    det = M11 * M22 - M12 * M21
    a1 = (F1 * M22 - F2 * M12) / det
    a2 = (M11 * F2 - M21 * F1) / det

    subs = {m1: M1, m2: M2, l1: L1, l2: L2, g: G}
    a1_sub = a1.subs(subs)
    a2_sub = a2.subs(subs)

    result = (a1_sub, a2_sub, (th1, th2, w1, w2))
    _DERIVATION_CACHE["symengine"] = result
    return result


def derive_symbanafis() -> tuple[Any, Any, Any]:
    if "symbanafis" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["symbanafis"]

    from fractions import Fraction
    from symb_anafis import Context, Expr, Substitutions

    ctx = Context()
    th1, th2, w1, w2 = ctx.symbols("th1", "th2", "w1", "w2")
    m1, m2, l1, l2, g = ctx.symbols("m1", "m2", "L1", "L2", "g")

    x1 = l1 * th1.sin()
    y1 = -l1 * th1.cos()
    x2 = x1 + l2 * th2.sin()
    y2 = y1 - l2 * th2.cos()

    vx1 = l1 * w1 * th1.cos()
    vy1 = l1 * w1 * th1.sin()
    vx2 = vx1 + l2 * w2 * th2.cos()
    vy2 = vy1 + l2 * w2 * th2.sin()

    half = Expr(Fraction(1, 2))
    T = half * m1 * (vx1**2 + vy1**2) + half * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1 = L.derivative(w1)
    dL_dw2 = L.derivative(w2)
    dL_dth1 = L.derivative(th1)
    dL_dth2 = L.derivative(th2)

    M11 = dL_dw1.derivative(w1).compact()
    M12 = dL_dw1.derivative(w2).compact()
    M21 = dL_dw2.derivative(w1).compact()
    M22 = dL_dw2.derivative(w2).compact()

    dt_term1 = (dL_dw1.derivative(th1) * w1 + dL_dw1.derivative(th2) * w2).compact()
    dt_term2 = (dL_dw2.derivative(th1) * w1 + dL_dw2.derivative(th2) * w2).compact()

    F1 = (dL_dth1 - dt_term1).compact()
    F2 = (dL_dth2 - dt_term2).compact()

    det = (M11 * M22 - M12 * M21).compact()
    a1 = ((F1 * M22 - F2 * M12) / det).compact()
    a2 = ((M11 * F2 - M21 * F1) / det).compact()

    subs = Substitutions({m1: M1, m2: M2, l1: L1, l2: L2, g: G})
    a1_sub = subs.apply(a1)
    a2_sub = subs.apply(a2)

    result = (a1_sub, a2_sub, (th1, th2, w1, w2))
    _DERIVATION_CACHE["symbanafis"] = result
    return result


def setup_sympy() -> tuple[Callable[..., Any] | None, float]:
    try:
        import sympy as sp
        print("SymPy: Setting up...")
        t0 = time.perf_counter()
        a1, a2, vars_sym = derive_sympy()

        func = sp.lambdify(vars_sym, [a1, a2], "numpy", cse=True)
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms")

        def evaluate(s: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            res = func(s[0], s[1], s[2], s[3])
            return res[0], res[1]

        return evaluate, setup_time
    except ImportError:
        print("   SymPy not available, skipping.")
        return None, 0.0


def setup_symengine() -> tuple[Callable[..., Any] | None, float]:
    try:
        import symengine as se
        print("SymEngine: Setting up...")
        t0 = time.perf_counter()
        a1, a2, vars_sym = derive_symengine()

        func = se.Lambdify(vars_sym, [a1, a2], backend="llvm")
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms (LLVM backend)")

        def evaluate(s: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            inp = np.ascontiguousarray(s.T)
            out = func(inp)
            return out[:, 0], out[:, 1]

        return evaluate, setup_time
    except Exception as exc:
        print(f"   SymEngine not available or LLVM backend failed ({exc}), skipping.")
        return None, 0.0


def setup_symbanafis() -> tuple[Callable[..., Any] | None, float]:
    try:
        print("SymbAnaFis: Setting up...")
        t0 = time.perf_counter()
        a1, a2, vars_sym = derive_symbanafis()

        prog_a1 = a1.compile(list(vars_sym), backend="f64")
        prog_a2 = a2.compile(list(vars_sym), backend="f64")

        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.1f} ms")

        def evaluate(s: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            cols = [s[0], s[1], s[2], s[3]]
            r1 = np.asarray(prog_a1.evaluate_batch(cols), dtype=np.float64)
            r2 = np.asarray(prog_a2.evaluate_batch(cols), dtype=np.float64)
            return r1, r2

        return evaluate, setup_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0


def simulate(eval_fn: Callable[..., Any], initial_states: np.ndarray) -> tuple[float, np.ndarray]:
    vis_count = 50
    vis_idx = np.linspace(0, N_PENDULUMS - 1, vis_count, dtype=int)

    states = initial_states.copy()
    history = np.zeros((N_STEPS, 4, vis_count))

    dt = DT
    dt_half = dt * 0.5
    dt_sixth = dt / 6.0

    k1 = np.zeros_like(states)
    k2 = np.zeros_like(states)
    k3 = np.zeros_like(states)
    k4 = np.zeros_like(states)
    temp = np.zeros_like(states)
    acc = np.zeros_like(states)

    t0 = time.perf_counter()

    for i in range(N_STEPS):
        history[i] = states[:, vis_idx]

        # RK4 Step 1
        a1, a2 = eval_fn(states)
        k1[0], k1[1] = states[2], states[3]
        k1[2], k1[3] = a1, a2

        # RK4 Step 2
        np.multiply(k1, dt_half, out=acc)
        np.add(states, acc, out=temp)
        a1, a2 = eval_fn(temp)
        k2[0], k2[1] = temp[2], temp[3]
        k2[2], k2[3] = a1, a2

        # RK4 Step 3
        np.multiply(k2, dt_half, out=acc)
        np.add(states, acc, out=temp)
        a1, a2 = eval_fn(temp)
        k3[0], k3[1] = temp[2], temp[3]
        k3[2], k3[3] = a1, a2

        # RK4 Step 4
        np.multiply(k3, dt, out=acc)
        np.add(states, acc, out=temp)
        a1, a2 = eval_fn(temp)
        k4[0], k4[1] = temp[2], temp[3]
        k4[2], k4[3] = a1, a2

        # Accumulate: states += (k1 + 2*k2 + 2*k3 + k4) * dt / 6
        np.add(k1, k2, out=acc)
        np.add(acc, k2, out=acc)
        np.add(acc, k3, out=acc)
        np.add(acc, k3, out=acc)
        np.add(acc, k4, out=acc)
        np.multiply(acc, dt_sixth, out=acc)
        np.add(states, acc, out=states)

        states[2] *= DAMPING
        states[3] *= DAMPING

    run_time = time.perf_counter() - t0
    return run_time, history


def main() -> None:
    print("Double Pendulum Lagrangian Benchmark")
    print(f"Pendulums: {N_PENDULUMS:,} | Steps: {N_STEPS} | dt: {DT}")
    print("-" * 56)

    np.random.seed(42)
    initial_states = np.zeros((4, N_PENDULUMS))
    initial_states[0] = np.random.uniform(2.5, 3.0, N_PENDULUMS)
    initial_states[1] = np.random.uniform(0.5, 1.5, N_PENDULUMS)

    engines = [
        ("SymPy", setup_sympy, "#60a5fa"),
        ("SymEngine", setup_symengine, "#34d399"),
        ("SymbAnaFis", setup_symbanafis, "#fbbf24"),
    ]

    results: list[dict[str, Any]] = []

    for name, setup_fn, color in engines:
        try:
            eval_fn, dt_time = setup_fn()
            if eval_fn is None:
                continue

            eval_fn(initial_states[:, :100])
            print(f"Running simulation with {name}...")

            sim_states = initial_states.copy()
            r_time, hist = simulate(eval_fn, sim_states)
            print(f"   Execution time: {r_time:.4f}s")

            results.append({
                "name": name,
                "prep_time": dt_time,
                "run_time": r_time,
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
    fig, gs = create_dashboard_figure(f"Double Pendulum Benchmark: {N_PENDULUMS:,} Pendulums")
    axes_map = {0: gs[0, 0], 1: gs[0, 1], 2: gs[1, 0]}
    subplots = []

    for i, res in enumerate(results[:3]):
        ax = fig.add_subplot(axes_map[i], facecolor="#0f172a")
        ax.set_facecolor("#0f172a")
        ax.set_xlim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_ylim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_aspect("equal")
        ax.axis("off")
        ax.set_title(res["name"], color=res["color"], fontsize=14, fontweight="bold")

        lines = []
        balls1 = []
        balls2 = []
        hist = res["history"]
        n_vis = hist.shape[2]

        for _ in range(n_vis):
            (ln,) = ax.plot([], [], "-", color=res["color"], alpha=0.4, lw=1.5)
            lines.append(ln)
            (b1,) = ax.plot([], [], "o", color="white", ms=3, alpha=0.6)
            balls1.append(b1)
            (b2,) = ax.plot([], [], "o", color=res["color"], ms=5, alpha=0.8)
            balls2.append(b2)

        subplots.append({
            "ax": ax,
            "lines": lines,
            "balls1": balls1,
            "balls2": balls2,
            "hist": hist,
        })

    ax_perf = fig.add_subplot(gs[1, 1], facecolor="#1e293b")
    plot_performance_panel(ax_perf, results, title="Physics Loop Performance")

    def update(frame: int) -> list[Any]:
        for sub in subplots:
            hist = sub["hist"]
            lines = sub["lines"]
            balls1 = sub["balls1"]
            balls2 = sub["balls2"]

            idx = min(frame, hist.shape[0] - 1)
            state = hist[idx]

            for k in range(len(lines)):
                th1_val = state[0, k]
                th2_val = state[1, k]
                x1_val = L1 * np.sin(th1_val)
                y1_val = -L1 * np.cos(th1_val)
                x2_val = x1_val + L2 * np.sin(th2_val)
                y2_val = y1_val - L2 * np.cos(th2_val)

                lines[k].set_data([0, x1_val, x2_val], [0, y1_val, y2_val])
                balls1[k].set_data([x1_val], [y1_val])
                balls2[k].set_data([x2_val], [y2_val])

        return []

    ani = animation.FuncAnimation(fig, update, frames=N_STEPS, interval=30, blit=False)
    save_benchmark_animation(ani, "double_pendulum_quad.mp4", fps=30, dpi=100, cq=26)


if __name__ == "__main__":
    main()
