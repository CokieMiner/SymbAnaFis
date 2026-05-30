import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import LogLocator

from precision_suite.viz_constants import CLASS_COLORS as COLORS, CLASS_ORDER, CLASS_MARKERS, DEFAULT_DPI  # type: ignore
from precision_suite.classifier import LABELS  # type: ignore
from precision_suite.viz_utils import _safe_ulp, _pole_locations, _domain_limits, _get_bounds  # type: ignore


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
        marker = CLASS_MARKERS.get(cls, ".")
        ax.scatter(xs, ys, s=8 if marker == "x" else 3, color=COLORS.get(cls, "k"), label=LABELS.get(cls, cls), marker=marker, alpha=0.6)

    ax.set_yscale("log")
    ax.yaxis.set_major_locator(LogLocator(base=10.0))
    ax.set_ylabel("ULP error (log scale)")
    ax.grid(True, which="major", linestyle="-", linewidth=0.8, color="0.8")
    ax.grid(True, which="minor", linestyle=":", linewidth=0.5, color="0.9")



def _annotate_poles_and_limits(ax, func_name: str):
    bounds = _get_bounds(func_name)
    poles = _pole_locations(func_name, bounds)
    limits = _domain_limits(func_name, bounds)
    for p in poles:
        ax.axvline(p, color="blue", linestyle="--", linewidth=1.0, alpha=0.5, ymin=0.05, zorder=0, label="Pole" if p == poles[0] else "")
    for l in limits:
        ax.axvline(l, color="0.7", linestyle=":", linewidth=0.6, ymin=0.05, zorder=0)


def _annotate_zeros(ax, entries: list[dict]):
    zeros = []
    pts = sorted([e for e in entries if e.get("reference") is not None], key=lambda e: e["x"])
    for i in range(len(pts) - 1):
        x1, ref1 = pts[i]["x"], pts[i]["reference"]
        x2, ref2 = pts[i+1]["x"], pts[i+1]["reference"]
        
        # Skip NaNs or Infs
        if np.isnan(ref1) or np.isnan(ref2) or np.isinf(ref1) or np.isinf(ref2):
            continue
            
        # Detect sign change
        if (ref1 > 0 and ref2 < 0) or (ref1 < 0 and ref2 > 0):
            # Ensure it's a root crossing and not jumping across a pole
            if abs(ref1) < 1.0 and abs(ref2) < 1.0:
                t = -ref1 / (ref2 - ref1)
                z = x1 + t * (x2 - x1)
                zeros.append(z)
                
    for i, z in enumerate(zeros):
        ax.axvline(z, color="magenta", linestyle="-.", linewidth=1.0, alpha=0.5, ymin=0.05, zorder=0, label="Zero" if i == 0 else "")


def _annotate_poles_and_limits_3d(ax, func_name: str):
    bounds = _get_bounds(func_name)
    poles = _pole_locations(func_name, bounds)
    limits = _domain_limits(func_name, bounds)
    if not poles and not limits:
        return
        
    ylim = ax.get_ylim()
    zlim = ax.get_zlim()
    
    xlim = ax.get_xlim()
    
    for i, p in enumerate(poles):
        # Draw a blue dashed line on the floor plane across the Y-axis
        ax.plot([p, p], [ylim[0], ylim[1]], [zlim[0], zlim[0]], 
                color="blue", linestyle="--", linewidth=1.0, alpha=0.5, 
                zorder=0, label="Pole" if i == 0 else "")
        if func_name == "beta":
            ax.plot([xlim[0], xlim[1]], [p, p], [zlim[0], zlim[0]], 
                    color="blue", linestyle="--", linewidth=1.0, alpha=0.5, zorder=0)
    
    for l in limits:
        ax.plot([l, l], [ylim[0], ylim[1]], [zlim[0], zlim[0]], 
                color="black", linestyle=":", linewidth=1.0, alpha=0.8, zorder=0)

def _annotate_zeros_panel_1d(ax, groups, y_coords):
    zlim = ax.get_zlim()
    first_label = True
    for key, entries in groups.items():
        y_val = y_coords[key]
        zeros = []
        pts = sorted([e for e in entries if e.get("reference") is not None], key=lambda e: e["x"])
        for i in range(len(pts) - 1):
            x1, ref1 = pts[i]["x"], pts[i]["reference"]
            x2, ref2 = pts[i+1]["x"], pts[i+1]["reference"]
            if np.isnan(ref1) or np.isnan(ref2) or np.isinf(ref1) or np.isinf(ref2):
                continue
            if (ref1 > 0 and ref2 < 0) or (ref1 < 0 and ref2 > 0):
                if abs(ref1) < 1.0 and abs(ref2) < 1.0:
                    t = -ref1 / (ref2 - ref1)
                    z = x1 + t * (x2 - x1)
                    zeros.append(z)
        
        for z in zeros:
            ax.plot([z, z], [y_val - 0.4, y_val + 0.4], [zlim[0], zlim[0]],
                    color="magenta", linewidth=2.0, alpha=0.8, zorder=0,
                    label="Zero" if first_label else "")
            first_label = False


def plot_2d(entries: list[dict], backend: str, func_name: str, out_path: str, dpi: int = DEFAULT_DPI):
    fig, ax = plt.subplots(figsize=(12, 6))
    _scatter_by_class(ax, entries, backend, func_name)

    _annotate_poles_and_limits(ax, func_name)
    _annotate_zeros(ax, entries)

    ax.set_xlabel("x")
    ax.set_title(f"{func_name} — {backend}-bit")
    ax.legend(fontsize=9, loc="center left", bbox_to_anchor=(1.02, 0.5), frameon=True, facecolor="white", framealpha=0.9)
    fig.savefig(out_path, dpi=dpi, bbox_inches="tight")
    plt.close(fig)


def plot_panel_1d(groups: dict[tuple, list[dict]], backend: str, func_name: str,
                  out_path: str, dpi: int = DEFAULT_DPI):
    if not groups:
        return
    
    fig = plt.figure(figsize=(12, 8))
    ax = fig.add_subplot(111, projection='3d')
    
    keys = sorted(groups.keys())
    y_labels = [",".join(str(k) for k in key) for key in keys]
    y_coords = {key: i for i, key in enumerate(keys)}
    
    all_zs = []
    
    for cls in CLASS_ORDER:
        xs, ys, zs = [], [], []
        for key, entries in groups.items():
            y_val = y_coords[key]
            for e in entries:
                if e.get("_class", "unknown") == cls:
                    u = max(_safe_ulp(e.get("_ulp"), backend, func_name), 1e-1)
                    xs.append(e["x"])
                    ys.append(y_val)
                    z_val = np.log10(u)
                    zs.append(z_val)
                    all_zs.append(z_val)
                    
        if xs:
            marker = CLASS_MARKERS.get(cls, ".")
            ax.scatter(xs, ys, zs, s=8 if marker == "x" else 4, 
                       color=COLORS.get(cls, "k"), label=LABELS.get(cls, cls), 
                       marker=marker, alpha=0.7)

    # Reference planes for ULP thresholds
    xlim = ax.get_xlim()
    ylim = ax.get_ylim()
    X, Y = np.meshgrid(np.linspace(xlim[0], xlim[1], 2), np.linspace(ylim[0], ylim[1], 2))
    
    # 1 ULP (log10(1) = 0)
    ax.plot_surface(X, Y, np.zeros_like(X), color='#2ecc71', alpha=0.15)
    # 5 ULP (log10(5) ≈ 0.699)
    ax.plot_surface(X, Y, np.full_like(X, np.log10(5)), color='#f1c40f', alpha=0.15)
    # 10 ULP (log10(10) = 1)
    ax.plot_surface(X, Y, np.full_like(X, np.log10(10)), color='#e67e22', alpha=0.15)
    
    n_keys = len(keys)
    if n_keys <= 20:
        ax.set_yticks(range(n_keys))
        ax.set_yticklabels(y_labels)
    else:
        step = max(1, n_keys // 10)
        ax.set_yticks(range(0, n_keys, step))
        ax.set_yticklabels([y_labels[i] for i in range(0, n_keys, step)])
        
    _annotate_poles_and_limits_3d(ax, func_name)
    _annotate_zeros_panel_1d(ax, groups, y_coords)
    
    ax.set_xlabel("x")
    ax.set_ylabel("Discrete Args")
    ax.set_zlabel("ULP error (log10)")
    ax.set_title(f"{func_name} — {backend}-bit")
    
    ax.legend(fontsize=9, loc="center left", bbox_to_anchor=(1.1, 0.5), frameon=True, facecolor="white", framealpha=0.9)
    fig.savefig(out_path, dpi=dpi, bbox_inches="tight")
    plt.close(fig)


def plot_panel_heatmap(groups: dict[tuple, list[dict]], backend: str, func_name: str,
                        out_path: str, dpi: int = DEFAULT_DPI):
    all_pts = []
    for entries in groups.values():
        all_pts.extend([e for e in entries if "y" in e])
        
    if not all_pts:
        return
        
    plot_3d(all_pts, backend, func_name, out_path, dpi)


def plot_3d(entries: list[dict], backend: str, func_name: str, out_path: str, dpi: int = DEFAULT_DPI):
    pts = [e for e in entries if "y" in e]
    if not pts:
        return
        
    fig = plt.figure(figsize=(12, 8))
    ax = fig.add_subplot(111, projection='3d')
    
    for cls in CLASS_ORDER:
        xs, ys, zs = [], [], []
        for e in pts:
            if e.get("_class", "unknown") == cls:
                u = max(_safe_ulp(e.get("_ulp"), backend, func_name), 1e-1)
                xs.append(e["x"])
                ys.append(e["y"])
                zs.append(np.log10(u))
                
        if xs:
            marker = CLASS_MARKERS.get(cls, ".")
            ax.scatter(xs, ys, zs, s=8 if marker == "x" else 4, 
                       color=COLORS.get(cls, "k"), label=LABELS.get(cls, cls), 
                       marker=marker, alpha=0.7)

    xlim = ax.get_xlim()
    ylim = ax.get_ylim()
    X, Y = np.meshgrid(np.linspace(xlim[0], xlim[1], 2), np.linspace(ylim[0], ylim[1], 2))
    
    ax.plot_surface(X, Y, np.zeros_like(X), color='#2ecc71', alpha=0.15)
    ax.plot_surface(X, Y, np.full_like(X, np.log10(5)), color='#f1c40f', alpha=0.15)
    ax.plot_surface(X, Y, np.full_like(X, np.log10(10)), color='#e67e22', alpha=0.15)
    
    _annotate_poles_and_limits_3d(ax, func_name)
    
    ax.set_xlabel("x")
    ax.set_ylabel("y")
    ax.set_zlabel("ULP error (log10)")
    ax.set_title(f"{func_name} — {backend}-bit")
    
    ax.legend(fontsize=9, loc="center left", bbox_to_anchor=(1.1, 0.5), frameon=True, facecolor="white", framealpha=0.9)
    fig.savefig(out_path, dpi=dpi, bbox_inches="tight")
    plt.close(fig)
