"""Identity-based direct evaluation and reusable scalar register programs."""

from __future__ import annotations

from fractions import Fraction
from numbers import Real
from typing import Any, Sequence

try:
    import numpy as np
except ImportError:
    np = None  # type: ignore[assignment]

from . import _native
from ._expression import Expr, Symbol
from ._number import Number


class Evaluator:
    """Compile once and reuse a scalar workspace with ordered Symbol parameters.

    ``number`` preserves exact/complex arithmetic and returns Number values.
    ``f64`` returns floats and permits the native plan's FMA/reassociation policy.
    Batch inputs accept Python sequences or contiguous NumPy arrays.
    Each clone owns an independent workspace.
    """

    __slots__ = ("_native", "_backend", "_parameters")

    def __init__(self, expression: object, parameters=None, *, backend: str = "number") -> None:
        model = Expr(expression)
        chosen = model.symbols if parameters is None else tuple(parameters)
        self._native = _native._Evaluator(model._native, [Expr(value)._native for value in chosen], backend)
        self._backend = backend
        self._parameters = tuple(Symbol._wrap(value) for value in self._native.parameters())

    @staticmethod
    def _is_buffer(values: object) -> bool:
        if np is not None and isinstance(values, np.ndarray):
            return True
        return hasattr(values, "__buffer__") or hasattr(values, "__array_interface__")

    @classmethod
    def _values(cls, values, backend: str):
        if backend == "number":
            return [Number._native_for(value) for value in values]
        if backend != "f64":
            raise ValueError("backend must be 'number' or 'f64'")
        if cls._is_buffer(values):
            return values
        if hasattr(values, "tolist") and not isinstance(values, (list, tuple)):
            return values.tolist()
        result = []
        for value in values:
            if isinstance(value, bool) or not isinstance(value, (Number, Real, Fraction)):
                raise TypeError("the f64 backend requires real numeric inputs")
            result.append(float(value))
        return result

    @classmethod
    def direct(cls, expression: object, bindings=(), *, backend: str = "number") -> Number | float:
        pairs = list(bindings.items() if hasattr(bindings, "items") else bindings)
        parameters = [Expr(symbol)._native for symbol, _ in pairs]
        numbers = cls._values((value for _, value in pairs), backend)
        result = _native._Evaluator.direct(Expr(expression)._native, parameters, numbers, backend)
        return Number._wrap(result) if backend == "number" else result

    @property
    def parameters(self) -> tuple[Symbol, ...]:
        return self._parameters

    @property
    def backend(self) -> str:
        return self._backend

    @property
    def instruction_count(self) -> int:
        return self._native.instruction_count()

    @property
    def workspace_size(self) -> int:
        return self._native.workspace_size()

    def evaluate(self, values: Sequence[Any] | np.ndarray) -> Number | float:
        result = self._native.evaluate(self._values(values, self._backend))
        return Number._wrap(result) if self._backend == "number" else result

    def evaluate_batch(
        self,
        columns: Sequence[Sequence[Any] | np.ndarray] | np.ndarray,
        *,
        rows: int | None = None,
        out: list[float] | np.ndarray | None = None,
    ) -> list[Number] | list[float] | np.ndarray:
        values = [self._values(column, self._backend) for column in columns]
        count = (len(values[0]) if values else 0) if rows is None else rows

        target_out = out
        if (
            target_out is None
            and self._backend == "f64"
            and np is not None
            and any(self._is_buffer(c) for c in values)
        ):
            target_out = np.empty(count, dtype=np.float64)

        result = self._native.evaluate_batch(values, count, target_out)
        return [Number._wrap(value) for value in result] if self._backend == "number" else result

    def copy(self) -> Evaluator:
        value = object.__new__(type(self))
        value._native = self._native.copy()
        value._backend, value._parameters = self._backend, self._parameters
        return value

    def disassemble(self) -> str:
        return self._native.disassemble()
