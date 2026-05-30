"""Precision viz CLI.

Pyright in some editor setups cannot infer the tools/ path; suppress missing-module
reports for this file so the IDE stops flagging local-package imports.
"""
import argparse
import json
from collections import defaultdict
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

from precision_suite.config import JSON_PATH  # type: ignore
from precision_suite.classifier import is_overflowed, is_correct_overflow, classify, ulp_error  # type: ignore
from precision_suite.viz_plots import plot_2d, plot_3d, plot_panel_1d, plot_panel_heatmap  # type: ignore
from precision_suite.viz_intervals import print_domain_report  # type: ignore
from precision_suite.viz_utils import (  # type: ignore
    _normalize_entries, _classify_args, _group_by_discrete,
)
from .viz_constants import (  # type: ignore
    CHARTS_DIR, DEFAULT_BINS, DEFAULT_MAX_SLICES,
)

plt.rcParams.update({
    "axes.grid": True,
    "grid.alpha": 0.25,
    "axes.spines.top": False,
    "axes.spines.right": False,
    "legend.frameon": False,
})


def _enrich_entry(e: dict) -> dict:
    e = dict(e)
    backend = e.get("backend", "unknown")
    ae, ref = e.get("abs_error"), e.get("reference")

    if is_correct_overflow(e):
        e["_ulp"] = 0.0
    elif backend == "rug" and e.get("flint_in_bounds") is True:
        e["_ulp"] = 0.0
    elif "exact_ulp" in e and e["exact_ulp"] is not None:
        e["_ulp"] = e["exact_ulp"]
    elif ae is not None and ref is not None:
        e["_ulp"] = ulp_error(ae, ref, e.get("precision_bits", 53))
    else:
        e["_ulp"] = float("inf")

    if is_overflowed(e):
        e["_class"] = "overflow"
    else:
        e["_class"] = classify(e)

    if backend == "rug" and e.get("function") == "sinc" and e.get("_ulp") == 0.0:
        e["_ulp"] = 1.0

    return e


def load_and_enrich(json_path: Path) -> list[dict]:
    raw = json.loads(json_path.read_text())
    entries = raw.get("results", raw if isinstance(raw, list) else [])
    return [_enrich_entry(e) for e in entries]


def group_by_backend_func(entries: list[dict]) -> dict[str, dict[str, list[dict]]]:
    result: dict[str, dict[str, list[dict]]] = defaultdict(lambda: defaultdict(list))
    for e in entries:
        result[e.get("backend", "unknown")][e.get("function", "unknown")].append(e)
    return {k: dict(v) for k, v in result.items()}


def run_viz_cli():
    ap = argparse.ArgumentParser(description="Precision visualization and domain analysis")
    ap.add_argument("--all", action="store_true", help="Generate all charts")
    ap.add_argument("--function", type=str, help="Single function name")
    ap.add_argument("--backend", type=str, help="Filter backend (32, 64, rug)")
    ap.add_argument("--report", action="store_true", help="Print domain interval report only")
    ap.add_argument("--bins", type=int, default=DEFAULT_BINS, help="Bins for domain interval analysis")
    ap.add_argument("--max-slices", type=int, default=DEFAULT_MAX_SLICES, help="Max slices for multi-arg plots")
    ap.add_argument("--json", type=str, default=str(JSON_PATH),
                    help="Path to verify_results.json")
    args = ap.parse_args()

    json_path = Path(args.json)
    if not json_path.exists():
        print(f"File not found: {json_path}", file=__import__("sys").stderr)
        __import__("sys").exit(1)

    entries = load_and_enrich(json_path)
    grouped = group_by_backend_func(entries)

    if args.backend:
        grouped = {args.backend: grouped.get(args.backend, {})}

    if args.report:
        print_domain_report(grouped, bins=max(10, args.bins))
        return

    target_funcs = {args.function} if args.function else None

    for backend, funcs in sorted(grouped.items()):
        if backend == "rug":
            print("  rug: plots skipped (guaranteed ULP)")
            continue
        out_dir = CHARTS_DIR / backend
        out_dir.mkdir(parents=True, exist_ok=True)

        for func_name in sorted(funcs):
            if target_funcs and func_name not in target_funcs:
                continue
            raw_entries = funcs[func_name]
            func_entries = _normalize_entries(raw_entries, func_name)
            disc_keys, cont_keys = _classify_args(func_name)
            n_disc = len(disc_keys)
            n_cont = len(cont_keys)

            if n_disc == 0:
                if n_cont == 1:
                    out_path = out_dir / f"{func_name}.png"
                    plot_2d(func_entries, backend, func_name, str(out_path), dpi=160)
                    print(f"  {backend}/{func_name}.png ({len(func_entries)} pts)")
                elif n_cont == 2:
                    out_path = out_dir / f"{func_name}_3d.png"
                    plot_3d(func_entries, backend, func_name, str(out_path), dpi=160)
                    print(f"  {backend}/{func_name}_3d.png ({len(func_entries)} pts)")
            else:
                panels = _group_by_discrete(func_entries, disc_keys)
                if n_cont == 1:
                    out_path = out_dir / f"{func_name}_3d.png"
                    plot_panel_1d(panels, backend, func_name, str(out_path), dpi=160)
                    print(f"  {backend}/{func_name}_3d.png ({len(func_entries)} pts)")
                elif n_cont == 2:
                    out_path = out_dir / f"{func_name}_3d.png"
                    plot_panel_heatmap(panels, backend, func_name, str(out_path), dpi=160)
                    print(f"  {backend}/{func_name}_3d.png ({len(func_entries)} pts)")

    print(f"\nCharts saved to {CHARTS_DIR}")


if __name__ == "__main__":
    run_viz_cli()
