//! One builtin or caller step, bound to its compiled rules and domain
//! contract, and the builtin rewrite-rule tables compiled once per sequence.
//!
//! A capture accepts a node whose property is proved. The
//! assumption-dependent tables hold the identities whose zero, realness, or
//! principal-range hypotheses the caller supplies; their captures accept any
//! property that is not refuted.
//!
//! Square roots of products, products and quotients of stored square roots,
//! and sums of logarithms use the direct branch identities, which bound the
//! principal phases of their arguments. A product template of rational powers
//! would also match roots inferred from integral powers, such as
//! \(\pi=\sqrt{\pi}\sqrt{\pi}\).

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::{sync::Arc, vec, vec::Vec};

use crate::{
    core::{ExactIdentity, Expr, ExprView, Postorder, Property, Truth},
    errors::{RewriteError, SimplifyError},
    polynomial::PolynomialWork,
    rewrite::{Pattern, RewriteRule},
};

use super::{
    Algebraic, Branches, Domain, DomainSafety, Identities, Limits, Polynomials, Powers,
    PreparedPlan, SimplificationStep,
};

pub struct PreparedStep<'step> {
    step: &'step SimplificationStep,
    plan: &'step PreparedPlan,
    safety: DomainSafety,
}

impl<'step> PreparedStep<'step> {
    pub const fn new(
        step: &'step SimplificationStep,
        plan: &'step PreparedPlan,
        safety: DomainSafety,
    ) -> Self {
        Self { step, plan, safety }
    }

    /// Run the step's direct identities, then one pass of its compiled rules,
    /// then, for Factor and Compilation, factoring. Each stage that changes
    /// the expression validates its output. At each node the direct
    /// identities are tried in order and the first that applies is kept.
    ///
    /// Compilation has no direct identity that its rules can match, so its
    /// cube-root identity shares the factoring traversal.
    ///
    /// The builtin identities number compact storage in symbols and constants
    /// as a leaf: its terms, a literal times positive powers of distinct
    /// scalar leaves, are fixed points of every builtin step, so the storage is
    /// read whole and its terms are never built. Caller steps visit every
    /// logical node.
    ///
    /// A builtin step's direct identities, and its factoring, read only the
    /// node and the domain contract: a node they keep is kept again, even
    /// under a smaller budget, since an exhausted budget is an error and never
    /// a kept node. So each such map records the nodes it keeps, one bit per
    /// step and contract, and later runs, in this or any sequence, skip them
    /// with their descendants. The rule pass reads the builtins below each
    /// node and visits every node.
    pub fn apply(&self, expression: &Expr, limits: Limits) -> Result<Expr, SimplifyError> {
        let safety = self.safety;
        let stage = self.stage_bit();
        let mut result = match self.step {
            SimplificationStep::Rewrite(rule) => {
                rule.apply_all(expression, limits.matches(), None)?
            }
            SimplificationStep::Map(callback) => {
                expression.map(|node| Ok::<_, SimplifyError>(callback(node)))?
            }
            SimplificationStep::Expand => {
                let mut remaining = limits.expansion();
                Postorder::settled(expression, stage)
                    .map(|node| Polynomials::expand(node, &mut remaining, limits.expansion()))?
            }
            SimplificationStep::Factor | SimplificationStep::Compilation => expression.clone(),
            SimplificationStep::Algebraic => {
                let mut work = PolynomialWork::new(limits.polynomial());
                Postorder::settled(expression, stage).map(|node| {
                    let direct = Algebraic::cancel(node, safety)
                        .or_else(|| Powers::combine(node, safety))
                        .or_else(|| Algebraic::combine_fractions(node))
                        .or_else(|| Algebraic::absolute(node));
                    let candidate = direct.as_ref().unwrap_or(node);
                    Ok::<_, SimplifyError>(
                        Algebraic::polynomial_cancellation(candidate, safety, &mut work)?
                            .or(direct),
                    )
                })?
            }
            SimplificationStep::Roots => Postorder::settled(expression, stage).map(|node| {
                Ok::<_, SimplifyError>(
                    Branches::combine_roots(node, safety)
                        .or_else(|| Identities::root(node, safety)),
                )
            })?,
            SimplificationStep::Exponential => {
                Postorder::settled(expression, stage).map(|node| {
                    Ok::<_, SimplifyError>(
                        Branches::combine_logarithms(node, safety)
                            .or_else(|| Identities::logarithm(node, safety)),
                    )
                })?
            }
            SimplificationStep::Trigonometric => {
                Postorder::settled(expression, stage).map(|node| {
                    Ok::<_, SimplifyError>(
                        Identities::angle(node).or_else(|| Identities::parity(node)),
                    )
                })?
            }
            SimplificationStep::Hyperbolic => Postorder::settled(expression, stage)
                .map(|node| Ok::<_, SimplifyError>(Identities::parity(node)))?,
        };
        if result != *expression {
            limits.validate(&result)?;
        }
        if let Some(rules) = self.plan.rules(safety) {
            let next = rules.apply(&result, limits.matches());
            if next != result {
                limits.validate(&next)?;
                result = next;
            }
        }
        let factored = match self.step {
            // The Factor step also owns the binomial identities.
            SimplificationStep::Factor => Postorder::settled(&result, stage)
                .map(|node| Ok::<_, SimplifyError>(Polynomials::factor(node, true)))?,
            // \(x^{1/3}=\operatorname{cbrt}(x)\) for positive \(x\). The exact
            // exponent must be stored; inferred power groups introduce new roots.
            SimplificationStep::Compilation => Postorder::settled(&result, stage).map(|node| {
                Ok::<_, SimplifyError>(
                    if let ExprView::Pow(base, exponent) = node.view()
                        && exponent.exact_ratio() == Some((1, 3))
                        && base.properties().get(Property::Positive) == Truth::True
                    {
                        Some(base.cbrt())
                    } else {
                        Polynomials::factor(node, false)
                    },
                )
            })?,
            SimplificationStep::Algebraic
            | SimplificationStep::Trigonometric
            | SimplificationStep::Hyperbolic
            | SimplificationStep::Exponential
            | SimplificationStep::Roots
            | SimplificationStep::Expand
            | SimplificationStep::Rewrite(_)
            | SimplificationStep::Map(_) => return Ok(result),
        };
        if factored != result {
            limits.validate(&factored)?;
        }
        Ok(factored)
    }

    /// The rules a step adds under one domain contract, in application order.
    /// Assumption-dependent mode runs them after the preserving rules.
    ///
    /// # Errors
    /// A destination that uses a capture missing from its source. Every builtin
    /// table compiles, which the tests check.
    #[expect(
        clippy::too_many_lines,
        reason = "one table of rule templates per builtin step"
    )]
    pub fn rules(
        step: &SimplificationStep,
        safety: DomainSafety,
    ) -> Result<Vec<RewriteRule>, RewriteError> {
        let capture = |name: &str, property: Property| {
            Pattern::capture(
                name,
                Some(Arc::new(move |node: &Expr| {
                    Domain::admits(node.properties().get(property), safety)
                })),
            )
        };
        let a = capture("a", Property::Finite);
        let b = capture("b", Property::Finite);
        let real = capture("r", Property::Real);
        let nonzero = capture("n", Property::Nonzero);
        let positive = capture("p", Property::Positive);
        let nonnegative = capture("s", Property::Nonnegative);
        let one = Pattern::exact(1);
        let double = Mul::mul(&a, 2);
        let triple = Mul::mul(&a, 3);
        let templates = match (step, safety) {
            (SimplificationStep::Algebraic, DomainSafety::Preserving) => vec![
                (a.abs().abs(), a.abs()),
                (a.pow(2).abs(), a.abs().pow(2)),
                (real.abs().pow(2), real.pow(2)),
                (real.signum().signum(), real.signum()),
                (nonzero.abs().signum(), one),
                (Mul::mul(real.abs(), real.signum()), real),
            ],
            (SimplificationStep::Algebraic, DomainSafety::AssumptionDependent) => vec![
                (real.abs().pow(2), real.pow(2)),
                (real.signum().signum(), real.signum()),
                (nonzero.abs().signum(), one),
                (Mul::mul(real.abs(), real.signum()), real),
            ],
            (SimplificationStep::Exponential, DomainSafety::Preserving) => {
                let base = Pattern::capture(
                    "base",
                    Some(Arc::new(|node: &Expr| {
                        node.properties().get(Property::Positive) == Truth::True
                            && Sub::sub(node, 1).properties().get(Property::Nonzero) == Truth::True
                    })),
                );
                vec![
                    (nonzero.ln().exp(), nonzero.clone()),
                    (real.exp().ln(), real.clone()),
                    (Mul::mul(&a, nonzero.ln()).exp(), nonzero.pow(&a)),
                    (Mul::mul(a.exp(), b.exp()), Add::add(&a, &b).exp()),
                    (Div::div(a.exp(), b.exp()), Sub::sub(&a, &b).exp()),
                    (positive.pow(&real).ln(), Mul::mul(&real, positive.ln())),
                    (positive.pow(&real).log2(), Mul::mul(&real, positive.log2())),
                    (
                        positive.pow(&real).log10(),
                        Mul::mul(&real, positive.log10()),
                    ),
                    (base.log(&base), one),
                ]
            }
            (SimplificationStep::Exponential, DomainSafety::AssumptionDependent) => vec![
                (nonzero.ln().exp(), nonzero.clone()),
                (real.exp().ln(), real),
                (Mul::mul(&a, nonzero.ln()).exp(), nonzero.pow(&a)),
            ],
            (SimplificationStep::Trigonometric, DomainSafety::Preserving) => {
                let atan_domain = Pattern::capture(
                    "d",
                    Some(Arc::new(|node: &Expr| {
                        node.properties().get(Property::Finite) == Truth::True
                            && Add::add(node.pow(2), 1).properties().get(Property::Nonzero)
                                == Truth::True
                    })),
                );
                let cosine_nonzero = Pattern::capture(
                    "c",
                    Some(Arc::new(|node: &Expr| {
                        node.properties().get(Property::Finite) == Truth::True
                            && node.cos().properties().get(Property::Nonzero) == Truth::True
                    })),
                );
                let half = Pattern::exact(Div::div(Expr::pi(), 2));
                vec![
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
                ]
            }
            (SimplificationStep::Trigonometric, DomainSafety::AssumptionDependent) => vec![
                (real.sin().asin(), real.clone()),
                (real.cos().acos(), real.clone()),
                (real.tan().atan(), real),
                (a.atan().tan(), a.clone()),
                (Div::div(&one, a.tan()), a.cot()),
                (Sub::sub(a.sec().pow(2), a.tan().pow(2)), one),
            ],
            (SimplificationStep::Hyperbolic, DomainSafety::Preserving) => {
                let atanh_domain = Pattern::capture(
                    "d",
                    Some(Arc::new(|node: &Expr| {
                        node.properties().get(Property::Finite) == Truth::True
                            && Sub::sub(1_i32, node.pow(2))
                                .properties()
                                .get(Property::Nonzero)
                                == Truth::True
                    })),
                );
                let cosh_nonzero = Pattern::capture(
                    "c",
                    Some(Arc::new(|node: &Expr| {
                        node.properties().get(Property::Finite) == Truth::True
                            && node.cosh().properties().get(Property::Nonzero) == Truth::True
                    })),
                );
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
                    (nonnegative.cosh().acosh(), nonnegative),
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
            (SimplificationStep::Hyperbolic, DomainSafety::AssumptionDependent) => vec![
                (real.sinh().asinh(), real.clone()),
                (real.tanh().atanh(), real.clone()),
                (real.cosh().acosh(), real),
                (a.atanh().tanh(), a.clone()),
                (Div::div(&one, a.tanh()), a.coth()),
                (Add::add(a.tanh().pow(2), a.sech().pow(2)), one),
            ],
            // Real cube roots.
            (SimplificationStep::Roots, DomainSafety::Preserving) => {
                vec![(real.pow(3).cbrt(), real), (a.cbrt().pow(3), a)]
            }
            // Native evaluation forms retaining the original principal domains.
            (SimplificationStep::Compilation, DomainSafety::Preserving) => {
                let log_domain = Pattern::capture(
                    "d",
                    Some(Arc::new(|node: &Expr| {
                        node.properties().get(Property::Finite) == Truth::True
                            && Add::add(node, 1).properties().get(Property::Nonzero) == Truth::True
                    })),
                );
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
            (SimplificationStep::Compilation, DomainSafety::AssumptionDependent) => vec![
                (Div::div(nonzero.sin(), &nonzero), nonzero.sinc()),
                (real.exp_m1().ln_1p(), real),
                (a.ln_1p().exp_m1(), a),
            ],
            // The root identities are direct and read the mode themselves.
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
            .map(|(source, destination)| RewriteRule::new(source, destination))
            .collect()
    }

    const fn stage_bit(&self) -> u32 {
        let step_mask = match self.step {
            SimplificationStep::Algebraic => 0b11,
            SimplificationStep::Roots => 0b11 << 2,
            SimplificationStep::Exponential => 0b11 << 4,
            SimplificationStep::Trigonometric => 0b11 << 6,
            SimplificationStep::Hyperbolic => 0b11 << 8,
            SimplificationStep::Expand => 0b11 << 10,
            SimplificationStep::Factor => 0b11 << 12,
            SimplificationStep::Compilation => 0b11 << 14,
            SimplificationStep::Rewrite(_) | SimplificationStep::Map(_) => 0,
        };
        let safety_mask = match self.safety {
            DomainSafety::Preserving => 0x5555_5555,
            DomainSafety::AssumptionDependent => 0xAAAA_AAAA,
        };
        step_mask & safety_mask
    }
}
