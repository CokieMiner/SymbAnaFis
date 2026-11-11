"""SymbAnaFis Python Quickstart Example (zero external dependencies)."""

from symb_anafis import Context, Evaluator, Expr, Property


def main() -> None:
    ctx = Context()
    x, y = ctx.symbols("x", "y")

    # 1. Expressions and multivariable calculus
    f = x**2 + x * y + y**2
    print(f"f(x, y) = {f}")
    print(f"∇f = {f.gradient([x, y])}")
    print(f"H(f) = {f.hessian([x, y])}")
    print(f"Δf = {f.laplacian([x, y])}")

    # 2. Multi-output Jacobian
    print(f"J = {Expr.jacobian([f, x * y], [x, y])}")

    # 3. Direct canonical simplification & domain assumptions
    identity = x.sin() ** 2 + x.cos() ** 2
    print(f"{identity} -> {identity.compact()}")

    x.set_properties([Property.POSITIVE])
    print(f"sqrt(x^2) for x > 0 -> {(x**2).sqrt().compact()}")
    x.set_properties([])

    # 4. Expansion & factoring
    poly = (x + 1) ** 3
    print(f"{poly} expanded = {poly.expand()}")
    print(f"{x**2 + x * y} factored = {(x**2 + x * y).factor()}")

    # 5. Exact, native JIT, and standalone WebAssembly evaluation
    exact = Evaluator(f, [x, y])
    print(f"f(2, 3) exact = {exact.evaluate([2, 3])}")

    native = Evaluator(f, [x, y], backend="f64", native=True)
    print(f"f(2, 3) native JIT = {native.evaluate([2.0, 3.0])}")
    print(f"WASM binary size = {len(native.to_wasm_bytes())} bytes")


if __name__ == "__main__":
    main()
