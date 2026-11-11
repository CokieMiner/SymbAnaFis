"""Representation-preserving scalar values with native integer/rational capacity."""

from __future__ import annotations

from fractions import Fraction
from numbers import Complex, Integral, Real

from . import _native


class Number:
    """Exact integers/fractions and approximate floats, including complex components.

    Integers have signed 64-bit capacity; reduced rational denominators have
    unsigned 64-bit capacity. Unsupported Python integers raise OverflowError.
    ``float`` and ``complex`` conversions explicitly request approximation.
    """

    __slots__ = ("_native",)
    __hash__ = None

    def __init__(self, value: object = 0) -> None:
        self._native = self._native_for(value)

    @staticmethod
    def _native_for(value: object) -> _native._Number:
        if isinstance(value, Number):
            return value._native
        if not isinstance(value, (int, float, complex, Fraction)):
            if isinstance(value, Integral):
                value = int(value)
            elif isinstance(value, Real):
                value = float(value)
            elif isinstance(value, Complex):
                value = complex(value)
        return _native._Number(value)

    @classmethod
    def _wrap(cls, native: _native._Number) -> Number:
        value = object.__new__(cls)
        value._native = native
        return value

    @classmethod
    def complex(cls, real: object, imaginary: object) -> Number:
        """Construct exact or approximate real components without float coercion."""
        return cls._wrap(_native._Number.complex(cls(real)._native, cls(imaginary)._native))

    @property
    def real(self) -> int | float | Fraction:
        return self._native.components()[0]

    @property
    def imag(self) -> int | float | Fraction:
        return self._native.components()[1]

    @property
    def is_exact(self) -> bool:
        return all(not isinstance(value, float) for value in self._native.components())

    @property
    def is_finite(self) -> bool:
        return self._native.is_finite()

    @property
    def is_real(self) -> bool:
        return self._native.is_real()

    def to_python(self) -> int | float | Fraction | complex:
        """Return a Python scalar; nonreal components are explicitly approximated."""
        real, imaginary = self._native.components()
        return real if imaginary == 0 else complex(float(real), float(imaginary))

    def __float__(self) -> float:
        if not self.is_real:
            raise TypeError("a nonreal Number cannot be converted to float")
        return float(self.real)

    def __int__(self) -> int:
        if not self.is_real:
            raise TypeError("a nonreal Number cannot be converted to int")
        return int(self.real)

    def __complex__(self) -> complex:
        real, imaginary = self._native.components()
        return complex(float(real), float(imaginary))

    def __bool__(self) -> bool:
        return not self._native.is_zero()

    def __str__(self) -> str:
        return str(self._native)

    def __repr__(self) -> str:
        real, imaginary = self._native.components()
        return f"Number({real!r})" if imaginary == 0 else f"Number.complex({real!r}, {imaginary!r})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Number):
            return self._native.equal(other._native)
        real, imaginary = self._native.components()
        if isinstance(other, complex):
            return real == other.real and imaginary == other.imag
        if isinstance(other, (int, float, Fraction)):
            return imaginary == 0 and real == other
        return NotImplemented

    def _binary(self, operation: str, other: object, reverse: bool = False) -> Number:
        if not isinstance(other, (Number, int, float, complex, Fraction)):
            return NotImplemented
        operand = Number(other)
        left, right = (operand, self) if reverse else (self, operand)
        return Number._wrap(left._native.binary(operation, right._native))

    def __add__(self, other: object) -> Number:
        return self._binary("add", other)

    def __radd__(self, other: object) -> Number:
        return self._binary("add", other, True)

    def __sub__(self, other: object) -> Number:
        return self._binary("sub", other)

    def __rsub__(self, other: object) -> Number:
        return self._binary("sub", other, True)

    def __mul__(self, other: object) -> Number:
        return self._binary("mul", other)

    def __rmul__(self, other: object) -> Number:
        return self._binary("mul", other, True)

    def __truediv__(self, other: object) -> Number:
        return self._binary("div", other)

    def __rtruediv__(self, other: object) -> Number:
        return self._binary("div", other, True)

    def __pow__(self, other: object, modulo: object = None) -> Number:
        if modulo is not None:
            raise TypeError("modular powers are not scalar symbolic powers")
        return self._binary("pow", other)

    def __rpow__(self, other: object, modulo: object = None) -> Number:
        if modulo is not None:
            raise TypeError("modular powers are not scalar symbolic powers")
        return self._binary("pow", other, True)

    def __neg__(self) -> Number:
        return Number._wrap(self._native.negative())

    def __pos__(self) -> Number:
        return self
