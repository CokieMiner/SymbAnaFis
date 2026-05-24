import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import LogLocator

from precision_suite.viz_constants import CLASS_COLORS as COLORS, CLASS_ORDER, DEFAULT_DPI  # type: ignore
from precision_suite.classifier import LABELS  # type: ignore
from precision_suite.viz_utils import _safe_ulp, _envelope_line_for_entries, _pole_locations, _domain_limits  # type: ignore


def _scatter_by_class(ax, entries: list[dict], backend: str, func_name: str):
    by_cls = {}
    for e in entries:
        cls = e.get("_class", "unknown")
        by_cls.setdefault(cls, []).append(e)

    for cls in CLASS_ORDER:
        pts = by_cls.get(cls, [])
        if not pts:
            continue
        xs = [p["x"] for p in pts]
        ys = [max(_safe_ulp(p.get("_ulp"), backend, func_name), 1e-1) for p in pts]
        ax.scatter(xs, ys, s=6, color=COLORS.get(cls, "k"), label=LABELS.get(cls, cls))

    ax.set_yscale("log")
    ax.yaxis.set_major_locator(LogLocator(base=10.0))
    ax.set_ylabel("ULP error (log scale)")


def _annotate_poles_and_limits(ax, func_name: str):
    poles = _pole_locations(func_name, None)
    limits = _domain_limits(func_name, None)
    for p in poles:
        ax.axvline(p, color="0.5", linestyle="--", linewidth=0.8, ymin=0.05)
    for l in limits:
        ax.axvline(l, color="0.7", linestyle=":", linewidth=0.6, ymin=0.05)


def plot_2d(entries: list[dict], backend: str, func_name: str, out_path: str, dpi: int = DEFAULT_DPI):
    fig, ax = plt.subplots(figsize=(8, 4))
    _scatter_by_class(ax, entries, backend, func_name)

    xs = [e["x"] for e in entries]
    envelope_x, envelope_y = _envelope_line_for_entries(entries, backend, func_name)
    ax.plot(envelope_x, envelope_y, color="k", linewidth=0.8, alpha=0.6)

    _annotate_poles_and_limits(ax, func_name)

    ax.set_title(f"{func_name} — {backend}-bit")
    ax.legend(fontsize=8, loc="best")
    fig.tight_layout()
    fig.savefig(out_path, dpi=dpi)
    plt.close(fig)


def plot_panel_1d(groups: dict[tuple, list[dict]], backend: str, func_name: str,
                  out_path: str, dpi: int = DEFAULT_DPI):
    if not groups:
        return
    keys = sorted(groups.keys())
    n = len(keys)
    cols = min(4, n)
    rows = (n + cols - 1) // cols
    fig, axes = plt.subplots(rows, cols, figsize=(5 * cols, 3.5 * rows), squeeze=False)

    for i, key in enumerate(keys):
        r, c = divmod(i, cols)
        ax = axes[r][c]
        _scatter_by_class(ax, groups[key], backend, func_name)
        label = ",".join(str(k) for k in key)
        ax.set_title(f"({label})")
        ax.set_xlabel("x" if r == rows - 1 else "")
        ax.set_ylabel("ULP" if c == 0 else "")
        if ax.get_legend() is not None:
            ax.get_legend().remove()

    for i in range(n, rows * cols):
        r, c = divmod(i, cols)
        axes[r][c].set_visible(False)

    fig.suptitle(f"{func_name} — {backend}-bit", fontsize=12)
    handles, labels = axes[0][0].get_legend_handles_labels()
    if handles:
        fig.legend(handles, labels, loc="lower center", ncol=len(handles), fontsize=8)
    fig.tight_layout(rect=[0, 0.06, 1, 0.95])
    fig.savefig(out_path, dpi=dpi)
    plt.close(fig)


def plot_panel_heatmap(groups: dict[tuple, list[dict]], backend: str, func_name: str,
                        out_path: str, dpi: int = DEFAULT_DPI):
    import matplotlib.colors as mcolors
    if not groups:
        return
    keys = sorted(groups.keys())
    n = len(keys)
    cols = min(3, n)
    rows = (n + cols - 1) // cols
    fig, axes = plt.subplots(rows, cols, figsize=(5 * cols, 4 * rows), squeeze=False)

    for i, key in enumerate(keys):
        r, c = divmod(i, cols)
        ax = axes[r][c]
        pts = [e for e in groups[key] if "y" in e]
        if not pts:
            ax.set_visible(False)
            continue
        xs_all = [e["x"] for e in pts]
        ys_all = [e["y"] for e in pts]
        N = 40
        xi = np.linspace(min(xs_all), max(xs_all), N)
        yi = np.linspace(min(ys_all), max(ys_all), N)
        grid = np.full((N, N), np.nan)
        for e in pts:
            ri = int(np.clip(np.searchsorted(yi, e["y"]) - 1, 0, N - 2))
            cj = int(np.clip(np.searchsorted(xi, e["x"]) - 1, 0, N - 2))
            v = max(_safe_ulp(e.get("_ulp"), backend, func_name), 1e-1)
            grid[ri, cj] = v if np.isnan(grid[ri, cj]) else max(grid[ri, cj], v)

        valid = grid[np.isfinite(grid)]
        if valid.size == 0:
            ax.set_visible(False)
            continue
        norm = mcolors.LogNorm(vmin=np.nanmin(valid), vmax=np.nanmax(valid))
        im = ax.imshow(grid, origin="lower", cmap="viridis", norm=norm, aspect="auto",
                       extent=[xi[0], xi[-1], yi[0], yi[-1]])
        label = ",".join(str(k) for k in key)
        ax.set_title(f"({label})")
        ax.set_xlabel("x" if r == rows - 1 else "")
        ax.set_ylabel("y" if c == 0 else "")

    for i in range(n, rows * cols):
        r, c = divmod(i, cols)
        axes[r][c].set_visible(False)

    fig.suptitle(f"{func_name} — {backend}-bit", fontsize=12)
    fig.tight_layout(rect=[0, 0.02, 1, 0.95])
    fig.savefig(out_path, dpi=dpi)
    plt.close(fig)


def plot_3d(entries: list[dict], backend: str, func_name: str, out_path: str, dpi: int = DEFAULT_DPI):
    import matplotlib.colors as mcolors
    pts = [e for e in entries if "y" in e]
    if not pts:
        return
    xs_all = [e["x"] for e in pts]
    ys_all = [e["y"] for e in pts]
    N = 60
    xi = np.linspace(min(xs_all), max(xs_all), N)
    yi = np.linspace(min(ys_all), max(ys_all), N)
    grid = np.full((N, N), np.nan)
    for e in pts:
        i = int(np.clip(np.searchsorted(yi, e["y"]) - 1, 0, N - 2))
        j = int(np.clip(np.searchsorted(xi, e["x"]) - 1, 0, N - 2))
        v = max(_safe_ulp(e.get("_ulp"), backend, func_name), 1e-1)
        grid[i, j] = v if np.isnan(grid[i, j]) else max(grid[i, j], v)

    fig, ax = plt.subplots(figsize=(8, 5))
    valid = grid[np.isfinite(grid)]
    if valid.size == 0:
        plt.close(fig)
        return
    norm = mcolors.LogNorm(vmin=np.nanmin(valid), vmax=np.nanmax(valid))
    im = ax.imshow(grid, origin="lower", cmap="viridis", norm=norm, aspect="auto",
                   extent=[xi[0], xi[-1], yi[0], yi[-1]])
    fig.colorbar(im, ax=ax, label="ULP (log scale)")
    ax.set_xlabel("x")
    ax.set_ylabel("arg2")
    ax.set_title(f"{func_name} — {backend}-bit")
    fig.tight_layout()
    fig.savefig(out_path, dpi=dpi)
    plt.close(fig)
