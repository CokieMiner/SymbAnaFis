"""Scoped names whose issued symbols and function handles retain their owners."""

from __future__ import annotations

from typing import Callable, Iterable, Mapping, Sequence

from . import _native
from ._expression import Expr, Symbol
from ._function import Function
from ._properties import Property

_TemplateSource = object | Callable[..., object]


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
        """Get or create a scoped symbol with the given name in this context."""
        return Symbol._wrap(self._native.symb(name))  # type: ignore[return-value]

    symbol = symb

    def anon(self) -> Symbol:
        """Create a distinct anonymous scoped symbol in this context."""
        return Symbol._wrap(self._native.anon())  # type: ignore[return-value]

    def symbols(self, *names: str) -> tuple[Symbol, ...]:
        """Get or create multiple scoped symbols in this context."""
        return tuple(self.symb(name) for name in names)

    def get_symbol(self, name: str) -> Symbol | None:
        value = self._native.get_symbol(name)
        return None if value is None else Symbol._wrap(value)  # type: ignore[return-value]

    @property
    def symbol_names(self) -> tuple[str, ...]:
        return tuple(self._native.symbol_names())

    @property
    def symbol_count(self) -> int:
        return self._native.symbol_count()

    def remove_symbol(self, name: str) -> bool:
        return self._native.remove_symbol(name)

    def clear_symbols(self) -> None:
        self._native.clear_symbols()

    def clear_functions(self) -> None:
        self._native.clear_functions()

    def clear_all(self) -> None:
        self._native.clear_all()

    @property
    def is_empty(self) -> bool:
        return self._native.is_empty()

    def reset(self) -> None:
        """Create a fresh namespace; existing expression and function handles survive."""
        self._native.reset()

    def parse(self, source: str) -> Expr:
        return Expr._wrap(self._native.parse(source))

    def register_function(
        self,
        name: str,
        arity: int | tuple[int, int],
        *,
        body: _TemplateSource | None = None,
        partials: Mapping[int, _TemplateSource | None] | Sequence[_TemplateSource | None] | None = None,
        parameters: Iterable[Symbol] | None = None,
        returns: Iterable[Property | str] = (),
    ) -> Function:
        """Register owning symbolic templates, evaluating Python builders once.

        ``arity`` accepts a fixed nonnegative integer or an inclusive ``(min, max)``
        range (variable arities require ``body=None`` and ``partials=None``).
        Builders receive positional Symbol parameters. With a body and omitted
        partials, supported derivatives are generated from that template;
        unsupported derivatives remain symbolic at the call site. Explicit sparse
        partials (including {}) retain the chosen derivative coverage. Templates
        cannot contain unresolved differential operators. Plain Expr templates
        should supply the parameter symbols they refer to. ``returns`` attaches
        conservative return-domain properties to the registered definition.
        """
        if isinstance(arity, tuple):
            if len(arity) != 2:
                raise ValueError("arity range must be a (min_arity, max_arity) tuple")
            min_arity, max_arity = arity
            if (
                isinstance(min_arity, bool)
                or isinstance(max_arity, bool)
                or not isinstance(min_arity, int)
                or not isinstance(max_arity, int)
                or min_arity < 0
                or max_arity < min_arity
            ):
                raise ValueError("arity range bounds must satisfy 0 <= min <= max")
        else:
            if isinstance(arity, bool) or not isinstance(arity, int) or arity < 0:
                raise ValueError("arity must be a nonnegative integer or (min, max) tuple")
            min_arity = max_arity = arity

        if self.get_function(name) is not None or _native._Function.builtin(name) is not None:
            raise ValueError(f"function name already registered or reserved: {name!r}")

        return_props = [Property(prop).value for prop in returns]
        has_templates = body is not None or partials is not None
        if not has_templates:
            if parameters is not None:
                sources = tuple(parameters)
                if min_arity != max_arity or len(sources) != min_arity or any(
                    not isinstance(value, Symbol) for value in sources
                ):
                    raise ValueError("parameters must contain one Symbol per argument")
                if len(set(sources)) != min_arity:
                    raise ValueError("template parameters must have distinct identities")
            native = self._native.register_function(
                name, min_arity, max_arity, [], None, [], return_props
            )
            return Function._wrap(native)

        if min_arity != max_arity:
            raise ValueError("body and partial templates require a fixed arity")
        fixed_arity = min_arity
        if parameters is None:
            owner = Context()
            sources = tuple(owner.symb(f"arg{index}") for index in range(fixed_arity))
        else:
            sources = tuple(parameters)
        if len(sources) != fixed_arity or any(not isinstance(value, Symbol) for value in sources):
            raise ValueError("parameters must contain one Symbol per argument")
        if len(set(sources)) != fixed_arity:
            raise ValueError("template parameters must have distinct identities")
        expression = None if body is None else Expr(body(*sources) if callable(body) else body)
        if partials is None:
            derivatives: dict[int, Expr] = {}
            if expression is not None:
                for index, parameter in enumerate(sources):
                    candidate = expression.derivative(parameter)
                    if not candidate._native.has_derivatives():
                        derivatives[index] = candidate
        else:
            provided = partials.items() if isinstance(partials, Mapping) else enumerate(partials)
            derivatives = {}
            for index, partial in provided:
                if (
                    isinstance(index, bool)
                    or not isinstance(index, int)
                    or not 0 <= index < fixed_arity
                ):
                    raise ValueError("partial index is outside the registered arity")
                if partial is not None:
                    derivatives[index] = Expr(partial(*sources) if callable(partial) else partial)
        native = self._native.register_function(
            name,
            fixed_arity,
            fixed_arity,
            [value._native for value in sources],
            None if expression is None else expression._native,
            [(index, value._native) for index, value in derivatives.items()],
            return_props,
        )
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
