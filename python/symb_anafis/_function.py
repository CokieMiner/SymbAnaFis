"""Known builtin or immutable custom function identities."""

from __future__ import annotations

from . import _native
from ._expression import Expr


class Function:
    """An owning function identity; unknown names never create symbolic calls."""

    __slots__ = ("_native",)
    _builtins: dict[str, Function] = {}

    def __new__(cls):
        raise TypeError("use Function.builtin() or Context.register_function()")

    @classmethod
    def _wrap(cls, native: _native._Function) -> Function:
        value = object.__new__(cls)
        value._native = native
        return value

    @classmethod
    def builtin(cls, name: str) -> Function:
        if name not in cls._builtins:
            native = _native._Function.builtin(name)
            if native is None:
                raise ValueError(f"unknown builtin function: {name!r}")
            cls._builtins[name] = cls._wrap(native)
        return cls._builtins[name]

    @property
    def name(self) -> str:
        return self._native.name()

    @property
    def arity(self) -> tuple[int, int]:
        return self._native.arity()

    def __call__(self, *arguments: object):
        from ._rewriting import Pattern
        if any(isinstance(value, Pattern) for value in arguments):
            return Pattern._wrap(_native._Pattern.call(self._native, [Pattern(value)._native for value in arguments]))
        return Expr._wrap(self._native.call([Expr(value)._native for value in arguments]))

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Function) and self._native.equal(other._native)

    def __repr__(self) -> str:
        return f"Function(name={self.name!r}, arity={self.arity!r})"
