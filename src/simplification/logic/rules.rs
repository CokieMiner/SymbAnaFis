//! Builtin rewrite-rule tables compiled once per step and domain contract.
//!
//! In preserving mode, captures require proved scalar properties. In
//! assumption-dependent mode, rules whose preconditions the caller may supply
//! replace their preserving counterparts in place so no pattern is matched
//! twice, while preserving-only rules keep their strict finite guards.
//!
//! Square roots of products, products and quotients of stored square roots,
//! and sums of logarithms use the direct branch identities, which bound the
//! principal phases of their arguments. A product template of rational powers
//! would also match roots inferred from integral powers, such as
//! $\pi=\sqrt{\pi}\sqrt{\pi}$.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::{sync::Arc, vec, vec::Vec};

use crate::{
    core::{Expr, Property, Truth},
    rewrite::{Pattern, RewriteRule},
};

use super::{Domain, DomainSafety, PreparedStep, SimplificationStep};

/// Capture a subexpression whose `property` is admitted under `safety`.
impl PreparedStep<'_> {
    /// Capture a subexpression whose `property` is admitted under `safety`.
    fn capture(name: &str, property: Property, safety: DomainSafety) -> Pattern {
        Pattern::capture(
            name,
            Some(Arc::new(move |node: &Expr| {
                Domain::admits(node.properties().get(property), safety)
            })),
        )
    }

    /// Capture a finite subexpression whose derived `guard(node)` is proved
    /// nonzero in preserving mode, or any non-refuted finite expression in
    /// assumption-dependent mode.
    fn guarded_nonzero(name: &str, safety: DomainSafety, guard: fn(&Expr) -> Expr) -> Pattern {
        if safety == DomainSafety::AssumptionDependent {
            Self::capture("a", Property::Finite, safety)
        } else {
            Pattern::capture(
                name,
                Some(Arc::new(move |node: &Expr| {
                    node.properties().get(Property::Finite) == Truth::True
                        && guard(node).properties().get(Property::Nonzero) == Truth::True
                })),
            )
        }
    }

    /// The complete rule table for `step` under `safety`, in application order.
    /// Returns an empty vector when `step` contributes no rules, or when
    /// `safety == DomainSafety::AssumptionDependent` adds no rule changes over
    /// `DomainSafety::Preserving` (so callers reuse the preserving table).
    ///
    /// # Panics
    /// Panics when a template's destination uses a capture its source does not
    /// bind. Every builtin template binds the captures it uses, which the tests
    /// check, and caller rules never reach this table.
    pub fn rules(step: &SimplificationStep, safety: DomainSafety) -> Vec<RewriteRule> {
        let templates = match (step, safety) {
            (SimplificationStep::Algebraic, _) => Self::algebraic_rules(safety),
            (SimplificationStep::Exponential, _) => Self::exponential_rules(safety),
            (SimplificationStep::Trigonometric, _) => Self::trigonometric_rules(safety),
            (SimplificationStep::Hyperbolic, _) => Self::hyperbolic_rules(safety),
            (SimplificationStep::Compilation, _) => Self::compilation_rules(safety),
            // Real cube roots; assumption-dependent root rules are direct.
            (SimplificationStep::Roots, DomainSafety::Preserving) => {
                let a = Self::capture("a", Property::Finite, DomainSafety::Preserving);
                let real = Self::capture("r", Property::Real, DomainSafety::Preserving);
                vec![(real.pow(3).cbrt(), real), (a.cbrt().pow(3), a)]
            }
            (SimplificationStep::Roots, DomainSafety::AssumptionDependent)
            | (
                SimplificationStep::Factor
                | SimplificationStep::Expand
                | SimplificationStep::Rewrite(_)
                | SimplificationStep::Map(_),
                _,
            ) => Vec::new(),
        };
        templates
            .into_iter()
            .map(|(source, destination)| {
                RewriteRule::new(source, destination)
                    .expect("a builtin template binds every capture it uses")
            })
            .collect()
    }

    fn algebraic_rules(safety: DomainSafety) -> Vec<(Pattern, Pattern)> {
        let finite = Self::capture("a", Property::Finite, DomainSafety::Preserving);
        let real = Self::capture("r", Property::Real, safety);
        let nonzero = Self::capture("n", Property::Nonzero, safety);
        vec![
            (finite.abs().abs(), finite.abs()),
            (finite.pow(2).abs(), finite.abs().pow(2)),
            (real.abs().pow(2), real.pow(2)),
            (real.signum().signum(), real.signum()),
            (nonzero.abs().signum(), Pattern::exact(1)),
            (Mul::mul(real.abs(), real.signum()), real),
        ]
    }

    fn exponential_rules(safety: DomainSafety) -> Vec<(Pattern, Pattern)> {
        let finite = Self::capture("a", Property::Finite, DomainSafety::Preserving);
        let other = Self::capture("b", Property::Finite, DomainSafety::Preserving);
        let exponent = Self::capture("a", Property::Finite, safety);
        let real = Self::capture("r", Property::Real, safety);
        let nonzero = Self::capture("n", Property::Nonzero, safety);
        let real_proved = Self::capture("r", Property::Real, DomainSafety::Preserving);
        let positive_proved = Self::capture("p", Property::Positive, DomainSafety::Preserving);
        let base = Pattern::capture(
            "base",
            Some(Arc::new(|node: &Expr| {
                node.properties().get(Property::Positive) == Truth::True
                    && Sub::sub(node, 1).properties().get(Property::Nonzero) == Truth::True
            })),
        );
        vec![
            (nonzero.ln().exp(), nonzero.clone()),
            (real.exp().ln(), real),
            (
                Mul::mul(&exponent, nonzero.ln()).exp(),
                nonzero.pow(&exponent),
            ),
            (
                Mul::mul(finite.exp(), other.exp()),
                Add::add(&finite, &other).exp(),
            ),
            (
                Div::div(finite.exp(), other.exp()),
                Sub::sub(&finite, &other).exp(),
            ),
            (
                positive_proved.pow(&real_proved).ln(),
                Mul::mul(&real_proved, positive_proved.ln()),
            ),
            (
                positive_proved.pow(&real_proved).log2(),
                Mul::mul(&real_proved, positive_proved.log2()),
            ),
            (
                positive_proved.pow(&real_proved).log10(),
                Mul::mul(&real_proved, positive_proved.log10()),
            ),
            (base.log(&base), Pattern::exact(1)),
        ]
    }

    fn trigonometric_rules(safety: DomainSafety) -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite, DomainSafety::Preserving);
        let b = Self::capture("b", Property::Finite, DomainSafety::Preserving);
        let one = Pattern::exact(1);
        let double = Mul::mul(&a, 2);
        let triple = Mul::mul(&a, 3);
        let half = Pattern::exact(Div::div(Expr::pi(), 2));
        let atan_domain = Self::guarded_nonzero("d", safety, |node| Add::add(node.pow(2), 1));
        let cosine_nonzero = Self::guarded_nonzero("c", safety, Expr::cos);
        let mut templates = vec![
            (Add::add(a.sin().pow(2), a.cos().pow(2)), one.clone()),
            (Sub::sub(&one, a.sin().pow(2)), a.cos().pow(2)),
            (Sub::sub(&one, a.cos().pow(2)), a.sin().pow(2)),
            (Add::add(&one, a.tan().pow(2)), a.sec().pow(2)),
            (Add::add(&one, a.cot().pow(2)), a.csc().pow(2)),
            (Sub::sub(a.sec().pow(2), &one), a.tan().pow(2)),
            (Sub::sub(a.csc().pow(2), &one), a.cot().pow(2)),
            (
                Sub::sub(cosine_nonzero.sec().pow(2), cosine_nonzero.tan().pow(2)),
                one.clone(),
            ),
            (Sub::sub(a.cos().pow(2), a.sin().pow(2)), double.cos()),
            (
                Sub::sub(a.sin().pow(2), a.cos().pow(2)),
                Neg::neg(double.cos()),
            ),
            (
                Mul::mul(Sub::sub(a.cos(), a.sin()), Add::add(a.cos(), a.sin())),
                double.cos(),
            ),
            (
                Pattern::product(vec![Pattern::exact(2), a.sin(), a.cos()]),
                double.sin(),
            ),
            (Mul::mul(a.sin(), a.cos()), Div::div(double.sin(), 2)),
            (
                Add::add(Mul::mul(a.sin(), b.cos()), Mul::mul(a.cos(), b.sin())),
                Add::add(&a, &b).sin(),
            ),
            (
                Sub::sub(Mul::mul(a.sin(), b.cos()), Mul::mul(a.cos(), b.sin())),
                Sub::sub(&a, &b).sin(),
            ),
            (
                Sub::sub(Mul::mul(a.cos(), b.cos()), Mul::mul(a.sin(), b.sin())),
                Add::add(&a, &b).cos(),
            ),
            (
                Add::add(Mul::mul(a.cos(), b.cos()), Mul::mul(a.sin(), b.sin())),
                Sub::sub(&a, &b).cos(),
            ),
            (
                Sub::sub(Mul::mul(a.sin(), 3), Mul::mul(a.sin().pow(3), 4)),
                triple.sin(),
            ),
            (
                Sub::sub(Mul::mul(a.cos().pow(3), 4), Mul::mul(a.cos(), 3)),
                triple.cos(),
            ),
            (Div::div(a.sin(), a.cos()), a.tan()),
            (Div::div(a.cos(), a.sin()), a.cot()),
            (Div::div(&one, a.cos()), a.sec()),
            (Div::div(&one, a.sin()), a.csc()),
            (Div::div(&one, cosine_nonzero.tan()), cosine_nonzero.cot()),
            (a.asin().sin(), a.clone()),
            (a.acos().cos(), a.clone()),
            (atan_domain.atan().tan(), atan_domain),
            (Sub::sub(&half, &a).sin(), a.cos()),
            (Sub::sub(&half, &a).cos(), a.sin()),
            (Sub::sub(&half, &a).tan(), a.cot()),
            (Sub::sub(&half, &a).cot(), a.tan()),
            (Sub::sub(&half, &a).sec(), a.csc()),
            (Sub::sub(&half, &a).csc(), a.sec()),
        ];
        if safety == DomainSafety::AssumptionDependent {
            let real = Self::capture("r", Property::Real, safety);
            templates.extend([
                (real.sin().asin(), real.clone()),
                (real.cos().acos(), real.clone()),
                (real.tan().atan(), real),
            ]);
        }
        templates
    }

    fn hyperbolic_rules(safety: DomainSafety) -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite, DomainSafety::Preserving);
        let b = Self::capture("b", Property::Finite, DomainSafety::Preserving);
        let real = Self::capture("r", Property::Real, safety);
        let one = Pattern::exact(1);
        let double = Mul::mul(&a, 2);
        let triple = Mul::mul(&a, 3);
        let atanh_domain = Self::guarded_nonzero("d", safety, |node| Sub::sub(1_i32, node.pow(2)));
        let cosh_nonzero = Self::guarded_nonzero("c", safety, Expr::cosh);
        let acosh_domain = if safety == DomainSafety::AssumptionDependent {
            real.clone()
        } else {
            Self::capture("s", Property::Nonnegative, DomainSafety::Preserving)
        };
        let exp = a.exp();
        let inverse = Neg::neg(&a).exp();
        let twice = double.exp();
        let exp_sum = Add::add(&exp, &inverse);
        let exp_difference = Sub::sub(&exp, &inverse);
        let twice_sum = Add::add(&twice, &one);
        let twice_difference = Sub::sub(&twice, &one);
        vec![
            (Sub::sub(a.cosh().pow(2), a.sinh().pow(2)), one.clone()),
            (Add::add(&one, a.sinh().pow(2)), a.cosh().pow(2)),
            (Sub::sub(a.cosh().pow(2), &one), a.sinh().pow(2)),
            (Sub::sub(&one, a.tanh().pow(2)), a.sech().pow(2)),
            (Sub::sub(a.coth().pow(2), &one), a.csch().pow(2)),
            (
                Add::add(cosh_nonzero.tanh().pow(2), cosh_nonzero.sech().pow(2)),
                one.clone(),
            ),
            (
                Pattern::product(vec![Pattern::exact(2), a.sinh(), a.cosh()]),
                double.sinh(),
            ),
            (Add::add(a.cosh().pow(2), a.sinh().pow(2)), double.cosh()),
            (
                Add::add(Mul::mul(a.sinh(), b.cosh()), Mul::mul(a.cosh(), b.sinh())),
                Add::add(&a, &b).sinh(),
            ),
            (
                Sub::sub(Mul::mul(a.sinh(), b.cosh()), Mul::mul(a.cosh(), b.sinh())),
                Sub::sub(&a, &b).sinh(),
            ),
            (
                Add::add(Mul::mul(a.cosh(), b.cosh()), Mul::mul(a.sinh(), b.sinh())),
                Add::add(&a, &b).cosh(),
            ),
            (
                Sub::sub(Mul::mul(a.cosh(), b.cosh()), Mul::mul(a.sinh(), b.sinh())),
                Sub::sub(&a, &b).cosh(),
            ),
            (
                Add::add(Mul::mul(a.sinh(), 3), Mul::mul(a.sinh().pow(3), 4)),
                triple.sinh(),
            ),
            (
                Sub::sub(Mul::mul(a.cosh().pow(3), 4), Mul::mul(a.cosh(), 3)),
                triple.cosh(),
            ),
            (Div::div(a.sinh(), a.cosh()), a.tanh()),
            (Div::div(a.cosh(), a.sinh()), a.coth()),
            (Div::div(&one, a.cosh()), a.sech()),
            (Div::div(&one, a.sinh()), a.csch()),
            (Div::div(&one, cosh_nonzero.tanh()), cosh_nonzero.coth()),
            (a.asinh().sinh(), a.clone()),
            (a.acosh().cosh(), a.clone()),
            (real.sinh().asinh(), real.clone()),
            (real.tanh().atanh(), real),
            (acosh_domain.cosh().acosh(), acosh_domain),
            (atanh_domain.atanh().tanh(), atanh_domain),
            (Div::div(&exp_difference, 2), a.sinh()),
            (Div::div(&exp_sum, 2), a.cosh()),
            (Div::div(&exp_difference, &exp_sum), a.tanh()),
            (Div::div(&exp_sum, &exp_difference), a.coth()),
            (Div::div(2_i32, &exp_sum), a.sech()),
            (Div::div(2_i32, &exp_difference), a.csch()),
            (Div::div(&twice_difference, &twice_sum), a.tanh()),
            (Div::div(&twice_sum, &twice_difference), a.coth()),
            (Div::div(Mul::mul(&exp, 2), &twice_sum), a.sech()),
            (Div::div(Mul::mul(&exp, 2), &twice_difference), a.csch()),
            (Div::div(&twice_sum, Mul::mul(&exp, 2)), a.cosh()),
            (Div::div(&twice_difference, Mul::mul(&exp, 2)), a.sinh()),
        ]
    }

    fn compilation_rules(safety: DomainSafety) -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite, DomainSafety::Preserving);
        let real = Self::capture("r", Property::Real, safety);
        let nonzero = Self::capture("n", Property::Nonzero, safety);
        let log_domain = Self::guarded_nonzero("d", safety, |node| Add::add(node, 1));
        vec![
            (Sub::sub(a.exp(), 1), a.exp_m1()),
            (Sub::sub(1_i32, a.exp()), Neg::neg(a.exp_m1())),
            (Add::add(&a, 1).ln(), a.ln_1p()),
            (Sub::sub(1_i32, a.erf()), a.erfc()),
            (Div::div(nonzero.sin(), &nonzero), nonzero.sinc()),
            (Add::add(a.exp_m1(), 1), a.exp()),
            (real.exp_m1().ln_1p(), real),
            (log_domain.ln_1p().exp_m1(), log_domain),
        ]
    }
}
