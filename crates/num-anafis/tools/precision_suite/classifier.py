import math
import mpmath as mp

NOISE_EPS_MULT = 5.0

LABELS = {
    "faithful":   "Faithfully rounded (≤ 1 ULP)",
    "good":       "Good (≤ 5 ULP)",
    "acceptable": "Acceptable (≤ 10 ULP)",
    "severe":     "Severe (> 10 ULP)",
    "noise":      "Machine noise",
    "overflow":   "Overflow/NaN",
}

POLES: dict = {
    "gamma":      lambda xs: xs[0].is_integer() and xs[0] <= 0,
    "lgamma":     lambda xs: xs[0].is_integer() and xs[0] <= 0,
    "digamma":    lambda xs: xs[0].is_integer() and xs[0] <= 0,
    "trigamma":   lambda xs: xs[0].is_integer() and xs[0] <= 0,
    "tetragamma": lambda xs: xs[0].is_integer() and xs[0] <= 0,
    "polygamma":  lambda xs: xs[0].is_integer() and xs[0] <= 0,
    "zeta":       lambda xs: xs[0] == 1.0,
    "zeta_deriv": lambda xs: xs[0] == 1.0,
    "beta":       lambda xs: (xs[0].is_integer() and xs[0] <= 0) or
                             (xs[1].is_integer() and xs[1] <= 0),
    "elliptic_k": lambda xs: xs[0] >= 1.0 or xs[0] <= -1.0,
}

def _is_at_pole(e: dict) -> bool:
    check = POLES.get(e.get("function", ""))
    if check is None:
        return False
    primary = e.get("input")
    extras = e.get("extras", [])
    if primary is None:
        return False
    args = [primary] + extras
    try:
        return check(args)
    except Exception:
        return False

def is_correct_overflow(e: dict) -> bool:
    our, ref = e.get("our_value"), e.get("reference")
    if our is None or ref is None: return False
    
    if math.isnan(our) and math.isnan(ref): return True
    if math.isnan(our) or math.isnan(ref): return False
    
    bits = e.get("precision_bits", 53)
    
    if math.isinf(our) and math.isinf(ref) and (our > 0) == (ref > 0):
        return True
    
    # Our value is infinite at a known mathematical pole → faithful
    if math.isinf(our) and not math.isinf(ref) and _is_at_pole(e):
        return True
    
    # Finite-range overflow check only meaningful for IEEE 754 formats
    if bits not in (24, 53, 113):
        return False
    
    max_val = 3.4028234663852886e+38 if bits == 24 else 1.7976931348623157e+308
    
    if math.isinf(our) and not math.isinf(ref):
        if our > 0 and ref >= max_val: return True
        if our < 0 and ref <= -max_val: return True
        
    if math.isinf(ref) and not math.isinf(our):
        if ref > 0 and our >= max_val: return True
        if ref < 0 and our <= -max_val: return True
        
    return False

def is_overflowed(e: dict) -> bool:
    if is_correct_overflow(e):
        return False
    our, ref = e.get("our_value"), e.get("reference")
    if our is None or ref is None: return False
    if e.get("overflow"): return True
    if math.isinf(our) or math.isnan(our): return True
    if math.isinf(ref) or math.isnan(ref): return True
    ae = e.get("abs_error")
    return ae is not None and math.isinf(ae)

def ulp_error(abs_err: float, ref: float, precision_bits: int) -> float:
    if abs_err == 0: return 0.0
    if math.isinf(abs_err) or math.isnan(abs_err): return float("inf")
    if math.isinf(ref)     or math.isnan(ref):     return float("inf")

    emin = {53: -1022, 24: -126, 113: -16382}.get(precision_bits)
    if ref == 0:
        exp = emin if emin is not None else 0
    else:
        exp = math.floor(math.log2(abs(ref)))
        if emin is not None:
            exp = max(exp, emin)
    try:
        return math.ldexp(abs_err, precision_bits - 1 - exp)
    except OverflowError:
        return float("inf")

def rel_error_func(e: dict) -> float:
    ae  = e.get("abs_error") or 0
    ref = e.get("reference") or 1
    if ref == 0: return float("inf")
    bits = e.get("precision_bits", 53)
    min_subnormal = 2**-1074 if bits == 53 else (2**-149 if bits == 24 else 0)
    if e.get("our_value") == 0.0 and abs(ref) <= min_subnormal:
        return 0.0
    return ae / abs(ref)

def is_noise(entry: dict) -> bool:
    bits = entry.get("precision_bits", 53)
    ae_hp = entry.get("abs_error_hp")
    if ae_hp is not None:
        try:
            ae_mp = mp.mpf(ae_hp)
            eps_mp = mp.power(2, 1 - bits)
            ref_hp = entry.get("reference_hp")
            if ae_mp > 0 and ae_mp <= NOISE_EPS_MULT * eps_mp:
                return True
            if ref_hp is not None:
                ref_mp = mp.mpf(ref_hp)
                if ae_mp <= NOISE_EPS_MULT * eps_mp * max(1, mp.fabs(ref_mp)):
                    return True
        except Exception:
            pass

    ae  = entry.get("abs_error", 0) or 0
    ref = entry.get("reference", 1) or 1
    eps = 2.0 ** (1 - bits)

    if eps > 0 and ae <= NOISE_EPS_MULT * eps:
        return True
    if eps > 0 and ae <= NOISE_EPS_MULT * eps * max(1.0, abs(ref)):
        return True
    return False


def classify(entry: dict) -> str:
    if is_correct_overflow(entry):
        return "faithful"

    ulp = entry["_ulp"]

    if ulp <= 1:  return "faithful"
    if ulp <= 5:  return "good"
    if ulp <= 10: return "acceptable"
    if is_noise(entry):
        return "noise"
    return "severe"
