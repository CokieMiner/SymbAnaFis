"""Structural templates, bounded rewriting, and simultaneous substitutions."""

from __future__ import annotations

from . import _native
from ._expression import Expr
from ._math import MathMethods
from ._properties import Property


class Pattern(MathMethods):
    """A structural template; scalar conditions require proved properties."""

    __slots__ = ("_native",)

    def __init__(self, value: object) -> None:
        self._native = value._native if isinstance(value, Pattern) else _native._Pattern.exact(Expr(value)._native)

    @classmethod
    def _wrap(cls, native: _native._Pattern) -> Pattern:
        value = object.__new__(cls)
        value._native = native
        return value

    @classmethod
    def capture(cls, name: str, *, properties=()) -> Pattern:
        return cls._wrap(_native._Pattern.capture(name, [Property(value).value for value in properties]))

    @classmethod
    def exact(cls, expression: object) -> Pattern:
        return cls._wrap(_native._Pattern.exact(Expr(expression)._native))

    def _apply(self, name: str, *arguments: object) -> Pattern:
        from ._function import Function
        values = [Pattern(value)._native for value in arguments]
        return Pattern._wrap(_native._Pattern.call(Function.builtin(name)._native, values))

    def _binary(self, operation: str, other: object, reverse: bool = False) -> Pattern:
        operand = Pattern(other)
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


class RewriteRule:
    """One memoized pass; custom identities and callbacks belong to the caller.

    Replacements receive a dict[str, Expr]. Guards return bool and can reject
    assignments before replacement. Match exhaustion and callback exceptions
    discard the provisional output; callback side effects cannot be rolled back.
    """

    __slots__ = ("_native",)

    def __init__(self, source: Pattern, replacement) -> None:
        if callable(replacement):
            def adapted(bindings):
                result = replacement({name: Expr._wrap(value) for name, value in bindings.items()})
                return Expr(result)._native
            destination = adapted
        else:
            destination = Pattern(replacement)._native
        self._native = _native._Rule(Pattern(source)._native, destination)

    def apply(self, expression: object, *, match_limit: int | None = 100_000, guard=None) -> Expr:
        accepted = None if guard is None else lambda captures: guard({
            name: Expr._wrap(value) for name, value in captures.items()
        })
        return Expr._wrap(self._native.apply(Expr(expression)._native, match_limit, accepted))


class Substitutions:
    """Replace concrete sources simultaneously, without revisiting destinations."""

    __slots__ = ("_native",)

    def __init__(self, entries) -> None:
        pairs = entries.items() if hasattr(entries, "items") else entries
        self._native = _native._Substitutions([(Expr(source)._native, Expr(destination)._native)
                                              for source, destination in pairs])

    def apply(self, expression: object) -> Expr:
        return Expr._wrap(self._native.apply(Expr(expression)._native))

    def __len__(self) -> int:
        return len(self._native)


class Substitution(Substitutions):
    """One concrete source/destination entry with the same simultaneous semantics."""

    __slots__ = ()

    def __init__(self, source: object, destination: object) -> None:
        super().__init__([(source, destination)])
