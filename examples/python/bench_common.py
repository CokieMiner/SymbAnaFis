"""Common utilities for SymbAnaFis benchmarks and graphical dashboards.

Provides shared plotting styles, dashboard grid creation, metric formatting,
and video export helpers without external dependencies beyond NumPy and Matplotlib.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import matplotlib.gridspec as gridspec
import matplotlib.pyplot as plt
import numpy as np

from video_writer import save_animation_mp4


def print_metric_summary(results: list[dict[str, Any]]) -> None:
    """Print formatted metrics table comparing preparation, run, and total time."""
    print("\nMetrics (preparation / execution / total)")
    print("-" * 56)
    for res in results:
        prep = res.get("prep_time", 0.0)
        run = res.get("run_time", 0.0)
        total = prep + run
        print(f"{res['name']:<14} prep={prep:>8.4f}s  run={run:>8.4f}s  total={total:>8.4f}s")
    print("-" * 56)


def create_dashboard_figure(
    title: str,
    figsize: tuple[int, int] = (16, 9),
) -> tuple[plt.Figure, gridspec.GridSpec]:
    """Create a dark-themed 2x2 Quad-View figure and GridSpec."""
    fig = plt.figure(figsize=figsize, facecolor="#0f172a")
    gs = gridspec.GridSpec(
        2, 2,
        width_ratios=[1, 1],
        height_ratios=[1, 1],
        wspace=0.1,
        hspace=0.15,
        left=0.05,
        right=0.95,
        top=0.90,
        bottom=0.05,
    )
    fig.suptitle(title, color="white", fontsize=20, fontweight="bold", y=0.98)
    return fig, gs


def plot_performance_panel(
    ax: plt.Axes,
    results: list[dict[str, Any]],
    title: str = "Calculation Time",
) -> None:
    """Draw a horizontal bar chart comparing execution time on the given axis."""
    names = [r["name"] for r in results]
    times = [r["run_time"] for r in results]
    colors = [r["color"] for r in results]
    y_pos = np.arange(len(names))

    ax.set_facecolor("#1e293b")
    ax.barh(y_pos, times, color=colors, alpha=0.9)
    ax.set_yticks(y_pos)
    ax.set_yticklabels(names, color="white", fontweight="bold", fontsize=11)
    ax.invert_yaxis()
    ax.set_xlabel("Time (s)", color="#94a3b8")
    ax.set_title(title, color="white", fontweight="bold", fontsize=13)

    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)
    ax.spines["left"].set_visible(False)
    ax.spines["bottom"].set_color("#475569")
    ax.tick_params(colors="#94a3b8")

    max_t = max(times) if times else 1.0
    for i, v in enumerate(times):
        ax.text(v + max_t * 0.02, i, f" {v:.4f}s", color="white", va="center", fontweight="bold")


def plot_stacked_performance_panel(
    ax: plt.Axes,
    results: list[dict[str, Any]],
    title: str = "Performance Comparison",
) -> None:
    """Draw stacked horizontal bars comparing preparation vs run time."""
    names = [r["name"] for r in results]
    r_times = [r["run_time"] for r in results]
    d_times = [r.get("prep_time", 0.0) for r in results]
    colors = [r["color"] for r in results]
    y_pos = np.arange(len(names))

    ax.set_facecolor("#1e293b")
    ax.barh(y_pos, d_times, align="center", color="#64748b", alpha=0.6, label="Prep")
    ax.barh(y_pos, r_times, left=d_times, align="center", color=colors, alpha=0.9, label="Run")

    ax.set_yticks(y_pos)
    ax.set_yticklabels(names, color="white", fontweight="bold", fontsize=11)
    ax.invert_yaxis()
    ax.set_xlabel("Time (s)", color="#94a3b8")
    ax.set_title(title, color="white", fontweight="bold", fontsize=13)

    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)
    ax.spines["left"].set_visible(False)
    ax.spines["bottom"].set_color("#475569")
    ax.tick_params(colors="#94a3b8")

    totals = [d + r for d, r in zip(d_times, r_times)]
    max_tot = max(totals) if totals else 1.0
    for i, total in enumerate(totals):
        ax.text(total + max_tot * 0.02, i, f" {total:.4f}s", color="white", va="center", fontweight="bold")


def save_benchmark_animation(
    ani: Any,
    filename: str,
    fps: int = 30,
    dpi: int = 100,
    cq: int = 24,
) -> Path:
    """Save an animation to the examples/videos directory using MP4 NVENC/CPU encoding."""
    video_dir = Path(__file__).resolve().parent.parent / "videos"
    video_dir.mkdir(parents=True, exist_ok=True)
    out_path = video_dir / filename
    backend = save_animation_mp4(ani, out_path, fps=fps, dpi=dpi, cq=cq)
    print(f"Saved animation to {out_path} ({backend} encoder)")
    return out_path
