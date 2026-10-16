//! Bounded generated checks for expression canonicalization and domain rules.

use alloc::{format, sync::Arc, vec, vec::Vec};

use proptest::{
    collection, prop_assert, prop_assert_eq, prop_assume, prop_oneof, proptest, sample,
    strategy::Strategy,
};

use crate::{
    collections::{HashMap, HashSet},
    core::{Context, Pattern, RewriteRule, UserFunction},
};

use super::super::{Expr, ExprView, FunctionName, NODES, Node, Symbol};

#[derive(Clone, Debug)]
enum Atom {
    Number(i8),
    Symbol(usize),
}

#[derive(Clone, Debug)]
struct Monomial {
    base: usize,
    coefficient: i8,
    exponent: u32,
}

fn atom_strategy() -> impl Strategy<Value = Atom> {
    prop_oneof![
        (-3_i8..=3).prop_map(Atom::Number),
        (0_usize..4).prop_map(Atom::Symbol),
    ]
}

fn monomial_strategy() -> impl Strategy<Value = Monomial> {
    (0_usize..3, prop_oneof![-3_i8..=-1, 1_i8..=3], 1_u32..=4).prop_map(
        |(base, coefficient, exponent)| Monomial {
            base,
            coefficient,
            exponent,
        },
    )
}

fn symbol(index: usize) -> Expr {
    Expr::symbol(format!("generated_expression_x_{index}"))
}

fn atom_expression(atom: &Atom) -> Expr {
    match atom {
        Atom::Number(value) => Expr::number(i64::from(*value)),
        Atom::Symbol(index) => symbol(*index),
    }
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "generated terms intentionally exercise symbolic multiplication"
)]
fn monomial_expression(monomial: &Monomial) -> Expr {
    Expr::number(i64::from(monomial.coefficient))
        * symbol(monomial.base).pow(i64::from(monomial.exponent))
}

#[test]
#[expect(
    clippy::erasing_op,
    reason = "zero times an unknown function is a domain-preserving symbolic edge"
)]
fn deterministic_domain_edges_remain_explicit() {
    let x = Expr::symbol("generated_domain_edge");
    assert!(matches!((&x / &x).view(), ExprView::Div(..)));
    assert!(matches!((0 * x.sin()).view(), ExprView::Product(_)));
    assert!(matches!(x.pow(0).view(), ExprView::Pow(..)));
    assert!(matches!(Expr::number(0).pow(0).view(), ExprView::Pow(..)));
}

proptest! {
    #[test]
    fn sums_and_products_are_permutation_and_flattening_invariant(
        atoms in collection::vec(atom_strategy(), 0..=8),
    ) {
        let operands: Vec<_> = atoms.iter().map(atom_expression).collect();
        let mut reversed = operands.clone();
        reversed.reverse();
        let sum = Expr::sum(operands.clone());
        let product = Expr::product(operands.clone());

        prop_assert!(sum.same_node(&Expr::sum(reversed.clone())));
        prop_assert!(product.same_node(&Expr::product(reversed)));
        prop_assert!(sum.same_node(&Expr::sum(vec![sum.clone()])));
        prop_assert!(product.same_node(&Expr::product(vec![product.clone()])));
        prop_assert!(sum.same_node(&Expr::sum(vec![Expr::sum(operands.clone())])));
        prop_assert!(product.same_node(&Expr::product(vec![Expr::product(operands)])));
    }

    #[test]
    fn singleton_and_empty_constructors_are_canonical(
        atom in atom_strategy(),
    ) {
        let expression = atom_expression(&atom);
        prop_assert!(Expr::sum(vec![expression.clone()]).same_node(&expression));
        prop_assert!(Expr::product(vec![expression.clone()]).same_node(&expression));
        prop_assert_eq!(Expr::sum(Vec::new()), Expr::number(0));
        prop_assert_eq!(Expr::product(Vec::new()), Expr::number(1));
    }

    #[test]
    fn distinct_symbol_sums_share_one_ordered_node(
        indices in collection::vec(0_usize..32, 1..=8),
    ) {
        let unique: HashSet<_> = indices.iter().copied().collect();
        prop_assume!(unique.len() == indices.len());
        prop_assume!(indices.len() >= 2);
        let operands: Vec<_> = indices
            .iter()
            .map(|index| Expr::symbol(format!("generated_distinct_symbol_{index}")))
            .collect();
        let expected = Expr::sum(operands.clone());
        let mut reversed = operands.clone();
        reversed.reverse();
        prop_assert!(Expr::sum(reversed).same_node(&expected));
        let all_symbol_terms = matches!(expected.view(), ExprView::Sum(terms)
            if terms.len() == operands.len()
                && terms.clone().all(|term| matches!(term.view(), ExprView::Symbol(_))));
        prop_assert!(all_symbol_terms);
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the independent integer oracle intentionally uses bounded arithmetic"
    )]
    fn bounded_monomials_are_idempotent_and_preserve_an_independent_oracle(
        monomials in collection::vec(monomial_strategy(), 0..=8),
        values in collection::vec(-2_i64..=2, 3),
    ) {
        let terms: Vec<_> = monomials.iter().map(monomial_expression).collect();
        let expression = Expr::sum(terms);
        prop_assert!(expression.same_node(&Expr::sum(vec![expression.clone()])));

        let replacements: HashMap<_, _> = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                (
                    Symbol::intern(&format!("generated_expression_x_{index}")),
                    Expr::number(*value),
                )
            })
            .collect();
        let expected = monomials.iter().fold(0_i64, |sum, monomial| {
            let value = *values
                .get(monomial.base)
                .expect("monomial bases are bounded by the value vector");
            sum + i64::from(monomial.coefficient) * value.pow(monomial.exponent)
        });
        prop_assert_eq!(expression.substitute_symbols(&replacements), Expr::number(expected));
    }

    #[test]
    fn positive_integer_factor_powers_collect_without_changing_the_base(
        base_index in 0_usize..3,
        exponents in collection::vec(1_u32..=5, 1..=6),
    ) {
        let base = symbol(base_index);
        let factors: Vec<_> = exponents
            .iter()
            .map(|exponent| base.pow(i64::from(*exponent)))
            .collect();
        let total = exponents.iter().sum::<u32>();
        prop_assert_eq!(Expr::product(factors), base.pow(i64::from(total)));
    }

    #[test]
    fn repeated_positive_powers_select_sparse_poly_and_keep_constants_outside(
        base_index in 0_usize..3,
        first in 1_u32..=3,
        second in 1_u32..=5,
        constant in 1_i64..=4,
    ) {
        prop_assume!(first != second);
        let base = symbol(base_index);
        let polynomial = Expr::sum(vec![
            base.pow(i64::from(first)),
            base.pow(i64::from(second)),
        ]);
        let Node::Poly(storage) = polynomial.node() else {
            return Err(proptest::test_runner::TestCaseError::fail("two distinct positive powers select compact storage"));
        };
        prop_assert_eq!(&storage.bases, core::slice::from_ref(&base));
        let sparse_positive = storage.terms.iter().all(|term| {
            !term.powers.is_empty()
                && term.powers.iter().all(|(_, exponent)| *exponent > 0)
        });
        prop_assert!(sparse_positive);

        let with_constant = Expr::sum(vec![Expr::number(constant), polynomial.clone()]);
        let keeps_outer_constant = matches!(with_constant.node(), Node::Sum(terms)
            if terms.iter().any(|term| term.same_node(&Expr::number(constant)))
                && terms.iter().any(|term| term.same_node(&polynomial)));
        prop_assert!(keeps_outer_constant);
    }

    #[test]
    fn builtin_function_identities_are_stable_and_wrong_arity_is_rejected(
        builtin_index in 0_usize..8,
        argument_value in -3_i64..=3,
    ) {
        let builtins = ["sin", "cos", "exp", "sqrt", "ln", "gamma", "erf", "zeta"];
        let name = *builtins
            .get(builtin_index)
            .expect("builtin index is generated within the table length");
        let argument = Expr::number(argument_value);
        let function = FunctionName::builtin(name).expect("the sampled builtin is known");
        let first = function.call(vec![argument.clone()]).expect("the builtin is unary");
        let second = function.call(vec![argument]).expect("the builtin is unary");
        prop_assert!(first.same_node(&second));
        prop_assert!(function.call(Vec::new()).is_err());
    }

    #[test]
    fn every_unary_builtin_helper_matches_its_registered_identity(
        helper_index in 0_usize..51,
        argument_value in -3_i64..=3,
    ) {
        type UnaryHelper = fn(&Expr) -> Expr;
        let helpers: &[(&str, UnaryHelper)] = &[
            ("sin", Expr::sin), ("cos", Expr::cos), ("tan", Expr::tan),
            ("cot", Expr::cot), ("sec", Expr::sec), ("csc", Expr::csc),
            ("asin", Expr::asin), ("acos", Expr::acos), ("atan", Expr::atan),
            ("acot", Expr::acot), ("asec", Expr::asec), ("acsc", Expr::acsc),
            ("sinh", Expr::sinh), ("cosh", Expr::cosh), ("tanh", Expr::tanh),
            ("coth", Expr::coth), ("sech", Expr::sech), ("csch", Expr::csch),
            ("asinh", Expr::asinh), ("acosh", Expr::acosh), ("atanh", Expr::atanh),
            ("acoth", Expr::acoth), ("asech", Expr::asech), ("acsch", Expr::acsch),
            ("exp", Expr::exp), ("exp_m1", Expr::exp_m1), ("exp_neg", Expr::exp_neg),
            ("ln", Expr::ln), ("ln_1p", Expr::ln_1p), ("log10", Expr::log10),
            ("log2", Expr::log2), ("sqrt", Expr::sqrt), ("cbrt", Expr::cbrt),
            ("floor", Expr::floor), ("ceil", Expr::ceil), ("round", Expr::round),
            ("trunc", Expr::trunc), ("fract", Expr::fract), ("abs", Expr::abs),
            ("signum", Expr::signum), ("sinc", Expr::sinc), ("erf", Expr::erf),
            ("erfc", Expr::erfc), ("gamma", Expr::gamma), ("lgamma", Expr::lgamma),
            ("digamma", Expr::digamma), ("trigamma", Expr::trigamma),
            ("tetragamma", Expr::tetragamma), ("zeta", Expr::zeta),
            ("elliptic_k", Expr::elliptic_k), ("elliptic_e", Expr::elliptic_e),
        ];
        let (name, helper) = helpers
            .get(helper_index)
            .copied()
            .expect("helper index is generated within the table length");
        let argument = Expr::number(argument_value);
        let function = FunctionName::builtin(name).expect("the helper's builtin is known");
        prop_assert!(helper(&argument).same_node(&function.call(vec![argument]).expect("the builtin is unary")));
    }

    #[test]
    fn parameterized_helpers_preserve_leading_argument_order(
        helper_index in 0_usize..13,
        parameter_value in -3_i64..=3,
        argument_value in -3_i64..=3,
    ) {
        type ParameterizedHelper = fn(&Expr, Expr) -> Expr;
        let helpers: &[(&str, ParameterizedHelper)] = &[
            ("polygamma", Expr::polygamma), ("besselj", Expr::besselj),
            ("bessel_j", Expr::besselj), ("bessely", Expr::bessely),
            ("bessel_y", Expr::bessely), ("besseli", Expr::besseli),
            ("bessel_i", Expr::besseli), ("besselk", Expr::besselk),
            ("bessel_k", Expr::besselk), ("zeta_deriv", Expr::zeta_deriv),
            ("log", Expr::log), ("hermite", Expr::hermite),
            ("lambert_w", Expr::lambertw),
        ];
        let (name, helper) = helpers
            .get(helper_index)
            .copied()
            .expect("helper index is generated within the table length");
        let argument = Expr::number(argument_value);
        let parameter = Expr::number(parameter_value);
        let function = FunctionName::builtin(name).expect("the helper's builtin is known");
        prop_assert!(helper(&argument, parameter.clone()).same_node(&function.call(vec![parameter, argument]).expect("the builtin accepts two arguments")));
    }

    #[test]
    fn contexts_keep_equal_function_names_distinct(
        function_index in 0_usize..4,
        argument_value in -2_i64..=2,
    ) {
        let name = format!("generated_context_function_{function_index}");
        let first_context = Context::new();
        let second_context = Context::new();
        let first_function = first_context.register_function(&name, UserFunction::new(1..=1)).expect("the custom name is unbound");
        let second_function = second_context.register_function(&name, UserFunction::new(1..=1)).expect("the other namespace is independent");
        let argument = Expr::number(argument_value);
        let first = first_function.call(vec![argument.clone()]).expect("the function is unary");
        let second = second_function.call(vec![argument]).expect("the function is unary");
        prop_assert!(!first.same_node(&second));
    }

    #[test]
    fn simultaneous_substitution_updates_every_matching_edge(
        mut indices in collection::vec(0_usize..4, 1..=8),
        target in 0_usize..4,
    ) {
        if !indices.contains(&target) {
            *indices
                .first_mut()
                .expect("the generated index vector is non-empty") = target;
        }
        let input_args: Vec<_> = indices
            .iter()
            .map(|index| {
                Expr::symbol(format!("generated_substitution_symbol_{index}")).sin()
            })
            .collect();
        let replacement = Expr::symbol("generated_substitution_replacement");
        let context = Context::new();
        let pair = context.register_function("pair", UserFunction::default()).expect("the custom name is unbound");
        let input = pair.call(input_args).expect("pair is variadic");
        let result = input.substitute_symbol(
            Symbol::intern(&format!("generated_substitution_symbol_{target}")),
            &replacement,
        );
        let expected_args: Vec<_> = indices
            .iter()
            .map(|index| {
                if *index == target {
                    replacement.sin()
                } else {
                    Expr::symbol(format!("generated_substitution_symbol_{index}")).sin()
                }
            })
            .collect();
        prop_assert_eq!(result, pair.call(expected_args).expect("pair is variadic"));
    }

    #[test]
    fn unchanged_mapping_visits_a_generated_shared_dag_once(
        symbol_index in 0_usize..32,
        edge_count in 2_usize..=8,
    ) {
        let shared = Expr::symbol(format!("generated_map_symbol_{symbol_index}")).sin();
        let edges = vec![shared; edge_count];
        let context = Context::new();
        let pair = context.register_function("pair", UserFunction::default()).expect("the custom name is unbound");
        let root = pair.call(edges).expect("pair is variadic");
        let mut visited = HashMap::new();
        let result = root.map(|node| {
            let count = visited.entry(node.clone()).or_insert(0_usize);
            *count = count.saturating_add(1);
            Ok::<_, ()>(None)
        }).expect("the counting transformation cannot fail");
        prop_assert_eq!(visited.len(), 3);
        prop_assert!(visited.values().all(|count| *count == 1));
        prop_assert!(result.same_node(&root));
    }

    #[test]
    fn context_symbols_with_equal_names_remain_identity_aware(
        name_index in 0_usize..32,
        edge_count in 1_usize..=6,
        derivative_order in 1_u32..=5,
    ) {
        let name = format!("generated_context_symbol_{name_index}");
        let first_context = Context::new();
        let second_context = Context::new();
        let first = first_context.symb(&name);
        let other = second_context.symb(&name);
        let replacement = Expr::symbol(format!("generated_context_replacement_{name_index}"));
        let derivative = Expr::derivative_symbol(Expr::number(derivative_order), first, derivative_order);
        let pair = first_context.register_function("pair", UserFunction::default()).expect("the custom name is unbound");
        let identity_expression = pair.call(vec![derivative, other.to_expr()]).expect("pair is variadic");
        let identities = HashSet::from([first, other]);
        prop_assert_eq!(identity_expression.symbols(), identities.clone());
        prop_assert!(identity_expression.contains_symbol(first));
        prop_assert!(identity_expression.has_free_symbols(&HashSet::from([first])));
        prop_assert!(!identity_expression.has_free_symbols(&identities));
        let mut input_edges = vec![first.to_expr().sin(); edge_count];
        input_edges.push(other.to_expr().cos());
        let input = pair.call(input_edges).expect("pair is variadic");
        let result = input.substitute_symbol(first, &replacement);
        let mut expected_edges = vec![replacement.sin(); edge_count];
        expected_edges.push(other.to_expr().cos());
        prop_assert_eq!(result, pair.call(expected_edges).expect("pair is variadic"));
    }

    #[test]
    fn generated_reclamation_releases_parents_but_keeps_children(
        parent_index in 0_usize..32,
        child_index in 0_usize..32,
    ) {
        let parent_name = format!("generated_reclaim_parent_{parent_index}");
        let child = Expr::symbol(format!("generated_reclaim_child_{child_index}"));
        let context = Context::new();
        let function = context.register_function(&parent_name, UserFunction::new(1..=1)).expect("the custom name is unbound");
        let parent = function.call(vec![child.clone()]).expect("the function is unary");
        let observer = Arc::downgrade(parent.0.as_ref().expect("a live handle owns its node"));
        let key = Node::Function {
            name: function,
            args: vec![child.clone()],
        };
        prop_assert!(NODES.contains_weak(&key, observer.as_ptr()));
        let clone = parent.clone();
        drop(parent);
        prop_assert!(observer.upgrade().is_some());
        drop(clone);
        prop_assert!(observer.upgrade().is_none());
        prop_assert!(!NODES.contains_weak(&key, observer.as_ptr()));
        prop_assert_eq!(child.as_symbol(), Some(Symbol::intern(&format!(
            "generated_reclaim_child_{child_index}"
        ))));
    }

    #[test]
    fn generated_symbol_rebinding_never_reuses_an_identity(
        name_index in 0_usize..32,
    ) {
        let name = format!("generated_rebind_symbol_{name_index}");
        let old = Symbol::intern(&name);
        let expression = old.to_expr();
        prop_assert!(Symbol::remove(&name));
        let rebound = Symbol::intern(&name);
        prop_assert!(old != rebound);
        prop_assert!(expression.same_node(&old.to_expr()));
        prop_assert!(!expression.same_node(&rebound.to_expr()));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "generated rendering cases intentionally construct symbolic operations"
    )]
    fn generated_rendering_preserves_precedence_and_coefficients(
        x_index in 0_usize..32,
        y_index in 0_usize..32,
        coefficient in 2_i64..=7,
        negative_magnitude in 2_i64..=7,
        power in 2_u32..=5,
    ) {
        prop_assume!(x_index != y_index);
        let x = Expr::symbol(format!("renderx{x_index}"));
        let y = Expr::symbol(format!("rendery{y_index}"));
        prop_assert_eq!(format!("{}", &x + 1), format!("1 + renderx{x_index}"));
        prop_assert_eq!(format!("{}", Expr::number(coefficient) * &x), format!("{coefficient}*renderx{x_index}"));
        prop_assert_eq!(format!("{}", (&x + &y).pow(2)), format!("(renderx{x_index} + rendery{y_index})^2"));
        prop_assert_eq!((&x / &y).to_latex(), format!("\\frac{{renderx{x_index}}}{{rendery{y_index}}}"));
        let power_value = i64::from(power);
        let next_power = power_value + 1;
        let negative_coefficient = -negative_magnitude;
        let polynomial = Expr::number(negative_coefficient) * x.pow(power_value)
            + Expr::number(coefficient) * x.pow(next_power);
        let rendered = format!("{polynomial}");
        let rendered_power = format!("renderx{x_index}^{power}");
        let latex_power = format!("^{{{power}}}");
        prop_assert!(rendered.contains(&rendered_power));
        prop_assert!(rendered.contains('-'));
        prop_assert!(polynomial.to_latex().contains(&latex_power));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "generated polynomial matching checks numeric remainder construction"
    )]
    fn generated_poly_views_match_sums_and_rewrite_remainders(
        base_index in 0_usize..8,
        first in 2_u32..=4,
        second in 2_u32..=5,
        constant in 1_i64..=4,
        replacement in 5_i64..=20,
    ) {
        prop_assume!(first != second);
        let base = Expr::symbol(format!("generated_pattern_base_{base_index}"));
        let polynomial = Expr::sum(vec![
            base.pow(i64::from(first)),
            base.pow(i64::from(second)),
        ]);
        let pattern = Pattern::sum(polynomial.children().map(Pattern::exact).collect());
        let rule = RewriteRule::new(pattern, Pattern::exact(replacement))
            .expect("exact projected terms form a valid pattern");
        prop_assert_eq!(rule.apply_all(&polynomial, None, None), Ok(Expr::number(replacement)));
        let with_constant = &polynomial + constant;
        prop_assert_eq!(
            rule.apply_all(&with_constant, None, None),
            Ok(Expr::number(replacement + constant))
        );
    }

    #[test]
    fn generated_higher_arity_helpers_keep_argument_order(
        helper_index in 0_usize..5,
        first_value in -3_i64..=3,
        second_value in -3_i64..=3,
        third_value in -3_i64..=3,
    ) {
        let x = Expr::symbol("generated_arity_x");
        let first = Expr::number(first_value);
        let second = Expr::number(second_value);
        let third = Expr::number(third_value);
        let (helper, name, arguments) = match helper_index {
            0 => (x.beta(first.clone()), "beta", vec![x, first]),
            1 => (x.atan2(first.clone()), "atan2", vec![x, first]),
            2 => (x.log_base(first.clone()), "log_base", vec![x, first]),
            3 => (
                x.assoc_legendre(first.clone(), second.clone()),
                "assoc_legendre", vec![first, second, x],
            ),
            _ => (
                x.spherical_harmonic(first.clone(), second.clone(), third.clone()),
                "spherical_harmonic", vec![first, second, x, third],
            ),
        };
        let textual = FunctionName::builtin(name).expect("the sampled builtin is known")
            .call(arguments).expect("the helper supplies the registered arity");
        prop_assert!(helper.same_node(&textual));
        let is_function = matches!(helper.view(), ExprView::Function { .. });
        prop_assert!(is_function);
    }

    #[test]
    fn polynomial_views_have_unique_positive_powers(
        exponents in collection::vec(1_u32..=5, 2..=6),
    ) {
        let mut distinct = HashSet::new();
        for exponent in &exponents {
            distinct.insert(*exponent);
        }
        prop_assume!(distinct.len() >= 2);
        let base = symbol(0);
        let polynomial = Expr::sum(
            exponents
                .iter()
                .map(|exponent| base.pow(i64::from(*exponent)))
                .collect(),
        );
        let Node::Poly(storage) = polynomial.node() else {
            return Err(proptest::test_runner::TestCaseError::fail("distinct powers select compact storage"));
        };
        let mut seen = HashSet::new();
        for term in &storage.terms {
            for (_, exponent) in &term.powers {
                prop_assert!(*exponent > 0);
                prop_assert!(seen.insert(*exponent));
            }
        }
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "generated opaque bases exercise symbolic sum and quotient construction"
    )]
    fn opaque_repeated_bases_select_poly_without_expanding(
        base_kind in 0_usize..4,
        first in 2_u32..=4,
        second in 2_u32..=5,
    ) {
        prop_assume!(first != second);
        let x = symbol(0);
        let y = symbol(1);
        let base = match base_kind {
            0 => x,
            1 => x.sin(),
            2 => &x + &y,
            _ => &x / &y,
        };
        let expression = Expr::sum(vec![
            base.pow(i64::from(first)),
            base.pow(i64::from(second)),
        ]);
        let Node::Poly(polynomial) = expression.node() else {
            return Err(proptest::test_runner::TestCaseError::fail("repeated opaque bases select compact storage"));
        };
        prop_assert_eq!(&polynomial.bases, core::slice::from_ref(&base));
        let remains_unexpanded = matches!(base.pow(100).view(), ExprView::Pow(_, _));
        prop_assert!(remains_unexpanded);
    }

    #[test]
    fn nonpositive_and_fractional_powers_stay_general_products(
        base_index in 0_usize..3,
        negative in -3_i64..=-1,
        fractional in sample::select(vec![0.25_f64, 0.5, 1.5]),
    ) {
        let base = symbol(base_index);
        let factors = vec![base.pow(negative), base.pow(fractional)];
        let product = Expr::product(factors.clone());
        let keeps_factors = matches!(product.view(), ExprView::Product(stored)
            if stored.len() == factors.len());
        prop_assert!(keeps_factors);
        for factor in factors {
            prop_assert!(product.children().any(|child| child == factor));
        }
    }
}
