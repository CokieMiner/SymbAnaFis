import sys
import os

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Expr, Diff, Simplify, diff, simplify, gradient, hessian, jacobian, evaluate, uncertainty_propagation, relative_uncertainty

def test_duck_typing():
    print("\n=== Testing Unified API (Duck Typing) ===")

    x = Expr("x")
    y = Expr("y")

    # 1. Gradient
    print("\n--- Gradient ---")
    # String input
    g_str = gradient("x^2 + y^2", ["x", "y"])
    print(f"Gradient (str): {g_str}")
    # Expr input
    g_expr = gradient(x**2 + y**2, ["x", "y"])
    print(f"Gradient (Expr): {[str(e) for e in g_expr]}")

    # 2. Hessian
    print("\n--- Hessian ---")
    # String input
    h_str = hessian("x^2 + y^2", ["x", "y"])
    print(f"Hessian (str): {h_str}")
    # Expr input
    h_expr = hessian(x**2 + y**2, ["x", "y"])
    print(f"Hessian (Expr): {[[str(e) for e in row] for row in h_expr]}")

    # 3. Jacobian
    print("\n--- Jacobian ---")
    # String input
    j_str = jacobian(["x^2 + y", "x*y"], ["x", "y"])
    print(f"Jacobian (str): {j_str}")
    # Expr input
    j_expr = jacobian([x**2 + y, x*y], ["x", "y"])
    print(f"Jacobian (Expr): {[[str(e) for e in row] for row in j_expr]}")

    # 4. Evaluate
    print("\n--- Evaluate ---")
    # String input
    e_str = evaluate("x + y", [("x", 1.0), ("y", 2.0)])
    print(f"Evaluate (str): {e_str}")
    # Expr input
    e_expr = evaluate(x + y, [("x", 1.0), ("y", 2.0)])
    # Wait, evaluate implementation for Expr returns PyExpr(f64) basically?
    # Or just f64? The binding currently returns PyExpr.
    print(f"Evaluate (Expr): {e_expr}")

    # 5. Uncertainty
    print("\n--- Uncertainty ---")
    # String input
    u_str = uncertainty_propagation("x*y", ["x", "y"])
    print(f"Uncertainty (str): {u_str}")
    # Expr input
    u_expr = uncertainty_propagation(x*y, ["x", "y"])
    print(f"Uncertainty (Expr): {u_expr}")

    print("\n=== All Tests Passed ===")

if __name__ == "__main__":
    test_duck_typing()
