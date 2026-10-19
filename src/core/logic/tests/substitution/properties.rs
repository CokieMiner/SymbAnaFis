//! Generated simultaneous, exact, and polynomial substitution contracts.

use core::ops::Div;

use alloc::{format, vec, vec::Vec};

use proptest::{
    arbitrary::any, prop_assert, prop_assert_eq, prop_assert_ne, prop_assume, proptest,
};

use crate::{
    core::{
        Context, Expr, ExprView, Pattern, RewriteRule, Substitution, Substitutions, UserFunction,
    },
    errors::SubstitutionError,
};

proptest! {
    #[test]
    fn exact_function_substitutions_update_all_shared_edges(
        function_name in "f_[a-z]{1,12}",
        edge_count in 2_usize..=16,
        destination in -100_i64..=100,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let function = context.register_function(&function_name, UserFunction::new(2..=2)).expect("the custom name is unbound");
        let pair = context.register_function("pair", UserFunction::default()).expect("the custom name is unbound");
        let target = function.call(vec![x.clone(), y.clone()]).expect("the function accepts two arguments");
        let different = function.call(vec![y, x]).expect("the function accepts two arguments");
        let replacement = Expr::number(destination);
        let substitution = Substitution::new(target.clone(), replacement.clone());
        prop_assert_eq!(substitution.from(), &target);
        prop_assert_eq!(substitution.to(), &replacement);
        let mut inputs = vec![target; edge_count];
        inputs.push(different.clone());
        let mut expected = vec![replacement; edge_count];
        expected.push(different);
        prop_assert_eq!(
            substitution.apply_all(&pair.call(inputs).expect("pair is variadic")),
            pair.call(expected).expect("pair is variadic"),
        );
    }

    #[test]
    fn swaps_are_simultaneous_for_any_number_of_shared_edges(repetitions in 1_usize..=16) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let substitutions = Substitutions::new([
            (x.clone(), y.clone()), (y.clone(), x.clone()),
        ]).expect("distinct sources cannot conflict");
        let pair = context.register_function("pair", UserFunction::default()).expect("the custom name is unbound");
        let input = pair.call(vec![pair.call(vec![x.clone(), y.clone()]).expect("pair is variadic"); repetitions]).expect("pair is variadic");
        let expected = pair.call(vec![pair.call(vec![y, x]).expect("pair is variadic"); repetitions]).expect("pair is variadic");
        prop_assert_eq!(substitutions.apply_all(&input), expected);
        prop_assert_eq!(substitutions.len(), 2);
    }

    #[test]
    fn cyclic_permutations_follow_original_sources(
        count in 2_usize..=8,
        shift in 1_usize..=7,
        reverse_entries in any::<bool>(),
    ) {
        prop_assume!(shift < count);
        let context = Context::new();
        let originals: Vec<_> = (0..count)
            .map(|index| context.symb(&format!("x{index}")).expect("symbol allocation").to_expr())
            .collect();
        let mut destinations = originals.clone();
        destinations.rotate_left(shift);
        let mut entries: Vec<_> = originals
            .iter()
            .cloned()
            .zip(destinations.iter().cloned())
            .collect();
        if reverse_entries {
            entries.reverse();
        }
        let substitutions = Substitutions::new(entries).expect("the cycle has unique sources");
        let tuple = context.register_function("tuple", UserFunction::default()).expect("the custom name is unbound");
        prop_assert_eq!(
            substitutions.apply_all(&tuple.call(originals).expect("tuple is variadic")),
            tuple.call(destinations).expect("tuple is variadic"),
        );
    }

    #[test]
    fn concrete_arithmetic_sources_accept_expression_destinations(
        source_kind in 0_u8..=3,
        exponent in 2_i64..=9,
        constant in -100_i64..=100,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let source = match source_kind {
            0 => Expr::sum(vec![x.clone(), y.clone()]),
            1 => Expr::product(vec![x.clone(), y.clone()]),
            2 => Div::div(&x, &y),
            _ => x.pow(exponent),
        };
        let destination = Expr::sum(vec![y.clone(), Expr::number(constant)]);
        let tuple = context.register_function("tuple", UserFunction::new(3..=3)).expect("the custom name is unbound");
        prop_assert_eq!(
            Substitution::new(source.clone(), destination.clone())
                .apply_all(&tuple.call(vec![source, x.clone(), y.clone()]).expect("tuple accepts three arguments")),
            tuple.call(vec![destination, x, y]).expect("tuple accepts three arguments"),
        );
    }

    #[test]
    fn original_parent_matches_take_precedence_over_children(
        parent_destination in -100_i64..=100,
        child_destination in -100_i64..=100,
        reverse_entries in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let parent = f.call(vec![x.clone()]).expect("f is unary");
        let parent_result = Expr::number(parent_destination);
        let child_result = Expr::number(child_destination);
        let mut entries = vec![(parent.clone(), parent_result.clone()), (x.clone(), child_result.clone())];
        if reverse_entries {
            entries.reverse();
        }
        let substitutions = Substitutions::new(entries).expect("distinct sources cannot conflict");
        prop_assert_eq!(
            substitutions.apply_all(&pair.call(vec![parent, x]).expect("pair accepts two arguments")),
            pair.call(vec![parent_result, child_result]).expect("pair accepts two arguments"),
        );
    }

    #[test]
    fn rebuilt_parents_are_not_matched_as_original_nodes(value in -100_i64..=100) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let original_parent = f.call(vec![x.clone()]).expect("f is unary");
        let rebuilt_parent = f.call(vec![y.clone()]).expect("f is unary");
        let result = Expr::number(value);
        let substitutions = Substitutions::new([
            (x, y), (rebuilt_parent.clone(), result.clone()),
        ]).expect("distinct sources cannot conflict");
        prop_assert_eq!(
            substitutions.apply_all(&pair.call(vec![original_parent, rebuilt_parent.clone()]).expect("pair accepts two arguments")),
            pair.call(vec![rebuilt_parent, result]).expect("pair accepts two arguments"),
        );
    }

    #[test]
    fn destinations_containing_sources_are_inserted_once(depth in 1_usize..=16) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let mut destination = x.clone();
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        for _ in 0..depth {
            destination = f.call(vec![destination]).expect("f is unary");
        }
        let substitution = Substitution::new(x.clone(), destination.clone());
        prop_assert_eq!(&substitution.apply_all(&x), &destination);
        prop_assert_eq!(
            substitution.apply_all(&pair.call(vec![x.clone(), x]).expect("pair accepts two arguments")),
            pair.call(vec![destination.clone(), destination]).expect("pair accepts two arguments"),
        );
    }

    #[test]
    fn repeated_sources_accept_equal_destinations_and_reject_conflicts(
        first_value in -100_i64..=100,
        second_value in -100_i64..=100,
        copies in 1_usize..=16,
    ) {
        let context = Context::new();
        let source = context.symb("source")?.to_expr().sin();
        let first = Expr::number(first_value);
        let second = Expr::number(second_value);
        let repeated = Substitutions::new(vec![(source.clone(), first.clone()); copies])
            .expect("identical destinations are idempotent");
        prop_assert_eq!(repeated.len(), 1);
        prop_assert_eq!(repeated.get(&source), Some(&first));
        let combined = Substitutions::new([
            (source.clone(), first.clone()), (source.clone(), second.clone()),
        ]);
        if first_value == second_value {
            prop_assert_eq!(combined.expect("equal destinations agree").len(), 1);
        } else {
            prop_assert_eq!(combined.expect_err("different destinations conflict"), SubstitutionError::Conflict {
                from: source, first, second,
            });
        }
    }

    #[test]
    fn numeric_equality_does_not_erase_destination_representation(value in -100_i32..=100) {
        let context = Context::new();
        let source = context.symb("source")?.to_expr();
        let integer = Expr::number(i64::from(value));
        let approximate = Expr::number(f64::from(value));
        prop_assert_ne!(&integer, &approximate);
        let entries = Substitutions::new([
            (source.clone(), integer.clone()), (source.clone(), approximate.clone()),
        ]);
        prop_assert_eq!(entries.expect_err("distinct literal representations conflict"), SubstitutionError::Conflict {
            from: source, first: integer, second: approximate,
        });
    }

    #[test]
    fn identity_parent_entries_preserve_the_original_subtree(value in -100_i64..=100) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let parent = f.call(vec![x.clone()]).expect("f is unary");
        let substitutions = Substitutions::new([
            (parent.clone(), parent.clone()), (x, Expr::number(value)),
        ]).expect("distinct sources cannot conflict");
        prop_assert_eq!(substitutions.apply_all(&parent), parent);
    }

    #[test]
    fn logical_monomials_match_inside_polynomials_and_explicit_edges(
        first_power in 2_i64..=8,
        second_power in 2_i64..=8,
        replacement_value in -100_i64..=100,
    ) {
        prop_assume!(first_power != second_power);
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let target = x.pow(first_power);
        let polynomial = Expr::sum(vec![target.clone(), x.pow(second_power), y.pow(first_power), y.pow(second_power)]);
        prop_assert!(matches!(polynomial.view(), ExprView::Sum(_)));
        let replacement = Expr::number(replacement_value);
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let input = pair.call(vec![polynomial, target.clone()]).expect("pair accepts two arguments");
        let substitution = Substitution::new(target, replacement.clone());
        let other = if second_power < first_power {
            x.pow(second_power)
        } else {
            let copies = second_power.checked_div(first_power).expect("generated powers are positive");
            let remainder = second_power.checked_rem(first_power).expect("generated powers are positive");
            let repeated = replacement.pow(copies);
            if remainder == 0 { repeated } else { Expr::product(vec![repeated, x.pow(remainder)]) }
        };
        let expected = Expr::sum(vec![replacement.clone(), other, y.pow(first_power), y.pow(second_power)]);
        prop_assert_eq!(substitution.apply_all(&input), pair.call(vec![expected, replacement]).expect("pair accepts two arguments"));
    }

    #[test]
    fn projected_exponents_are_original_literals_not_rebuilt_match_targets(
        original_power in 2_i64..=5,
        other_power in 6_i64..=9,
        replacement_power in 10_i64..=13,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let original = Expr::sum(vec![x.pow(original_power), x.pow(other_power)]);
        let substitutions = Substitutions::new([
            (Expr::number(original_power), Expr::number(replacement_power)),
            (x.pow(replacement_power), Expr::number(99)),
        ]).expect("distinct sources cannot conflict");
        prop_assert_eq!(
            substitutions.apply_all(&original),
            Expr::sum(vec![x.pow(replacement_power), x.pow(other_power)]),
        );
    }

    #[test]
    fn whole_polynomial_matches_override_monomial_substitutions(
        first_power in 2_i64..=5,
        second_power in 6_i64..=9,
        value in -100_i64..=100,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let term = x.pow(first_power);
        let polynomial = Expr::sum(vec![term.clone(), x.pow(second_power)]);
        let destination = Expr::number(value);
        let substitutions = Substitutions::new([
            (polynomial.clone(), destination.clone()), (term, Expr::number(99)),
        ]).expect("distinct sources cannot conflict");
        prop_assert_eq!(substitutions.apply_all(&polynomial), destination);
    }

    #[test]
    fn empty_and_absent_substitutions_preserve_canonical_roots(power in 2_i64..=8) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let absent = context.symb("absent")?.to_expr();
        let next_power = power.checked_add(1).expect("the generated exponent is bounded");
        let polynomial = Expr::sum(vec![x.pow(power), x.pow(next_power)]);
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let input = pair.call(vec![x, polynomial]).expect("pair accepts two arguments");
        let empty = Substitutions::new([]).expect("an empty set has no conflicts");
        prop_assert!(empty.is_empty());
        prop_assert_eq!(&empty.apply_all(&input), &input);
        prop_assert_eq!(&Substitutions::default().apply_all(&input), &input);
        let absent_entries = Substitutions::new([(absent.clone(), Expr::number(1))])
            .expect("a single entry has no conflicts");
        prop_assert_eq!(&absent_entries.apply_all(&input), &input);
        prop_assert_eq!(Substitution::new(absent, Expr::number(1)).apply_all(&input), input);
    }

    #[test]
    fn same_spelling_symbols_and_functions_keep_context_identity(value in -100_i64..=100) {
        let first = Context::new();
        let second = Context::new();
        let x = first.symb("x")?.to_expr();
        let other = second.symb("x")?.to_expr();
        let first_function = first.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let second_function = second.register_function("f", UserFunction::new(1..=1)).expect("the other namespace is independent");
        let tuple = first.register_function("tuple", UserFunction::new(3..=3)).expect("the custom name is unbound");
        let target = first_function.call(vec![x.clone()]).expect("f is unary");
        let different_symbol = first_function.call(vec![other]).expect("f is unary");
        let different_function = second_function.call(vec![x]).expect("f is unary");
        let destination = Expr::number(value);
        let input = tuple.call(vec![target.clone(), different_symbol.clone(), different_function.clone()]).expect("tuple accepts three arguments");
        prop_assert_ne!(&target, &different_symbol);
        prop_assert_ne!(&target, &different_function);
        prop_assert_eq!(
            Substitution::new(target, destination.clone()).apply_all(&input),
            tuple.call(vec![destination, different_symbol, different_function]).expect("tuple accepts three arguments"),
        );
    }

    #[test]
    fn rules_allow_unmatched_literal_symbols_in_the_replacement(value in -100_i64..=100) {
        let context = Context::new();
        let literal = context.symb("literal")?.to_expr();
        let capture = Pattern::capture("argument", None);
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let tuple = context.register_function("tuple", UserFunction::new(3..=3)).expect("the custom name is unbound");
        let replacement = Pattern::apply(tuple.clone(), vec![capture.clone(), Pattern::exact(literal.clone()), capture.clone()]).expect("tuple accepts three patterns");
        let rule = RewriteRule::new(Pattern::apply(f.clone(), vec![capture]).expect("f is unary"), replacement)
            .expect("literal expressions require no captures");
        let argument = Expr::number(value);
        let input = f.call(vec![argument.clone()]).expect("f is unary");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(tuple.call(vec![argument.clone(), literal, argument]).expect("tuple accepts three arguments")));
    }
}
