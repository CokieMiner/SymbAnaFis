"""Type stubs for symb_anafis"""

from typing import Optional, List, Dict, Callable, Any, Tuple

__version__: str

# =============================================================================
# Core Functions
# =============================================================================

def diff(
    formula: str,
    var: str,
    fixed_vars: Optional[List[str]] = None,
    custom_functions: Optional[List[str]] = None,
) -> str:
    """
    Differentiate a mathematical expression symbolically.

    Args:
        formula: Mathematical expression to differentiate (e.g., "x^2 + sin(x)")
        var: Variable to differentiate with respect to (e.g., "x")
        fixed_vars: Optional list of symbols that are constants (e.g., ["a", "b"])
        custom_functions: Optional list of user-defined function names (e.g., ["f", "g"])

    Returns:
        The derivative as a simplified string expression.

    Raises:
        ValueError: If the expression cannot be parsed or differentiated.

    Example:
        >>> diff("x^2 + sin(x)", "x")
        '2 * x + cos(x)'
        >>> diff("a * x^2", "x", fixed_vars=["a"])
        '2 * a * x'
    """
    ...

def simplify(
    formula: str,
    fixed_vars: Optional[List[str]] = None,
    custom_functions: Optional[List[str]] = None,
) -> str:
    """
    Simplify a mathematical expression.

    Args:
        formula: Mathematical expression to simplify (e.g., "x + x + x")
        fixed_vars: Optional list of symbols that are constants
        custom_functions: Optional list of user-defined function names

    Returns:
        The simplified expression as a string.

    Raises:
        ValueError: If the expression cannot be parsed.

    Example:
        >>> simplify("x + x + x")
        '3 * x'
        >>> simplify("sin(x)^2 + cos(x)^2")
        '1'
    """
    ...

def parse(
    formula: str,
    fixed_vars: Optional[List[str]] = None,
    custom_functions: Optional[List[str]] = None,
) -> str:
    """
    Parse a mathematical expression and return its string representation.

    Args:
        formula: Mathematical expression to parse
        fixed_vars: Optional list of symbols that are constants
        custom_functions: Optional list of user-defined function names

    Returns:
        The parsed expression as a normalized string.

    Raises:
        ValueError: If the expression cannot be parsed.
    """
    ...

def evaluate(
    formula: str,
    vars: List[Tuple[str, float]],
) -> str:
    """
    Evaluate a string expression with given variable values.

    Args:
        formula: Expression string to evaluate
        vars: List of (name, value) tuples

    Returns:
        Evaluated expression as string.

    Example:
        >>> evaluate("x^2 + y", [("x", 3.0), ("y", 1.0)])
        '10'
    """
    ...

# =============================================================================
# Multi-Variable Calculus
# =============================================================================

def gradient(formula: str, vars: List[str]) -> List[str]:
    """
    Compute the gradient of a scalar expression.

    Args:
        formula: String formula to differentiate
        vars: List of variable names to differentiate with respect to

    Returns:
        List of partial derivative strings [∂f/∂x₁, ∂f/∂x₂, ...]

    Example:
        >>> gradient("x^2 + y^2", ["x", "y"])
        ['2*x', '2*y']
    """
    ...

def hessian(formula: str, vars: List[str]) -> List[List[str]]:
    """
    Compute the Hessian matrix of a scalar expression.

    Args:
        formula: String formula to differentiate twice
        vars: List of variable names

    Returns:
        2D list of second partial derivatives [[∂²f/∂x₁², ∂²f/∂x₁∂x₂, ...], ...]
    """
    ...

def jacobian(formulas: List[str], vars: List[str]) -> List[List[str]]:
    """
    Compute the Jacobian matrix of a vector function.

    Args:
        formulas: List of string formulas (vector function)
        vars: List of variable names

    Returns:
        2D list where J[i][j] = ∂fᵢ/∂xⱼ
    """
    ...

# =============================================================================
# Uncertainty Propagation
# =============================================================================

def uncertainty_propagation_py(
    formula: str,
    variables: List[str],
    variances: Optional[List[float]] = None,
) -> str:
    """
    Compute uncertainty propagation for an expression.

    Args:
        formula: Expression string
        variables: List of variable names to propagate uncertainty for
        variances: Optional list of variance values (σ²) for each variable.
                   If None, uses symbolic variances σ_x², σ_y², etc.

    Returns:
        String representation of the uncertainty expression σ_f

    Example:
        >>> uncertainty_propagation_py("x + y", ["x", "y"])
        "sqrt(sigma_x^2 + sigma_y^2)"
    """
    ...

def relative_uncertainty_py(
    formula: str,
    variables: List[str],
    variances: Optional[List[float]] = None,
) -> str:
    """
    Compute relative uncertainty for an expression.

    Args:
        formula: Expression string
        variables: List of variable names
        variances: Optional list of variance values (σ²) for each variable

    Returns:
        String representation of σ_f / |f|
    """
    ...

# =============================================================================
# Parallel Evaluation (requires "parallel" feature)
# =============================================================================

def evaluate_parallel_py(
    expressions: List[str],
    variables: List[List[str]],
    values: List[List[List[Optional[float]]]],
) -> List[List[str]]:
    """
    Parallel evaluation of multiple expressions at multiple points.

    Args:
        expressions: List of expression strings
        variables: List of variable name lists, one per expression
        values: 3D list of values: [expr_idx][var_idx][point_idx]
                Use None for a value to keep it symbolic (SKIP)

    Returns:
        2D list of result strings: [expr_idx][point_idx]

    Example:
        >>> evaluate_parallel_py(
        ...     ["x^2", "x + y"],
        ...     [["x"], ["x", "y"]],
        ...     [[[1.0, 2.0, 3.0]], [[1.0, 2.0], [3.0, 4.0]]]
        ... )
        [["1", "4", "9"], ["4", "6"]]
    """
    ...

# =============================================================================
# Expr Class
# =============================================================================

class Expr:
    """
    Symbolic expression object for building expressions programmatically.

    Example:
        >>> x = Expr("x")
        >>> y = Expr("y")
        >>> expr = x ** 2 + y
        >>> print(expr)
        x^2 + y
    """

    def __init__(self, name: str) -> None:
        """Create a symbolic expression from a variable name."""
        ...

    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __add__(self, other: "Expr") -> "Expr": ...
    def __sub__(self, other: "Expr") -> "Expr": ...
    def __mul__(self, other: "Expr") -> "Expr": ...
    def __truediv__(self, other: "Expr") -> "Expr": ...
    def __pow__(self, other: "Expr | int | float") -> "Expr": ...
    def __rpow__(self, other: int | float) -> "Expr": ...

    # Mathematical functions
    def sin(self) -> "Expr": ...
    def cos(self) -> "Expr": ...
    def tan(self) -> "Expr": ...
    def cot(self) -> "Expr": ...
    def sec(self) -> "Expr": ...
    def csc(self) -> "Expr": ...
    def asin(self) -> "Expr": ...
    def acos(self) -> "Expr": ...
    def atan(self) -> "Expr": ...
    def acot(self) -> "Expr": ...
    def asec(self) -> "Expr": ...
    def acsc(self) -> "Expr": ...
    def sinh(self) -> "Expr": ...
    def cosh(self) -> "Expr": ...
    def tanh(self) -> "Expr": ...
    def coth(self) -> "Expr": ...
    def sech(self) -> "Expr": ...
    def csch(self) -> "Expr": ...
    def asinh(self) -> "Expr": ...
    def acosh(self) -> "Expr": ...
    def atanh(self) -> "Expr": ...
    def acoth(self) -> "Expr": ...
    def asech(self) -> "Expr": ...
    def acsch(self) -> "Expr": ...
    def exp(self) -> "Expr": ...
    def ln(self) -> "Expr": ...
    def log(self) -> "Expr": ...
    def log10(self) -> "Expr": ...
    def log2(self) -> "Expr": ...
    def sqrt(self) -> "Expr": ...
    def cbrt(self) -> "Expr": ...
    def abs(self) -> "Expr": ...
    def signum(self) -> "Expr": ...
    def sinc(self) -> "Expr": ...
    def erf(self) -> "Expr": ...
    def erfc(self) -> "Expr": ...
    def gamma(self) -> "Expr": ...
    def digamma(self) -> "Expr": ...
    def trigamma(self) -> "Expr": ...
    def tetragamma(self) -> "Expr": ...
    def floor(self) -> "Expr": ...
    def ceil(self) -> "Expr": ...
    def round(self) -> "Expr": ...
    def elliptic_k(self) -> "Expr": ...
    def elliptic_e(self) -> "Expr": ...
    def exp_polar(self) -> "Expr": ...
    def polygamma(self, n: int) -> "Expr": ...
    def beta(self, other: "Expr") -> "Expr": ...
    def zeta(self) -> "Expr": ...
    def lambertw(self) -> "Expr": ...
    def besselj(self, n: int) -> "Expr": ...
    def bessely(self, n: int) -> "Expr": ...
    def besseli(self, n: int) -> "Expr": ...
    def besselk(self, n: int) -> "Expr": ...
    def pow(self, exp: float) -> "Expr": ...

    # Output formats
    def to_latex(self) -> str:
        """Convert expression to LaTeX string."""
        ...

    def to_unicode(self) -> str:
        """Convert expression to Unicode string (with Greek symbols, superscripts)."""
        ...

    # Expression info
    def node_count(self) -> int:
        """Get the number of nodes in the expression tree."""
        ...

    def max_depth(self) -> int:
        """Get the maximum depth of the expression tree."""
        ...

    def substitute(self, var: str, value: "Expr") -> "Expr":
        """Substitute a variable with another expression."""
        ...

    def evaluate(self, vars: Dict[str, float]) -> "Expr":
        """Evaluate the expression with given variable values."""
        ...

# =============================================================================
# Diff Builder Class
# =============================================================================

class Diff:
    """
    Builder for differentiation operations with configuration options.

    Example:
        >>> d = Diff().fixed_var("a").domain_safe(True)
        >>> d.diff_str("a*x^2", "x")
        '2*a*x'
    """

    def __init__(self) -> None: ...

    def domain_safe(self, safe: bool) -> "Diff":
        """Enable/disable domain-safe simplifications."""
        ...

    def fixed_var(self, var: str) -> "Diff":
        """Mark a variable as a constant."""
        ...

    def max_depth(self, depth: int) -> "Diff":
        """Set maximum expression depth limit."""
        ...

    def max_nodes(self, nodes: int) -> "Diff":
        """Set maximum node count limit."""
        ...

    def custom_derivative(
        self,
        name: str,
        callback: Callable[["Expr", str, "Expr"], "Expr"],
    ) -> "Diff":
        """
        Register a custom derivative rule for a function.

        Args:
            name: Function name (e.g., "my_func")
            callback: Function(inner, var, inner_prime) -> derivative

        Returns:
            Self for method chaining.
        """
        ...

    def diff_str(self, formula: str, var: str) -> str:
        """Differentiate a string formula."""
        ...

    def differentiate(self, expr: Expr, var: str) -> Expr:
        """Differentiate an Expr object."""
        ...

# =============================================================================
# Simplify Builder Class
# =============================================================================

class Simplify:
    """
    Builder for simplification operations with configuration options.

    Example:
        >>> s = Simplify().domain_safe(True)
        >>> s.simplify_str("x + x + x")
        '3*x'
    """

    def __init__(self) -> None: ...

    def domain_safe(self, safe: bool) -> "Simplify":
        """Enable/disable domain-safe simplifications."""
        ...

    def fixed_var(self, var: str) -> "Simplify":
        """Mark a variable as a constant."""
        ...

    def max_depth(self, depth: int) -> "Simplify":
        """Set maximum expression depth limit."""
        ...

    def max_nodes(self, nodes: int) -> "Simplify":
        """Set maximum node count limit."""
        ...

    def simplify(self, expr: Expr) -> Expr:
        """Simplify an Expr object."""
        ...

    def simplify_str(self, formula: str) -> str:
        """Simplify a string formula."""
        ...
