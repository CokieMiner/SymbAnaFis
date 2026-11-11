"""Mathematical methods and NumPy ufunc dispatch shared by expressions and patterns."""

from __future__ import annotations

from typing import Any, Callable, TypeVar

try:
    import numpy as np
except ImportError:
    np = None  # type: ignore[assignment]

_Self = TypeVar("_Self", bound="MathMethods")

_UNARY_UFUNCS: dict[str, Callable[[Any], Any]] = {
    "negative": lambda x: -x,
    "positive": lambda x: +x,
    "absolute": lambda x: x.abs(),
    "fabs": lambda x: x.abs(),
    "sign": lambda x: x.signum(),
    "conjugate": lambda x: x.conj(),
    "conj": lambda x: x.conj(),
    "square": lambda x: x**2,
    "reciprocal": lambda x: 1 / x,
    "sqrt": lambda x: x.sqrt(),
    "cbrt": lambda x: x.cbrt(),
    "exp": lambda x: x.exp(),
    "expm1": lambda x: x.exp_m1(),
    "exp2": lambda x: 2**x,
    "log": lambda x: x.ln(),
    "log1p": lambda x: x.ln_1p(),
    "log2": lambda x: x.log2(),
    "log10": lambda x: x.log10(),
    "sin": lambda x: x.sin(),
    "cos": lambda x: x.cos(),
    "tan": lambda x: x.tan(),
    "arcsin": lambda x: x.asin(),
    "arccos": lambda x: x.acos(),
    "arctan": lambda x: x.atan(),
    "sinh": lambda x: x.sinh(),
    "cosh": lambda x: x.cosh(),
    "tanh": lambda x: x.tanh(),
    "arcsinh": lambda x: x.asinh(),
    "arccosh": lambda x: x.acosh(),
    "arctanh": lambda x: x.atanh(),
    "floor": lambda x: x.floor(),
    "ceil": lambda x: x.ceil(),
    "trunc": lambda x: x.trunc(),
    "rint": lambda x: x.round(),
}

_BINARY_UFUNCS: dict[str, Callable[[Any, Any, Callable[[Any], Any]], Any]] = {
    "add": lambda a, b, _: a + b,
    "subtract": lambda a, b, _: a - b,
    "multiply": lambda a, b, _: a * b,
    "true_divide": lambda a, b, _: a / b,
    "divide": lambda a, b, _: a / b,
    "power": lambda a, b, _: a**b,
    "float_power": lambda a, b, _: a**b,
    "arctan2": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).atan2(b),
    "hypot": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).hypot(b),
    "maximum": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).max(b),
    "minimum": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).min(b),
    "remainder": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).modulo(b),
    "mod": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).modulo(b),
    "fmod": lambda a, b, coerce: (a if isinstance(a, MathMethods) else coerce(a)).rem(b),
}


class MathMethods:
    """Construct registered builtin calls on their regular principal branches."""

    __slots__ = ()

    def _apply(self: _Self, name: str, *arguments: object) -> _Self:
        raise NotImplementedError

    @classmethod
    def _coerce_ufunc_operand(cls, value: object) -> Any:
        raise NotImplementedError

    def __array_ufunc__(self, ufunc: Any, method: str, *inputs: Any, **kwargs: Any) -> Any:
        if method != "__call__":
            return NotImplemented
        if any(k != "out" or v is not None for k, v in kwargs.items()):
            return NotImplemented

        name = getattr(ufunc, "__name__", None)
        if len(inputs) == 1:
            unary_op = _UNARY_UFUNCS.get(name)
            if unary_op is None:
                return NotImplemented
            arg = inputs[0]
            if np is not None and isinstance(arg, np.ndarray):
                arr = np.asarray(arg, dtype=object)
                out = np.empty(arr.shape, dtype=object)
                out.flat = [unary_op(item) for item in arr.flat]
                return out
            return unary_op(arg)

        if len(inputs) == 2:
            binary_op = _BINARY_UFUNCS.get(name)
            if binary_op is None:
                return NotImplemented
            # Prefer Pattern coercion if either operand is a Pattern; otherwise use self's class.
            coerce = self._coerce_ufunc_operand
            for item in inputs:
                if isinstance(item, MathMethods) and type(item).__name__ == "Pattern":
                    coerce = item._coerce_ufunc_operand
                    break
            lhs, rhs = inputs
            if np is not None and (isinstance(lhs, np.ndarray) or isinstance(rhs, np.ndarray)):
                b_lhs, b_rhs = np.broadcast_arrays(
                    np.asarray(lhs, dtype=object),
                    np.asarray(rhs, dtype=object),
                )
                out = np.empty(b_lhs.shape, dtype=object)
                out.flat = [binary_op(a, b, coerce) for a, b in zip(b_lhs.flat, b_rhs.flat)]
                return out
            return binary_op(lhs, rhs, coerce)

        return NotImplemented

    def sin(self: _Self) -> _Self: return self._apply("sin", self)
    def cos(self: _Self) -> _Self: return self._apply("cos", self)
    def tan(self: _Self) -> _Self: return self._apply("tan", self)
    def cot(self: _Self) -> _Self: return self._apply("cot", self)
    def sec(self: _Self) -> _Self: return self._apply("sec", self)
    def csc(self: _Self) -> _Self: return self._apply("csc", self)
    def asin(self: _Self) -> _Self: return self._apply("asin", self)
    def acos(self: _Self) -> _Self: return self._apply("acos", self)
    def atan(self: _Self) -> _Self: return self._apply("atan", self)
    def acot(self: _Self) -> _Self: return self._apply("acot", self)
    def asec(self: _Self) -> _Self: return self._apply("asec", self)
    def acsc(self: _Self) -> _Self: return self._apply("acsc", self)
    def sinh(self: _Self) -> _Self: return self._apply("sinh", self)
    def cosh(self: _Self) -> _Self: return self._apply("cosh", self)
    def tanh(self: _Self) -> _Self: return self._apply("tanh", self)
    def coth(self: _Self) -> _Self: return self._apply("coth", self)
    def sech(self: _Self) -> _Self: return self._apply("sech", self)
    def csch(self: _Self) -> _Self: return self._apply("csch", self)
    def asinh(self: _Self) -> _Self: return self._apply("asinh", self)
    def acosh(self: _Self) -> _Self: return self._apply("acosh", self)
    def atanh(self: _Self) -> _Self: return self._apply("atanh", self)
    def acoth(self: _Self) -> _Self: return self._apply("acoth", self)
    def asech(self: _Self) -> _Self: return self._apply("asech", self)
    def acsch(self: _Self) -> _Self: return self._apply("acsch", self)
    def exp(self: _Self) -> _Self: return self._apply("exp", self)
    def exp_m1(self: _Self) -> _Self: return self._apply("exp_m1", self)
    def ln(self: _Self) -> _Self: return self._apply("ln", self)
    def ln_1p(self: _Self) -> _Self: return self._apply("ln_1p", self)
    def log10(self: _Self) -> _Self: return self._apply("log10", self)
    def log2(self: _Self) -> _Self: return self._apply("log2", self)
    def sqrt(self: _Self) -> _Self: return self._apply("sqrt", self)
    def cbrt(self: _Self) -> _Self: return self._apply("cbrt", self)
    def floor(self: _Self) -> _Self: return self._apply("floor", self)
    def ceil(self: _Self) -> _Self: return self._apply("ceil", self)
    def round(self: _Self) -> _Self: return self._apply("round", self)
    def trunc(self: _Self) -> _Self: return self._apply("trunc", self)
    def fract(self: _Self) -> _Self: return self._apply("fract", self)
    def abs(self: _Self) -> _Self: return self._apply("abs", self)
    def signum(self: _Self) -> _Self: return self._apply("signum", self)
    def re(self: _Self) -> _Self: return self._apply("re", self)
    def im(self: _Self) -> _Self: return self._apply("im", self)
    def conj(self: _Self) -> _Self: return self._apply("conj", self)
    def heaviside(self: _Self) -> _Self: return self._apply("heaviside", self)
    def sinc(self: _Self) -> _Self: return self._apply("sinc", self)
    def erf(self: _Self) -> _Self: return self._apply("erf", self)
    def erfc(self: _Self) -> _Self: return self._apply("erfc", self)
    def gamma(self: _Self) -> _Self: return self._apply("gamma", self)
    def lgamma(self: _Self) -> _Self: return self._apply("lgamma", self)
    def digamma(self: _Self) -> _Self: return self._apply("digamma", self)
    def trigamma(self: _Self) -> _Self: return self._apply("trigamma", self)
    def tetragamma(self: _Self) -> _Self: return self._apply("tetragamma", self)
    def zeta(self: _Self) -> _Self: return self._apply("zeta", self)
    def elliptic_k(self: _Self) -> _Self: return self._apply("elliptic_k", self)
    def elliptic_e(self: _Self) -> _Self: return self._apply("elliptic_e", self)
    def polygamma(self: _Self, order: object) -> _Self: return self._apply("polygamma", order, self)
    def besselj(self: _Self, order: object) -> _Self: return self._apply("besselj", order, self)
    def bessely(self: _Self, order: object) -> _Self: return self._apply("bessely", order, self)
    def besseli(self: _Self, order: object) -> _Self: return self._apply("besseli", order, self)
    def besselk(self: _Self, order: object) -> _Self: return self._apply("besselk", order, self)
    def zeta_deriv(self: _Self, order: object) -> _Self: return self._apply("zeta_deriv", order, self)
    def log(self: _Self, base: object) -> _Self: return self._apply("log", base, self)
    def hermite(self: _Self, degree: object) -> _Self: return self._apply("hermite", degree, self)
    def lambertw(self: _Self, branch: object = 0) -> _Self: return self._apply("lambertw", branch, self)
    def beta(self: _Self, other: object) -> _Self: return self._apply("beta", self, other)
    def atan2(self: _Self, x: object) -> _Self: return self._apply("atan2", self, x)
    def hypot(self: _Self, other: object) -> _Self: return self._apply("hypot", self, other)
    def max(self: _Self, other: object) -> _Self: return self._apply("max", self, other)
    def min(self: _Self, other: object) -> _Self: return self._apply("min", self, other)
    def modulo(self: _Self, other: object) -> _Self: return self._apply("mod", self, other)
    def rem(self: _Self, other: object) -> _Self: return self._apply("rem", self, other)
    def assoc_legendre(self: _Self, degree: object, order: object) -> _Self:
        return self._apply("assoc_legendre", degree, order, self)
    def clamp(self: _Self, min: object, max: object) -> _Self:
        return self._apply("clamp", self, min, max)

    def __abs__(self: _Self) -> _Self: return self.abs()

    def __round__(self: _Self, ndigits: int | None = None) -> _Self:
        if ndigits is not None:
            raise TypeError("symbolic round() does not accept ndigits")
        return self.round()
