import math
import sys
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
    "32" : ("python,backend32", False),
    "64" : ("python,backend64", False),
    # "rug": ("python,backendrug", True),
}

MPMATH_PREC = 50
RUG_PREC_RANGE = (53, 2048)
SAMPLES = 10000
TARGETED_SAMPLES = 2000
RUG_SAMPLES = 800

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
    "lambert_w":          [(-1/math.e + 1e-7, 1000, False), (-1/math.e + 1e-12, -1e-8, False)],
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
    "lambert_w": lambda x, k: mp.lambertw(x, k),
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

FLINT_MAP = {}
if HAS_FLINT:
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
        "lambert_w": lambda x, k: flint.arb(x).lambertw(k),
        "sinc":      lambda x: flint.arb(x).sinc(),
        "hermite":   lambda x, n: flint.arb(x).hermite_h(int(n)),
        "assoc_legendre": lambda x, l, m: flint.arb(x).legendre_p(int(l), int(m)),
        "beta":      lambda x, y: flint.arb(x).gamma() * flint.arb(y).gamma() / flint.arb(x+y).gamma(),
    }

# --- Paths ---
SCRIPT_DIR = Path(__file__).resolve().parent.parent
CRATE_DIR = SCRIPT_DIR.parent
JSON_PATH = CRATE_DIR / "tools" / "results" / "verify_results.json"

def _find_workspace_root() -> Path:
    d = CRATE_DIR.resolve()
    while d != d.parent:
        if (d / "Cargo.toml").exists() and "[workspace]" in (d / "Cargo.toml").read_text():
            return d
        d = d.parent
    return CRATE_DIR

WORKSPACE_ROOT = _find_workspace_root()
