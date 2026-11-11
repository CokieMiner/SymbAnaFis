"""Real first-order covariance contraction with explicit modelling hypotheses."""

from __future__ import annotations

from typing import Iterable

from . import _native
from ._context import Context
from ._expression import Expr, Symbol


class CovarianceMatrix:
    """Square symmetric covariance expressions in a declared coordinate order.

    Unknown scalar conditions and positive semidefiniteness belong to the caller.
    Symmetry uses canonical symbolic identity or exact numeric value equality,
    without epsilon tolerances. Diagonal entries are variances.
    """

    __slots__ = ("_native",)

    def __init__(self, entries: Iterable[Iterable[object]]) -> None:
        self._native = _native._CovarianceMatrix(
            [[Expr(value)._native for value in row] for row in entries]
        )

    @classmethod
    def diagonal(cls, variances: Iterable[object]) -> CovarianceMatrix:
        value = object.__new__(cls)
        value._native = _native._CovarianceMatrix.diagonal(
            [Expr(item)._native for item in variances]
        )
        return value

    @classmethod
    def from_factor(cls, factor: Iterable[Iterable[object]]) -> CovarianceMatrix:
        """Construct from a square factor L with Sigma = L @ L.T (e.g. Cholesky)."""
        value = object.__new__(cls)
        value._native = _native._CovarianceMatrix.from_factor(
            [[Expr(item)._native for item in row] for row in factor]
        )
        return value

    @classmethod
    def symbolic_diagonal(
        cls,
        context: Context,
        variables: Iterable[Symbol | Expr],
    ) -> CovarianceMatrix:
        """Construct an independent symbolic covariance with sigma_{var}^2 entries in context."""
        if not isinstance(context, Context):
            raise TypeError("context must be a Context instance")
        value = object.__new__(cls)
        value._native = _native._CovarianceMatrix.symbolic_diagonal(
            context._native,
            [Expr(variable)._native for variable in variables],
        )
        return value

    @classmethod
    def symbolic_correlated(
        cls,
        context: Context,
        variables: Iterable[Symbol | Expr],
    ) -> CovarianceMatrix:
        """Construct a correlated symbolic covariance with sigma_{var}^2 and rho_{a}_{b}*sigma_a*sigma_b entries."""
        if not isinstance(context, Context):
            raise TypeError("context must be a Context instance")
        value = object.__new__(cls)
        value._native = _native._CovarianceMatrix.symbolic_correlated(
            context._native,
            [Expr(variable)._native for variable in variables],
        )
        return value

    @property
    def dimension(self) -> int:
        return self._native.dimension()

    def get(self, row: int, column: int) -> Expr | None:
        if not 0 <= row < self.dimension or not 0 <= column < self.dimension:
            return None
        value = self._native.get(row, column)
        return None if value is None else Expr._wrap(value)

    def contract(self, sensitivities: Iterable[object]) -> Expr:
        """Contract a sensitivity vector g directly into variance g.T @ Sigma @ g."""
        return Expr._wrap(
            self._native.contract([Expr(item)._native for item in sensitivities])
        )

    def __getitem__(self, indices: tuple[int, int]) -> Expr:
        value = self.get(*indices)
        if value is None:
            raise IndexError("covariance coordinate is out of range")
        return value

    def __repr__(self) -> str:
        return f"CovarianceMatrix(dimension={self.dimension})"


class Uncertainty:
    """A real, local first-order model with variance g.T @ covariance @ g.

    Real coordinates, regular derivatives, physical covariance validity, and
    adequacy of the linear approximation are caller-owned hypotheses.
    """

    __slots__ = ("_native",)

    def __new__(cls) -> Uncertainty:
        raise TypeError("use Uncertainty.propagate()")

    @classmethod
    def propagate(
        cls,
        expression: object,
        variables: Iterable[Symbol | Expr],
        covariance: CovarianceMatrix,
    ) -> Uncertainty:
        value = object.__new__(cls)
        value._native = _native._Uncertainty.propagate(
            Expr(expression)._native,
            [Expr(variable)._native for variable in variables],
            covariance._native,
        )
        return value

    def recontract(self, covariance: CovarianceMatrix) -> Expr:
        """Contract the retained sensitivities against another covariance without redifferentiating."""
        return Expr._wrap(self._native.recontract(covariance._native))

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

    def __repr__(self) -> str:
        return f"Uncertainty(expression={str(self.expression)!r}, variance={str(self.variance)!r})"

    def _repr_latex_(self) -> str:
        return f"${self.expression.to_latex()} \\pm {self.standard_deviation().to_latex()}$"
