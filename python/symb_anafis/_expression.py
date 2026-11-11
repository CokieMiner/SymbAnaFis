"""Immutable expressions, owning symbols, and ordered multivariable calculus."""

from __future__ import annotations

import dataclasses
from fractions import Fraction
from numbers import Complex as PyComplexABC, Integral as PyIntegralABC, Real as PyRealABC
from typing import TYPE_CHECKING, Any, Callable, Iterable, Mapping, TypeVar, overload

from . import _native
from ._math import MathMethods
from ._number import Number, _BOOL_TYPES
from ._properties import Properties, Property

if TYPE_CHECKING:
    from ._context import Context
    from ._evaluation import BackendName, Evaluator
    from ._function import Function

_Acc = TypeVar("_Acc")

# Populated once when `_function` and `_rewriting` are imported, avoiding per-op lazy imports.
_FUNCTION_TYPE: type[Any] | None = None
_PATTERN_TYPE: type[Any] | None = None

_SCALAR_OPERAND_TYPES = (
    Number,
    int,
    float,
    complex,
    Fraction,
    str,
    PyIntegralABC,
    PyRealABC,
    PyComplexABC,
)


def _function_cls() -> type[Function]:
    if _FUNCTION_TYPE is not None:
        return _FUNCTION_TYPE
    from ._function import Function
    return Function


class _JacobianDescriptor:
    @overload
    def __get__(
        self, instance: None, owner: type[Expr] | None = None
    ) -> Callable[[Iterable[object] | Expr, Iterable[Symbol | Expr]], list[list[Expr]]]: ...

    @overload
    def __get__(
        self, instance: Expr, owner: type[Expr] | None = None
    ) -> Callable[..., list[list[Expr]]]: ...

    def __get__(self, instance: Expr | None, owner: type[Expr] | None = None):
        if instance is None:
            return self._class_jacobian

        def _instance_jacobian(
            variables_or_outputs: Iterable[object] | Expr,
            maybe_variables: Iterable[Symbol | Expr] | None = None,
        ) -> list[list[Expr]]:
            if maybe_variables is not None:
                return self._class_jacobian(variables_or_outputs, maybe_variables)
            return self._class_jacobian([instance], variables_or_outputs)  # type: ignore[arg-type]

        return _instance_jacobian

    @staticmethod
    def _class_jacobian(
        outputs: Iterable[object] | Expr,
        variables: Iterable[Symbol | Expr],
    ) -> list[list[Expr]]:
        if isinstance(outputs, Expr):
            outputs = [outputs]
        rows = _native._Expr.jacobian(
            [Expr(value)._native for value in outputs],
            [Expr(value)._native for value in variables],
        )
        return [[Expr._wrap(value) for value in row] for row in rows]


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
    def _coerce_ufunc_operand(cls, value: object) -> Expr:
        return Expr(value)

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
    def sum(cls, values: Iterable[object]) -> Expr:
        return cls._wrap(_native._Expr.sum([Expr(value)._native for value in values]))

    @classmethod
    def product(cls, values: Iterable[object]) -> Expr:
        return cls._wrap(_native._Expr.product([Expr(value)._native for value in values]))

    def _apply(self, name: str, *arguments: object) -> Expr:
        return _function_cls().builtin(name)(*arguments)

    def _binary(self, operation: str, other: object, reverse: bool = False) -> Expr:
        if _PATTERN_TYPE is not None and isinstance(other, _PATTERN_TYPE):
            return NotImplemented
        if isinstance(other, _BOOL_TYPES):
            raise TypeError("booleans are not scalar numeric inputs")
        if not isinstance(other, Expr) and not isinstance(other, _SCALAR_OPERAND_TYPES):
            return NotImplemented
        operand = other if isinstance(other, Expr) else Expr(other)
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
        return tuple(Expr._wrap(value) for value in self._native.symbols())  # type: ignore[misc]

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
        fn_cls = _function_cls()
        function = self._native.function()
        derivative = self._native.derivative_metadata()
        return ExprView(
            self.kind,
            self.args,
            fn_cls._wrap(function) if function is not None else None,
            Symbol._wrap(derivative[0]) if derivative is not None else None,  # type: ignore[arg-type]
            derivative[1] if derivative is not None else None,
        )

    def derivative(self, variable: Symbol | Expr, order: int = 1) -> Expr:
        return Expr._wrap(self._native.derivative(Expr(variable)._native, order))

    def partial_derivatives(
        self,
        orders: Mapping[Symbol | Expr, int] | Iterable[tuple[Symbol | Expr, int]],
    ) -> Expr:
        """Compute mixed partial derivatives in the order given by ``(symbol, order)`` pairs or a mapping."""
        pairs = orders.items() if isinstance(orders, Mapping) else orders
        native_orders = []
        for variable, order in pairs:
            if isinstance(order, bool) or not isinstance(order, int) or order < 0:
                raise ValueError("derivative order must be a nonnegative integer")
            native_orders.append((Expr(variable)._native, order))
        return Expr._wrap(self._native.partial_derivatives(native_orders))

    def directional_derivative(
        self,
        direction: Mapping[Symbol | Expr, object] | Iterable[tuple[Symbol | Expr, object]],
    ) -> Expr:
        """Compute the directional derivative along ``(symbol, component)`` pairs or a mapping."""
        pairs = direction.items() if isinstance(direction, Mapping) else direction
        native_direction = [
            (Expr(variable)._native, Expr(component)._native)
            for variable, component in pairs
        ]
        return Expr._wrap(self._native.directional_derivative(native_direction))

    def gradient(self, variables: Iterable[Symbol | Expr]) -> list[Expr]:
        return [
            Expr._wrap(value)
            for value in self._native.gradient([Expr(value)._native for value in variables])
        ]

    jacobian = _JacobianDescriptor()

    def hessian(self, variables: Iterable[Symbol | Expr]) -> list[list[Expr]]:
        return [
            [Expr._wrap(value) for value in row]
            for row in self._native.hessian([Expr(value)._native for value in variables])
        ]

    def laplacian(self, variables: Iterable[Symbol | Expr]) -> Expr:
        """Compute the Laplacian (sum of unmixed second partial derivatives) over ``variables``."""
        return Expr._wrap(self._native.laplacian([Expr(value)._native for value in variables]))

    def substitute(self, source: object, destination: object) -> Expr:
        from ._rewriting import Substitution
        return Substitution(source, destination).apply(self)

    def map(self, callback: Callable[[Expr], object | None]) -> Expr:
        def adapted(native: _native._Expr) -> _native._Expr | None:
            result = callback(Expr._wrap(native))
            return None if result is None else Expr(result)._native
        return Expr._wrap(self._native.map(adapted))

    def fold(self, initial: _Acc, callback: Callable[[_Acc, Expr], _Acc]) -> _Acc:
        return self._native.fold(initial, lambda state, value: callback(state, Expr._wrap(value)))

    def compact(self) -> Expr:
        """Simplify the expression to minimize logical DAG size using canonical identities and factoring."""
        return Expr._wrap(self._native.compact())

    def expand(self) -> Expr:
        """Expand polynomial products and powers using bounded distributivity."""
        return Expr._wrap(self._native.expand())

    def factor(self) -> Expr:
        """Factor common subexpressions and composite terms under sums."""
        return Expr._wrap(self._native.factor())

    def contains_var(self, name: str) -> bool:
        """Check whether a variable with the given string name appears in the expression."""
        return self._native.contains_var(name)

    def contains_symbol(self, symbol: Symbol | Expr) -> bool:
        """Check whether a specific scoped symbol identity appears in the expression."""
        return self._native.contains_symbol(Expr(symbol)._native)

    @property
    def functions(self) -> tuple[Function, ...]:
        """Tuple of distinct Function identities occurring in the expression, sorted by name."""
        fn_cls = _function_cls()
        return tuple(fn_cls._wrap(value) for value in self._native.functions())

    def contains_function(self, function: Function) -> bool:
        """Check whether a specific function identity appears in the expression."""
        fn_cls = _function_cls()
        if not isinstance(function, fn_cls):
            raise TypeError("expected a Function identity")
        return self._native.contains_function(function._native)

    def has_free_symbols(self, excluded: Iterable[Symbol | Expr] = ()) -> bool:
        """Check whether any symbol identity outside ``excluded`` appears in the expression."""
        return self._native.has_free_symbols([Expr(value)._native for value in excluded])

    def has_free_variables(self, excluded: Iterable[str] = ()) -> bool:
        """Check whether any variable name outside ``excluded`` appears in the expression."""
        if isinstance(excluded, (str, bytes)):
            raise TypeError("excluded must be an iterable of variable names, not a bare string")
        return self._native.has_free_variables(list(excluded))

    def __contains__(self, item: object) -> bool:
        """Test whether a variable name (str), Symbol, or Function is present in the expression."""
        if isinstance(item, str):
            return self.contains_var(item)
        if _FUNCTION_TYPE is not None and isinstance(item, _FUNCTION_TYPE):
            return self.contains_function(item)
        if isinstance(item, (Expr, Symbol)):
            return self.contains_symbol(item)
        return False

    @property
    def variables(self) -> tuple[str, ...]:
        """Tuple of unique variable names present in the expression, in lexicographical order."""
        return tuple(self._native.variables())

    @property
    def is_constant(self) -> bool:
        """Whether no symbol or derivative variable appears anywhere in the expression."""
        return self._native.is_constant()

    @property
    def is_zero(self) -> bool:
        """Whether this expression is numerically zero (exact or floating-point)."""
        return self._native.is_zero()

    @property
    def is_one(self) -> bool:
        """Whether this expression is numerically one (exact or floating-point)."""
        return self._native.is_one()

    @property
    def is_neg_one(self) -> bool:
        """Whether this expression is numerically negative one (exact or floating-point)."""
        return self._native.is_neg_one()

    @property
    def is_exact_zero(self) -> bool:
        """Whether this expression is the exact integer literal zero (excluding 0.0 / -0.0)."""
        return self._native.is_exact_zero()

    @property
    def is_exact_one(self) -> bool:
        """Whether this expression is the exact integer literal one (excluding 1.0)."""
        return self._native.is_exact_one()

    def evaluate(
        self,
        bindings: Mapping[Symbol | Expr, Any] | Iterable[tuple[Symbol | Expr, Any]] = (),
        *,
        backend: BackendName = "number",
    ) -> Number | float | complex:
        from ._evaluation import Evaluator
        return Evaluator._direct(self, bindings, backend=backend)


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

    def set_properties(self, properties: Iterable[Property | str] = ()) -> None:
        """Replace the closed domain assumptions on this symbol identity (empty resets to default)."""
        self._native.set_properties([Property(prop).value for prop in properties])

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
