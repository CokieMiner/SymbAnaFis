"""3-DOF Planar Robot Arm: Symbolic Jacobian, Correlated Uncertainty & IK Dashboard.

Demonstrates a realistic symbolic-to-numeric engineering workflow:
1. Symbolic forward kinematics for a 3-link planar manipulator.
2. Symbolic 2x3 Jacobian J(theta) = d(x, y) / d(th1, th2, th3).
3. Symbolic first-order correlated uncertainty propagation Sigma_p = J Sigma_th J^T
   (end-effector variances var_x, var_y and cross-covariance cov_xy).
4. Joint 11-output compilation sharing trig/CSE across kinematics, Jacobian,
   and covariance expressions.
5. Ensemble Damped Least-Squares (Levenberg-Marquardt) inverse kinematics across
   50,000 robot arms tracking a figure-8 trajectory that approaches workspace
   singularities, rendering live 2-sigma uncertainty ellipses.

Compares SymPy, SymEngine (LLVM), Symbolica (SymJIT), SymbAnaFis (JIT 1T), and
SymbAnaFis (JIT+Par).
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
    plot_stacked_performance_panel,
    print_metric_summary,
    save_benchmark_animation,
)

warnings.filterwarnings("ignore")

# CONFIGURATION
N_ARMS = 50_000
N_STEPS = 420
GENERATE_VIDEO = True
REQUIRE_ALL_ENGINES = False

# Robot geometry (total reach R = 2.6)
L1, L2, L3 = 1.1, 0.9, 0.6
MAX_REACH = L1 + L2 + L3
AXIS_LIMIT = 2.95

# Correlated joint-encoder uncertainty (radians)
SIGMA_1, SIGMA_2, SIGMA_3 = 0.045, 0.040, 0.035
RHO_12, RHO_23 = 0.45, 0.35

# Damped Least-Squares IK settings
IK_GAIN = 0.35
DAMPING_SQ = 0.04

# Unit circle for 2-sigma (95%) uncertainty ellipse rendering
_ELLIPSE_ANGLES = np.linspace(0.0, 2.0 * np.pi, 64)
_UNIT_CIRCLE = np.vstack((np.cos(_ELLIPSE_ANGLES), np.sin(_ELLIPSE_ANGLES)))


def _cov_entries() -> list[list[float]]:
    c12 = RHO_12 * SIGMA_1 * SIGMA_2
    c23 = RHO_23 * SIGMA_2 * SIGMA_3
    return [
        [SIGMA_1**2, c12, 0.0],
        [c12, SIGMA_2**2, c23],
        [0.0, c23, SIGMA_3**2],
    ]


def target_trajectory(n_steps: int) -> tuple[np.ndarray, np.ndarray]:
    """Figure-8 Lissajous path reaching near the outer singularity (r ~ 2.48)."""
    t = np.linspace(0.0, 2.0 * np.pi, n_steps, endpoint=False)
    tx = 2.38 * np.sin(t)
    ty = 1.45 * np.sin(2.0 * t)
    return tx, ty


def setup_sympy() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        import sympy as sp

        print("SymPy: Setting up...")
        t_sym = time.perf_counter()
        th1, th2, th3 = sp.symbols("th1 th2 th3")
        vars_s = [th1, th2, th3]

        p1 = th1
        p2 = th1 + th2
        p3 = th1 + th2 + th3
        x_e = L1 * sp.cos(p1) + L2 * sp.cos(p2) + L3 * sp.cos(p3)
        y_e = L1 * sp.sin(p1) + L2 * sp.sin(p2) + L3 * sp.sin(p3)

        gx = [sp.diff(x_e, v) for v in vars_s]
        gy = [sp.diff(y_e, v) for v in vars_s]
        cov = _cov_entries()

        var_x = sum(gx[i] * cov[i][j] * gx[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        var_y = sum(gy[i] * cov[i][j] * gy[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        cov_xy = sum(gx[i] * cov[i][j] * gy[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        outputs = [x_e, y_e, *gx, *gy, var_x, var_y, cov_xy]
        sym_time = time.perf_counter() - t_sym

        t_comp = time.perf_counter()
        func = sp.lambdify(vars_s, outputs, "numpy", cse=True)
        comp_time = time.perf_counter() - t_comp
        print(f"   Symbolic: {sym_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms")

        def evaluate(q: np.ndarray) -> tuple[np.ndarray, ...]:
            return tuple(func(q[0], q[1], q[2]))

        return evaluate, sym_time, comp_time
    except ImportError:
        print("   SymPy not available, skipping.")
        return None, 0.0, 0.0


def setup_symengine() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        import symengine as se

        print("SymEngine: Setting up...")
        t_sym = time.perf_counter()
        th1, th2, th3 = se.symbols("th1 th2 th3")
        vars_s = [th1, th2, th3]

        p1 = th1
        p2 = th1 + th2
        p3 = th1 + th2 + th3
        x_e = L1 * se.cos(p1) + L2 * se.cos(p2) + L3 * se.cos(p3)
        y_e = L1 * se.sin(p1) + L2 * se.sin(p2) + L3 * se.sin(p3)

        gx = [x_e.diff(v) for v in vars_s]
        gy = [y_e.diff(v) for v in vars_s]
        cov = _cov_entries()

        var_x = sum(gx[i] * cov[i][j] * gx[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        var_y = sum(gy[i] * cov[i][j] * gy[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        cov_xy = sum(gx[i] * cov[i][j] * gy[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        outputs = [x_e, y_e, *gx, *gy, var_x, var_y, cov_xy]
        sym_time = time.perf_counter() - t_sym

        t_comp = time.perf_counter()
        func = se.Lambdify(vars_s, outputs, backend="llvm", cse=True)
        comp_time = time.perf_counter() - t_comp
        print(f"   Symbolic: {sym_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms (LLVM)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(q: np.ndarray) -> tuple[np.ndarray, ...]:
            n = q.shape[1]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 3), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = q[0]
            inp[:, 1] = q[1]
            inp[:, 2] = q[2]
            out = func(inp)
            return tuple(out[:, k] for k in range(11))

        return evaluate, sym_time, comp_time
    except Exception as exc:
        print(f"   SymEngine not available ({exc}), skipping.")
        return None, 0.0, 0.0


def setup_symbolica() -> tuple[Callable[..., Any] | None, float, float]:
    try:
        from symbolica import Expression, S

        print("Symbolica: Setting up...")
        load_symbolica_license()
        t_sym = time.perf_counter()
        th1, th2, th3 = S("th1"), S("th2"), S("th3")
        vars_s = [th1, th2, th3]

        p1 = th1
        p2 = th1 + th2
        p3 = th1 + th2 + th3
        x_e = L1 * p1.cos() + L2 * p2.cos() + L3 * p3.cos()
        y_e = L1 * p1.sin() + L2 * p2.sin() + L3 * p3.sin()

        gx = [x_e.derivative(v) for v in vars_s]
        gy = [y_e.derivative(v) for v in vars_s]
        cov = _cov_entries()

        var_x = sum(gx[i] * cov[i][j] * gx[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        var_y = sum(gy[i] * cov[i][j] * gy[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        cov_xy = sum(gx[i] * cov[i][j] * gy[j] for i in range(3) for j in range(3) if cov[i][j] != 0.0)
        outputs = [x_e, y_e, *gx, *gy, var_x, var_y, cov_xy]
        sym_time = time.perf_counter() - t_sym

        t_comp = time.perf_counter()
        ev = Expression.evaluator_multiple(outputs, vars_s, jit_compile=True)
        ev.evaluate(np.zeros((1, 3), dtype=np.float64))
        comp_time = time.perf_counter() - t_comp
        print(f"   Symbolic: {sym_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms (SymJIT)")

        inp_cache: list[np.ndarray | None] = [None]

        def evaluate(q: np.ndarray) -> tuple[np.ndarray, ...]:
            n = q.shape[1]
            inp = inp_cache[0]
            if inp is None or inp.shape[0] != n:
                inp = np.empty((n, 3), dtype=np.float64)
                inp_cache[0] = inp
            inp[:, 0] = q[0]
            inp[:, 1] = q[1]
            inp[:, 2] = q[2]
            out = ev.evaluate(inp)
            return tuple(out[:, k] for k in range(11))

        return evaluate, sym_time, comp_time
    except Exception as exc:
        print(f"   Symbolica not available ({exc}), skipping.")
        return None, 0.0, 0.0


def setup_symbanafis(parallel: bool = True) -> tuple[Callable[..., Any] | None, float, float]:
    try:
        from symb_anafis import (
            Context,
            CovarianceMatrix,
            Evaluator,
            Expr,
            SimplificationSequence,
            Uncertainty,
        )

        mode = "Multi-Output Native JIT + Rayon" if parallel else "Multi-Output Native JIT (1T)"
        print(f"SymbAnaFis ({'Par' if parallel else '1T'}): Setting up...")

        t_sym = time.perf_counter()
        ctx = Context()
        th1, th2, th3 = ctx.symbols("th1", "th2", "th3")
        vars_s = [th1, th2, th3]

        p1 = th1
        p2 = th1 + th2
        p3 = th1 + th2 + th3
        x_e = L1 * p1.cos() + L2 * p2.cos() + L3 * p3.cos()
        y_e = L1 * p1.sin() + L2 * p2.sin() + L3 * p3.sin()

        # Native symbolic Jacobian and correlated Uncertainty propagation
        jac = Expr.jacobian([x_e, y_e], vars_s)
        cov_raw = _cov_entries()
        cov = CovarianceMatrix(cov_raw)
        unc_x = Uncertainty.propagate(x_e, vars_s, cov)
        unc_y = Uncertainty.propagate(y_e, vars_s, cov)

        # Cross-covariance Cov(x, y) = grad(x)^T Sigma grad(y)
        gx, gy = jac[0], jac[1]
        cov_xy = Expr.sum([
            gx[i] * cov_raw[i][j] * gy[j]
            for i in range(3)
            for j in range(3)
            if cov_raw[i][j] != 0.0
        ])

        raw_outputs = [x_e, y_e, *gx, *gy, unc_x.variance, unc_y.variance, cov_xy]
        sym_time = time.perf_counter() - t_sym

        t_comp = time.perf_counter()
        opt = SimplificationSequence.compilation()
        outputs = [opt.simplify(expr).expression for expr in raw_outputs]
        prog = Evaluator(outputs, vars_s, backend="f64", native=True)
        comp_time = time.perf_counter() - t_comp
        print(f"   Symbolic: {sym_time * 1000:.2f} ms | Compile: {comp_time * 1000:.2f} ms ({mode})")

        out_cache: list[tuple[np.ndarray, ...] | None] = [None]

        def evaluate(q: np.ndarray) -> tuple[np.ndarray, ...]:
            n = q.shape[1]
            outs = out_cache[0]
            if outs is None or outs[0].shape[0] != n:
                outs = tuple(np.empty(n, dtype=np.float64) for _ in range(11))
                out_cache[0] = outs
            prog.evaluate_batch([q[0], q[1], q[2]], out=outs, parallel=parallel)
            return outs

        return evaluate, sym_time, comp_time
    except ImportError as exc:
        print(f"   SymbAnaFis import error: {exc}")
        return None, 0.0, 0.0


def simulate(
    eval_fn: Callable[..., Any],
    initial_q: np.ndarray,
    tx: np.ndarray,
    ty: np.ndarray,
    record_history: bool,
) -> tuple[float, np.ndarray | None, np.ndarray | None]:
    """Run vectorized Damped Least-Squares IK + covariance evaluation."""
    n_steps = tx.shape[0]
    n_arms = initial_q.shape[1]
    vis_count = min(40, n_arms)
    vis_idx = np.linspace(0, n_arms - 1, vis_count, dtype=int)

    q = initial_q.copy()
    hist_q = np.empty((n_steps, 3, vis_count), dtype=np.float64) if record_history else None
    # Hero arm (index 0): [x, y, var_x, var_y, cov_xy]
    hist_hero = np.empty((n_steps, 5), dtype=np.float64) if record_history else None

    ex = np.empty(n_arms, dtype=np.float64)
    ey = np.empty(n_arms, dtype=np.float64)
    a00 = np.empty(n_arms, dtype=np.float64)
    a01 = np.empty(n_arms, dtype=np.float64)
    a11 = np.empty(n_arms, dtype=np.float64)
    inv_det = np.empty(n_arms, dtype=np.float64)
    w0 = np.empty(n_arms, dtype=np.float64)
    w1 = np.empty(n_arms, dtype=np.float64)
    tmp = np.empty(n_arms, dtype=np.float64)

    t0 = time.perf_counter()
    for i in range(n_steps):
        x, y, j00, j01, j02, j10, j11, j12, vx, vy, cxy = eval_fn(q)

        if hist_q is not None and hist_hero is not None:
            hist_q[i] = q[:, vis_idx]
            hist_hero[i, 0] = x[0]
            hist_hero[i, 1] = y[0]
            hist_hero[i, 2] = vx[0]
            hist_hero[i, 3] = vy[0]
            hist_hero[i, 4] = cxy[0]

        # Tracking error e = target - current
        np.subtract(tx[i], x, out=ex)
        np.subtract(ty[i], y, out=ey)

        # Damped normal matrix A = J J^T + lambda^2 I_2
        np.multiply(j00, j00, out=a00)
        np.multiply(j01, j01, out=tmp)
        a00 += tmp
        np.multiply(j02, j02, out=tmp)
        a00 += tmp
        a00 += DAMPING_SQ

        np.multiply(j00, j10, out=a01)
        np.multiply(j01, j11, out=tmp)
        a01 += tmp
        np.multiply(j02, j12, out=tmp)
        a01 += tmp

        np.multiply(j10, j10, out=a11)
        np.multiply(j11, j11, out=tmp)
        a11 += tmp
        np.multiply(j12, j12, out=tmp)
        a11 += tmp
        a11 += DAMPING_SQ

        # Analytic 2x2 solve: w = A^{-1} e
        np.multiply(a00, a11, out=inv_det)
        np.multiply(a01, a01, out=tmp)
        inv_det -= tmp
        np.divide(IK_GAIN, inv_det, out=inv_det)

        np.multiply(a11, ex, out=w0)
        np.multiply(a01, ey, out=tmp)
        w0 -= tmp
        w0 *= inv_det

        np.multiply(a00, ey, out=w1)
        np.multiply(a01, ex, out=tmp)
        w1 -= tmp
        w1 *= inv_det

        # Joint update dq = J^T w
        np.multiply(j00, w0, out=tmp)
        q[0] += tmp
        np.multiply(j10, w1, out=tmp)
        q[0] += tmp

        np.multiply(j01, w0, out=tmp)
        q[1] += tmp
        np.multiply(j11, w1, out=tmp)
        q[1] += tmp

        np.multiply(j02, w0, out=tmp)
        q[2] += tmp
        np.multiply(j12, w1, out=tmp)
        q[2] += tmp

    return time.perf_counter() - t0, hist_q, hist_hero


def _ellipse_points(cx: float, cy: float, vx: float, vy: float, cxy: float) -> tuple[np.ndarray, np.ndarray]:
    """Compute 2-sigma (95%) confidence ellipse coordinates from a 2x2 covariance."""
    cov = np.array([[max(vx, 0.0), cxy], [cxy, max(vy, 0.0)]], dtype=np.float64)
    eigvals, eigvecs = np.linalg.eigh(cov)
    radii = 2.0 * np.sqrt(np.maximum(eigvals, 0.0))
    pts = (eigvecs * radii) @ _UNIT_CIRCLE
    return cx + pts[0], cy + pts[1]


def main() -> None:
    parser = argparse.ArgumentParser(description="3-DOF Robot Arm Jacobian & Uncertainty Benchmark")
    parser.add_argument("--no-video", action="store_true", help="Skip MP4 dashboard generation")
    parser.add_argument("--arms", type=int, default=N_ARMS)
    parser.add_argument("--steps", type=int, default=N_STEPS)
    args = parser.parse_args()

    n_arms, n_steps = args.arms, args.steps
    generate_video = GENERATE_VIDEO and not args.no_video

    print("3-DOF Robot Arm Benchmark (Symbolic Jacobian + Correlated Uncertainty + IK)")
    print(f"Arms: {n_arms:,} | Steps: {n_steps} | Joint outputs: 11")
    print("-" * 74)

    tx, ty = target_trajectory(n_steps)
    rng = np.random.default_rng(42)
    initial_q = np.empty((3, n_arms), dtype=np.float64)
    initial_q[0] = 0.6 + rng.uniform(-0.18, 0.18, n_arms)
    initial_q[1] = -0.9 + rng.uniform(-0.18, 0.18, n_arms)
    initial_q[2] = 0.7 + rng.uniform(-0.18, 0.18, n_arms)
    # Keep hero arm centered
    initial_q[:, 0] = [0.6, -0.9, 0.7]

    engines = [
        ("SymPy", setup_sympy, "#60a5fa", True),
        ("SymEngine (LLVM)", setup_symengine, "#34d399", True),
        ("Symbolica (JIT)", setup_symbolica, "#c084fc", True),
        ("SymbAnaFis (JIT 1T)", lambda: setup_symbanafis(parallel=False), "#f97316", False),
        ("SymbAnaFis (JIT+Par)", lambda: setup_symbanafis(parallel=True), "#fbbf24", True),
    ]

    results: list[dict[str, Any]] = []
    for name, setup_fn, color, show_panel in engines:
        eval_fn, sym_t, comp_t = setup_fn()
        if eval_fn is None:
            continue

        # Warm up lazy SIMD kernels before timing
        eval_fn(initial_q[:, :128])

        print(f"Running {name}...")
        run_time, hist_q, hist_hero = simulate(
            eval_fn, initial_q, tx, ty, generate_video and show_panel
        )
        print(f"   Execution time: {run_time:.4f}s")

        results.append({
            "name": name,
            "prep_time": sym_t + comp_t,
            "run_time": run_time,
            "color": color,
            "show_panel": show_panel,
            "hist_q": hist_q,
            "hist_hero": hist_hero,
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
    print("\nGenerating Multi-Engine Robot Arm & Uncertainty Dashboard Video...")
    fig, sim_slots, perf_slot = create_dashboard_figure(
        f"3-DOF Robot Arm IK & 2σ Uncertainty: {n_arms:,} Arms (11 Symbolic Outputs)",
        n_engines=len(sim_results),
    )
    subplots = []

    reach_x = MAX_REACH * np.cos(_ELLIPSE_ANGLES)
    reach_y = MAX_REACH * np.sin(_ELLIPSE_ANGLES)

    for slot, res in zip(sim_slots, sim_results):
        ax = fig.add_subplot(slot, facecolor="#0b1120")
        ax.set_xlim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_ylim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_aspect("equal")
        ax.set_xticks([])
        ax.set_yticks([])
        for spine in ax.spines.values():
            spine.set_color("#1e293b")
            spine.set_linewidth(1.2)
        ax.set_title(res["name"], color=res["color"], fontsize=13, fontweight="bold", pad=8)

        # Workspace boundary and target Lissajous guide
        ax.plot(reach_x, reach_y, color="#334155", linestyle="--", linewidth=1.0, alpha=0.65)
        ax.plot(tx, ty, color="#475569", linewidth=1.0, alpha=0.45)

        # Ensemble arms + bright hero arm + live 2-sigma uncertainty ellipse
        lc = LineCollection([], colors=res["color"], alpha=0.22, linewidths=1.0, zorder=2)
        ax.add_collection(lc)
        (hero_line,) = ax.plot([], [], color=res["color"], linewidth=2.6, zorder=4)
        joints_scat = ax.scatter([], [], s=24, c="white", alpha=0.9, zorder=5)
        (ellipse_line,) = ax.plot([], [], color="#f8fafc", linewidth=1.8, alpha=0.95, zorder=6)
        (ellipse_fill,) = ax.fill([], [], color=res["color"], alpha=0.25, zorder=3)
        target_dot = ax.scatter([], [], s=52, c="#f43f5e", marker="x", linewidths=2.0, zorder=7)

        subplots.append({
            "lc": lc,
            "hero_line": hero_line,
            "joints_scat": joints_scat,
            "ellipse_line": ellipse_line,
            "ellipse_fill": ellipse_fill,
            "target_dot": target_dot,
            "hist_q": res["hist_q"],
            "hist_hero": res["hist_hero"],
        })

    ax_perf = fig.add_subplot(perf_slot, facecolor="#1e293b")
    plot_stacked_performance_panel(ax_perf, results, title="Prep + IK Loop Time")

    def update(frame: int) -> list[Any]:
        idx = min(frame, n_steps - 1)
        artists: list[Any] = []
        for sub in subplots:
            q = sub["hist_q"][idx]
            p1 = q[0]
            p2 = p1 + q[1]
            p3 = p2 + q[2]

            x1 = L1 * np.cos(p1)
            y1 = L1 * np.sin(p1)
            x2 = x1 + L2 * np.cos(p2)
            y2 = y1 + L2 * np.sin(p2)
            x3 = x2 + L3 * np.cos(p3)
            y3 = y2 + L3 * np.sin(p3)

            segs = np.zeros((q.shape[1], 4, 2), dtype=np.float64)
            segs[:, 1, 0], segs[:, 1, 1] = x1, y1
            segs[:, 2, 0], segs[:, 2, 1] = x2, y2
            segs[:, 3, 0], segs[:, 3, 1] = x3, y3
            sub["lc"].set_segments(segs)

            sub["hero_line"].set_data([0.0, x1[0], x2[0], x3[0]], [0.0, y1[0], y2[0], y3[0]])
            sub["joints_scat"].set_offsets(
                np.array([[0.0, 0.0], [x1[0], y1[0]], [x2[0], y2[0]], [x3[0], y3[0]]])
            )

            hx, hy, vx, vy, cxy = sub["hist_hero"][idx]
            ex, ey = _ellipse_points(hx, hy, vx, vy, cxy)
            sub["ellipse_line"].set_data(ex, ey)
            sub["ellipse_fill"].set_xy(np.column_stack((ex, ey)))
            sub["target_dot"].set_offsets(np.array([[tx[idx], ty[idx]]]))

            artists.extend((
                sub["lc"],
                sub["hero_line"],
                sub["joints_scat"],
                sub["ellipse_line"],
                sub["ellipse_fill"],
                sub["target_dot"],
            ))
        return artists

    ani = animation.FuncAnimation(fig, update, frames=n_steps, interval=30, blit=True)
    save_benchmark_animation(ani, "robot_arm_uncertainty_quad.mp4", fps=30, dpi=100, cq=24)


if __name__ == "__main__":
    main()
