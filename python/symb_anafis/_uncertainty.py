"""Real first-order covariance contraction with explicit modelling hypotheses."""

from __future__ import annotations

from . import _native
from ._expression import Expr


class CovarianceMatrix:
    """Square symmetric covariance expressions in a declared coordinate order.

    Unknown scalar conditions and positive semidefiniteness belong to the caller.
    Symmetry uses canonical symbolic identity or exact numeric value equality,
    without epsilon tolerances. Diagonal entries are variances.
    """

    __slots__ = ("_native",)

    def __init__(self, entries) -> None:
        self._native = _native._CovarianceMatrix([[Expr(value)._native for value in row] for row in entries])

    @classmethod
    def diagonal(cls, variances) -> CovarianceMatrix:
        value = object.__new__(cls)
        value._native = _native._CovarianceMatrix.diagonal([Expr(item)._native for item in variances])
        return value

    @property
    def dimension(self) -> int:
        return self._native.dimension()

    def get(self, row: int, column: int) -> Expr | None:
        if not 0 <= row < self.dimension or not 0 <= column < self.dimension:
            return None
        value = self._native.get(row, column)
        return None if value is None else Expr._wrap(value)

    def __getitem__(self, indices: tuple[int, int]) -> Expr:
        value = self.get(*indices)
        if value is None:
            raise IndexError("covariance coordinate is out of range")
        return value


class Uncertainty:
    """A real, local first-order model with variance g.T @ covariance @ g.

    Real coordinates, regular derivatives, physical covariance validity, and
    adequacy of the linear approximation are caller-owned hypotheses.
    """

    __slots__ = ("_native",)

    def __new__(cls):
        raise TypeError("use Uncertainty.propagate()")

    @classmethod
    def propagate(cls, expression: object, variables, covariance: CovarianceMatrix) -> Uncertainty:
        value = object.__new__(cls)
        value._native = _native._Uncertainty.propagate(Expr(expression)._native,
            [Expr(variable)._native for variable in variables], covariance._native)
        return value

    @property
    def expression(self) -> Expr:
        return Expr._wrap(self._native.expression())

    @property
    def sensitivities(self) -> tuple[Expr, ...]:
        return tuple(Expr._wrap(value) for value in self._native.sensitivities())

    @property
    def variance(self) -> Expr:
        return Expr._wrap(self._native.variance())

    def standard_deviation(self) -> Expr:
        return Expr._wrap(self._native.standard_deviation())

    def relative(self) -> Expr:
        return Expr._wrap(self._native.relative())
