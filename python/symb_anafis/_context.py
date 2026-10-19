"""Scoped names whose issued symbols and function handles retain their owners."""

from __future__ import annotations

from . import _native
from ._expression import Expr, Symbol
from ._function import Function


class Context:
    """An isolated namespace for parsing, symbols, and custom functions."""

    __slots__ = ("_native",)

    def __init__(self) -> None:
        self._native = _native._Context()

    @classmethod
    def default(cls) -> Context:
        """The Python module's shared default namespace, independent of Rust globals."""
        return _DEFAULT_CONTEXT

    @property
    def id(self) -> int:
        return self._native.id()

    def symb(self, name: str) -> Symbol:
        return Symbol._wrap(self._native.symb(name))

    def symbols(self, *names: str) -> tuple[Symbol, ...]:
        return tuple(self.symb(name) for name in names)

    def get_symbol(self, name: str) -> Symbol | None:
        value = self._native.get_symbol(name)
        return None if value is None else Symbol._wrap(value)

    @property
    def symbol_names(self) -> tuple[str, ...]:
        return tuple(self._native.symbol_names())

    def remove_symbol(self, name: str) -> bool:
        return self._native.remove_symbol(name)

    def clear_symbols(self) -> None:
        self._native.clear_symbols()

    def reset(self) -> None:
        """Create a fresh namespace; existing expression and function handles survive."""
        self._native.reset()

    def parse(self, source: str) -> Expr:
        return Expr._wrap(self._native.parse(source))

    def register_function(self, name: str, arity: int, *, body=None, partials=None,
                          parameters=None) -> Function:
        """Register owning symbolic templates, evaluating Python builders once.

        Builders receive positional Symbol parameters. With a body and omitted
        partials, supported derivatives are generated from that template;
        unsupported derivatives remain symbolic at the call site. Explicit sparse
        partials (including {}) retain the chosen derivative coverage. Templates
        cannot contain unresolved differential operators. Plain Expr templates
        should supply the parameter symbols they refer to.
        """
        if isinstance(arity, bool) or not isinstance(arity, int) or arity < 0:
            raise ValueError("arity must be a nonnegative integer")
        if self.get_function(name) is not None or _native._Function.builtin(name) is not None:
            raise ValueError(f"function name already registered or reserved: {name!r}")
        if parameters is None:
            owner = Context()
            sources = tuple(owner.symb(f"arg{index}") for index in range(arity))
        else:
            sources = tuple(parameters)
        if len(sources) != arity or any(not isinstance(value, Symbol) for value in sources):
            raise ValueError("parameters must contain one Symbol per argument")
        if len(set(sources)) != arity:
            raise ValueError("template parameters must have distinct identities")
        expression = None if body is None else Expr(body(*sources) if callable(body) else body)
        if partials is None:
            derivatives = {}
            if expression is not None:
                for index, parameter in enumerate(sources):
                    candidate = expression.derivative(parameter)
                    if not candidate._native.has_derivatives():
                        derivatives[index] = candidate
        else:
            provided = partials.items() if hasattr(partials, "items") else enumerate(partials)
            derivatives = {}
            for index, partial in provided:
                if isinstance(index, bool) or not isinstance(index, int) or not 0 <= index < arity:
                    raise ValueError("partial index is outside the registered arity")
                if partial is not None:
                    derivatives[index] = Expr(partial(*sources) if callable(partial) else partial)
        native = self._native.register_function(name, arity, [value._native for value in sources],
            None if expression is None else expression._native,
            [(index, value._native) for index, value in derivatives.items()])
        return Function._wrap(native)

    def get_function(self, name: str) -> Function | None:
        value = self._native.get_function(name)
        return None if value is None else Function._wrap(value)

    @property
    def functions(self) -> tuple[Function, ...]:
        return tuple(Function._wrap(value) for value in self._native.functions())

    def remove_function(self, name: str) -> bool:
        return self._native.remove_function(name)

    def __repr__(self) -> str:
        return f"Context(id={self.id}, symbols={self.symbol_names!r})"


_DEFAULT_CONTEXT = Context()
