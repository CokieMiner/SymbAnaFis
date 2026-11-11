"""Double Pendulum Benchmark and Chaotic Dynamics Dashboard.

Derives the equations of motion for an ideal double pendulum symbolically
from the Lagrangian L = T - V, inverts the 2x2 mass matrix symbolically
via Cramer's rule, and integrates trajectories using RK4.

Compares analytical derivation and numerical evaluation speed across
SymPy, SymEngine (LLVM), Symbolica (SymJIT), and SymbAnaFis (Native JIT + Rayon).
"""

from __future__ import annotations

import argparse
import sys
import time
import warnings
from pathlib import Path
from typing import Any, Callable

import matplotlib.animation as animation
from matplotlib.collections import LineCollection
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

_DERIVATION_CACHE: dict[str, Any] = {}


def derive_sympy() -> tuple[Any, Any, tuple[Any, ...]]:
    if "sympy" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["sympy"]

    import sympy as sp

    th1, th2, w1, w2 = sp.symbols("th1 th2 w1 w2")
    m1, m2, l1, l2, g = sp.symbols("m1 m2 L1 L2 g")

    y1 = -l1 * sp.cos(th1)
    y2 = y1 - l2 * sp.cos(th2)

    vx1 = l1 * w1 * sp.cos(th1)
    vy1 = l1 * w1 * sp.sin(th1)
    vx2 = vx1 + l2 * w2 * sp.cos(th2)
    vy2 = vy1 + l2 * w2 * sp.sin(th2)

    T = 0.5 * m1 * (vx1**2 + vy1**2) + 0.5 * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1, dL_dw2 = sp.diff(L, w1), sp.diff(L, w2)
    dL_dth1, dL_dth2 = sp.diff(L, th1), sp.diff(L, th2)

    M11, M12 = sp.diff(dL_dw1, w1), sp.diff(dL_dw1, w2)
    M21, M22 = sp.diff(dL_dw2, w1), sp.diff(dL_dw2, w2)

    dt_term1 = sp.diff(dL_dw1, th1) * w1 + sp.diff(dL_dw1, th2) * w2
    dt_term2 = sp.diff(dL_dw2, th1) * w1 + sp.diff(dL_dw2, th2) * w2

    F1, F2 = dL_dth1 - dt_term1, dL_dth2 - dt_term2
    det = M11 * M22 - M12 * M21
    a1 = (F1 * M22 - F2 * M12) / det
    a2 = (M11 * F2 - M21 * F1) / det

    subs = {m1: M1, m2: M2, l1: L1, l2: L2, g: G}
    result = (a1.subs(subs), a2.subs(subs), (th1, th2, w1, w2))
    _DERIVATION_CACHE["sympy"] = result
    return result


def derive_symengine() -> tuple[Any, Any, tuple[Any, ...]]:
    if "symengine" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["symengine"]

    import symengine as se

    th1, th2, w1, w2 = se.symbols("th1 th2 w1 w2")
    m1, m2, l1, l2, g = se.symbols("m1 m2 L1 L2 g")

    y1 = -l1 * se.cos(th1)
    y2 = y1 - l2 * se.cos(th2)

    vx1 = l1 * w1 * se.cos(th1)
    vy1 = l1 * w1 * se.sin(th1)
    vx2 = vx1 + l2 * w2 * se.cos(th2)
    vy2 = vy1 + l2 * w2 * se.sin(th2)

    T = 0.5 * m1 * (vx1**2 + vy1**2) + 0.5 * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1, dL_dw2 = L.diff(w1), L.diff(w2)
    dL_dth1, dL_dth2 = L.diff(th1), L.diff(th2)

    M11, M12 = dL_dw1.diff(w1), dL_dw1.diff(w2)
    M21, M22 = dL_dw2.diff(w1), dL_dw2.diff(w2)

    dt_term1 = dL_dw1.diff(th1) * w1 + dL_dw1.diff(th2) * w2
    dt_term2 = dL_dw2.diff(th1) * w1 + dL_dw2.diff(th2) * w2

    F1, F2 = dL_dth1 - dt_term1, dL_dth2 - dt_term2
    det = M11 * M22 - M12 * M21
    a1 = (F1 * M22 - F2 * M12) / det
    a2 = (M11 * F2 - M21 * F1) / det

    subs = {m1: M1, m2: M2, l1: L1, l2: L2, g: G}
    result = (a1.subs(subs), a2.subs(subs), (th1, th2, w1, w2))
    _DERIVATION_CACHE["symengine"] = result
    return result


def derive_symbolica() -> tuple[Any, Any, tuple[Any, ...]]:
    if "symbolica" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["symbolica"]

    from symbolica import E, S

    th1, th2, w1, w2 = S("th1"), S("th2"), S("w1"), S("w2")
    m1, m2, l1, l2, g = S("m1"), S("m2"), S("L1"), S("L2"), S("g")

    y1 = -l1 * E("cos(th1)")
    y2 = y1 - l2 * E("cos(th2)")

    vx1 = l1 * w1 * E("cos(th1)")
    vy1 = l1 * w1 * E("sin(th1)")
    vx2 = vx1 + l2 * w2 * E("cos(th2)")
    vy2 = vy1 + l2 * w2 * E("sin(th2)")

    half = E("1/2")
    T = half * m1 * (vx1**2 + vy1**2) + half * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1, dL_dw2 = L.derivative(w1), L.derivative(w2)
    dL_dth1, dL_dth2 = L.derivative(th1), L.derivative(th2)

    M11, M12 = dL_dw1.derivative(w1), dL_dw1.derivative(w2)
    M21, M22 = dL_dw2.derivative(w1), dL_dw2.derivative(w2)

    dt_term1 = dL_dw1.derivative(th1) * w1 + dL_dw1.derivative(th2) * w2
    dt_term2 = dL_dw2.derivative(th1) * w1 + dL_dw2.derivative(th2) * w2

    F1, F2 = dL_dth1 - dt_term1, dL_dth2 - dt_term2
    det = M11 * M22 - M12 * M21
    a1 = (F1 * M22 - F2 * M12) / det
    a2 = (M11 * F2 - M21 * F1) / det

    for sym, val in ((m1, M1), (m2, M2), (l1, L1), (l2, L2), (g, G)):
        val_e = E(str(val))
        a1 = a1.replace(sym, val_e)
        a2 = a2.replace(sym, val_e)

    result = (a1, a2, (th1, th2, w1, w2))
    _DERIVATION_CACHE["symbolica"] = result
    return result


def derive_symbanafis() -> tuple[Any, Any, Any]:
    if "symbanafis" in _DERIVATION_CACHE:
        return _DERIVATION_CACHE["symbanafis"]

    from symb_anafis import Context, Expr, Substitutions

    ctx = Context()
    th1, th2, w1, w2 = ctx.symbols("th1", "th2", "w1", "w2")
    m1, m2, l1, l2, g = ctx.symbols("m1", "m2", "L1", "L2", "g")

    y1 = -l1 * th1.cos()
    y2 = y1 - l2 * th2.cos()

    vx1 = l1 * w1 * th1.cos()
    vy1 = l1 * w1 * th1.sin()
    vx2 = vx1 + l2 * w2 * th2.cos()
    vy2 = vy1 + l2 * w2 * th2.sin()

    half = Expr(1) / 2
    T = half * m1 * (vx1**2 + vy1**2) + half * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V

    dL_dw1, dL_dw2 = L.derivative(w1), L.derivative(w2)
    dL_dth1, dL_dth2 = L.derivative(th1), L.derivative(th2)

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
    result = (subs.apply(a1), subs.apply(a2), (th1, th2, w1, w2))
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
        print(f"   Setup: {setup_time * 1000:.2f} ms")

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
        print(f"   Setup: {setup_time * 1000:.2f} ms (LLVM)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(s: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = s.shape[1]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 4), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = s[0]
            inp[:, 1] = s[1]
            inp[:, 2] = s[2]
            inp[:, 3] = s[3]
            out = func(inp)
            return out[:, 0], out[:, 1]

        return evaluate, setup_time
    except Exception as exc:
        print(f"   SymEngine not available ({exc}), skipping.")
        return None, 0.0


def setup_symbolica() -> tuple[Callable[..., Any] | None, float]:
    try:
        from symbolica import Expression

        print("Symbolica: Setting up...")
        load_symbolica_license()
        t0 = time.perf_counter()
        a1, a2, vars_sym = derive_symbolica()

        ev = Expression.evaluator_multiple([a1, a2], list(vars_sym), jit_compile=True)
        ev.evaluate(np.zeros((1, 4), dtype=np.float64))
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms (SymJIT)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(s: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = s.shape[1]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 4), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = s[0]
            inp[:, 1] = s[1]
            inp[:, 2] = s[2]
            inp[:, 3] = s[3]
            out = ev.evaluate(inp)
            return out[:, 0], out[:, 1]

        return evaluate, setup_time
    except Exception as exc:
        print(f"   Symbolica not available ({exc}), skipping.")
        return None, 0.0


def setup_symbanafis(parallel: bool = True) -> tuple[Callable[..., Any] | None, float]:
    try:
        from symb_anafis import Evaluator, SimplificationSequence

        mode = "Multi-Output Native JIT + Rayon" if parallel else "Multi-Output Native JIT (1T)"
        print(f"SymbAnaFis ({'Par' if parallel else '1T'}): Setting up...")
        _DERIVATION_CACHE.pop("symbanafis", None)
        t0 = time.perf_counter()
        a1, a2, vars_sym = derive_symbanafis()
        opt = SimplificationSequence.compilation()
        a1 = opt.simplify(a1).expression
        a2 = opt.simplify(a2).expression

        prog = Evaluator([a1, a2], list(vars_sym), backend="f64", native=True)
        setup_time = time.perf_counter() - t0
        print(f"   Setup: {setup_time * 1000:.2f} ms ({mode})")

        out_cache: list[tuple[np.ndarray, np.ndarray] | None] = [None]

        def evaluate(s: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
            n = s.shape[1]
            outs = out_cache[0]
            if outs is None or outs[0].shape[0] != n:
                outs = (np.empty(n, dtype=np.float64), np.empty(n, dtype=np.float64))
                out_cache[0] = outs
            prog.evaluate_batch([s[0], s[1], s[2], s[3]], out=outs, parallel=parallel)
            return outs

        return evaluate, setup_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0


def simulate(
    eval_fn: Callable[..., Any],
    initial_states: np.ndarray,
    n_steps: int,
    record_history: bool,
) -> tuple[float, np.ndarray | None]:
    n_pendulums = initial_states.shape[1]
    vis_count = min(50, n_pendulums)
    vis_idx = np.linspace(0, n_pendulums - 1, vis_count, dtype=int)

    states = initial_states.copy()
    history = np.empty((n_steps, 4, vis_count), dtype=np.float64) if record_history else None

    dt = DT
    dt_half = dt * 0.5
    dt_sixth = dt / 6.0

    k1 = np.empty_like(states)
    k2 = np.empty_like(states)
    k3 = np.empty_like(states)
    k4 = np.empty_like(states)
    temp = np.empty_like(states)
    acc = np.empty_like(states)

    t0 = time.perf_counter()
    for i in range(n_steps):
        if history is not None:
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

        # Combine RK4 increments in-place
        np.add(k2, k3, out=acc)
        acc *= 2.0
        acc += k1
        acc += k4
        acc *= dt_sixth
        states += acc

        # Gentle velocity damping
        states[2] *= DAMPING
        states[3] *= DAMPING

    return time.perf_counter() - t0, history


def main() -> None:
    parser = argparse.ArgumentParser(description="Double Pendulum Benchmark")
    parser.add_argument("--no-video", action="store_true", help="Skip MP4 dashboard generation")
    parser.add_argument("--pendulums", type=int, default=N_PENDULUMS)
    parser.add_argument("--steps", type=int, default=N_STEPS)
    args = parser.parse_args()

    n_pendulums, n_steps = args.pendulums, args.steps
    generate_video = GENERATE_VIDEO and not args.no_video

    print("Double Pendulum Benchmark (Symbolic Lagrangian + RK4)")
    print(f"Pendulums: {n_pendulums:,} | Steps: {n_steps} | dt: {DT}")
    print("-" * 56)

    rng = np.random.default_rng(42)
    initial_states = np.zeros((4, n_pendulums), dtype=np.float64)
    initial_states[0] = np.pi * 0.75 + rng.uniform(-0.02, 0.02, n_pendulums)
    initial_states[1] = np.pi * 0.75 + rng.uniform(-0.02, 0.02, n_pendulums)

    engines = [
        ("SymPy", setup_sympy, "#60a5fa", True),
        ("SymEngine (LLVM)", setup_symengine, "#34d399", True),
        ("Symbolica (JIT)", setup_symbolica, "#c084fc", True),
        ("SymbAnaFis (JIT 1T)", lambda: setup_symbanafis(parallel=False), "#f97316", False),
        ("SymbAnaFis (JIT+Par)", lambda: setup_symbanafis(parallel=True), "#fbbf24", True),
    ]

    results: list[dict[str, Any]] = []
    for name, setup, color, show_panel in engines:
        fn, s_time = setup()
        if not fn:
            continue

        fn(initial_states[:, :128])

        print(f"Running {name}...")
        run_time, history = simulate(fn, initial_states, n_steps, generate_video and show_panel)
        print(f"   Execution time: {run_time:.4f}s")
        results.append({
            "name": name,
            "prep_time": s_time,
            "run_time": run_time,
            "color": color,
            "show_panel": show_panel,
            "history": history,
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
    print("\nGenerating Multi-Engine Dashboard Video (vectorized LineCollection)...")
    fig, sim_slots, perf_slot = create_dashboard_figure(
        f"Double Pendulum Benchmark: {n_pendulums:,} Pendulums", n_engines=len(sim_results)
    )
    subplots = []

    for slot, res in zip(sim_slots, sim_results):
        ax = fig.add_subplot(slot, facecolor="#0f172a")
        ax.set_xlim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_ylim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_aspect("equal")
        ax.axis("off")
        ax.set_title(res["name"], color=res["color"], fontsize=13, fontweight="bold")

        lc = LineCollection([], colors=res["color"], alpha=0.4, linewidths=1.5)
        ax.add_collection(lc)
        b1 = ax.scatter([], [], s=12, c="white", alpha=0.65, zorder=3)
        b2 = ax.scatter([], [], s=24, c=res["color"], alpha=0.85, zorder=4)
        subplots.append({"lc": lc, "b1": b1, "b2": b2, "hist": res["history"]})

    ax_perf = fig.add_subplot(perf_slot, facecolor="#1e293b")
    plot_performance_panel(ax_perf, results, title="Physics Loop Performance")

    def update(frame: int) -> list[Any]:
        artists = []
        for sub in subplots:
            hist = sub["hist"]
            state = hist[min(frame, hist.shape[0] - 1)]
            th1, th2 = state[0], state[1]
            x1 = L1 * np.sin(th1)
            y1 = -L1 * np.cos(th1)
            x2 = x1 + L2 * np.sin(th2)
            y2 = y1 - L2 * np.cos(th2)

            segs = np.zeros((th1.shape[0], 3, 2), dtype=np.float64)
            segs[:, 1, 0] = x1
            segs[:, 1, 1] = y1
            segs[:, 2, 0] = x2
            segs[:, 2, 1] = y2
            sub["lc"].set_segments(segs)
            sub["b1"].set_offsets(np.column_stack((x1, y1)))
            sub["b2"].set_offsets(np.column_stack((x2, y2)))
            artists.extend((sub["lc"], sub["b1"], sub["b2"]))
        return artists

    ani = animation.FuncAnimation(fig, update, frames=n_steps, interval=30, blit=True)
    save_benchmark_animation(ani, "double_pendulum_quad.mp4", fps=30, dpi=100, cq=24)


if __name__ == "__main__":
    main()
