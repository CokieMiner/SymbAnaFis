"""
SymbAnaFis - Fast symbolic differentiation library

A high-performance symbolic mathematics library written in Rust,
providing fast differentiation and simplification of mathematical expressions.

3-15x faster than SymPy for differentiation + simplification.

Example:
    >>> import symb_anafis
    >>> symb_anafis.diff("x^3 + 2*x^2 + x", "x")
    '3 * x^2 + 4 * x + 1'
    >>> symb_anafis.simplify("sin(x)^2 + cos(x)^2")
    '1'
"""

from .symb_anafis import diff, simplify, parse, __version__

__all__ = ["diff", "simplify", "parse", "__version__"]
