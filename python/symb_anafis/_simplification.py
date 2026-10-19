"""Reusable preset and caller-defined sequences backed by native compiled plans."""

from __future__ import annotations

import dataclasses
from enum import Enum
from types import MappingProxyType

from . import _native
from ._expression import Expr
from ._rewriting import RewriteRule


class SimplificationStep(str, Enum):
    ALGEBRAIC = "algebraic"
    TRIGONOMETRIC = "trigonometric"
    HYPERBOLIC = "hyperbolic"
    EXPONENTIAL = "exponential"
    ROOTS = "roots"
    EXPAND = "expand"
    FACTOR = "factor"
    COMPILATION = "compilation"


class SimplificationTarget(str, Enum):
    COMPACT = "compact"
    EXPANDED = "expanded"
    FACTORED = "factored"
    COMPILATION = "compilation"


class DomainSafety(str, Enum):
    PRESERVING = "preserving"
    ASSUMPTION_DEPENDENT = "assumption_dependent"


class SimplificationTermination(str, Enum):
    FIXED_POINT = "fixed_point"
    CYCLE = "cycle"
    ITERATION_LIMIT = "iteration_limit"


@dataclasses.dataclass(frozen=True)
class Simplification:
    """The best visited candidate and the sequence's termination metadata."""

    expression: Expr
    termination: SimplificationTermination
    iterations: int


class SimplificationSequence:
    """An immutable ordered program; builders return independent sequence objects.

    Builtins preserve proved domains by default. Assumption-dependent identities
    and all caller rules/maps require the caller's mathematical hypotheses.
    Mapping callbacks return Expr-compatible values or None; scores are pure
    nonnegative native-width integers. Callback exceptions propagate unchanged.
    """

    __slots__ = ("_native", "_steps", "_options", "_score")
    _default_compact = None

    def __init__(self, steps=(), *, target=SimplificationTarget.COMPACT,
                 domain_safety=DomainSafety.PRESERVING, max_iterations: int = 16,
                 max_nodes: int | None = None, max_depth: int | None = None,
                 expansion_limit: int | None = None, polynomial_limit: int | None = None,
                 match_limit: int | None = None, score=None, _preset: str = "empty") -> None:
        unlimited = (1 << 64) - 1
        options = dict(target=SimplificationTarget(target), domain_safety=DomainSafety(domain_safety),
                       max_iterations=max_iterations, max_nodes=max_nodes, max_depth=max_depth,
                       expansion_limit=expansion_limit, polynomial_limit=polynomial_limit,
                       match_limit=match_limit)
        self._options = MappingProxyType(options)
        native_limits = (
            max_iterations,
            unlimited if max_nodes is None else max_nodes,
            unlimited if max_depth is None else max_depth,
            unlimited if expansion_limit is None else expansion_limit,
            unlimited if polynomial_limit is None else polynomial_limit,
        )
        self._native = _native._Sequence(_preset, options["target"].value, options["domain_safety"].value,
            native_limits, match_limit)
        self._steps = tuple(SimplificationStep(name) for name in self._native.steps())
        self._score = score
        for step in steps:
            self._append(step)
        if score is not None:
            self._native.score_with(lambda native: score(Expr._wrap(native)))

    @classmethod
    def compact(cls, **options) -> SimplificationSequence:
        options.setdefault("target", SimplificationTarget.COMPACT)
        return cls(_preset="compact", **options)

    @classmethod
    def expanded(cls, **options) -> SimplificationSequence:
        options.setdefault("target", SimplificationTarget.EXPANDED)
        return cls(_preset="expanded", **options)

    @classmethod
    def factored(cls, **options) -> SimplificationSequence:
        options.setdefault("target", SimplificationTarget.FACTORED)
        return cls(_preset="factored", **options)

    @classmethod
    def compilation(cls, **options) -> SimplificationSequence:
        options.setdefault("target", SimplificationTarget.COMPILATION)
        return cls(_preset="compilation", **options)

    @classmethod
    def _default(cls) -> SimplificationSequence:
        if cls._default_compact is None:
            cls._default_compact = cls.compact()
        return cls._default_compact

    @property
    def steps(self) -> tuple:
        return self._steps

    def _append(self, step) -> None:
        if isinstance(step, RewriteRule):
            self._native.then_rule(step._native)
        elif callable(step):
            def adapted(native):
                result = step(Expr._wrap(native))
                return None if result is None else Expr(result)._native
            self._native.then_map(adapted)
        else:
            step = SimplificationStep(step)
            self._native.then_builtin(step.value)
        self._steps += (step,)

    def _copy(self) -> SimplificationSequence:
        value = object.__new__(type(self))
        value._native = self._native.copy()
        value._steps, value._options, value._score = self._steps, self._options, self._score
        return value

    def then(self, step) -> SimplificationSequence:
        value = self._copy()
        value._append(step)
        return value

    def _replace(self, **changes) -> SimplificationSequence:
        options = dict(self._options)
        options.update(changes)
        return type(self)(self._steps, score=self._score, **options)

    def target(self, target) -> SimplificationSequence:
        options = dict(self._options)
        options["target"] = SimplificationTarget(target)
        return type(self)(self._steps, **options)

    def domain_safety(self, safety) -> SimplificationSequence:
        return self._replace(domain_safety=DomainSafety(safety))

    def max_iterations(self, limit: int) -> SimplificationSequence:
        return self._replace(max_iterations=limit)

    def max_nodes(self, limit: int) -> SimplificationSequence:
        return self._replace(max_nodes=limit)

    def max_depth(self, limit: int) -> SimplificationSequence:
        return self._replace(max_depth=limit)

    def expansion_limit(self, limit: int) -> SimplificationSequence:
        return self._replace(expansion_limit=limit)

    def polynomial_limit(self, limit: int) -> SimplificationSequence:
        return self._replace(polynomial_limit=limit)

    def match_limit(self, limit: int | None) -> SimplificationSequence:
        return self._replace(match_limit=limit)

    def score_with(self, callback) -> SimplificationSequence:
        if not callable(callback):
            raise TypeError("score must be callable")
        value = self._copy()
        value._score = callback
        value._native.score_with(lambda native: callback(Expr._wrap(native)))
        return value

    def simplify(self, expression: object) -> Simplification:
        selected, reason, iterations = self._native.simplify(Expr(expression)._native)
        return Simplification(Expr._wrap(selected), SimplificationTermination(reason), iterations)
