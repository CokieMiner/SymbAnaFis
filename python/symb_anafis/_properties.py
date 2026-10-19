"""Read-only conservative scalar proofs; unknown facts do not authorize identities."""

from __future__ import annotations

from enum import Enum
from types import MappingProxyType


class Property(str, Enum):
    FINITE = "finite"
    REAL = "real"
    INTEGER = "integer"
    RATIONAL = "rational"
    ZERO = "zero"
    NONZERO = "nonzero"
    POSITIVE = "positive"
    NEGATIVE = "negative"
    NONNEGATIVE = "nonnegative"
    NONPOSITIVE = "nonpositive"


class Truth(Enum):
    FALSE = False
    TRUE = True
    UNKNOWN = None

    def __bool__(self) -> bool:
        if self is Truth.UNKNOWN:
            raise TypeError("an unknown scalar property has no truth value")
        return self is Truth.TRUE


class Properties:
    """One immutable snapshot of scalar proofs for an expression DAG."""

    __slots__ = ("_facts",)

    def __init__(self, facts: list[tuple[str, bool | None]]) -> None:
        self._facts = MappingProxyType({Property(name): Truth(value) for name, value in facts})

    def get(self, property: Property | str) -> Truth:
        return self._facts[Property(property)]

    def __getitem__(self, property: Property | str) -> Truth:
        return self.get(property)

    def __iter__(self):
        return iter(self._facts)

    def __repr__(self) -> str:
        return f"Properties({dict(self._facts)!r})"
