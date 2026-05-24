import json
import random
import argparse
from datetime import datetime, timezone

from .config import BACKENDS, JSON_PATH, SAMPLES, RUG_SAMPLES
from .evaluator import build_and_import_backend, run_tests

def run_verification_cli():
    ap = argparse.ArgumentParser(description="Precision verification runner")
    ap.add_argument("--seed", type=int, default=random.randint(0, 2**31))
    ap.add_argument("--samples", type=int, default=SAMPLES)
    args = ap.parse_args()

    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    print(
        f"Run: {run_id} | Seed: {args.seed} | Samples: {args.samples}/func | "
        f"Backends: {list(BACKENDS)}"
    )

    all_results = []

    for label, (features, no_default) in BACKENDS.items():
        print(f"\n=== Backend: {label} ({features}) ===")
        try:
            mod = build_and_import_backend(label, features, no_default)
            backend_samples = RUG_SAMPLES if label == "rug" else args.samples
            results, failures = run_tests(mod, label, args.seed, run_id, backend_samples)
            all_results.extend(results)
            print(f"    {len(results)} results")
            if failures: print(f"    Failures: {failures}")

            JSON_PATH.parent.mkdir(parents=True, exist_ok=True)
            with open(JSON_PATH, "w") as f:
                json.dump({"results": all_results}, f, indent=2)
        except Exception as e:
            print(f"    FAILED: {e}")

    print(f"\nTotal {len(all_results)} results in {JSON_PATH}")
