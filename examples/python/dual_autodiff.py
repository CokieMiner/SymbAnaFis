#!/usr/bin/env python3
"""
Dual Numbers for Automatic Differentiation (Python)

This example demonstrates using SymbAnaFis's Dual number implementation
for automatic differentiation. Dual numbers enable exact derivative computation
through algebraic manipulation.

This is not the focus of the crate but could be useful for automatic differentiation
without symbolic expressions.

Dual numbers extend real numbers with an infinitesimal ε where ε² = 0.
A dual number a + bε represents both a value (a) and its derivative (b).

Run with: python examples/dual_autodiff.py
"""

from symb_anafis import Dual
import math


def quadratic_example():
    """Evaluate f(x) = x² + 3x + 1 and its derivative at x = 2"""
    print("=== Quadratic Function: f(x) = x² + 3x + 1 ===")

    # Create dual number: x = 2 + 1ε (value 2, derivative 1)
    x = Dual(2.0, 1.0)

    # Compute f(x) = x² + 3x + 1
    fx = x * x + Dual.constant(3.0) * x + Dual.constant(1.0)

    print(f"f(2) = {fx.val}")        # Should be 2² + 3*2 + 1 = 11
    print(f"f'(2) = {fx.eps}")       # Should be 2*2 + 3 = 7
    print("Expected: f(2) = 11, f'(2) = 7")
    print()


def transcendental_example():
    """Evaluate f(x) = sin(x) * exp(x) and its derivative"""
    print("=== Transcendental Function: f(x) = sin(x) * exp(x) ===")

    x = Dual(1.0, 1.0)  # x = 1 + 1ε

    # f(x) = sin(x) * exp(x)
    sin_x = x.sin()
    exp_x = x.exp()
    fx = sin_x * exp_x

    # f'(x) = cos(x) * exp(x) + sin(x) * exp(x) = exp(x) * (cos(x) + sin(x))
    expected_deriv = math.exp(x.val) * (math.cos(x.val) + math.sin(x.val))

    print(f"f(1) = {fx.val:.6f}")
    print(f"f'(1) = {fx.eps:.6f}")
    print(f"Expected f'(1) = {expected_deriv:.6f}")
    print(f"Error: {abs(fx.eps - expected_deriv):.2e}")
    print()


def higher_order_example():
    """Higher-order derivatives using dual numbers"""
    print("=== Higher-Order Derivatives ===")


    # For f(x) = x³, f'(x) = 3x², f''(x) = 6x, f'''(x) = 6

    # First derivative: f'(2) where f(x) = x³
    x = Dual(2.0, 1.0)  # x + 1ε
    fx = x * x * x       # x³
    print("f(x) = x³ at x = 2:")
    print(f"f(2) = {fx.val}")      # 8
    print(f"f'(2) = {fx.eps}")     # 12

    # Second derivative: differentiate f'(x) = 3x²
    # We know f'(x) = 3x², so f''(x) = 6x
    x_second = Dual(2.0, 1.0)  # Fresh x with eps=1
    f_second = Dual.constant(3.0) * x_second * x_second  # 3x²
    print(f"f''(2) = {f_second.eps}")  # d/dx(3x²) at x=2 = 6x = 12

    # Third derivative: differentiate f''(x) = 6x
    # We know f''(x) = 6x, so f'''(x) = 6
    x_third = Dual(2.0, 1.0)  # Fresh x with eps=1
    f_third = Dual.constant(6.0) * x_third  # 6x (not just constant!)
    print(f"f'''(2) = {f_third.eps}")  # d/dx(6x) = 6




def chain_rule_example():
    """Chain rule example: f(x) = sin(x² + 1)"""
    print("=== Chain Rule: f(x) = sin(x² + 1) ===")

    x = Dual(1.5, 1.0)

    # f(x) = sin(x² + 1)
    inner = x * x + Dual.constant(1.0)        # x² + 1
    fx = inner.sin()           # sin(x² + 1)

    # f'(x) = cos(x² + 1) * (2x)
    expected_deriv = math.cos(inner.val) * (2.0 * x.val)

    print(f"f(1.5) = {fx.val:.6f}")
    print(f"f'(1.5) = {fx.eps:.6f}")
    print(f"Expected f'(1.5) = {expected_deriv:.6f}")
    print(f"Error: {abs(fx.eps - expected_deriv):.2e}")
    print()


def mixed_operations_example():
    """Demonstrate mixed operations with regular numbers"""
    print("=== Mixed Operations ===")

    x = Dual(3.0, 1.0)  # x + 1ε

    # Mix with regular floats
    result1 = x + 5.0      # Dual + float
    result2 = 2.0 * x      # float * Dual
    result3 = x ** 2       # Dual ** int (power operator)
    result4 = 2.0 ** x     # float ** Dual (exponential)

    print(f"x = {x}")
    print(f"x + 5.0 = {result1}")
    print(f"2.0 * x = {result2}")
    print(f"x ** 2 = {result3}")
    print(f"2.0 ** x = {result4}")
    print()


def main():
    print("╔══════════════════════════════════════════════════════════════════╗")
    print("║          SYMB ANAFIS: DUAL NUMBERS (Python)                      ║")
    print("║          Automatic Differentiation via Algebraic Manipulation    ║")
    print("╚══════════════════════════════════════════════════════════════════╝\n")

    quadratic_example()
    transcendental_example()
    higher_order_example()
    chain_rule_example()
    mixed_operations_example()

    print("Dual numbers provide exact derivatives through algebraic manipulation!")
    print("ε² = 0 enables forward-mode automatic differentiation.")


if __name__ == "__main__":
    main()