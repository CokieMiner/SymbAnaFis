"""SymbAnaFis Python Quickstart Example."""

from symb_anafis import Context, Expr


def main():
    ctx = Context()
    x, y = ctx.symbols("x", "y")

    # 1. Expressions and calculus
    f = x**2 + x * y + y**2
    print(f"f(x, y) = {f}")
    print(f"∇f = {f.gradient([x, y])}")
    print(f"H(f) = {f.hessian([x, y])}")

    # 2. Multi-output Jacobian
    print(f"J = {Expr.jacobian([f, x * y], [x, y])}")

    # 3. Direct canonical simplification
    identity = x.sin()**2 + x.cos()**2
    print(f"{identity} -> {identity.compact()}")

    # 4. Expansion & factoring
    poly = (x + 1)**3
    expanded = poly.expand()
    print(f"{poly} expanded = {expanded}")
    print(f"{x**2 + x * y} factored = {(x**2 + x * y).factor()}")

    # 5. Compiled evaluation
    program = f.compile([x, y])
    print(f"f(2, 3) compiled = {program.evaluate([2.0, 3.0])}")


if __name__ == "__main__":
    main()
