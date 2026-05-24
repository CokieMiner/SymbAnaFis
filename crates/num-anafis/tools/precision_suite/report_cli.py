import json
import math
import sys
import argparse
from collections import defaultdict

from .config import JSON_PATH # type: ignore
from .classifier import is_overflowed, is_correct_overflow, classify, LABELS, ulp_error # type: ignore

def run_report_cli():
    parser = argparse.ArgumentParser(description="Analyze ULP results from verify_results.json")
    parser.add_argument("--backend", help="Filter by backend (e.g., 32, 64)", type=str)
    parser.add_argument("--function", help="Filter by function name", type=str)
    parser.add_argument("--only-severe", help="Only consider 'severe' or 'overflow' cases", action="store_true")
    args = parser.parse_args()

    if not JSON_PATH.exists():
        print(f"File not found: {JSON_PATH}", file=sys.stderr)
        sys.exit(1)

    raw = json.loads(JSON_PATH.read_text())
    entries = raw.get("results", raw if isinstance(raw, list) else [])

    # Filter and enrich valid entries
    valid_entries = []
    for e in entries:
        backend = e.get("backend", "unknown")
        func = e.get("function", "unknown")
        if backend == "unknown" or func == "unknown":
            continue

        if args.backend and backend != args.backend: continue
        if args.function and func != args.function: continue

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

        valid_entries.append(e)

    # Group by backend and function
    by_backend = defaultdict(lambda: defaultdict(list))
    for e in valid_entries:
        backend = e.get("backend", "unknown")
        func = e.get("function", "unknown")
        by_backend[backend][func].append(e)

    # Iterate over backends
    for backend, by_func in sorted(by_backend.items()):
        print("\n" + "=" * 90)
        print(f"BACKEND: {backend}")
        print("=" * 90)

        backend_entries = [e for funcs in by_func.values() for e in funcs]
        total = len(backend_entries)

        # Calculate percentages
        counts = defaultdict(int)
        for e in backend_entries:
            key = "overflow" if is_overflowed(e) else classify(e)
            counts[key] += 1

        print(f"Total evaluated inputs: {total}")
        print("-" * 50)
        print(f"{'Category':<30} | {'Count':<8} | {'Percentage'}")
        print("-" * 50)
        for key in ["faithful", "good", "acceptable", "severe", "noise", "overflow"]:
            c = counts[key]
            pct = (c / total * 100) if total > 0 else 0
            print(f"{LABELS[key]:<30} | {c:<8} | {pct:6.2f}%")
        print("-" * 50)

        # Display top 10 worst results per function
        print(f"\nTOP 10 WORST RESULTS PER FUNCTION ({backend})")

        for func, original_items in sorted(by_func.items()):
            items = original_items

            # Only severe filtering logic
            if args.only_severe:
                items = [it for it in items if classify(it) == "severe" or is_overflowed(it)]
                if not items:
                    continue
            else:
                # Always filter out machine noise from the top 10 list
                items = [it for it in items if classify(it) != "noise"]
                if not items:
                    continue

            print("\n" + "-" * 90)
            print(f"Function: {func} (Evaluated: {len(original_items)}, Shown: {len(items)})")
            print("-" * 90)

            # Sort items by worst ULP first
            def sort_key(x):
                u = x["_ulp"]
                return u if not math.isinf(u) and not math.isnan(u) else float('inf')

            sorted_items = sorted(items, key=sort_key, reverse=True)
            top_worst = sorted_items[:10]

            if backend == "rug":
                print(f"{'Rank':<10} | {'ULP':<10} | {'Prec':<6} | {'Abs Err':<12} | {'Input':<35} | {'Our Value':<25} | {'Reference':<25}")
                print("-" * 138)
            else:
                print(f"{'Rank':<10} | {'ULP':<10} | {'Abs Err':<12} | {'Input':<35} | {'Our Value':<25} | {'Reference':<25}")
                print("-" * 129)
            for i, e in enumerate(top_worst):
                u = e["_ulp"]
                uh_hp = e.get("exact_ulp_hp")
                if uh_hp is not None:
                    ulp_str = uh_hp if len(uh_hp) <= 12 else f"{u:.1f}"
                else:
                    ulp_str = f"{u:.1f}" if not math.isinf(u) and not math.isnan(u) else str(u)

                # Check for overflow/NaN explicitly in rank string if it's not a normal ULP error
                rank_str = f"#{i+1}"
                if is_overflowed(e):
                    rank_str += " (OVF)"
                elif classify(e) == "noise":
                    rank_str += " (NSE)"

                inp = e.get("input", "N/A")
                if isinstance(inp, float): inp = f"{inp:.6g}"

                extras = e.get("extras", [])
                if not extras:
                    inp_str = f"x={inp}"
                elif func == "spherical_harmonic" and len(extras) == 3:
                    inp_str = f"m={extras[0]:.4g}, n={extras[1]:.4g}, theta={extras[2]:.4g}, phi={inp}"
                elif func == "beta" and len(extras) == 1:
                    inp_str = f"a={inp}, b={extras[0]:.4g}"
                elif len(extras) == 1:
                    inp_str = f"n={extras[0]:.4g}, x={inp}"
                elif len(extras) == 2:
                    inp_str = f"m={extras[0]:.4g}, n={extras[1]:.4g}, x={inp}"
                else:
                    ext_strs = [f"p{j}={ext:.4g}" for j, ext in enumerate(extras)]
                    ext_strs.append(f"x={inp}")
                    inp_str = ", ".join(ext_strs)

                ae_hp = e.get("abs_error_hp")
                if ae_hp is not None:
                    ae_str = ae_hp if len(ae_hp) <= 20 else f"{ae_hp[:13]}..{ae_hp[-4:]}"
                else:
                    ae = e.get("abs_error")
                    if ae is None: ae_str = "N/A"
                    elif math.isinf(ae) or math.isnan(ae): ae_str = str(ae)
                    else: ae_str = f"{ae:.2e}"

                our_hp = e.get("our_value_hp")
                ref_hp = e.get("reference_hp")

                if our_hp is not None and ref_hp is not None:
                    our_str = our_hp
                    ref_str = ref_hp
                else:
                    our = e.get("our_value")
                    ref = e.get("reference")
                    bits = e.get("precision_bits", 53)
                    target_decimals = int(math.ceil(bits * 0.30103)) + 1
                    our_str = f"{our:.{target_decimals}g}" if isinstance(our, float) else str(our)
                    ref_str = f"{ref:.{target_decimals}g}" if isinstance(ref, float) else str(ref)

                if backend == "rug":
                    prec_str = str(e.get("precision_bits", "N/A"))
                    print(f"{rank_str:<10} | {ulp_str:<10} | {prec_str:<6} | {ae_str:<12} | {inp_str:<35} | {our_str:<25} | {ref_str:<25}")
                else:
                    print(f"{rank_str:<10} | {ulp_str:<10} | {ae_str:<12} | {inp_str:<35} | {our_str:<25} | {ref_str:<25}")
