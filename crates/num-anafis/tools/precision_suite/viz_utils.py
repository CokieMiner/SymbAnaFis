"""Utility helpers for visualization.

Pyright in some setups cannot infer the tools/ path. Suppress missing-module
reports for this file so the IDE stops flagging local-package imports.
"""
import math
import numpy as np

from precision_suite.config import FUNCTIONS  # type: ignore
from precision_suite.viz_constants import ( # type: ignore
    DOMAIN_LIMITS,
    INTEGER_POLE_FUNCS,
    POLE_POINTS,
    ULP_CAP,
    ULP_FLOOR,
)  


def _normalize_entries(entries: list[dict], func_name: str) -> list[dict]:
    """Add x, y, order, order2 keys from input/extras based on function layout."""
    for e in entries:
        e["x"] = e.get("input", 0.0)
        extras = e.get("extras", [])

        if func_name in {"bessel_j", "bessel_y", "bessel_i", "bessel_k",
                          "polygamma", "zeta_deriv", "hermite"}:
            # (x, order)
            if extras:
                e["order"] = int(round(extras[0]))

        elif func_name == "beta":
            if extras:
                e["y"] = extras[0]

        elif func_name == "assoc_legendre":
            # (x, l, m)
            if len(extras) >= 2:
                e["order"] = int(round(extras[0]))
                e["order2"] = int(round(extras[1]))

        elif func_name == "spherical_harmonic":
            # (theta, l, m, phi) -> x=theta, order=l, order2=m, y=phi
            if len(extras) >= 3:
                e["order"] = int(round(extras[0]))
                e["order2"] = int(round(extras[1]))
                e["y"] = extras[2]

    return entries


def _classify_args(func_name: str) -> tuple[list[str], list[str]]:
    """Return (discrete_keys, continuous_keys) for a function."""
    if func_name in {"bessel_j", "bessel_y", "bessel_i", "bessel_k",
                      "polygamma", "zeta_deriv", "hermite"}:
        return (["order"], ["x"])
    if func_name == "beta":
        return ([], ["x", "y"])
    if func_name == "assoc_legendre":
        return (["order", "order2"], ["x"])
    if func_name == "spherical_harmonic":
        return (["order", "order2"], ["x", "y"])
    return ([], ["x"])


def _group_by_discrete(entries: list[dict], discrete_keys: list[str]) -> dict[tuple, list[dict]]:
    """Group entries by discrete key tuples. Returns {(val,), [entries]...}."""
    from collections import defaultdict
    groups: dict[tuple, list[dict]] = defaultdict(list)
    for e in entries:
        key = tuple(e.get(k) for k in discrete_keys)
        groups[key].append(e)
    return dict(groups)


def _safe_ulp(ulp: float | None, backend: str | None = None, func_name: str | None = None) -> float:
    if ulp is None or math.isnan(ulp) or math.isinf(ulp):
        return ULP_CAP
    if backend == "rug" and func_name == "sinc" and ulp == 0.0:
        return 1.0
    if ulp <= 0:
        return ULP_FLOOR
    return min(max(ulp, ULP_FLOOR), ULP_CAP)


def _get_bounds(func_name: str) -> tuple[float, float] | None:
    spec = FUNCTIONS.get(func_name)
    if not spec:
        return None
    lo, hi, _ = spec[0]
    return float(lo), float(hi)


def _pole_locations(func_name: str, bounds: tuple[float, float] | None) -> list[float]:
    poles = list(POLE_POINTS.get(func_name, []))
    if func_name in INTEGER_POLE_FUNCS and bounds:
        lo, hi = bounds
        lo, hi = min(lo, hi), max(lo, hi)
        lo_i = math.ceil(min(lo, hi))
        hi_i = math.floor(max(lo, hi))
        for n in range(lo_i, hi_i + 1):
            if n <= 0:
                poles.append(float(n))
    return sorted(set(poles))


def _domain_limits(func_name: str, bounds: tuple[float, float] | None) -> list[float]:
    limits = list(DOMAIN_LIMITS.get(func_name, []))
    if bounds:
        limits.extend([bounds[0], bounds[1]])
    return sorted(set(limits))


def _select_evenly(values: list[float], max_items: int) -> list[float]:
    if len(values) <= max_items:
        return values
    idxs = np.linspace(0, len(values) - 1, max_items).round().astype(int)
    return [values[i] for i in idxs]


def _bin_entries(entries: list[dict], func_name: str, bins: int) -> list[dict]:
    if not entries:
        return []

    bounds = _get_bounds(func_name)
    xs = np.array([e.get("input", 0.0) for e in entries], dtype=float)
    lo = float(np.min(xs))
    hi = float(np.max(xs))
    if bounds:
        lo, hi = bounds
        lo, hi = min(lo, hi), max(lo, hi)
    if lo == hi:
        return [{"start": lo, "end": hi, "entries": entries}]

    edges = np.linspace(lo, hi, max(2, bins) + 1)
    buckets: list[list[dict]] = [[] for _ in range(len(edges) - 1)]
    for e in entries:
        x = float(e.get("input", 0.0))
        idx = int(np.searchsorted(edges, x, side="right") - 1)
        idx = max(0, min(idx, len(buckets) - 1))
        buckets[idx].append(e)

    binned = []
    for i, items in enumerate(buckets):
        if not items:
            continue
        binned.append({"start": float(edges[i]), "end": float(edges[i + 1]), "entries": items})
    return binned


def _envelope_line_for_entries(entries: list[dict], backend: str, func_name: str, bins: int = 200):
    """Compute a simple envelope line (x, y) across binned domain midpoints."""
    binned = _bin_entries(entries, func_name, bins)
    if not binned:
        return [], []
    xs = [(b["start"] + b["end"]) / 2.0 for b in binned]
    ys = [
        max((_safe_ulp(e.get("_ulp"), backend, func_name)
             for e in b["entries"] if e.get("_class") != "overflow"),
            default=ULP_FLOOR)
        for b in binned
    ]
    ys = [max(y, 1e-1) for y in ys]
    return xs, ys
