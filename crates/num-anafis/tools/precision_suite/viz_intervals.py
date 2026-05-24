"""
Interval analysis utilities.

Suppress pyright missing-module reports for editor setups that don't include
the tools/ folder in their analysis path.
"""
import math
from collections import defaultdict

from precision_suite.viz_utils import _bin_entries, _get_bounds  # type: ignore
from precision_suite.viz_constants import ( # type: ignore
    DEFAULT_BINS,
)
from precision_suite.config import TARGETED_POLE_EPS_RANGE  # type: ignore
from precision_suite.classifier import LABELS  # type: ignore
from collections import Counter
from precision_suite.viz_utils import _safe_ulp  # type: ignore
from precision_suite.viz_utils import _pole_locations, _domain_limits  # type: ignore
from precision_suite.viz_constants import MULTI_ARG  # type: ignore

def _classify_bin(items: list[dict], backend: str, func_name: str) -> tuple[str, float, str | None]:
    counts = Counter(e.get("_class", "unknown") for e in items)
    total = len(items)
    max_ulp = max(_safe_ulp(e.get("_ulp"), backend, func_name) for e in items)

    if counts.get("overflow"):
        return "overflow", max_ulp, "overflow/NaN"

    noise_frac = counts.get("noise", 0) / max(1, total)
    if noise_frac >= 0.8 and max_ulp <= 1:
        return "noise", max_ulp, "machine noise"

    if max_ulp <= 1:
        return "faithful", max_ulp, None
    if max_ulp <= 5:
        return "good", max_ulp, None
    if max_ulp <= 10:
        return "acceptable", max_ulp, None

    return "severe", max_ulp, None


def _reason_for_interval(start: float, end: float, items: list[dict], func_name: str) -> str:
    bounds = _get_bounds(func_name)
    poles = _pole_locations(func_name, bounds)
    limits = _domain_limits(func_name, bounds)
    pole_eps_hi = TARGETED_POLE_EPS_RANGE[1] if TARGETED_POLE_EPS_RANGE else None

    for p in poles:
        if start <= p <= end:
            return "pole proximity"
        if pole_eps_hi is not None and min(abs(start - p), abs(end - p)) <= pole_eps_hi:
            return "pole proximity"
    for lim in limits:
        if start <= lim <= end:
            return "domain edge"

    max_mag = max(abs(start), abs(end))
    if max_mag >= 1e3:
        return "large |x|"

    if any(e.get("targeted") for e in items):
        return "targeted stress"

    return "mixed behavior"


def _merge_intervals(bins: list[dict]) -> list[dict]:
    if not bins:
        return []
    merged = [bins[0]]
    for b in bins[1:]:
        prev = merged[-1]
        if b["class"] == prev["class"] and b.get("reason") == prev.get("reason"):
            prev["end"] = b["end"]
            prev["max_ulp"] = max(prev["max_ulp"], b["max_ulp"])
        else:
            merged.append(b)
    return merged


def _interval_summary_lines(intervals: list[dict], max_ranges: int = 3) -> list[str]:
    by_class: dict[str, list[tuple[float, float]]] = defaultdict(list)
    for iv in intervals:
        by_class[iv["class"]].append((iv["start"], iv["end"]))

    lines = []
    for cls in ["faithful", "good", "acceptable", "severe", "noise", "overflow"]:
        ranges = by_class.get(cls, [])
        if not ranges:
            continue
        label = LABELS.get(cls, cls).split("(")[0].strip()
        shown = ranges[:max_ranges]
        fmt_ranges = ", ".join([f"[{a:.3g}, {b:.3g}]" for a, b in shown])
        suffix = "" if len(ranges) <= max_ranges else f" (+{len(ranges) - max_ranges} more)"
        lines.append(f"{label}: {fmt_ranges}{suffix}")
    return lines


def compute_intervals(entries: list[dict], backend: str, func_name: str, bins: int = DEFAULT_BINS) -> list[dict]:
    binned = _bin_entries(entries, func_name, bins)
    intervals = []
    for b in binned:
        cls, max_ulp, reason = _classify_bin(b["entries"], backend, func_name)
        interval = {
            "start": b["start"],
            "end": b["end"],
            "class": cls,
            "max_ulp": max_ulp,
        }
        if cls in ("severe", "overflow"):
            interval["reason"] = reason or _reason_for_interval(b["start"], b["end"], b["entries"], func_name)
        intervals.append(interval)
    return _merge_intervals(intervals)


def format_ulp_range(max_ulp: float) -> str:
    if math.isinf(max_ulp):
        return "∞"
    if max_ulp <= 1:
        return "≤ 1"
    if max_ulp <= 5:
        return "≤ 5"
    if max_ulp <= 10:
        return "≤ 10"
    if max_ulp <= 100:
        return f"≤ {int(max_ulp)}"
    return "> 100"


def print_domain_report(grouped: dict[str, dict[str, list[dict]]], bins: int = DEFAULT_BINS):
    print("\n# Domain Interval Analysis\n")
    print("For each function, input ranges grouped by precision classification (binned).\n")

    for backend, funcs in sorted(grouped.items()):
        if backend == "rug":
            print("## Rug backend\n")
            print("All functions guarantee 0 ULP (faithful rounding).")
            print("Exception: sinc guarantees 1 ULP.\n")
            continue

        print(f"## Backend: {backend}-bit\n")
        for func_name in sorted(funcs):
            entries = funcs[func_name]
            intervals = compute_intervals(entries, backend, func_name, bins)
            note = " (collapsed over extra args)" if func_name in MULTI_ARG else ""
            print(f"### {func_name}{note}")
            print(f"({len(entries)} samples)\n")
            print(f"| Interval | Class | Max ULP | Reason |")
            print(f"|-----------|-------|---------|--------|")

            for iv in intervals:
                label = LABELS.get(iv["class"], iv["class"])
                label_short = label.split("(")[0].strip()
                reason = iv.get("reason", "") if iv["class"] in ("severe", "overflow") else ""
                print(f"| [{iv['start']:.6g}, {iv['end']:.6g}] | {label_short} | {format_ulp_range(iv['max_ulp'])} | {reason} |")

            print()
