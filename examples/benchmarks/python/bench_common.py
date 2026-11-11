"""Common utilities for SymbAnaFis Python benchmarks and animated dashboards.

Provides Symbolica license loading (from environment or repo `.env`), adaptive
3- and 4-engine widescreen dashboard layouts, metric summaries with speedups,
and hardware-accelerated MP4 export.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import matplotlib.gridspec as gridspec
import matplotlib.pyplot as plt
import numpy as np

# Ensure sibling helper imports work regardless of current working directory.
_THIS_DIR = Path(__file__).resolve().parent
if str(_THIS_DIR) not in sys.path:
    sys.path.insert(0, str(_THIS_DIR))

from video_writer import save_animation_mp4


def load_symbolica_license() -> bool:
    """Configure Symbolica's license key from `SYMBOLICA_LICENSE` or repo `.env`."""
    import symbolica

    key = os.environ.get("SYMBOLICA_LICENSE", "").strip()
    if not key:
        for parent in Path(__file__).resolve().parents:
            env_file = parent / ".env"
            if env_file.is_file():
                for raw in env_file.read_text(encoding="utf-8").splitlines():
                    line = raw.strip()
                    if not line or line.startswith("#") or "=" not in line:
                        continue
                    k, v = line.split("=", 1)
                    if k.strip() == "SYMBOLICA_LICENSE":
                        key = v.strip().strip('"').strip("'")
                        break
            if key:
                break
    if not key:
        return False
    try:
        symbolica.set_license_key(key)
        return True
    except Exception as exc:
        print(f"   Warning: Symbolica license key failed ({exc})")
        return False


def print_metric_summary(results: list[dict[str, Any]]) -> None:
    """Print formatted metrics table comparing preparation, run, total time, and speedup."""
    fastest_run = min((r["run_time"] for r in results if r["run_time"] > 0), default=0.0)
    print("\nMetrics (preparation / execution / total / speedup vs slowest)")
    print("-" * 74)
    slowest_run = max((r["run_time"] for r in results), default=0.0)
    for res in results:
        prep = res.get("prep_time", 0.0)
        run = res.get("run_time", 0.0)
        total = prep + run
        vs_slow = f"{slowest_run / run:6.1f}x" if run > 0 else "   inf"
        tag = " *" if fastest_run > 0 and run == fastest_run else ""
        print(
            f"{res['name']:<22} prep={prep:>8.4f}s  run={run:>8.4f}s  "
            f"total={total:>8.4f}s  {vs_slow}{tag}"
        )
    print("-" * 74)


def create_dashboard_figure(
    title: str,
    n_engines: int = 4,
    figsize: tuple[float, float] | None = None,
) -> tuple[plt.Figure, list[Any], Any]:
    """Create a dark-themed dashboard supporting up to 4 simulation panels + performance chart.

    Returns ``(fig, sim_slots, perf_slot)`` where ``sim_slots`` has one GridSpec
    slot per active engine and ``perf_slot`` is the performance panel slot.
    """
    if n_engines <= 3:
        fig = plt.figure(figsize=figsize or (16, 9), facecolor="#0f172a")
        gs = gridspec.GridSpec(
            2,
            2,
            width_ratios=[1, 1],
            height_ratios=[1, 1],
            wspace=0.22,
            hspace=0.18,
            left=0.04,
            right=0.96,
            top=0.90,
            bottom=0.09,
        )
        sim_slots = [gs[0, 0], gs[0, 1], gs[1, 0]][:n_engines]
        perf_slot = gs[1, 1]
    else:
        fig = plt.figure(figsize=figsize or (19.2, 10.8), facecolor="#0f172a")
        gs_sim = gridspec.GridSpec(
            2,
            2,
            wspace=0.08,
            hspace=0.14,
            left=0.025,
            right=0.615,
            top=0.90,
            bottom=0.055,
        )
        gs_perf = gridspec.GridSpec(
            1,
            1,
            left=0.745,
            right=0.975,
            top=0.90,
            bottom=0.09,
        )
        sim_slots = [gs_sim[0, 0], gs_sim[0, 1], gs_sim[1, 0], gs_sim[1, 1]]
        perf_slot = gs_perf[0, 0]

    fig.suptitle(title, color="white", fontsize=20, fontweight="bold", y=0.965)
    return fig, sim_slots, perf_slot


def plot_performance_panel(
    ax: plt.Axes,
    results: list[dict[str, Any]],
    title: str = "Calculation Time",
) -> None:
    """Draw a horizontal bar chart comparing execution time with relative speedups."""
    names = [r["name"] for r in results]
    times = [r["run_time"] for r in results]
    colors = [r["color"] for r in results]
    y_pos = np.arange(len(names))

    ax.set_facecolor("#1e293b")
    ax.set_axisbelow(True)
    ax.grid(axis="x", color="#334155", linestyle="--", alpha=0.55)
    ax.barh(y_pos, times, color=colors, alpha=0.92, height=0.58)
    ax.set_yticks(y_pos)
    ax.set_yticklabels(names, color="white", fontweight="bold", fontsize=11)
    ax.invert_yaxis()
    ax.set_xlabel("Time (s) — lower is faster", color="#94a3b8", fontsize=11, labelpad=8)
    ax.set_title(title, color="white", fontweight="bold", fontsize=13, pad=10)

    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)
    ax.spines["left"].set_color("#475569")
    ax.spines["bottom"].set_color("#475569")
    ax.tick_params(colors="#94a3b8")

    max_t = max(times) if times else 1.0
    ax.set_xlim(0, max_t * 1.44)
    for i, v in enumerate(times):
        speedup = f" ({max_t / v:.1f}x)" if v > 0 and max_t / v >= 1.05 else ""
        ax.text(
            v + max_t * 0.02,
            i,
            f" {v:.4f}s{speedup}",
            color="white",
            va="center",
            fontweight="bold",
            fontsize=10,
        )


def plot_stacked_performance_panel(
    ax: plt.Axes,
    results: list[dict[str, Any]],
    title: str = "Performance Comparison",
) -> None:
    """Draw stacked horizontal bars comparing preparation vs execution time."""
    names = [r["name"] for r in results]
    r_times = [r["run_time"] for r in results]
    d_times = [r.get("prep_time", 0.0) for r in results]
    colors = [r["color"] for r in results]
    y_pos = np.arange(len(names))

    ax.set_facecolor("#1e293b")
    ax.set_axisbelow(True)
    ax.grid(axis="x", color="#334155", linestyle="--", alpha=0.55)
    ax.barh(y_pos, d_times, align="center", color="#64748b", alpha=0.65, height=0.58, label="Prep")
    ax.barh(y_pos, r_times, left=d_times, align="center", color=colors, alpha=0.92, height=0.58, label="Run")

    ax.set_yticks(y_pos)
    ax.set_yticklabels(names, color="white", fontweight="bold", fontsize=11)
    ax.invert_yaxis()
    ax.set_xlabel("Time (s) — lower is faster", color="#94a3b8", fontsize=11, labelpad=8)
    ax.set_title(title, color="white", fontweight="bold", fontsize=13, pad=10)
    ax.legend(facecolor="#0f172a", edgecolor="#475569", labelcolor="white", loc="lower right")

    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)
    ax.spines["left"].set_color("#475569")
    ax.spines["bottom"].set_color("#475569")
    ax.tick_params(colors="#94a3b8")

    totals = [d + r for d, r in zip(d_times, r_times)]
    max_tot = max(totals) if totals else 1.0
    ax.set_xlim(0, max_tot * 1.44)
    for i, total in enumerate(totals):
        speedup = f" ({max_tot / total:.1f}x)" if total > 0 and max_tot / total >= 1.05 else ""
        ax.text(
            total + max_tot * 0.02,
            i,
            f" {total:.4f}s{speedup}",
            color="white",
            va="center",
            fontweight="bold",
            fontsize=10,
        )


def save_benchmark_animation(
    ani: Any,
    filename: str,
    fps: int = 30,
    dpi: int = 100,
    cq: int = 24,
) -> Path:
    """Save an animation to the examples/videos directory using MP4 NVENC/CPU encoding."""
    video_dir = Path(__file__).resolve().parent.parent.parent / "videos"
    video_dir.mkdir(parents=True, exist_ok=True)
    out_path = video_dir / filename
    backend = save_animation_mp4(ani, out_path, fps=fps, dpi=dpi, cq=cq)
    print(f"Saved animation to {out_path} ({backend} encoder)")
    return out_path
