#!/usr/bin/env python3
"""
Classify results by ULP error into 4 bands:
  - Machine noise : abs_err <= NOISE_EPS_MULT * machine_eps * max(1.0, |ref|) (relative error floor)
  - ≤ 1  ULP      : faithfully rounded
  - ≤ 5  ULP      : good
  - ≤ 10 ULP      : acceptable
  - > 10 ULP      : severe
"""
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

JSON_PATH = Path(__file__).resolve().parent.parent / "verify_results.json"

NOISE_EPS_MULT = 4.0

def machine_eps(precision_bits: int) -> float:
    return NOISE_EPS_MULT ** (1 - precision_bits)   # f64: ~4.4e-16, f32: ~2.4e-7

def ulp_error(abs_err, ref, precision_bits):
    if abs_err == 0: return 0.0
    if math.isinf(abs_err) or math.isnan(abs_err): return float("inf")
    if math.isinf(ref)     or math.isnan(ref):     return float("inf")

    emin = {53: -1022, 24: -126, 113: -16382}.get(precision_bits)
    if ref == 0:
        exp = emin if emin is not None else None
        if exp is None: return float("inf")
    else:
        exp = math.floor(math.log2(abs(ref)))
        if emin is not None:
            exp = max(exp, emin)
    try:
        return math.ldexp(abs_err, precision_bits - 1 - exp)
    except OverflowError:
        return float("inf")

def is_correct_overflow(e):
    our, ref = e.get("our_value"), e.get("reference")
    if our is None or ref is None: return False
    
    if math.isnan(our) and math.isnan(ref): return True
    if math.isnan(our) or math.isnan(ref): return False
    
    bits = e.get("precision_bits", 53)
    max_val = 3.4028234663852886e+38 if bits == 24 else 1.7976931348623157e+308
    
    if math.isinf(our) and math.isinf(ref) and (our > 0) == (ref > 0):
        return True
        
    if math.isinf(our) and not math.isinf(ref):
        if our > 0 and ref >= max_val: return True
        if our < 0 and ref <= -max_val: return True
        
    if math.isinf(ref) and not math.isinf(our):
        if ref > 0 and our >= max_val: return True
        if ref < 0 and our <= -max_val: return True
        
    return False

def is_overflowed(e):
    if is_correct_overflow(e):
        return False
    our, ref = e.get("our_value"), e.get("reference")
    if our is None or ref is None: return False
    if e.get("overflow"): return True
    if math.isinf(our) or math.isnan(our): return True
    if math.isinf(ref) or math.isnan(ref): return True
    ae = e.get("abs_error")
    return ae is not None and math.isinf(ae)

def rel_error_func(e):
    ae  = e.get("abs_error") or 0
    ref = e.get("reference") or 1
    if ref == 0: return float("inf")
    bits = e.get("precision_bits", 53)
    min_subnormal = 2**-1074 if bits == 53 else (2**-149 if bits == 24 else 0)
    if e.get("our_value") == 0.0 and abs(ref) <= min_subnormal:
        return 0.0
    return ae / abs(ref)

def classify(entry):
    if is_correct_overflow(entry):
        return "faithful"
        
    ae   = entry.get("abs_error", 0) or 0
    ref  = entry.get("reference", 1) or 1
    bits = entry.get("precision_bits", 53)
    ulp  = entry["_ulp"]
    eps  = 2.0 ** (1 - bits)

    if ulp <= 1:  return "faithful"
    
    if ae <= NOISE_EPS_MULT * eps:
        return "noise"

    if ulp <= 5:  return "good"
    if ulp <= 10: return "acceptable"

    if ae <= NOISE_EPS_MULT * eps * max(1.0, abs(ref)):
        return "noise"
        
    return "severe"

import argparse

def format_hex_float(val):
    if val is None or math.isinf(val) or math.isnan(val):
        return str(val)
    try:
        return float(val).hex()
    except Exception:
        return str(val)

def main():
    parser = argparse.ArgumentParser(description="Analyze ULP results from verify_results.json")
    parser.add_argument("--backend", help="Filter by backend (e.g., f32, f64)", type=str)
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
        ae, ref = e.get("abs_error"), e.get("reference")
        if ae is None or ref is None: continue
        
        backend = e.get("backend", "unknown")
        func = e.get("function", "unknown")
        
        if args.backend and backend != args.backend: continue
        if args.function and func != args.function: continue
        
        if is_correct_overflow(e):
            e["_ulp"] = 0.0
        elif backend == "rug" and e.get("flint_in_bounds") is True:
            e["_ulp"] = 0.0
        else:
            e["_ulp"] = ulp_error(ae, ref, e.get("precision_bits", 53))
            
        valid_entries.append(e)

    # Group by backend and function
    by_backend = defaultdict(lambda: defaultdict(list))
    for e in valid_entries:
        backend = e.get("backend", "unknown")
        func = e.get("function", "unknown")
        by_backend[backend][func].append(e)

    LABELS = {
        "faithful":   "Faithfully rounded (≤ 1 ULP)",
        "good":       "Good (≤ 5 ULP)",
        "acceptable": "Acceptable (≤ 10 ULP)",
        "severe":     "Severe (> 10 ULP)",
        "noise":      "Machine noise",
        "overflow":   "Overflow/NaN",
    }

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
                
            print(f"{'Rank':<10} | {'ULP':<10} | {'Abs Err':<12} | {'Input':<35} | {'Our Value':<25} | {'Reference':<25}")
            print("-" * 129)
            for i, e in enumerate(top_worst):
                u = e["_ulp"]
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
                
                ae = e.get("abs_error")
                if ae is None: ae_str = "N/A"
                elif math.isinf(ae) or math.isnan(ae): ae_str = str(ae)
                else: ae_str = f"{ae:.2e}"
                
                our = e.get("our_value")
                ref = e.get("reference")
                
                bits = e.get("precision_bits", 53)
                target_decimals = int(math.ceil(bits * 0.30103)) + 1
                our_str = f"{our:.{target_decimals}g}" if isinstance(our, float) else str(our)
                ref_str = f"{ref:.{target_decimals}g}" if isinstance(ref, float) else str(ref)
                
                print(f"{rank_str:<10} | {ulp_str:<10} | {ae_str:<12} | {inp_str:<35} | {our_str:<25} | {ref_str:<25}")

if __name__ == "__main__":
    main()
