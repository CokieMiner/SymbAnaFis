"""Bounded rewrites, ordered sequences, and transactional Python callback errors."""

import unittest

from symb_anafis import (
    Context, DomainSafety, Expr, Pattern, Property, RewriteError, RewriteRule,
    SimplificationSequence, SimplificationStep, SimplificationTarget, SimplifyError,
    Substitution, Substitutions,
)


class ConcreteReplacements(unittest.TestCase):
    def test_simultaneous_swaps_groups_and_conflict_detection(self):
        x, y, z, u = Context().symbols("x", "y", "z", "u")
        self.assertEqual(Substitutions({x: y, y: x}).apply(x**2+y), y**2+x)
        self.assertEqual(Substitution(x+y, u).apply((x+y)**2+z), u**2+z)
        self.assertEqual(len(Substitutions([(x, y), (x, y)])), 1)
        with self.assertRaises(ValueError):
            Substitutions([(x, y), (x, z)])

    def test_mapping_shares_nodes_and_does_not_revisit_destinations(self):
        x = Context().symb("x")
        seen = []

        def replacement(expression):
            seen.append(expression)
            return x.cos() if expression == x.sin() else None

        expression = x.sin()**2+x.sin()**3
        self.assertEqual(expression.map(replacement), x.cos()**2+x.cos()**3)
        self.assertEqual(seen.count(x.sin()), 1)
        self.assertNotIn(x.cos(), seen)
        self.assertEqual(len(expression.fold(set(), lambda values, node: values | {node})), len(seen))


class PatternRules(unittest.TestCase):
    def test_commutative_remainders_conditions_and_work_bounds(self):
        x, y = Context().symbols("x", "y")
        angle = Pattern.capture("angle")
        rule = RewriteRule(angle.sin()**2+angle.cos()**2, 1)
        self.assertEqual(rule.apply(x.sin()**2+x.cos()**2+y), 1+y)
        self.assertEqual(rule.apply(x.sin()**2+x.cos()**2, match_limit=0), Expr(1))
        second = Pattern.capture("second")
        bounded = RewriteRule(angle.sin()+second.cos(), 0)
        with self.assertRaises(RewriteError):
            bounded.apply(x.sin()+y.cos()+x.exp(), match_limit=0)
        real = Pattern.capture("real", properties=[Property.REAL])
        guarded = RewriteRule((real**2).sqrt(), real.abs())
        self.assertEqual(guarded.apply((x**2).sqrt()), (x**2).sqrt())
        self.assertEqual(guarded.apply((Expr.pi()**2).sqrt()), Expr.pi().abs())

    def test_replacement_callbacks_receive_accepted_captures_once(self):
        x, y = Context().symbols("x", "y")
        capture = Pattern.capture("angle")
        calls = []

        def replacement(captures):
            calls.append(captures["angle"])
            return captures["angle"].cos()

        rule = RewriteRule(capture.sin(), replacement)
        self.assertEqual(rule.apply(x.sin()**2+x.sin()**3), x.cos()**2+x.cos()**3)
        self.assertEqual(calls, [x])
        calls.clear()
        self.assertEqual(rule.apply(x.sin()+y.sin(), guard=lambda values: values["angle"] == y), x.sin()+y.cos())
        self.assertEqual(calls, [y])

    def test_callback_and_guard_errors_propagate_without_output_or_stale_error(self):
        x = Context().symb("x")
        angle = Pattern.capture("angle")
        marker = ValueError("replacement failure")
        should_fail = [True]

        def replacement(captures):
            if should_fail[0]:
                raise marker
            return captures["angle"].cos()

        rule = RewriteRule(angle.sin(), replacement)
        original = x.sin()
        with self.assertRaises(ValueError) as failure:
            rule.apply(original)
        self.assertIs(failure.exception, marker)
        self.assertEqual(original, x.sin())
        should_fail[0] = False
        self.assertEqual(rule.apply(original), x.cos())

        def guard(_):
            raise marker

        with self.assertRaises(ValueError) as failure:
            rule.apply(original, guard=guard)
        self.assertIs(failure.exception, marker)


class SimplificationPrograms(unittest.TestCase):
    def test_presets_custom_steps_and_clone_independence(self):
        x, y = Context().symbols("x", "y")
        self.assertEqual(SimplificationSequence.compact().simplify(x.sin()**2+x.cos()**2).expression, Expr(1))
        self.assertEqual(SimplificationSequence.expanded().simplify((x+1)**2).expression, x**2+2*x+1)
        self.assertEqual(SimplificationSequence.factored().simplify(x**2-y**2).expression, (x-y)*(x+y))
        self.assertEqual(SimplificationSequence.compilation().simplify(x.exp()-1).expression, x.exp_m1())
        empty = SimplificationSequence()
        custom = empty.then(SimplificationStep.TRIGONOMETRIC)
        self.assertEqual(empty.steps, ())
        self.assertEqual(custom.steps, (SimplificationStep.TRIGONOMETRIC,))
        self.assertEqual(custom.simplify(x.sin()**2+x.cos()**2).expression, Expr(1))
        preset = SimplificationSequence.expanded()
        self.assertEqual(SimplificationSequence(preset.steps, target=SimplificationTarget.EXPANDED)
                         .simplify((x+1)**2).expression, x**2+2*x+1)

    def test_native_limits_and_explicit_assumption_mode(self):
        x = Context().symb("x")
        expression = x/x
        self.assertEqual(SimplificationSequence.compact().simplify(expression).expression, expression)
        self.assertEqual(SimplificationSequence.compact().domain_safety(DomainSafety.ASSUMPTION_DEPENDENT)
                         .simplify(expression).expression, Expr(1))
        with self.assertRaises(SimplifyError):
            SimplificationSequence.compact(max_nodes=0).simplify(x)
        self.assertEqual(SimplificationSequence.compact().max_iterations(0).simplify(x.sin()).expression, x.sin())

    def test_maps_callback_rules_and_custom_scores_share_the_native_runner(self):
        x = Context().symb("x")
        angle = Pattern.capture("angle")
        rule = RewriteRule(angle.sin(), lambda values: values["angle"].cos())
        custom = SimplificationSequence().then(rule).then(lambda node: 1 if node == x else None)
        result = custom.score_with(lambda node: node.node_count).simplify(x.sin())
        self.assertEqual(result.expression, Expr(1).cos())
        self.assertEqual(custom.max_nodes(100).simplify(x.sin()).expression, Expr(1).cos())
        self.assertEqual(custom.target(SimplificationTarget.COMPACT).simplify(x.sin()).expression, Expr(1).cos())

    def test_sequence_callback_failures_are_not_silenced_or_retained(self):
        x = Context().symb("x")
        marker = RuntimeError("mapping failed")
        should_fail = [True]

        def mapping(node):
            if should_fail[0]:
                raise marker
            return Expr(1) if node == x else None

        sequence = SimplificationSequence().then(mapping)
        with self.assertRaises(RuntimeError) as failure:
            sequence.simplify(x)
        self.assertIs(failure.exception, marker)
        should_fail[0] = False
        self.assertEqual(sequence.simplify(x).expression, Expr(1))
        with self.assertRaises(OverflowError):
            SimplificationSequence.compact().score_with(lambda _: -1).simplify(x)

    def test_mapping_and_folding_exception_identity(self):
        x = Context().symb("x")
        marker = KeyError("visit failed")

        def fail(*_):
            raise marker

        for operation in (lambda: x.sin().map(fail), lambda: x.sin().fold(None, fail)):
            with self.assertRaises(KeyError) as failure:
                operation()
            self.assertIs(failure.exception, marker)
