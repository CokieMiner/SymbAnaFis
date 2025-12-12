"""
SymbAnaFis - Fast symbolic differentiation library

A high-performance symbolic mathematics library written in Rust,
providing fast differentiation and simplification of mathematical expressions.


Example:
    >>> import symb_anafis
    >>> symb_anafis.diff("x^3 + 2*x^2 + x", "x")
    '3*x^2+4*x+1'
    >>> symb_anafis.simplify("sin(x)^2 + cos(x)^2")
    '1'
"""

from .symb_anafis import (
    # Core functions
    diff,
    simplify,
    parse,
    evaluate,
    # Classes
    Expr,
    Diff,
    Simplify,
    # Multi-variable calculus
    gradient,
    hessian,
    jacobian,
    # Uncertainty propagation
    uncertainty_propagation_py,
    relative_uncertainty_py,
    # Version
    __version__,
)

# Try to import parallel evaluation (only available with parallel feature)
try:
    from .symb_anafis import evaluate_parallel_py
except ImportError:
    evaluate_parallel_py = None

__all__ = [
    # Core functions
    "diff",
    "simplify", 
    "parse",
    "evaluate",
    # Classes
    "Expr",
    "Diff",
    "Simplify",
    # Multi-variable calculus
    "gradient",
    "hessian",
    "jacobian",
    # Uncertainty propagation
    "uncertainty_propagation_py",
    "relative_uncertainty_py",
    # Parallel (if available)
    "evaluate_parallel_py",
    # Version
    "__version__",
]
