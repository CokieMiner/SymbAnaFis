"""Type stubs for symb_anafis"""

from typing import Optional, List, Dict, Callable, Any, Tuple, Union

__version__: str

# =============================================================================
# Core Functions
# =============================================================================

def diff(
    formula: str,
    var: str,
    known_symbols: Optional[List[str]] = None,
    custom_functions: Optional[List[str]] = None,
) -> str:
    """
    Differentiate a mathematical expression symbolically.

    Args:
        formula: Mathematical expression to differentiate (e.g., "x^2 + sin(x)")
        var: Variable to differentiate with respect to (e.g., "x")
        known_symbols: Optional list of multi-character symbols (e.g., ["alpha", "beta"])
        custom_functions: Optional list of user-defined function names (e.g., ["f", "g"])

    Returns:
        The derivative as a simplified string expression.

    Raises:
        ValueError: If the expression cannot be parsed or differentiated.

    Example:
        >>> diff("x^2 + sin(x)", "x")
        '2 * x + cos(x)'
        >>> diff("alpha * x^2", "x", known_symbols=["alpha"])
        '2 * alpha * x'
    """
    ...

def simplify(
    formula: str,
    known_symbols: Optional[List[str]] = None,
    custom_functions: Optional[List[str]] = None,
) -> str:
    """
    Simplify a mathematical expression.

    Args:
        formula: Mathematical expression to simplify (e.g., "x + x + x")
        known_symbols: Optional list of multi-character symbols
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
    known_symbols: Optional[List[str]] = None,
    custom_functions: Optional[List[str]] = None,
) -> str:
    """
    Parse a mathematical expression and return its string representation.

    Args:
        formula: Mathematical expression to parse
        known_symbols: Optional list of multi-character symbols
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

def uncertainty_propagation(
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
        >>> uncertainty_propagation("x + y", ["x", "y"])
        "sqrt(sigma_x^2 + sigma_y^2)"
    """
    ...

def relative_uncertainty(
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
# Symbol Management Functions
# =============================================================================

def symb(name: str) -> Symbol:
    """Create or get a symbol by name."""
    ...

def symb_new(name: str) -> Symbol:
    """Create a new symbol (fails if already exists)."""
    ...

def symb_get(name: str) -> Symbol:
    """Get an existing symbol (fails if not found)."""
    ...

def symbol_exists(name: str) -> bool:
    """Check if a symbol exists."""
    ...

def symbol_count() -> int:
    """Get count of symbols in global context."""
    ...

def symbol_names() -> List[str]:
    """Get all symbol names in global context."""
    ...

def remove_symbol(name: str) -> bool:
    """Remove a symbol from global context."""
    ...

def clear_symbols() -> None:
    """Clear all symbols from global context."""
    ...

# =============================================================================
# Visitor Utilities
# =============================================================================

def count_nodes(expr: Expr) -> int:
    """Count the number of nodes in an expression tree."""
    ...

def collect_variables(expr: Expr) -> set[str]:
    """Collect all unique variable names in an expression."""
    ...

# =============================================================================
# Parallel Evaluation (feature-gated)
# =============================================================================

def evaluate_parallel(
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
        >>> evaluate_parallel(
        ...     ["x^2", "x + y"],
        ...     [["x"], ["x", "y"]],
        ...     [[[1.0, 2.0, 3.0]], [[1.0, 2.0], [3.0, 4.0]]]
        ... )
        [["1", "4", "9"], ["4", "6"]]
    """
    ...

def eval_f64(
    expressions: List[Expr],
    var_names: List[List[str]],
    data: List[List[List[float]]],
) -> List[List[float]]:
    """
    High-performance parallel batch evaluation for multiple expressions.

    Args:
        expressions: List of Expr objects
        var_names: List of variable name lists, one per expression
        data: 3D list of values: data[expr_idx][var_idx] = [values...]

    Returns:
        2D list of results: result[expr_idx][point_idx]
    """
    ...

# =============================================================================
# Expr Class
# =============================================================================

class Expr:
    """
    Symbolic expression object for building expressions programmatically.

    Supports arithmetic operations, mathematical functions, and symbolic manipulation.

    Example:
        >>> x = Expr("x")
        >>> y = Expr("y")
        >>> expr = x ** 2 + y
        >>> print(expr)
        x^2 + y
    """

    def __init__(self, name: str) -> None:
        """Create a symbolic expression from a variable name or expression string."""
        ...

    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

    # Arithmetic operators
    def __add__(self, other: "Expr") -> "Expr": ...
    def __sub__(self, other: "Expr") -> "Expr": ...
    def __mul__(self, other: "Expr") -> "Expr": ...
    def __truediv__(self, other: "Expr") -> "Expr": ...
    def __pow__(self, other: "Expr | int | float") -> "Expr": ...
    def __neg__(self) -> "Expr": ...

    # Reverse operators for number + Expr, etc.
    def __radd__(self, other: float) -> "Expr": ...
    def __rsub__(self, other: float) -> "Expr": ...
    def __rmul__(self, other: float) -> "Expr": ...
    def __rtruediv__(self, other: float) -> "Expr": ...
    def __rpow__(self, other: float) -> "Expr": ...

    # Trigonometric functions
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

    # Hyperbolic functions
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

    # Exponential and logarithmic
    def exp(self) -> "Expr": ...
    def ln(self) -> "Expr": ...
    def log(self) -> "Expr": ...
    def log10(self) -> "Expr": ...
    def log2(self) -> "Expr": ...

    # Powers and roots
    def sqrt(self) -> "Expr": ...
    def cbrt(self) -> "Expr": ...
    def pow(self, exp: float) -> "Expr": ...

    # Special functions
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
    def polygamma(self, n: float) -> "Expr": ...
    def beta(self, other: "Expr") -> "Expr": ...
    def zeta(self) -> "Expr": ...
    def lambertw(self) -> "Expr": ...
    def besselj(self, n: float) -> "Expr": ...
    def bessely(self, n: float) -> "Expr": ...
    def besseli(self, n: float) -> "Expr": ...
    def besselk(self, n: float) -> "Expr": ...

    # Multi-argument functions
    def atan2(self, x: "Expr") -> "Expr": ...
    def hermite(self, n: int) -> "Expr": ...
    def assoc_legendre(self, l: int, m: int) -> "Expr": ...
    def spherical_harmonic(self, l: int, m: int, phi: "Expr") -> "Expr": ...
    def ynm(self, l: int, m: int, phi: "Expr") -> "Expr": ...
    def zeta_deriv(self, n: int) -> "Expr": ...

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

    def is_symbol(self) -> bool:
        """Check if expression is a raw symbol."""
        ...

    def is_number(self) -> bool:
        """Check if expression is a constant number."""
        ...

    def is_zero(self) -> bool:
        """Check if expression is effectively zero."""
        ...

    def is_one(self) -> bool:
        """Check if expression is effectively one."""
        ...

    # Operations
    def substitute(self, var: str, value: "Expr") -> "Expr":
        """Substitute a variable with another expression."""
        ...

    def evaluate(self, vars: Dict[str, float]) -> "Expr":
        """Evaluate the expression with given variable values.
        
        Args:
            vars: Dictionary mapping variable names to float values
            
        Returns:
            Evaluated expression (may be numeric or symbolic if variables remain)
        """
        ...

    def diff(self, var: str) -> "Expr":
        """Differentiate this expression."""
        ...

    def simplify(self) -> "Expr":
        """Simplify this expression."""
        ...

# =============================================================================
# Diff Builder Class
# =============================================================================

class Diff:
    """
    Builder for differentiation operations with configuration options.

    Example:
        >>> d = Diff().domain_safe(True)
        >>> d.diff_str("a*x^2", "x", ["a"])
        '2*a*x'
    """

    def __init__(self) -> None: ...

    def domain_safe(self, safe: bool) -> "Diff":
        """Enable/disable domain-safe simplifications."""
        ...

    def max_depth(self, depth: int) -> "Diff":
        """Set maximum expression depth limit."""
        ...

    def max_nodes(self, nodes: int) -> "Diff":
        """Set maximum node count limit."""
        ...

    def with_context(self, context: Context) -> "Diff":
        """Set the context for symbol resolution."""
        ...

    def fixed_var(self, var: str) -> "Diff":
        """Mark a variable as fixed (constant) during differentiation."""
        ...

    def user_fn(
        self,
        name: str,
        arity: int,
        partial_callback: Callable[[List["Expr"]], "Expr"],
    ) -> "Diff":
        """
        Register a user-defined function with a partial derivative callback.

        Args:
            name: Function name (e.g., "my_func")
            arity: Number of arguments the function takes
            partial_callback: Function that takes a list of argument expressions
                            and returns the partial derivative w.r.t. the first argument

        Returns:
            Self for method chaining.

        Example:
            >>> def my_partial(args):
            ...     # For f(u), return ∂f/∂u = 3u²
            ...     return 3 * args[0] ** 2
            >>> diff = Diff().user_fn("my_func", 1, my_partial)
        """
        ...

    def diff_str(self, formula: str, var: str) -> str:
        """Differentiate a string formula."""
        ...

    def differentiate(self, expr: Expr, var: str) -> Expr:
        """Differentiate an Expr object with respect to a variable name."""
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

    def max_depth(self, depth: int) -> "Simplify":
        """Set maximum expression depth limit."""
        ...

    def max_nodes(self, nodes: int) -> "Simplify":
        """Set maximum node count limit."""
        ...

    def with_context(self, context: Context) -> "Simplify":
        """Set the context for symbol resolution."""
        ...

    def fixed_var(self, var: str) -> "Simplify":
        """Mark a variable as a known symbol during simplification."""
        ...

    def simplify(self, expr: Expr) -> Expr:
        """Simplify an Expr object."""
        ...

    def simplify_str(self, formula: str) -> str:
        """Simplify a string formula."""
        ...

# =============================================================================
# Symbol Class
# =============================================================================

class Symbol:
    """Lightweight, copyable symbol reference."""

    def __init__(self, name: str) -> None:
        """Create or get a symbol by name."""
        ...

    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

    def name(self) -> Optional[str]: ...
    def id(self) -> int: ...
    def to_expr(self) -> Expr: ...

    # Arithmetic with other symbols/exprs
    def __add__(self, other: Expr) -> Expr: ...
    def __mul__(self, other: Expr) -> Expr: ...
    def __sub__(self, other: Expr) -> Expr: ...
    def __truediv__(self, other: Expr) -> Expr: ...

# =============================================================================
# Context Class
# =============================================================================

class Context:
    """
    Unified context for namespace isolation and function registry.

    Each Context maintains its own set of symbols and functions, independent of
    the global registry. Useful for isolating symbol namespaces.

    Example:
        >>> ctx = Context()
        >>> x = ctx.symb("x")
        >>> ctx.with_function("f", 1, lambda args: args[0] * 2)
    """

    def __init__(self) -> None:
        """Create a new empty context."""
        ...

    def symb(self, name: str) -> Symbol:
        """Create or get a symbol in this context."""
        ...
    
    def symb_new(self, name: str) -> Symbol:
        """Create a new symbol (fails if already exists in this context)."""
        ...
    
    def get_symbol(self, name: str) -> Optional[Symbol]:
        """Get an existing symbol by name (returns None if not found)."""
        ...
    
    def contains_symbol(self, name: str) -> bool:
        """Check if a symbol exists in this context."""
        ...
    
    def symbol_count(self) -> int:
        """Get the number of symbols in this context."""
        ...
    
    def symbol_names(self) -> List[str]:
        """List all symbol names in this context."""
        ...
    
    def is_empty(self) -> bool:
        """Check if context has no symbols."""
        ...
    
    def remove_symbol(self, name: str) -> bool:
        """Remove a symbol from the context."""
        ...
    
    def clear_all(self) -> None:
        """Clear all symbols and functions from the context."""
        ...
    
    def with_function(
        self,
        name: str,
        arity: int,
        callback: Callable[[List[Expr]], Expr]
    ) -> "Context":
        """Register a user function with a partial derivative callback."""
        ...
    
    def id(self) -> int:
        """Get the context's unique ID."""
        ...

# =============================================================================
# CompiledEvaluator Class
# =============================================================================

class CompiledEvaluator:
    """Compiled expression evaluator for fast numeric evaluation."""

    def __init__(self, expr: Expr, params: Optional[List[str]] = None) -> None:
        """Compile an expression with specified parameter order."""
        ...

    def evaluate(self, params: List[float]) -> float:
        """Evaluate at a single point."""
        ...

    def eval_batch(self, columns: List[List[float]]) -> List[float]:
        """Batch evaluate at multiple points."""
        ...

    def param_names(self) -> List[str]: ...
    def param_count(self) -> int: ...
    def instruction_count(self) -> int: ...
    def stack_size(self) -> int: ...

# =============================================================================
# Dual Class
# =============================================================================

class Dual:
    """
    Dual number for automatic differentiation.

    A dual number `a + bε` represents both a value (`a`) and its derivative (`b`),
    where `ε` is an infinitesimal satisfying `ε² = 0`.

    This enables exact derivative computation through algebraic manipulation.

    Example:
        >>> x = Dual(2.0, 1.0)  # x + 1ε (derivative w.r.t. x is 1)
        >>> fx = x * x + Dual.constant(3.0) * x + Dual.constant(1.0)
        >>> fx.val, fx.eps  # (11.0, 7.0) - f(2) = 11, f'(2) = 7
        (11.0, 7.0)
    """

    def __init__(self, val: float, eps: float) -> None:
        """
        Create a new dual number.

        Args:
            val: The real value component
            eps: The infinitesimal derivative component
        """
        ...

    @staticmethod
    def constant(val: float) -> "Dual":
        """
        Create a constant dual number (derivative = 0).

        Args:
            val: The constant value

        Returns:
            Dual number with eps = 0
        """
        ...

    @property
    def val(self) -> float:
        """The real value component."""
        ...

    @property
    def eps(self) -> float:
        """The infinitesimal derivative component."""
        ...

    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

    # Arithmetic operators
    def __add__(self, other: "Union[Dual, float, int]") -> "Dual": ...
    def __sub__(self, other: "Union[Dual, float, int]") -> "Dual": ...
    def __mul__(self, other: "Union[Dual, float, int]") -> "Dual": ...
    def __truediv__(self, other: "Union[Dual, float, int]") -> "Dual": ...
    def __pow__(self, other: "Union[Dual, float, int]", modulo: None = None) -> "Dual": ...
    def __neg__(self) -> "Dual": ...

    # Reverse operators
    def __radd__(self, other: Union[float, int]) -> "Dual": ...
    def __rsub__(self, other: Union[float, int]) -> "Dual": ...
    def __rmul__(self, other: Union[float, int]) -> "Dual": ...
    def __rtruediv__(self, other: Union[float, int]) -> "Dual": ...
    def __rpow__(self, other: Union[float, int], modulo: None = None) -> "Dual": ...

    # Trigonometric functions
    def sin(self) -> "Dual": ...
    def cos(self) -> "Dual": ...
    def tan(self) -> "Dual": ...
    def asin(self) -> "Dual": ...
    def acos(self) -> "Dual": ...
    def atan(self) -> "Dual": ...

    # Hyperbolic functions
    def sinh(self) -> "Dual": ...
    def cosh(self) -> "Dual": ...
    def tanh(self) -> "Dual": ...
    def asinh(self) -> "Dual": ...
    def acosh(self) -> "Dual": ...
    def atanh(self) -> "Dual": ...

    # Exponential and logarithmic
    def exp(self) -> "Dual": ...
    def exp2(self) -> "Dual": ...
    def ln(self) -> "Dual": ...
    def log2(self) -> "Dual": ...
    def log10(self) -> "Dual": ...

    # Powers and roots
    def sqrt(self) -> "Dual": ...
    def cbrt(self) -> "Dual": ...
    def powf(self, other: "Dual") -> "Dual": ...
    def powi(self, n: int) -> "Dual": ...

    # Special functions
    def abs(self) -> "Dual": ...
    def erf(self) -> "Dual": ...
    def erfc(self) -> "Dual": ...
    def gamma(self) -> "Dual": ...
    def digamma(self) -> "Dual": ...
    def trigamma(self) -> "Dual": ...
    def polygamma(self, n: int) -> "Dual": ...
    def zeta(self) -> "Dual": ...
    def lambert_w(self) -> "Dual": ...
    def bessel_j(self, n: int) -> "Dual": ...
    def sinc(self) -> "Dual": ...
    def sign(self) -> "Dual": ...
    def elliptic_k(self) -> "Dual": ...
    def elliptic_e(self) -> "Dual": ...
    def beta(self, b: "Dual") -> "Dual": ...

    # Float trait methods
    def signum(self) -> "Dual": ...
    def floor(self) -> "Dual": ...
    def ceil(self) -> "Dual": ...
    def round(self) -> "Dual": ...
    def trunc(self) -> "Dual": ...
    def fract(self) -> "Dual": ...
    def recip(self) -> "Dual": ...
    def exp_m1(self) -> "Dual": ...
    def ln_1p(self) -> "Dual": ...
    def hypot(self, other: "Dual") -> "Dual": ...
    def atan2(self, other: "Dual") -> "Dual": ...
    def log(self, base: "Dual") -> "Dual": ...
