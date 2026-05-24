import math
import random
import struct
from .config import (
    FUNCTIONS, TARGETED_POLE_COUNT, TARGETED_POLE_EPS_RANGE,
    TARGETED_ZETA_EPS_RANGE, TARGETED_SINC_EPS_RANGE, TARGETED_ELLIPTIC_U_RANGE,
    TARGETED_BESSEL_ORDER_RANGE, TARGETED_BESSEL_SMALL_X_RANGE, TARGETED_SAMPLES,
    TARGETED_POLYGAMMA_ORDER_RANGE
)

def to_f32(val: float) -> float:
    return struct.unpack('f', struct.pack('f', val))[0]

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
        elif backend == "32":
            value = to_f32(value)
        normalized.append(value)

    # Special enforcements for associated legendre and spherical harmonic
    if func_name in ("assoc_legendre", "spherical_harmonic"):
        l, m = normalized[1], normalized[2]
        if abs(m) > l:
            normalized[2] = float(int(m) % (int(l) + 1))

    return normalized

def _sample_arg(lo: float, hi: float, rng: random.Random) -> float:
    width = hi - lo
    if width <= 0:
        return lo
    r = rng.random()
    if r < 0.20:
        return rng.uniform(lo, lo + 0.05 * width)
    elif r < 0.40:
        return rng.uniform(hi - 0.05 * width, hi)
    else:
        return rng.uniform(lo, hi)

def generate_args(func_name: str, args_spec: list, rng: random.Random, backend: str) -> list[float]:
    args = [_sample_arg(lo, hi, rng) for (lo, hi, _) in args_spec]
    return finalize_args(func_name, args, args_spec, backend)

def pole_count_for(func_name: str) -> int:
    lo = FUNCTIONS[func_name][0][0]
    max_k = abs(math.ceil(lo)) if lo < 0 else 0
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
