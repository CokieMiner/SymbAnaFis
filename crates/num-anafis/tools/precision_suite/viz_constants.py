import math

from .config import JSON_PATH # type: ignore

CHARTS_DIR = JSON_PATH.parent / "charts"

CLASS_COLORS = {
    "faithful":   "#2ecc71",
    "good":       "#f1c40f",
    "acceptable": "#e67e22",
    "severe":     "#e74c3c",
    "noise":      "#95a5a6",
    "overflow":   "#7f8c8d",
}

CLASS_ORDER = ["faithful", "good", "acceptable", "severe", "noise", "overflow"]
CLASS_MARKERS = {
    "faithful": ".",
    "good": ".",
    "acceptable": ".",
    "severe": ".",
    "noise": ".",
    "overflow": "x",
}

SINGLE_ARG = {
    "erf", "erfc", "gamma", "lgamma", "digamma", "trigamma", "tetragamma",
    "zeta", "sinc", "elliptic_k", "elliptic_e",
}

MULTI_ARG = {
    "bessel_j", "bessel_y", "bessel_i", "bessel_k",
    "polygamma", "beta", "zeta_deriv", "hermite",
    "assoc_legendre", "spherical_harmonic", "lambert_w",
}

TWO_ARG = {
    "bessel_j", "bessel_y", "bessel_i", "bessel_k",
    "polygamma", "beta", "zeta_deriv", "hermite",
}

DEFAULT_BINS = 80
DEFAULT_MAX_SLICES = 12
ULP_FLOOR = 1e-20
ULP_CAP = 1e6
MAX_POLE_MARKERS = 12
RUG_NOTE = "Rug guarantees faithful rounding (0 ULP). Sinc: 1 ULP guarantee."
DEFAULT_DPI = 160

INTEGER_POLE_FUNCS = {
    "gamma", "lgamma", "digamma", "trigamma", "tetragamma", "polygamma", "beta",
}

POLE_POINTS = {
    "zeta": [1.0],
    "zeta_deriv": [1.0],
    "lambert_w": [-1 / math.e],
    "elliptic_k": [-1.0, 1.0],
    "elliptic_e": [-1.0, 1.0],
    "bessel_y": [0.0],
    "bessel_k": [0.0],
}

DOMAIN_LIMITS = {
    "lambert_w": [-1 / math.e],
    "elliptic_k": [-1.0, 1.0],
    "elliptic_e": [-1.0, 1.0],
    "bessel_y": [0.0],
    "bessel_k": [0.0],
}
