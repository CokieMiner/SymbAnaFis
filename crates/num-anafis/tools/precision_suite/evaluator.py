import os
import sys
import math
import random
import shutil
import tempfile
import sysconfig
import subprocess
from collections import Counter
from pathlib import Path

try:
    import mpmath as mp
except ImportError:
    pass

try:
    # pyrefly: ignore [missing-import]
    import flint
    HAS_FLINT = True
except ImportError:
    HAS_FLINT = False

# pyrefly: ignore [missing-import]
from .config import (
    WORKSPACE_ROOT, MPMATH_MAP, FLINT_MAP, RUG_PREC_RANGE, TARGETED_SAMPLES, FUNCTIONS
)
# pyrefly: ignore [missing-import]
from .generators import generate_args, finalize_args, TARGETED_GENERATORS

def compute_references(name: str, args: list[float], prec_bits: int) -> dict[str, float]:
    refs = {}
    
    # 1. mpmath
    mp.mp.dps = math.ceil(prec_bits * 0.30103) + 5
    if HAS_FLINT:
        flint.ctx.prec = prec_bits + 10
    try:
        result = MPMATH_MAP[name](*[mp.mpf(x) for x in args])
        if isinstance(result, mp.mpc):
            result = result.real
        try:
            refs["mpmath"] = float(result)
        except OverflowError:
            refs["mpmath"] = float("inf") * float(mp.sign(result))
        refs["mpmath_mpf"] = result
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
    env["PYO3_PYTHON"] = sys.executable
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
    result = None
    try:
        a = module.s(primary_arg)
        scalar_args = [module.s(e) for e in extras]
        result = getattr(a, func_name)(*scalar_args)
        
        # Convert to float safely, handling overflow
        result_str = str(result)
        try:
            our_val = float(result_str)
        except OverflowError:
            our_val = float("inf") if not result_str.startswith("-") else float("-inf")
            
        if math.isinf(our_val) or math.isnan(our_val):
            overflow = math.isinf(our_val)
    except Exception:
        failed = True

    refs = compute_references(func_name, args, prec or 53)
    ref_val = refs.get("mpmath")

    abs_err = rel_err = exact_ulp = abs_err_hp = exact_ulp_hp = None
    
    our_val_str = None
    if not failed and our_val is not None:
        our_val_str = str(our_val) if backend != "rug" else str(result)
        
    if our_val_str is not None and refs.get("mpmath_mpf") is not None:
        try:
            p_bits = prec or 53
            orig_prec = mp.mp.prec
            mp.mp.prec = p_bits
            try:
                our_val_mp = mp.mpf(our_val_str)
                ref_val_mp = mp.mpf(refs["mpmath_mpf"])
                
                if mp.isinf(our_val_mp) and not mp.isinf(ref_val_mp):
                    abs_err = float("inf")
                    rel_err = float("inf")
                    exact_ulp = float("inf")
                    abs_err_hp = "inf"
                    exact_ulp_hp = "inf"
                elif mp.isinf(ref_val_mp) and not mp.isinf(our_val_mp):
                    abs_err = float("inf")
                    rel_err = float("inf")
                    exact_ulp = float("inf")
                    abs_err_hp = "inf"
                    exact_ulp_hp = "inf"
                else:
                    abs_err_mp = mp.fabs(our_val_mp - ref_val_mp)
                    abs_err_hp = str(abs_err_mp)
                    try:
                        abs_err = float(abs_err_mp)
                    except OverflowError:
                        abs_err = float("inf")
                        
                    if ref_val_mp != 0:
                        try:
                            rel_err = float(abs_err_mp / mp.fabs(ref_val_mp))
                        except OverflowError:
                            rel_err = float("inf")
                    else:
                        rel_err = abs_err
                    
                    # Compute exact ULP
                    if abs_err_mp == 0:
                        exact_ulp = 0.0
                        exact_ulp_hp = "0.0"
                    else:
                        emin = {53: -1022, 24: -126, 113: -16382}.get(p_bits)
                        if ref_val_mp == 0:
                            exp = emin if emin is not None else 0
                        else:
                            exp = int(mp.floor(mp.log(mp.fabs(ref_val_mp), 2)))
                            if emin is not None:
                                exp = max(exp, emin)

                        exact_ulp_mp = abs_err_mp * mp.power(2, p_bits - 1 - exp)
                        exact_ulp_hp = str(exact_ulp_mp)
                        try:
                            exact_ulp = float(exact_ulp_mp)
                        except OverflowError:
                            exact_ulp = float("inf")
            finally:
                mp.mp.prec = orig_prec
        except Exception:
            pass

    # Fallback if mpmath exact error calculation failed
    if abs_err is None and our_val is not None and ref_val is not None:
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
        "our_value_hp": our_val_str,
        "reference_hp": str(refs["mpmath_mpf"]) if refs.get("mpmath_mpf") is not None else None,
        "abs_error": abs_err, "abs_error_hp": abs_err_hp,
        "rel_error": rel_err, "exact_ulp": exact_ulp, "exact_ulp_hp": exact_ulp_hp,
        "flint_lo": flint_lo, "flint_hi": flint_hi, "flint_in_bounds": flint_in,
        "overflow": overflow,
        "targeted": targeted,
    }

    return record, failed

def run_tests(module, backend: str, seed: int, run_id: str, samples: int, function_filter: str = None):
    results = []
    failures = Counter()
    rng = random.Random(seed)

    default_prec = module.get_precision() if backend != "rug" else None
    print(f"    Precision: {default_prec if default_prec else f'random {RUG_PREC_RANGE}'} bits")

    filter_set = set(function_filter.split(',')) if function_filter else None

    for func_name, args_spec in FUNCTIONS.items():
        if filter_set and func_name not in filter_set:
            continue
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
