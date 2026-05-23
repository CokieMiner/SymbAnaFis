#!/usr/bin/env python3
"""
Precision verification for num-anafis custom special-function implementations.
"""
import json
import math
import os
import random
import shutil
import struct
import subprocess
import sys
import sysconfig
import tempfile
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

try:
    import mpmath as mp
except ImportError:
    sys.exit("mpmath required: uv pip install mpmath")

try:
    # pyrefly: ignore [missing-import]
    import flint
    HAS_FLINT = True
except ImportError:
    HAS_FLINT = False

# --- Configuration ---
BACKENDS = {
    # "f64": ("python,backend64", False),
    # "f32": ("python,backend32", True),
    "rug": ("python,backendrug", True),
}

MPMATH_PREC = 50
RUG_PREC_RANGE = (53, 2048)
SAMPLES = 800
TARGETED_SAMPLES = 200

TARGETED_POLE_COUNT = 30
TARGETED_POLE_EPS_RANGE = (1e-10, 1e-2)
TARGETED_POLYGAMMA_ORDER_RANGE = (0, 5)
TARGETED_ZETA_EPS_RANGE = (1e-10, 1e-2)
TARGETED_SINC_EPS_RANGE = (1e-12, 1e-2)
TARGETED_ELLIPTIC_U_RANGE = (1.0, 6.0)
TARGETED_BESSEL_ORDER_RANGE = (0, 100)
TARGETED_BESSEL_SMALL_X_RANGE = (1e-12, 1e-3)

# Defines arguments as: [(lo, hi, is_int), ...]
FUNCTIONS = {
    "erf":                [(-10, 10, False)],
    "erfc":               [(-10, 30, False)],
    "gamma":              [(-30.5, 200.0, False)],
    "lgamma":             [(-30.5, 30.5, False)],
    "digamma":            [(-30.5, 30.5, False)],
    "trigamma":           [(-30.5, 30.5, False)],
    "tetragamma":         [(-30.5, 30.5, False)],
    "zeta":               [(-20.5, 50, False)],
    "lambert_w":          [(-1/math.e + 1e-7, 1000, False)],
    "lambert_wm1":        [(-1/math.e + 1e-12, -1e-8, False)],
    "sinc":               [(-100, 100, False)],
    "elliptic_k":         [(-0.999, 0.999, False)],
    "elliptic_e":         [(-0.999, 0.999, False)],
    "bessel_j":           [(-100, 100, False), (-10, 10, True)],
    "bessel_y":           [(0.001, 100, False), (-10, 10, True)],
    "bessel_i":           [(-100, 100, False), (-10, 10, True)],
    "bessel_k":           [(0.001, 100, False), (-10, 10, True)],
    "polygamma":          [(-15.5, 15.5, False), (0, 5, True)],
    "beta":               [(-15.5, 15.5, False), (-15.5, 15.5, False)],
    "zeta_deriv":         [(-10.5, 5, False), (0, 20, True)],
    "hermite":            [(-10, 10, False), (0, 10, True)],
    "assoc_legendre":     [(-0.999, 0.999, False), (0, 10, True), (-10, 10, True)],
    "spherical_harmonic": [(0, 3.14, False), (0, 10, True), (-10, 10, True), (0, 6.28, False)],
}

MPMATH_MAP = {
    "sinc": mp.sinc,
    "erf": mp.erf, "erfc": mp.erfc,
    "gamma": mp.gamma, "lgamma": mp.loggamma,
    "digamma": mp.digamma,
    "trigamma": lambda x: mp.polygamma(1, x),
    "tetragamma": lambda x: mp.polygamma(2, x),
    "zeta": mp.zeta,
    "lambert_w": mp.lambertw,
    "lambert_wm1": lambda x: mp.lambertw(x, -1),
    "elliptic_k": mp.ellipk, "elliptic_e": mp.ellipe,
    "bessel_j": lambda x, n: mp.besselj(int(n), x), "bessel_y": lambda x, n: mp.bessely(int(n), x),
    "bessel_i": lambda x, n: mp.besseli(int(n), x), "bessel_k": lambda x, n: mp.besselk(int(n), x),
    "polygamma": lambda x, n: mp.polygamma(round(n), x),
    "beta": mp.beta,
    "zeta_deriv": lambda x, n: mp.zeta(x, derivative=round(n)),
    "hermite": lambda x, n: mp.hermite(int(n), x),
    "assoc_legendre": lambda x, l, m: mp.legenp(int(l), int(m), x),
    "spherical_harmonic": lambda theta, l, m, phi: mp.spherharm(int(l), int(m), theta, phi),
}



FLINT_MAP = {
    "erf":       lambda x: flint.arb(x).erf(),
    "erfc":      lambda x: flint.arb(x).erfc(),
    "gamma":     lambda x: flint.arb(x).gamma(),
    "lgamma":    lambda x: flint.arb(x).lgamma(),
    "digamma":   lambda x: flint.arb(x).digamma(),
    "polygamma": lambda x, n: flint.arb(x).polygamma(int(n)),
    "zeta":      lambda x: flint.arb(x).zeta(),
    "bessel_j":  lambda x, n: flint.arb(x).bessel_j(int(n)),
    "bessel_y":  lambda x, n: flint.arb(x).bessel_y(int(n)),
    "bessel_i":  lambda x, n: flint.arb(x).bessel_i(int(n)),
    "bessel_k":  lambda x, n: flint.arb(x).bessel_k(int(n)),
    "lambert_w": lambda x: flint.arb(x).lambertw(),
    "lambert_wm1": lambda x: flint.arb(x).lambertw(-1),
    "sinc":      lambda x: flint.arb(x).sinc(),
    "hermite":   lambda x, n: flint.arb(x).hermite_h(int(n)),
    "assoc_legendre": lambda x, l, m: flint.arb(x).legendre_p(int(l), int(m)),
    "beta":      lambda x, y: flint.arb(x).gamma() * flint.arb(y).gamma() / flint.arb(x+y).gamma(),
}

# --- Paths ---
SCRIPT_DIR = Path(__file__).resolve().parent
CRATE_DIR = SCRIPT_DIR.parent
OUTPUT = CRATE_DIR / "verify_results.json"
PYTHON_EXE = Path(sys.executable).resolve()

def _find_workspace_root() -> Path:
    d = CRATE_DIR.resolve()
    while d != d.parent:
        if (d / "Cargo.toml").exists() and "[workspace]" in (d / "Cargo.toml").read_text():
            return d
        d = d.parent
    return CRATE_DIR

WORKSPACE_ROOT = _find_workspace_root()

# --- Core Logic ---
def to_f32(val: float) -> float:
    return struct.unpack('f', struct.pack('f', float(val)))[0]

def log_uniform(rng: random.Random, lo: float, hi: float) -> float:
    if lo <= 0 or hi <= 0:
        raise ValueError("log_uniform requires positive bounds")
    lo_log = math.log10(lo)
    hi_log = math.log10(hi)
    return 10 ** rng.uniform(lo_log, hi_log)

def random_sign(rng: random.Random) -> float:
    return -1.0 if rng.random() < 0.5 else 1.0

def finalize_args(
    func_name: str,
    args: list[float],
    args_spec: list,
    backend: str,
) -> list[float]:
    normalized = []
    for value, (_, _, is_int) in zip(args, args_spec):
        if is_int:
            value = float(round(value))
        elif backend == "f32":
            value = to_f32(value)
        normalized.append(value)

    # Special enforcements for associated legendre and spherical harmonic
    if func_name in ("assoc_legendre", "spherical_harmonic"):
        l, m = normalized[1], normalized[2]
        if abs(m) > l:
            normalized[2] = float(int(m) % (int(l) + 1))

    return normalized

def generate_args(func_name: str, args_spec: list, rng: random.Random, backend: str) -> list[float]:
    args = [rng.uniform(lo, hi) for (lo, hi, _) in args_spec]
    return finalize_args(func_name, args, args_spec, backend)

def pole_count_for(func_name: str) -> int:
    lo = FUNCTIONS[func_name][0][0]
    max_k = int(abs(math.ceil(lo))) if lo < 0 else 0
    return max(1, min(TARGETED_POLE_COUNT, max_k))

def sample_near_negative_integer(rng: random.Random, max_k: int | None = None) -> float:
    if max_k is None:
        max_k = TARGETED_POLE_COUNT
    max_k = max(1, min(TARGETED_POLE_COUNT, max_k))
    k = rng.randint(1, max_k)
    eps = log_uniform(rng, *TARGETED_POLE_EPS_RANGE)
    return -float(k) + random_sign(rng) * eps

def sample_zeta_near_one(rng: random.Random) -> float:
    eps = log_uniform(rng, *TARGETED_ZETA_EPS_RANGE)
    return 1.0 + random_sign(rng) * eps

def sample_sinc_near_zero(rng: random.Random) -> float:
    if rng.random() < 0.05:
        return 0.0
    eps = log_uniform(rng, *TARGETED_SINC_EPS_RANGE)
    return random_sign(rng) * eps

def sample_elliptic_near_one(rng: random.Random) -> float:
    u_lo, u_hi = TARGETED_ELLIPTIC_U_RANGE
    u = rng.uniform(u_lo, u_hi)
    base = 1.0 - 10 ** (-u)
    return random_sign(rng) * base

def sample_bessel_order(rng: random.Random) -> float:
    o_lo, o_hi = TARGETED_BESSEL_ORDER_RANGE
    n = rng.randint(o_lo, o_hi)
    if rng.random() < 0.5:
        n = -n
    return float(n)

def sample_bessel_large_order(rng: random.Random, func_name: str) -> list[float]:
    bessel_x_ranges = {
        "bessel_j": (-100.0, 100.0),
        "bessel_i": (-100.0, 100.0),
        "bessel_y": (0.001, 100.0),
        "bessel_k": (0.001, 100.0),
    }
    x_lo, x_hi = bessel_x_ranges[func_name]
    x = rng.uniform(x_lo, x_hi)
    return [x, sample_bessel_order(rng)]

def sample_bessel_small_x(rng: random.Random) -> list[float]:
    x = log_uniform(rng, *TARGETED_BESSEL_SMALL_X_RANGE)
    return [x, sample_bessel_order(rng)]

def sample_bessel_targeted(rng: random.Random, func_name: str) -> list[float]:
    r = rng.random()
    if r < 0.33:
        return sample_bessel_large_order(rng, func_name)
    elif r < 0.66:
        return sample_bessel_small_x(rng)
    else:
        x = rng.uniform(500.0, 1000.0)
        if func_name in ("bessel_j", "bessel_i") and rng.random() < 0.5:
            x = -x
        return [x, sample_bessel_order(rng)]

def sample_beta_pole_args(rng: random.Random) -> list[float]:
    if rng.random() < 0.5:
        x = sample_near_negative_integer(rng, pole_count_for("beta"))
        y = rng.uniform(-15.5, 15.5)
    else:
        y = sample_near_negative_integer(rng, pole_count_for("beta"))
        x = rng.uniform(-15.5, 15.5)
    return [x, y]

def sample_pole_args(rng: random.Random, func_name: str) -> list[float]:
    return [sample_near_negative_integer(rng, pole_count_for(func_name))]

def sample_polygamma_args(rng: random.Random) -> list[float]:
    o_lo, o_hi = TARGETED_POLYGAMMA_ORDER_RANGE
    order = rng.randint(o_lo, o_hi)
    return [sample_near_negative_integer(rng, pole_count_for("polygamma")), float(order)]

def sample_zeta_args(rng: random.Random) -> list[float]:
    return [sample_zeta_near_one(rng)]

def sample_sinc_args(rng: random.Random) -> list[float]:
    return [sample_sinc_near_zero(rng)]

def sample_elliptic_args(rng: random.Random) -> list[float]:
    return [sample_elliptic_near_one(rng)]

TARGETED_GENERATORS = {
    "gamma": lambda rng: sample_pole_args(rng, "gamma"),
    "lgamma": lambda rng: sample_pole_args(rng, "lgamma"),
    "digamma": lambda rng: sample_pole_args(rng, "digamma"),
    "trigamma": lambda rng: sample_pole_args(rng, "trigamma"),
    "tetragamma": lambda rng: sample_pole_args(rng, "tetragamma"),
    "polygamma": sample_polygamma_args,
    "elliptic_k": sample_elliptic_args,
    "elliptic_e": sample_elliptic_args,
    "zeta": sample_zeta_args,
    "zeta_deriv": lambda rng: [sample_zeta_near_one(rng), rng.randint(0, 20)],
    "beta": sample_beta_pole_args,
    "sinc": sample_sinc_args,
    "bessel_j": lambda rng: sample_bessel_targeted(rng, "bessel_j"),
    "bessel_i": lambda rng: sample_bessel_targeted(rng, "bessel_i"),
    "bessel_y": lambda rng: sample_bessel_targeted(rng, "bessel_y"),
    "bessel_k": lambda rng: sample_bessel_targeted(rng, "bessel_k"),
}

def compute_references(name: str, args: list[float], prec_bits: int) -> dict[str, float]:
    refs = {}
    
    # 1. mpmath
    mp.mp.dps = int(math.ceil(prec_bits * 0.30103)) + 5
    if HAS_FLINT:
        flint.ctx.prec = prec_bits + 10
    try:
        result = MPMATH_MAP[name](*[mp.mpf(x) for x in args])
        # Take real part for functions that may return complex (lgamma, etc.)
        if isinstance(result, mp.mpc):
            result = result.real
        refs["mpmath"] = float(result)
    except Exception:
        pass
        
    # 2. flint
    if HAS_FLINT:
        try:
            if name == "trigamma":
                arb = flint.arb(args[0]).polygamma(1)
            elif name == "tetragamma":
                arb = flint.arb(args[0]).polygamma(2)
            elif name in FLINT_MAP:
                arb = FLINT_MAP[name](*args)
            else:
                arb = None
            if arb is not None:
                refs["flint"] = float(arb.mid())
        except Exception:
            pass

    return refs

def compute_flint_bounds(name: str, args: list[float], prec_bits: int) -> tuple[float, float] | None:
    if not HAS_FLINT:
        return None
    flint.ctx.prec = prec_bits + 10
    try:
        if name == "trigamma":
            arb = flint.arb(args[0]).polygamma(1)
        elif name == "tetragamma":
            arb = flint.arb(args[0]).polygamma(2)
        elif name in FLINT_MAP:
            arb = FLINT_MAP[name](*args)
        else:
            return None
        return float(arb.mid()) - float(arb.rad()), float(arb.mid()) + float(arb.rad())
    except Exception:
        return None

def build_and_import_backend(label: str, features: str, no_default: bool):
    print(f"  Building {label} with features: {features}...", flush=True)
    cmd = ["cargo", "build", "--features", features, "-p", "num-anafis"]
    if no_default:
        cmd.append("--no-default-features")
        
    env = os.environ.copy()
    env["PYO3_PYTHON"] = str(PYTHON_EXE)
    subprocess.run(cmd, cwd=WORKSPACE_ROOT, check=True, env=env)
    
    target_dir = WORKSPACE_ROOT / "target" / "debug"
    so_path = next(target_dir.glob("libnum_anafis*.so"))
    
    import importlib
    ext = sysconfig.get_config_var("EXT_SUFFIX") or ".so"
    backend_dir = Path(tempfile.mkdtemp(prefix=f"na_{label}_"))
    shutil.copy2(so_path, backend_dir / f"num_anafis_py{ext}")
    sys.path.insert(0, str(backend_dir))
    importlib.invalidate_caches()
    sys.modules.pop("num_anafis_py", None)
    return importlib.import_module("num_anafis_py")

def evaluate_case(
    module,
    func_name: str,
    args: list[float],
    backend: str,
    rng: random.Random,
    default_prec: int | None,
    run_id: str,
    targeted: bool = False,
) -> tuple[dict, bool]:
    prec = default_prec
    if backend == "rug":
        prec = rng.randint(*RUG_PREC_RANGE)
        module.Scalar.set_precision(prec)

    primary_arg, *extras = args

    our_val = None
    overflow = False
    failed = False
    try:
        a = module.s(primary_arg)
        scalar_args = [module.s(e) for e in extras]
        result = getattr(a, func_name)(*scalar_args)
        our_val = float(str(result))
        if math.isinf(our_val) or math.isnan(our_val):
            overflow = math.isinf(our_val)
    except Exception:
        failed = True

    refs = compute_references(func_name, args, prec or 53)
    ref_val = refs.get("mpmath")

    abs_err = rel_err = None
    if our_val is not None and ref_val is not None:
        if math.isinf(our_val) and not math.isinf(ref_val):
            abs_err = float("inf")
            rel_err = float("inf")
        elif math.isinf(ref_val) and not math.isinf(our_val):
            abs_err = float("inf")
            rel_err = float("inf")
        else:
            abs_err = abs(our_val - ref_val)
            rel_err = abs_err / max(abs(ref_val), 1e-300) if ref_val != 0 else abs_err

    flint_lo = flint_hi = flint_in = None
    if backend == "rug" and HAS_FLINT:
        bounds = compute_flint_bounds(func_name, args, prec or 53)
        if bounds and our_val is not None:
            flint_lo, flint_hi = bounds
            flint_in = (flint_lo <= our_val <= flint_hi)

    record = {
        "run_id": run_id, "backend": backend, "precision_bits": prec,
        "function": func_name, "input": primary_arg, "extras": extras,
        "our_value": our_val, "reference": ref_val,
        "refs_all": refs,
        "abs_error": abs_err, "rel_error": rel_err,
        "flint_lo": flint_lo, "flint_hi": flint_hi, "flint_in_bounds": flint_in,
        "overflow": overflow,
        "targeted": targeted,
    }

    return record, failed

def run_tests(module, backend: str, seed: int, run_id: str, samples: int):
    results = []
    failures = Counter()
    rng = random.Random(seed)

    default_prec = module.get_precision() if backend != "rug" else None
    print(f"    Precision: {default_prec if default_prec else f'random {RUG_PREC_RANGE}'} bits")

    for func_name, args_spec in FUNCTIONS.items():
        print(f"    Testing {func_name}...", flush=True)
        for _ in range(samples):
            args = generate_args(func_name, args_spec, rng, backend)
            record, failed = evaluate_case(
                module,
                func_name,
                args,
                backend,
                rng,
                default_prec,
                run_id,
                targeted=False,
            )
            results.append(record)
            if failed:
                failures[func_name] += 1

        targeted_gen = TARGETED_GENERATORS.get(func_name)
        if targeted_gen and TARGETED_SAMPLES > 0:
            for _ in range(TARGETED_SAMPLES):
                args = targeted_gen(rng)
                args = finalize_args(func_name, args, args_spec, backend)
                record, failed = evaluate_case(
                    module,
                    func_name,
                    args,
                    backend,
                    rng,
                    default_prec,
                    run_id,
                    targeted=True,
                )
                results.append(record)
                if failed:
                    failures[func_name] += 1

    return results, dict(failures)

def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=random.randint(0, 2**31))
    ap.add_argument("--samples", type=int, default=SAMPLES)
    args = ap.parse_args()

    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    print(
        f"Run: {run_id} | Seed: {args.seed} | Samples: {args.samples}/func + {TARGETED_SAMPLES} targeted | "
        f"Backends: {list(BACKENDS)}"
    )

    all_results = []
    if OUTPUT.exists():
        try:
            with open(OUTPUT) as f:
                all_results = json.load(f).get("results", [])
        except Exception:
            pass

    for label, (features, no_default) in BACKENDS.items():
        print(f"\n=== Backend: {label} ({features}) ===")
        try:
            mod = build_and_import_backend(label, features, no_default)
            results, failures = run_tests(mod, label, args.seed, run_id, args.samples)
            all_results.extend(results)
            print(f"    {len(results)} results")
            if failures: print(f"    Failures: {failures}")
            
            with open(OUTPUT, "w") as f:
                json.dump({"results": all_results}, f, indent=2)
        except Exception as e:
            print(f"    FAILED: {e}")

    print(f"\nTotal {len(all_results)} results in {OUTPUT}")

if __name__ == "__main__":
    main()
