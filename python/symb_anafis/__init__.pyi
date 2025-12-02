"""Type stubs for symb_anafis"""

from typing import Optional, List

__version__: str

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
