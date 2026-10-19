"""Python GC edges and per-invocation callback state across native threads."""

import concurrent.futures
import gc
import unittest
import weakref

from symb_anafis import Context, Expr, Pattern, RewriteRule, SimplificationSequence, SimplificationStep


class Replacement:
    def __init__(self):
        self.owner = None

    def __call__(self, captures):
        return captures["angle"].cos()


class Mapping:
    def __init__(self):
        self.owner = None

    def __call__(self, expression):
        return None


class Score:
    def __init__(self):
        self.owner = None

    def __call__(self, expression):
        return expression.node_count


class CallbackOwnership(unittest.TestCase):
    def test_callback_rules_and_cloned_sequences_are_collected_without_hidden_cycles(self):
        x = Context().symb("x")
        angle = Pattern.capture("angle")
        callback = Replacement()
        reference = weakref.ref(callback)
        rule = RewriteRule(angle.sin(), callback)
        sequence = SimplificationSequence().then(rule)
        surviving = sequence.then(SimplificationStep.FACTOR)
        del callback, rule, sequence
        gc.collect()
        self.assertIsNotNone(reference())
        self.assertEqual(surviving.simplify(x.sin()).expression, x.cos())
        reference().owner = surviving
        del surviving
        gc.collect()
        self.assertIsNone(reference())

    def test_mapping_and_scoring_cycles_are_visible_to_python_gc(self):
        for callback_type in (Mapping, Score):
            callback = callback_type()
            reference = weakref.ref(callback)
            sequence = SimplificationSequence()
            sequence = sequence.then(callback) if isinstance(callback, Mapping) else sequence.score_with(callback)
            callback.owner = sequence
            del callback, sequence
            gc.collect()
            self.assertIsNone(reference())

    def test_function_builders_do_not_remain_as_runtime_python_callbacks(self):
        class Body:
            def __call__(self, value):
                return value**2

        callback = Body()
        reference = weakref.ref(callback)
        context = Context()
        function = context.register_function("square", 1, body=callback)
        del callback
        gc.collect()
        self.assertIsNone(reference())
        x = context.symb("x")
        self.assertEqual(function(x).evaluate({x: 3}), 9)

    def test_parallel_invocations_keep_exceptions_isolated(self):
        x, y = Context().symbols("x", "y")

        def replacement(node):
            if node == y:
                raise ValueError("only this input fails")
            return Expr(1) if node == x else None

        sequence = SimplificationSequence().then(replacement)
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            futures = [pool.submit(sequence.simplify, x if index % 2 == 0 else y) for index in range(24)]
            for index, future in enumerate(futures):
                if index % 2 == 0:
                    self.assertEqual(future.result().expression, Expr(1))
                else:
                    with self.assertRaisesRegex(ValueError, "only this input fails"):
                        future.result()
