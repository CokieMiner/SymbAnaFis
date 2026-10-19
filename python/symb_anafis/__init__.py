"""Symbolic mathematics through owning classes and scoped namespaces.

Use Context for symbols and parsing; Expr for arithmetic and calculus;
SimplificationSequence for presets or custom passes; Evaluator for numerical
programs; and Uncertainty for real first-order covariance propagation.
"""

from ._native import (
    EvaluationError, ParseError, RewriteError, SimplifyError, UncertaintyError,
)
from ._number import Number
from ._properties import Properties, Property, Truth
from ._expression import Expr, ExprView, Symbol
from ._function import Function
from ._context import Context
from ._rewriting import Pattern, RewriteRule, Substitution, Substitutions
from ._simplification import (
    DomainSafety, Simplification, SimplificationSequence, SimplificationStep,
    SimplificationTarget, SimplificationTermination,
)
from ._evaluation import Evaluator
from ._uncertainty import CovarianceMatrix, Uncertainty

__version__ = "0.9.0"
__all__ = [
    "Context", "CovarianceMatrix", "DomainSafety", "EvaluationError", "Evaluator",
    "Expr", "ExprView", "Function", "Number", "ParseError", "Pattern", "Properties",
    "Property", "RewriteError", "RewriteRule", "Simplification", "SimplificationSequence",
    "SimplificationStep", "SimplificationTarget", "SimplificationTermination", "SimplifyError",
    "Substitution", "Substitutions", "Symbol", "Truth", "Uncertainty", "UncertaintyError",
]
