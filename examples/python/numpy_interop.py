"""SymbAnaFis + NumPy Interoperability Example.

Demonstrates symbolic NumPy ufuncs (`__array_ufunc__`), zero-copy N-D meshgrid
evaluation, scalar broadcasting, native JIT + Rayon parallel execution, and
zero-copy complex128 buffers.
"""

from __future__ import annotations

import numpy as np
from symb_anafis import Context, Evaluator


def main() -> None:
    ctx = Context()
    x, y = ctx.symbols("x", "y")

    # 1. Symbolic NumPy ufuncs & symmetric array arithmetic
    field = np.sin(x) * np.cos(y) + np.log(1 + x**2 + y**2)
    print(f"field(x, y) = {field}")
    print(f"x + [1, 2, 3] = {x + np.array([1, 2, 3])}")

    # 2. Zero-copy 2D meshgrid evaluation with native JIT + Rayon
    prog = Evaluator(field, [x, y], backend="f64", native=True)
    X, Y = np.meshgrid(np.linspace(-2.0, 2.0, 64), np.linspace(-2.0, 2.0, 64))
    Z = prog.evaluate_batch([X, Y], parallel=True)
    print(f"2D grid shape = {Z.shape}, min = {Z.min():.4f}, max = {Z.max():.4f}")

    # 3. Scalar broadcasting & writable in-place out= buffer
    xs = np.linspace(0.0, 1.0, 5)
    out = np.empty_like(xs)
    prog.evaluate_batch([xs, 0.5], out=out)
    print(f"slice y=0.5: {np.round(out, 4)}")

    # 4. Zero-copy complex128 native JIT evaluation
    z = ctx.symb("z")
    cprog = Evaluator(np.exp(z) - 1, [z], backend="complex128", native=True)
    zs = np.array([0j, 1j * np.pi, 1 + 2j], dtype=np.complex128)
    print(f"exp(z) - 1 = {np.round(cprog.evaluate_batch([zs]), 4)}")


if __name__ == "__main__":
    main()
