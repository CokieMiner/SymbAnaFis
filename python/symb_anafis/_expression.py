"""Immutable expressions, owning symbols, and ordered multivariable calculus."""

from __future__ import annotations

import dataclasses
from typing import TYPE_CHECKING

from . import _native
from ._math import MathMethods
from ._number import Number
from ._properties import Properties

if TYPE_CHECKING:
    from ._context import Context
    from ._function import Function
    from ._simplification import SimplificationSequence


class Expr(MathMethods):
    """An immutable canonical DAG handle; equality compares Expr structure.

    Numeric operands are converted without float promotion. Strings are parsed
    through an explicit Context or Context.default(). Other contexts retain
    distinct symbol identities, even when their names are equal.
    """

    __slots__ = ("_native",)

    def __init__(self, value: object = 0, *, context: Context | None = None) -> None:
        if isinstance(value, Expr):
            native = value._native
        elif isinstance(value, str):
            from ._context import Context
            native = (context if context is not None else Context.default()).parse(value)._native
        else:
            native = _native._Expr(Number(value)._native)
        object.__setattr__(self, "_native", native)

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError("expressions are immutable")

    @classmethod
    def _wrap(cls, native: _native._Expr) -> Expr:
        value = object.__new__(Symbol if native.kind() == "symbol" else Expr)
        object.__setattr__(value, "_native", native)
        return value

    @classmethod
    def pi(cls) -> Expr:
        return cls._wrap(_native._Expr.constant("pi"))

    @classmethod
    def e(cls) -> Expr:
        return cls._wrap(_native._Expr.constant("e"))

    @classmethod
    def i(cls) -> Expr:
        return cls._wrap(_native._Expr.constant("i"))

    @classmethod
    def sum(cls, values) -> Expr:
        return cls._wrap(_native._Expr.sum([Expr(value)._native for value in values]))

    @classmethod
    def product(cls, values) -> Expr:
        return cls._wrap(_native._Expr.product([Expr(value)._native for value in values]))

    def _apply(self, name: str, *arguments: object) -> Expr:
        from ._function import Function
        return Function.builtin(name)(*arguments)

    def _binary(self, operation: str, other: object, reverse: bool = False) -> Expr:
        from ._rewriting import Pattern
        if isinstance(other, Pattern):
            return NotImplemented
        operand = Expr(other)
        left, right = (operand, self) if reverse else (self, operand)
        return Expr._wrap(left._native.binary(operation, right._native))

    def __add__(self, other: object) -> Expr: return self._binary("add", other)
    def __radd__(self, other: object) -> Expr: return self._binary("add", other, True)
    def __sub__(self, other: object) -> Expr: return self._binary("sub", other)
    def __rsub__(self, other: object) -> Expr: return self._binary("sub", other, True)
    def __mul__(self, other: object) -> Expr: return self._binary("mul", other)
    def __rmul__(self, other: object) -> Expr: return self._binary("mul", other, True)
    def __truediv__(self, other: object) -> Expr: return self._binary("div", other)
    def __rtruediv__(self, other: object) -> Expr: return self._binary("div", other, True)

    def __pow__(self, exponent: object, modulo: object = None) -> Expr:
        if modulo is not None:
            raise TypeError("modular powers are not symbolic powers")
        return self._binary("pow", exponent)

    def __rpow__(self, base: object, modulo: object = None) -> Expr:
        if modulo is not None:
            raise TypeError("modular powers are not symbolic powers")
        return self._binary("pow", base, True)

    def __neg__(self) -> Expr:
        return Expr._wrap(self._native.negative())

    def __pos__(self) -> Expr:
        return self

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Expr) and self._native.equal(other._native)

    def __hash__(self) -> int:
        return hash(self._native.hash())

    def __bool__(self) -> bool:
        raise TypeError("symbolic expressions have no truth value; compare Expr values or query properties")

    def __str__(self) -> str:
        return self._native.format("text")

    def __repr__(self) -> str:
        return f"Expr({str(self)!r})"

    def to_latex(self) -> str:
        return self._native.format("latex")

    def to_unicode(self) -> str:
        return self._native.format("unicode")

    def _repr_latex_(self) -> str:
        return f"${self.to_latex()}$"

    @property
    def kind(self) -> str:
        return self._native.kind()

    @property
    def value(self) -> Number:
        number = self._native.number()
        if number is None:
            raise TypeError("this expression is not a numeric literal")
        return Number._wrap(number)

    @property
    def args(self) -> tuple[Expr, ...]:
        return tuple(Expr._wrap(value) for value in self._native.children())

    @property
    def symbols(self) -> tuple[Symbol, ...]:
        return tuple(Expr._wrap(value) for value in self._native.symbols())

    @property
    def properties(self) -> Properties:
        return Properties(self._native.facts())

    @property
    def node_count(self) -> int:
        return self._native.node_count()

    @property
    def max_depth(self) -> int:
        return self._native.max_depth()

    @property
    def view(self) -> ExprView:
        from ._function import Function
        function = self._native.function()
        derivative = self._native.derivative_metadata()
        return ExprView(self.kind, self.args, Function._wrap(function) if function is not None else None,
                        Symbol._wrap(derivative[0]) if derivative is not None else None,
                        derivative[1] if derivative is not None else None)

    def derivative(self, variable: Symbol, order: int = 1) -> Expr:
        return Expr._wrap(self._native.derivative(Expr(variable)._native, order))

    def gradient(self, variables) -> list[Expr]:
        return [Expr._wrap(value) for value in self._native.gradient([Expr(value)._native for value in variables])]

    @classmethod
    def jacobian(cls, outputs, variables) -> list[list[Expr]]:
        rows = _native._Expr.jacobian([Expr(value)._native for value in outputs],
                                     [Expr(value)._native for value in variables])
        return [[Expr._wrap(value) for value in row] for row in rows]

    def hessian(self, variables) -> list[list[Expr]]:
        return [[Expr._wrap(value) for value in row] for row in
                self._native.hessian([Expr(value)._native for value in variables])]

    def substitute(self, source: object, destination: object) -> Expr:
        from ._rewriting import Substitution
        return Substitution(source, destination).apply(self)

    def map(self, callback) -> Expr:
        def adapted(native):
            result = callback(Expr._wrap(native))
            return None if result is None else Expr(result)._native
        return Expr._wrap(self._native.map(adapted))

    def fold(self, initial, callback):
        return self._native.fold(initial, lambda state, value: callback(state, Expr._wrap(value)))

    def simplify(self, sequence: SimplificationSequence | None = None) -> Expr:
        from ._simplification import SimplificationSequence
        selected = sequence if sequence is not None else SimplificationSequence._default()
        return selected.simplify(self).expression

    def evaluate(self, bindings=(), *, backend: str = "number") -> Number | float:
        from ._evaluation import Evaluator
        return Evaluator.direct(self, bindings, backend=backend)

    def compile(self, parameters=None, *, backend: str = "number"):
        from ._evaluation import Evaluator
        return Evaluator(self, parameters, backend=backend)


class Symbol(Expr):
    """An owning symbol expression; no borrowed raw Rust ID escapes to Python."""

    __slots__ = ()

    def __init__(self, name: str, *, context: Context | None = None) -> None:
        from ._context import Context
        owner = context if context is not None else Context.default()
        object.__setattr__(self, "_native", owner._native.symb(name))

    @property
    def id(self) -> int:
        return self._native.symbol_id()

    @property
    def name(self) -> str | None:
        return self._native.symbol_name()

    def __repr__(self) -> str:
        return f"Symbol({self.name!r}, id={self.id})"


@dataclasses.dataclass(frozen=True)
class ExprView:
    """Logical structure with owning children, function, and derivative metadata."""

    kind: str
    args: tuple[Expr, ...]
    function: Function | None = None
    variable: Symbol | None = None
    order: int | None = None
