"""
SymbAnaFis - Fast symbolic differentiation library

A high-performance symbolic mathematics library written in Rust,
providing fast differentiation and simplification of mathematical expressions,
plus automatic differentiation via dual numbers.

Example:
    >>> import symb_anafis
    >>> symb_anafis.diff("x^3 + 2*x^2 + x", "x")
    '3*x^2+4*x+1'
    >>> symb_anafis.simplify("sin(x)^2 + cos(x)^2")
    '1'
    >>> x = symb_anafis.Dual(2.0, 1.0)  # x + 1ε
    >>> f = x * x + symb_anafis.Dual.constant(3.0) * x + symb_anafis.Dual.constant(1.0)
    >>> f.val, f.eps  # (11.0, 7.0) - f(2) = 11, f'(2) = 7
    (11.0, 7.0)
"""

from .symb_anafis import (
    # Core functions (string API)
    diff,
    simplify,
    parse,
    evaluate,
    evaluate_str,
    # Classes
    Expr,
    Diff,
    Simplify,
    Dual,
    Symbol,
    Context,
    CompiledEvaluator,
    # Multi-variable calculus (Expr API)
    gradient,
    hessian,
    jacobian,
    # Multi-variable calculus (string API)
    gradient_str,
    hessian_str,
    jacobian_str,
    # Uncertainty propagation
    uncertainty_propagation,
    relative_uncertainty,
    # Symbol management
    symb,
    symb_new,
    symb_get,
    symbol_exists,
    symbol_count,
    symbol_names,
    remove_symbol,
    clear_symbols,
    # Visitor utilities
    count_nodes,
    collect_variables,
    # Version
    __version__,
)

# Try to import parallel evaluation (only available with parallel feature)
try:
    from .symb_anafis import evaluate_parallel, eval_f64
except ImportError:
    evaluate_parallel = None
    eval_f64 = None

__all__ = [
    # Core functions (string API)
    "diff",
    "simplify", 
    "parse",
    "evaluate",
    "evaluate_str",
    # Classes
    "Expr",
    "Diff",
    "Simplify",
    "Dual",
    "Symbol",
    "Context",
    "CompiledEvaluator",
    # Multi-variable calculus (Expr API)
    "gradient",
    "hessian",
    "jacobian",
    # Multi-variable calculus (string API)
    "gradient_str",
    "hessian_str",
    "jacobian_str",
    # Uncertainty propagation
    "uncertainty_propagation",
    "relative_uncertainty",
    # Symbol management
    "symb",
    "symb_new",
    "symb_get",
    "symbol_exists",
    "symbol_count",
    "symbol_names",
    "remove_symbol",
    "clear_symbols",
    # Visitor utilities
    "count_nodes",
    "collect_variables",
    # Parallel (if available)
    "evaluate_parallel",
    "eval_f64",
    # Version
    "__version__",
]

