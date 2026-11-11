"""Structural templates, bounded rewriting, and simultaneous substitutions."""

from __future__ import annotations

from typing import Callable, Iterable, Iterator, Mapping

from . import _expression, _native
from ._expression import Expr, _SCALAR_OPERAND_TYPES
from ._function import Function
from ._math import MathMethods
from ._number import _BOOL_TYPES
from ._properties import Property


class Pattern(MathMethods):
    """A structural template; scalar conditions require proved properties."""

    __slots__ = ("_native",)

    def __init__(self, value: object) -> None:
        self._native = (
            value._native
            if isinstance(value, Pattern)
            else _native._Pattern.exact(Expr(value)._native)
        )

    @classmethod
    def _wrap(cls, native: _native._Pattern) -> Pattern:
        value = object.__new__(cls)
        value._native = native
        return value

    @classmethod
    def _coerce_ufunc_operand(cls, value: object) -> Pattern:
        return Pattern(value)

    @classmethod
    def capture(cls, name: str, *, properties: Iterable[Property | str] = ()) -> Pattern:
        return cls._wrap(
            _native._Pattern.capture(name, [Property(value).value for value in properties])
        )

    @classmethod
    def exact(cls, expression: object) -> Pattern:
        return cls._wrap(_native._Pattern.exact(Expr(expression)._native))

    @classmethod
    def sum(cls, values: Iterable[object]) -> Pattern:
        """Construct a normalized n-ary sum pattern."""
        return cls._wrap(_native._Pattern.sum([Pattern(value)._native for value in values]))

    @classmethod
    def product(cls, values: Iterable[object]) -> Pattern:
        """Construct a normalized n-ary product pattern."""
        return cls._wrap(_native._Pattern.product([Pattern(value)._native for value in values]))

    def _apply(self, name: str, *arguments: object) -> Pattern:
        values = [Pattern(value)._native for value in arguments]
        return Pattern._wrap(_native._Pattern.call(Function.builtin(name)._native, values))

    def _binary(self, operation: str, other: object, reverse: bool = False) -> Pattern:
        if isinstance(other, _BOOL_TYPES):
            raise TypeError("booleans are not scalar numeric inputs")
        if not isinstance(other, (Pattern, Expr)) and not isinstance(other, _SCALAR_OPERAND_TYPES):
            return NotImplemented
        operand = other if isinstance(other, Pattern) else Pattern(other)
        left, right = (operand, self) if reverse else (self, operand)
        return Pattern._wrap(left._native.binary(operation, right._native))

    def __add__(self, other: object) -> Pattern: return self._binary("add", other)
    def __radd__(self, other: object) -> Pattern: return self._binary("add", other, True)
    def __sub__(self, other: object) -> Pattern: return self._binary("sub", other)
    def __rsub__(self, other: object) -> Pattern: return self._binary("sub", other, True)
    def __mul__(self, other: object) -> Pattern: return self._binary("mul", other)
    def __rmul__(self, other: object) -> Pattern: return self._binary("mul", other, True)
    def __truediv__(self, other: object) -> Pattern: return self._binary("div", other)
    def __rtruediv__(self, other: object) -> Pattern: return self._binary("div", other, True)

    def __pow__(self, exponent: object, modulo: object = None) -> Pattern:
        if modulo is not None:
            raise TypeError("modular powers are not pattern powers")
        return self._binary("pow", exponent)

    def __rpow__(self, base: object, modulo: object = None) -> Pattern:
        if modulo is not None:
            raise TypeError("modular powers are not pattern powers")
        return self._binary("pow", base, True)

    def __neg__(self) -> Pattern:
        return Pattern._wrap(self._native.negative())

    def __pos__(self) -> Pattern:
        return self

    def __bool__(self) -> bool:
        raise TypeError("a rewrite template has no truth value")

    def __repr__(self) -> str:
        return f"Pattern(id=0x{id(self):x})"


class RewriteRule:
    """One memoized pass; custom identities and callbacks belong to the caller.

    Replacements receive a dict[str, Expr]. Guards return bool and can reject
    assignments before replacement. Match exhaustion and callback exceptions
    discard the provisional output; callback side effects cannot be rolled back.
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        source: Pattern | object,
        replacement: Pattern | Callable[[dict[str, Expr]], object] | object,
    ) -> None:
        if callable(replacement):
            def adapted(bindings: dict[str, _native._Expr]) -> _native._Expr:
                result = replacement({name: Expr._wrap(value) for name, value in bindings.items()})
                return Expr(result)._native
            destination: object = adapted
        else:
            destination = Pattern(replacement)._native
        self._native = _native._Rule(Pattern(source)._native, destination)  # type: ignore[arg-type]

    @property
    def required_functions(self) -> tuple[Function, ...]:
        """Function identities whose absence rules out a structural match."""
        return tuple(Function._wrap(fn) for fn in self._native.required_functions())

    def apply(
        self,
        expression: object,
        *,
        match_limit: int | None = 100_000,
        guard: Callable[[dict[str, Expr]], bool] | None = None,
    ) -> Expr:
        accepted = None if guard is None else lambda captures: guard({
            name: Expr._wrap(value) for name, value in captures.items()
        })
        return Expr._wrap(self._native.apply(Expr(expression)._native, match_limit, accepted))

    def __repr__(self) -> str:
        req = tuple(fn.name for fn in self.required_functions)
        return f"RewriteRule(required_functions={req!r})"


class Substitutions:
    """Replace concrete sources simultaneously, without revisiting destinations."""

    __slots__ = ("_native",)

    def __init__(
        self,
        entries: Mapping[object, object] | Iterable[tuple[object, object]],
    ) -> None:
        pairs = entries.items() if isinstance(entries, Mapping) else entries
        self._native = _native._Substitutions(
            [(Expr(source)._native, Expr(destination)._native) for source, destination in pairs]
        )

    def apply(self, expression: object) -> Expr:
        return Expr._wrap(self._native.apply(Expr(expression)._native))

    def get(self, source: object) -> Expr | None:
        value = self._native.get(Expr(source)._native)
        return None if value is None else Expr._wrap(value)

    def items(self) -> tuple[tuple[Expr, Expr], ...]:
        return tuple(
            (Expr._wrap(src), Expr._wrap(dst)) for src, dst in self._native.items()
        )

    @property
    def is_empty(self) -> bool:
        return self._native.is_empty()

    def __iter__(self) -> Iterator[tuple[Expr, Expr]]:
        return iter(self.items())

    def __len__(self) -> int:
        return len(self._native)

    def __bool__(self) -> bool:
        return not self.is_empty

    def __repr__(self) -> str:
        return f"Substitutions({len(self)} entries)"


class Substitution(Substitutions):
    """One concrete source/destination entry with the same simultaneous semantics."""

    __slots__ = ("_source", "_destination")

    def __init__(self, source: object, destination: object) -> None:
        self._source = Expr(source)
        self._destination = Expr(destination)
        super().__init__([(self._source, self._destination)])

    @property
    def source(self) -> Expr:
        return self._source

    @property
    def destination(self) -> Expr:
        return self._destination

    def __repr__(self) -> str:
        return f"Substitution({str(self._source)!r} -> {str(self._destination)!r})"


_expression._PATTERN_TYPE = Pattern
