//! Mathematical function definitions for the function registry
//!
//! Contains evaluation and symbolic differentiation rules for all supported functions.
//!
//! # Derivative References
//!
//! Derivative formulas follow standard calculus and DLMF:
//! - Trigonometric: Any standard calculus text, DLMF §4.21-4.28
//! - Hyperbolic: DLMF §4.35-4.37 <https://dlmf.nist.gov/4.35>
//! - Error function: DLMF §7.5 (d/dx erf(x) = 2/√π e^(-x²))
//! - Gamma: DLMF §5.2.1 (Γ'(x) = Γ(x)ψ(x))
//! - Polygamma: DLMF §5.15 (ψ'(x) = ψ₁(x))
//! - Bessel: DLMF §10.6, §10.29 (recurrence-based derivatives)
//! - Elliptic: DLMF §19.4 <https://dlmf.nist.gov/19.4>
//! - Lambert W: Corless et al. (1996) (W'(x) = W(x)/(x(1+W(x))))
use super::registry::FunctionDefinition;
use crate::Expr;
use crate::core::known_symbols::{
    ABS, ASSOC_LEGENDRE, BESSELI, BESSELJ, BESSELK, BESSELY, COS, COT, CSC, DIGAMMA, ELLIPTIC_E,
    ELLIPTIC_K, EXP, EXP_POLAR, GAMMA, HERMITE, LAMBERTW, POLYGAMMA, SEC, SIGNUM, SIN,
    SPHERICAL_HARMONIC, SQRT, TAN, TRIGAMMA, YNM, ZETA_DERIV, get_symbol,
};
use std::sync::Arc;

/// Return all function definitions for populating the registry
pub(crate) fn all_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Trigonometric
        FunctionDefinition {
            name: "sin",
            arity: 1..=1,
            eval: |args| Some(args[0].sin()),
            derivative: |args, arg_primes| {
                // d/dx sin(u) = cos(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(&COS), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cos",
            arity: 1..=1,
            eval: |args| Some(args[0].cos()),
            derivative: |args, arg_primes| {
                // d/dx cos(u) = -sin(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::func_multi_from_arcs_symbol(get_symbol(&SIN), vec![u])),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "tan",
            arity: 1..=1,
            eval: |args| Some(args[0].tan()),
            derivative: |args, arg_primes| {
                // d/dx tan(u) = sec^2(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::pow(
                        Expr::func_multi_from_arcs_symbol(get_symbol(&SEC), vec![u]),
                        Expr::number(2.0),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cot",
            arity: 1..=1,
            eval: |args| Some(1.0 / args[0].tan()),
            derivative: |args, arg_primes| {
                // d/dx cot(u) = -csc^2(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::pow(
                        Expr::func_multi_from_arcs_symbol(get_symbol(&CSC), vec![u]),
                        Expr::number(2.0),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "sec",
            arity: 1..=1,
            eval: |args| Some(1.0 / args[0].cos()),
            derivative: |args, arg_primes| {
                // d/dx sec(u) = sec(u)tan(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(get_symbol(&SEC), vec![Arc::clone(&u)]),
                        Expr::func_multi_from_arcs_symbol(get_symbol(&TAN), vec![u]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "csc",
            arity: 1..=1,
            eval: |args| Some(1.0 / args[0].sin()),
            derivative: |args, arg_primes| {
                // d/dx csc(u) = -csc(u)cot(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(get_symbol(&CSC), vec![Arc::clone(&u)]),
                        Expr::func_multi_from_arcs_symbol(get_symbol(&COT), vec![u]),
                    )),
                    u_prime,
                )
            },
        },
        // Inverse Trigonometric
        FunctionDefinition {
            name: "asin",
            arity: 1..=1,
            eval: |args| Some(args[0].asin()),
            derivative: |args, arg_primes| {
                // d/dx asin(u) = u' / sqrt(1 - u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::func_symbol(
                            get_symbol(&SQRT),
                            Expr::sub_expr(
                                Expr::number(1.0),
                                Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                            ),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "acos",
            arity: 1..=1,
            eval: |args| Some(args[0].acos()),
            derivative: |args, arg_primes| {
                // d/dx acos(u) = -u' / sqrt(1 - u^2) = -1/sqrt(1-u^2) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::func_symbol(
                            get_symbol(&SQRT),
                            Expr::sub_expr(
                                Expr::number(1.0),
                                Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                            ),
                        ),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "atan",
            arity: 1..=1,
            eval: |args| Some(args[0].atan()),
            derivative: |args, arg_primes| {
                // d/dx atan(u) = u' / (1 + u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::add_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "atan2",
            arity: 2..=2,
            eval: |args| Some(args[0].atan2(args[1])),
            derivative: |args, arg_primes| {
                // d/dx atan2(y, x) = (x*y' - y*x') / (x^2 + y^2)
                let y = args[0].clone();
                let x = args[1].clone();
                let y_prime = arg_primes[0].clone();
                let x_prime = arg_primes[1].clone();

                let numerator = Expr::sub_expr(
                    Expr::mul_expr(Expr::unwrap_arc(x.clone()), y_prime),
                    Expr::mul_expr(Expr::unwrap_arc(y.clone()), x_prime),
                );
                let denominator = Expr::add_expr(
                    Expr::pow_from_arcs(x.clone(), Arc::new(Expr::number(2.0))),
                    Expr::pow_from_arcs(y.clone(), Arc::new(Expr::number(2.0))),
                );

                Expr::div_expr(numerator, denominator)
            },
        },
        FunctionDefinition {
            name: "acot",
            arity: 1..=1,
            eval: |args| {
                let x = args[0];
                if x.abs() < 1e-15 {
                    Some(std::f64::consts::PI / 2.0)
                } else if x > 0.0 {
                    Some((1.0 / x).atan())
                } else {
                    Some((1.0 / x).atan() + std::f64::consts::PI)
                }
            },
            derivative: |args, arg_primes| {
                // d/dx acot(u) = -u' / (1 + u^2) = -1/(1+u^2) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::add_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                        ),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "asec",
            arity: 1..=1,
            eval: |args| Some((1.0 / args[0]).acos()),
            derivative: |args, arg_primes| {
                // d/dx asec(u) = u' / (|u| * sqrt(u^2 - 1))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_expr(
                            Expr::func_multi_from_arcs("abs", vec![u.clone()]),
                            Expr::func(
                                "sqrt",
                                Expr::sub_expr(
                                    Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                                    Expr::number(1.0),
                                ),
                            ),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "acsc",
            arity: 1..=1,
            eval: |args| Some((1.0 / args[0]).asin()),
            derivative: |args, arg_primes| {
                // d/dx acsc(u) = -u' / (|u| * sqrt(u^2 - 1)) = -1/(|u|*sqrt(u^2-1)) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            Arc::new(Expr::func_multi_from_arcs("abs", vec![u.clone()])),
                            Arc::new(Expr::func(
                                "sqrt",
                                Expr::sub_expr(
                                    Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                                    Expr::number(1.0),
                                ),
                            )),
                        ]),
                    )),
                    u_prime,
                )
            },
        },
        // Hyperbolic
        FunctionDefinition {
            name: "sinh",
            arity: 1..=1,
            eval: |args| Some(args[0].sinh()),
            derivative: |args, arg_primes| {
                // d/dx sinh(u) = cosh(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(Expr::func_multi_from_arcs("cosh", vec![u]), u_prime)
            },
        },
        FunctionDefinition {
            name: "cosh",
            arity: 1..=1,
            eval: |args| Some(args[0].cosh()),
            derivative: |args, arg_primes| {
                // d/dx cosh(u) = sinh(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(Expr::func_multi_from_arcs("sinh", vec![u]), u_prime)
            },
        },
        FunctionDefinition {
            name: "tanh",
            arity: 1..=1,
            eval: |args| Some(args[0].tanh()),
            derivative: |args, arg_primes| {
                // d/dx tanh(u) = (1 - tanh^2(u)) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::sub_expr(
                        Expr::number(1.0),
                        Expr::pow(
                            Expr::func_multi_from_arcs("tanh", vec![u]),
                            Expr::number(2.0),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "coth",
            arity: 1..=1,
            eval: |args| Some(1.0 / args[0].tanh()),
            derivative: |args, arg_primes| {
                // d/dx coth(u) = -csch^2(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::pow(
                        Expr::func_multi_from_arcs("csch", vec![u]),
                        Expr::number(2.0),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "sech",
            arity: 1..=1,
            eval: |args| Some(1.0 / args[0].cosh()),
            derivative: |args, arg_primes| {
                // d/dx sech(u) = -sech(u)tanh(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::mul_expr(
                        Expr::func_multi_from_arcs("sech", vec![Arc::clone(&u)]),
                        Expr::func_multi_from_arcs("tanh", vec![u]),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "csch",
            arity: 1..=1,
            eval: |args| Some(1.0 / args[0].sinh()),
            derivative: |args, arg_primes| {
                // d/dx csch(u) = -csch(u)coth(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::mul_expr(
                        Expr::func_multi_from_arcs("csch", vec![Arc::clone(&u)]),
                        Expr::func_multi_from_arcs("coth", vec![u]),
                    )),
                    u_prime,
                )
            },
        },
        // Inverse Hyperbolic
        FunctionDefinition {
            name: "asinh",
            arity: 1..=1,
            eval: |args| Some(args[0].asinh()),
            derivative: |args, arg_primes| {
                // d/dx asinh(u) = u' / sqrt(u^2 + 1)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::func(
                            "sqrt",
                            Expr::add_expr(
                                Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                                Expr::number(1.0),
                            ),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "acosh",
            arity: 1..=1,
            eval: |args| Some(args[0].acosh()),
            derivative: |args, arg_primes| {
                // d/dx acosh(u) = u' / sqrt(u^2 - 1)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::func(
                            "sqrt",
                            Expr::sub_expr(
                                Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                                Expr::number(1.0),
                            ),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "atanh",
            arity: 1..=1,
            eval: |args| Some(args[0].atanh()),
            derivative: |args, arg_primes| {
                // d/dx atanh(u) = u' / (1 - u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::sub_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "acoth",
            arity: 1..=1,
            eval: |args| Some(0.5 * ((args[0] + 1.0) / (args[0] - 1.0)).ln()),
            derivative: |args, arg_primes| {
                // d/dx acoth(u) = u' / (1 - u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::sub_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "asech",
            arity: 1..=1,
            eval: |args| Some((1.0 / args[0]).acosh()),
            derivative: |args, arg_primes| {
                // d/dx asech(u) = -u' / (u * sqrt(1 - u^2)) = -1/(u*sqrt(1-u^2)) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            u.clone(),
                            Arc::new(Expr::func_symbol(
                                get_symbol(&SQRT),
                                Expr::sub_expr(
                                    Expr::number(1.0),
                                    Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                                ),
                            )),
                        ]),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "acsch",
            arity: 1..=1,
            eval: |args| {
                if args[0].abs() < 1e-15 {
                    None
                } else {
                    Some((1.0 / args[0]).asinh())
                }
            },
            derivative: |args, arg_primes| {
                // d/dx acsch(u) = -u' / (|u| * sqrt(1 + u^2)) = -1/(|u|*sqrt(1+u^2)) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            Arc::new(Expr::func_multi_from_arcs_symbol(
                                get_symbol(&ABS),
                                vec![u.clone()],
                            )),
                            Arc::new(Expr::func_symbol(
                                get_symbol(&SQRT),
                                Expr::add_expr(
                                    Expr::number(1.0),
                                    Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(2.0))),
                                ),
                            )),
                        ]),
                    )),
                    u_prime,
                )
            },
        },
        // Roots / Exp / Log
        FunctionDefinition {
            name: "exp",
            arity: 1..=1,
            eval: |args| Some(args[0].exp()),
            derivative: |args, arg_primes| {
                // d/dx exp(u) = exp(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(&EXP), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "ln",
            arity: 1..=1,
            eval: |args| Some(args[0].ln()),
            derivative: |args, arg_primes| {
                // d/dx ln(u) = u' / u
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::pow_from_arcs(u.clone(), Arc::new(Expr::number(-1.0))),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "log",
            arity: 2..=2,
            eval: |args| {
                // log(base, x) = ln(x) / ln(base)
                let base = args[0];
                let x = args[1];
                if base <= 0.0 || base == 1.0 || x <= 0.0 {
                    None
                } else {
                    Some(x.ln() / base.ln())
                }
            },
            derivative: |args, arg_primes| {
                // log_b(x) = ln(x) / ln(b)
                // d/dt log_b(x) = (1/(x*ln(b))) * x' - (ln(x)/(b*ln(b)^2)) * b'
                //               = x'/(x*ln(b)) - b'*ln(x)/(b*ln(b)^2)
                let b = args[0].clone();
                let x = args[1].clone();
                let b_prime = arg_primes[0].clone();
                let x_prime = arg_primes[1].clone();

                // Term 1: x' / (x * ln(b))
                let ln_b = Expr::func_multi_from_arcs("ln", vec![b.clone()]);
                let term1 = Expr::div_expr(
                    x_prime,
                    Expr::mul_from_arcs(vec![x.clone(), Arc::new(ln_b.clone())]),
                );

                // Term 2: -b' * ln(x) / (b * ln(b)^2)
                let ln_x = Expr::func_multi_from_arcs("ln", vec![x.clone()]);
                let ln_b_sq = Expr::pow(ln_b, Expr::number(2.0));
                let term2 = Expr::negate(Expr::div_expr(
                    Expr::mul_expr(b_prime, ln_x),
                    Expr::mul_from_arcs(vec![b.clone(), Arc::new(ln_b_sq)]),
                ));

                Expr::add_expr(term1, term2)
            },
        },
        FunctionDefinition {
            name: "log10",
            arity: 1..=1,
            eval: |args| Some(args[0].log10()),
            derivative: |args, arg_primes| {
                // d/dx log10(u) = u' / (u * ln(10))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            u.clone(),
                            Arc::new(Expr::func("ln", Expr::number(10.0))),
                        ]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "log2",
            arity: 1..=1,
            eval: |args| Some(args[0].log2()),
            derivative: |args, arg_primes| {
                // d/dx log2(u) = u' / (u * ln(2))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            u.clone(),
                            Arc::new(Expr::func("ln", Expr::number(2.0))),
                        ]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "sqrt",
            arity: 1..=1,
            eval: |args| Some(args[0].sqrt()),
            derivative: |args, arg_primes| {
                // d/dx sqrt(u) = u' / (2 * sqrt(u))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_expr(
                            Expr::number(2.0),
                            Expr::func_multi_from_arcs_symbol(get_symbol(&SQRT), vec![u]),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cbrt",
            arity: 1..=1,
            eval: |args| Some(args[0].cbrt()),
            derivative: |args, arg_primes| {
                // d/dx cbrt(u) = u' / (3 * u^(2/3))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_expr(
                            Expr::number(3.0),
                            Expr::pow_from_arcs(
                                u.clone(),
                                Arc::new(Expr::div_expr(Expr::number(2.0), Expr::number(3.0))),
                            ),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        // Special Functions
        FunctionDefinition {
            name: "abs",
            arity: 1..=1,
            eval: |args| Some(args[0].abs()),
            derivative: |args, arg_primes| {
                // d/dx |u| = signum(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(&SIGNUM), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "signum",
            arity: 1..=1,
            eval: |args| Some(args[0].signum()),
            derivative: |_, _| {
                // d/dx signum(u) = 0 almost everywhere
                Expr::number(0.0)
            },
        },
        FunctionDefinition {
            name: "erf",
            arity: 1..=1,
            eval: |args| Some(crate::math::eval_erf(args[0])),
            derivative: |args, arg_primes| {
                // d/dx erf(u) = (2/sqrt(pi)) * exp(-u^2) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let pi = Expr::symbol("pi");
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::div_expr(Expr::number(2.0), Expr::func_symbol(get_symbol(&SQRT), pi)),
                        Expr::func_symbol(
                            get_symbol(&EXP),
                            Expr::negate(Expr::pow_from_arcs(
                                u.clone(),
                                Arc::new(Expr::number(2.0)),
                            )),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "erfc",
            arity: 1..=1,
            eval: |args| Some(1.0 - crate::math::eval_erf(args[0])),
            derivative: |args, arg_primes| {
                // d/dx erfc(u) = -d/dx erf(u)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let pi = Expr::symbol("pi");
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::div_expr(
                            Expr::number(-2.0),
                            Expr::func_symbol(get_symbol(&SQRT), pi),
                        ),
                        Expr::func_symbol(
                            get_symbol(&EXP),
                            Expr::negate(Expr::pow_from_arcs(
                                u.clone(),
                                Arc::new(Expr::number(2.0)),
                            )),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "gamma",
            arity: 1..=1,
            eval: |args| crate::math::eval_gamma(args[0]),
            derivative: |args, arg_primes| {
                // d/dx gamma(u) = gamma(u) * psi(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(get_symbol(&GAMMA), vec![u.clone()]),
                        Expr::func_multi_from_arcs_symbol(get_symbol(&DIGAMMA), vec![u]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "digamma",
            arity: 1..=1,
            eval: |args| crate::math::eval_digamma(args[0]),
            derivative: |args, arg_primes| {
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(&TRIGAMMA), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "trigamma",
            arity: 1..=1,
            eval: |args| crate::math::eval_trigamma(args[0]),
            derivative: |args, arg_primes| {
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(Expr::func_multi_from_arcs("tetragamma", vec![u]), u_prime)
            },
        },
        FunctionDefinition {
            name: "beta",
            arity: 2..=2,
            eval: |args| {
                let a = args[0];
                let b = args[1];
                let ga = crate::math::eval_gamma(a)?;
                let gb = crate::math::eval_gamma(b)?;
                let gab = crate::math::eval_gamma(a + b)?;
                Some(ga * gb / gab)
            },
            derivative: |args, arg_primes| {
                // d/dx beta(a,b) = beta(a,b) * (psi(a) - psi(a+b)) * a' + beta(a,b) * (psi(b) - psi(a+b)) * b'
                let a = args[0].clone();
                let b = args[1].clone();
                let a_prime = arg_primes[0].clone();
                let b_prime = arg_primes[1].clone();

                let beta_ab = Expr::func_multi_from_arcs("beta", vec![a.clone(), b.clone()]);
                let psi_a_plus_b = Expr::func_multi_from_arcs(
                    "digamma",
                    vec![Arc::new(Expr::sum_from_arcs(vec![a.clone(), b.clone()]))],
                );

                let term_a = Expr::mul_expr(
                    Expr::mul_expr(
                        beta_ab.clone(),
                        Expr::sub_expr(
                            Expr::func_multi_from_arcs("digamma", vec![a.clone()]),
                            psi_a_plus_b.clone(),
                        ),
                    ),
                    a_prime,
                );

                let term_b = Expr::mul_expr(
                    Expr::mul_expr(
                        beta_ab,
                        Expr::sub_expr(
                            Expr::func_multi_from_arcs("digamma", vec![b.clone()]),
                            psi_a_plus_b,
                        ),
                    ),
                    b_prime,
                );

                Expr::add_expr(term_a, term_b)
            },
        },
        FunctionDefinition {
            name: "besselj",
            arity: 2..=2,
            eval: |args| crate::math::bessel_j(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                let n = args[0].clone();
                let x = args[1].clone();
                let x_prime = arg_primes[1].clone();

                let half = Expr::number(0.5);
                let n_minus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(1.0))]);

                let j_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELJ),
                    vec![Arc::new(n_minus_1), x.clone()],
                );
                let j_next = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELJ),
                    vec![Arc::new(n_plus_1), x.clone()],
                );

                Expr::mul_expr(
                    Expr::mul_expr(half, Expr::sub_expr(j_prev, j_next)),
                    x_prime,
                )
            },
        },
        FunctionDefinition {
            name: "bessely",
            arity: 2..=2,
            eval: |args| crate::math::bessel_y(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                // d/dx Y_n(x) = (1/2)(Y_{n-1} - Y_{n+1}) * x'
                let n = args[0].clone();
                let x = args[1].clone();
                let x_prime = arg_primes[1].clone();

                let half = Expr::number(0.5);
                let n_minus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(1.0))]);

                let y_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELY),
                    vec![Arc::new(n_minus_1), x.clone()],
                );
                let y_next = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELY),
                    vec![Arc::new(n_plus_1), x.clone()],
                );

                Expr::mul_expr(
                    Expr::mul_expr(half, Expr::sub_expr(y_prev, y_next)),
                    x_prime,
                )
            },
        },
        FunctionDefinition {
            name: "besseli",
            arity: 2..=2,
            eval: |args| crate::math::bessel_i(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                // d/dx I_n(x) = (1/2)(I_{n-1} + I_{n+1}) * x'
                let n = args[0].clone();
                let x = args[1].clone();
                let x_prime = arg_primes[1].clone();

                let half = Expr::number(0.5);
                let n_minus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(1.0))]);

                let i_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELI),
                    vec![Arc::new(n_minus_1), x.clone()],
                );
                let i_next = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELI),
                    vec![Arc::new(n_plus_1), x.clone()],
                );

                Expr::mul_expr(
                    Expr::mul_expr(half, Expr::add_expr(i_prev, i_next)),
                    x_prime,
                )
            },
        },
        FunctionDefinition {
            name: "besselk",
            arity: 2..=2,
            eval: |args| crate::math::bessel_k(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                // d/dx K_n(x) = (-1/2)(K_{n-1} + K_{n+1}) * x'
                let n = args[0].clone();
                let x = args[1].clone();
                let x_prime = arg_primes[1].clone();

                let neg_half = Expr::number(-0.5);
                let n_minus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(1.0))]);

                let k_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELK),
                    vec![Arc::new(n_minus_1), x.clone()],
                );
                let k_next = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&BESSELK),
                    vec![Arc::new(n_plus_1), x.clone()],
                );

                Expr::mul_expr(
                    Expr::mul_expr(neg_half, Expr::add_expr(k_prev, k_next)),
                    x_prime,
                )
            },
        },
        FunctionDefinition {
            name: "polygamma",
            arity: 2..=2,
            eval: |args| crate::math::eval_polygamma(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                // d/dx polygamma(n, x) = polygamma(n+1, x) * x'
                let n = args[0].clone();
                let x = args[1].clone();
                let x_prime = arg_primes[1].clone();

                let n_plus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(1.0))]);
                let derivative = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&POLYGAMMA),
                    vec![Arc::new(n_plus_1), x],
                );

                Expr::mul_expr(derivative, x_prime)
            },
        },
        FunctionDefinition {
            name: "sinc",
            arity: 1..=1,
            eval: |args| {
                let x = args[0];
                if x.abs() < 1e-10 {
                    Some(1.0 - x * x / 6.0 + x.powi(4) / 120.0)
                } else {
                    Some(x.sin() / x)
                }
            },
            derivative: |args, arg_primes| {
                // d/dx sinc(u) = _sinc_deriv(u) * u'
                // where _sinc_deriv(u) = (u*cos(u) - sin(u))/u^2, handled carefully at u=0
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                
                // Use helper function to properly handle singularity at u=0
                let deriv_val = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&crate::core::known_symbols::SINC_DERIV),
                    vec![u],
                );
                Expr::mul_expr(deriv_val, u_prime)
            },
        },
        // Internal helper for sinc derivative - handles x=0 singularity
        FunctionDefinition {
            name: "_sinc_deriv",
            arity: 1..=1,
            eval: |args| {
                let x = args[0];
                // At x=0, sinc'(x) = 0 (from Taylor series: sinc(x) = 1 - x²/6 + ..., so sinc'(x) = -x/3 + ...)
                if x.abs() < 1e-6 {
                    // Use Taylor series: sinc'(x) ≈ -x/3 + x³/30 - x⁵/840 + ...
                    Some(-x / 3.0 + x.powi(3) / 30.0)
                } else {
                    // Standard formula: (x*cos(x) - sin(x))/x²
                    Some((x * x.cos() - x.sin()) / (x * x))
                }
            },
            derivative: |args, arg_primes| {
                // Second derivative of sinc - not commonly needed, return symbolic
                // d²/dx² sinc(x) would require computing the derivative of this function
                // For now, we'll use a symbolic representation
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                
                // Symbolic second derivative (not evaluated, just for consistency)
                // This won't be called in normal use since _sinc_deriv is internal
                Expr::mul_expr(
                    Expr::derivative(
                        Expr::func_multi_from_arcs("_sinc_deriv", vec![Arc::clone(&u)]),
                        "x",
                        1,
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "lambertw",
            arity: 1..=1,
            eval: |args| crate::math::eval_lambert_w(args[0]),
            derivative: |args, arg_primes| {
                // d/dx W(u) = W(u) / (u(1+W(u))) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let w = Expr::func_multi_from_arcs_symbol(get_symbol(&LAMBERTW), vec![u.clone()]);
                Expr::mul_expr(
                    Expr::div_expr(
                        w.clone(),
                        Expr::mul_from_arcs(vec![
                            u.clone(),
                            Arc::new(Expr::add_expr(Expr::number(1.0), w)),
                        ]),
                    ),
                    u_prime,
                )
            },
        },
        // Elliptic Integrals
        FunctionDefinition {
            name: "elliptic_k",
            arity: 1..=1,
            eval: |args| crate::math::eval_elliptic_k(args[0]),
            derivative: |args, arg_primes| {
                // d/dk K(k) = (E(k)/(k(1-k^2)) - K(k)/k) * k'
                let k = args[0].clone();
                let k_prime = arg_primes[0].clone();
                let big_k = Expr::func_multi_from_arcs("elliptic_k", vec![k.clone()]);
                let big_e = Expr::func_multi_from_arcs("elliptic_e", vec![k.clone()]);

                // term1 = E(k) / (k * (1 - k^2))
                let term1 = Expr::div_expr(
                    big_e,
                    Expr::mul_from_arcs(vec![
                        k.clone(),
                        Arc::new(Expr::sub_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(k.clone(), Arc::new(Expr::number(2.0))),
                        )),
                    ]),
                );
                // term2 = K(k) / k
                let term2 = Expr::div_from_arcs(Arc::new(big_k), k.clone());

                Expr::mul_expr(Expr::sub_expr(term1, term2), k_prime)
            },
        },
        FunctionDefinition {
            name: "elliptic_e",
            arity: 1..=1,
            eval: |args| crate::math::eval_elliptic_e(args[0]),
            derivative: |args, arg_primes| {
                // d/dk E(k) = (E(k) - K(k)) / k * k'
                let k = args[0].clone();
                let k_prime = arg_primes[0].clone();
                let big_k =
                    Expr::func_multi_from_arcs_symbol(get_symbol(&ELLIPTIC_K), vec![k.clone()]);
                let big_e =
                    Expr::func_multi_from_arcs_symbol(get_symbol(&ELLIPTIC_E), vec![k.clone()]);

                Expr::mul_expr(
                    Expr::div_from_arcs(Arc::new(Expr::sub_expr(big_e, big_k)), k),
                    k_prime,
                )
            },
        },
        // Other Special Functions
        FunctionDefinition {
            name: "zeta",
            arity: 1..=1,
            eval: |args| crate::math::eval_zeta_deriv(0, args[0]),
            derivative: |args, arg_primes| {
                // d/ds zeta(s) = zeta_deriv(1, s) * s'
                let s = args[0].clone();
                let s_prime = arg_primes[0].clone();
                // We use "zeta_deriv" with order 1
                let zp = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&ZETA_DERIV),
                    vec![Arc::new(Expr::number(1.0)), s],
                );
                Expr::mul_expr(zp, s_prime)
            },
        },
        FunctionDefinition {
            name: "zeta_deriv",
            arity: 2..=2,
            eval: |args| crate::math::eval_zeta_deriv(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                // d/ds zeta_deriv(n, s) = zeta_deriv(n+1, s) * s'
                // Assuming n is constant
                let n = args[0].clone();
                let s = args[1].clone();
                let s_prime = arg_primes[1].clone();

                let n_plus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(1.0))]);
                let next_deriv =
                    Expr::func_multi_from_arcs("zeta_deriv", vec![Arc::new(n_plus_1), s]);

                Expr::mul_expr(next_deriv, s_prime)
            },
        },
        FunctionDefinition {
            name: "hermite",
            arity: 2..=2,
            eval: |args| crate::math::eval_hermite(args[0].round() as i32, args[1]),
            derivative: |args, arg_primes| {
                // d/dx H_n(x) = 2n H_{n-1}(x) * x'
                // We assume n is constant w.r.t differentiation variable
                let n = args[0].clone();
                let x = args[1].clone();
                let x_prime = arg_primes[1].clone();

                let two_n = Expr::mul_from_arcs(vec![Arc::new(Expr::number(2.0)), n.clone()]);
                let n_minus_1 = Expr::sum_from_arcs(vec![n.clone(), Arc::new(Expr::number(-1.0))]);
                let h_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&HERMITE),
                    vec![Arc::new(n_minus_1), x],
                );

                Expr::mul_expr(Expr::mul_expr(two_n, h_prev), x_prime)
            },
        },
        FunctionDefinition {
            name: "assoc_legendre",
            arity: 3..=3,
            eval: |args| {
                crate::math::eval_assoc_legendre(
                    args[0].round() as i32,
                    args[1].round() as i32,
                    args[2],
                )
            },
            derivative: |args, arg_primes| {
                // d/dx P_l^m(x) = (l*x*P_l^m(x) - (l+m)*P_{l-1}^m(x)) / (x^2 - 1) * x'
                let l = args[0].clone();
                let m = args[1].clone();
                let x = args[2].clone();
                let x_prime = arg_primes[2].clone();

                let term1 = Expr::mul_expr(
                    Expr::mul_from_arcs(vec![l.clone(), x.clone()]),
                    Expr::func_multi_from_arcs_symbol(
                        get_symbol(&ASSOC_LEGENDRE),
                        vec![l.clone(), m.clone(), x.clone()],
                    ),
                );

                let l_plus_m = Expr::sum_from_arcs(vec![l.clone(), m.clone()]);
                let l_minus_1 = Expr::sum_from_arcs(vec![l.clone(), Arc::new(Expr::number(-1.0))]);
                let term2 = Expr::mul_expr(
                    l_plus_m,
                    Expr::func_multi_from_arcs_symbol(
                        get_symbol(&ASSOC_LEGENDRE),
                        vec![Arc::new(l_minus_1), m.clone(), x.clone()],
                    ),
                );

                let numerator = Expr::sub_expr(term1, term2);
                let denominator = Expr::sub_expr(
                    Expr::pow_from_arcs(x.clone(), Arc::new(Expr::number(2.0))),
                    Expr::number(1.0),
                );

                Expr::mul_expr(Expr::div_expr(numerator, denominator), x_prime)
            },
        },
        FunctionDefinition {
            name: "spherical_harmonic",
            arity: 4..=4,
            eval: |args| {
                crate::math::eval_spherical_harmonic(
                    args[0].round() as i32,
                    args[1].round() as i32,
                    args[2],
                    args[3],
                )
            },
            derivative: |args, arg_primes| {
                let l = args[0].clone();
                let m = args[1].clone();
                let theta = args[2].clone();
                let phi = args[3].clone();

                let theta_prime = arg_primes[2].clone();
                let phi_prime = arg_primes[3].clone();

                // Y chain rule
                let y_expr = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&SPHERICAL_HARMONIC),
                    vec![l.clone(), m.clone(), theta.clone(), phi.clone()],
                );

                // 1. d/dpsi contribution: -m * tan(m*phi) * Y * phi'
                let m_phi = Expr::mul_from_arcs(vec![m.clone(), phi.clone()]);
                let term_phi_part = Expr::mul_from_arcs(vec![
                    Arc::new(Expr::mul_from_arcs(vec![
                        Arc::new(Expr::number(-1.0)),
                        m.clone(),
                    ])),
                    Arc::new(Expr::func_symbol(get_symbol(&TAN), m_phi)),
                    Arc::new(y_expr.clone()),
                ]);
                let term_phi = Expr::mul_expr(term_phi_part, phi_prime);

                // 2. d/dtheta contribution:
                // derived as: Y * [ l*cot(theta) - (l+m)/sin(theta) * (P_{l-1}/P) ] * theta'
                let cos_theta =
                    Expr::func_multi_from_arcs_symbol(get_symbol(&COS), vec![theta.clone()]);
                let sin_theta =
                    Expr::func_multi_from_arcs_symbol(get_symbol(&SIN), vec![theta.clone()]);
                let l_minus_1 = Expr::sum_from_arcs(vec![l.clone(), Arc::new(Expr::number(-1.0))]);

                let p_expr = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&ASSOC_LEGENDRE),
                    vec![l.clone(), m.clone(), Arc::new(cos_theta.clone())],
                );
                let p_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&ASSOC_LEGENDRE),
                    vec![Arc::new(l_minus_1), m.clone(), Arc::new(cos_theta)],
                );

                let term_a = Expr::mul_expr(
                    y_expr.clone(),
                    Expr::mul_from_arcs(vec![
                        l.clone(),
                        Arc::new(Expr::func_multi_from_arcs_symbol(
                            get_symbol(&COT),
                            vec![theta.clone()],
                        )),
                    ]),
                );

                let l_plus_m = Expr::sum_from_arcs(vec![l.clone(), m.clone()]);
                let term_b = Expr::mul_expr(
                    Expr::div_expr(Expr::mul_expr(y_expr.clone(), l_plus_m), sin_theta),
                    Expr::div_expr(p_prev, p_expr),
                );

                let term_theta = Expr::mul_expr(Expr::sub_expr(term_a, term_b), theta_prime);

                Expr::add_expr(term_theta, term_phi)
            },
        },
        // Aliases and Test Helpers
        FunctionDefinition {
            name: "tetragamma",
            arity: 1..=1,
            eval: |args| crate::math::eval_tetragamma(args[0]),
            derivative: |args, arg_primes| {
                let u = args[0].clone();
                let u_prime = arg_primes[0].clone();
                let deriv = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&POLYGAMMA),
                    vec![Arc::new(Expr::number(3.0)), u],
                );
                Expr::mul_expr(deriv, u_prime)
            },
        },
        FunctionDefinition {
            name: "ynm",
            arity: 4..=4,
            eval: |args| {
                crate::math::eval_spherical_harmonic(
                    args[0].round() as i32,
                    args[1].round() as i32,
                    args[2],
                    args[3],
                )
            },
            derivative: |args, arg_primes| {
                let l = args[0].clone();
                let m = args[1].clone();
                let theta = args[2].clone();
                let phi = args[3].clone();

                let theta_prime = arg_primes[2].clone();
                let phi_prime = arg_primes[3].clone();

                let y_expr = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&YNM),
                    vec![l.clone(), m.clone(), theta.clone(), phi.clone()],
                );

                // d/dphi
                let m_phi = Expr::mul_from_arcs(vec![m.clone(), phi.clone()]);
                let term_phi = Expr::mul_expr(
                    Expr::mul_from_arcs(vec![
                        Arc::new(Expr::mul_from_arcs(vec![
                            Arc::new(Expr::number(-1.0)),
                            m.clone(),
                        ])),
                        Arc::new(Expr::func_multi_from_arcs("tan", vec![Arc::new(m_phi)])),
                        Arc::new(y_expr.clone()),
                    ]),
                    phi_prime,
                );

                // d/dtheta
                let cos_theta =
                    Expr::func_multi_from_arcs_symbol(get_symbol(&COS), vec![theta.clone()]);
                let sin_theta =
                    Expr::func_multi_from_arcs_symbol(get_symbol(&SIN), vec![theta.clone()]);
                let l_minus_1 = Expr::sum_from_arcs(vec![l.clone(), Arc::new(Expr::number(-1.0))]);

                let p_expr = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&ASSOC_LEGENDRE),
                    vec![l.clone(), m.clone(), Arc::new(cos_theta.clone())],
                );
                let p_prev = Expr::func_multi_from_arcs_symbol(
                    get_symbol(&ASSOC_LEGENDRE),
                    vec![Arc::new(l_minus_1), m.clone(), Arc::new(cos_theta)],
                );

                let term_a = Expr::mul_expr(
                    y_expr.clone(),
                    Expr::mul_from_arcs(vec![
                        l.clone(),
                        Arc::new(Expr::func_multi_from_arcs("cot", vec![theta.clone()])),
                    ]),
                );
                let l_plus_m = Expr::sum_from_arcs(vec![l.clone(), m.clone()]);
                let term_b = Expr::mul_expr(
                    Expr::div_expr(Expr::mul_expr(y_expr.clone(), l_plus_m), sin_theta),
                    Expr::div_expr(p_prev, p_expr),
                );

                let term_theta = Expr::mul_expr(Expr::sub_expr(term_a, term_b), theta_prime);

                Expr::add_expr(term_theta, term_phi)
            },
        },
        FunctionDefinition {
            name: "exp_polar",
            arity: 1..=1,
            eval: |args| Some(crate::math::eval_exp_polar(args[0])),
            derivative: |args, arg_primes| {
                let x = args[0].clone();
                let x_prime = arg_primes[0].clone();
                // d/dx exp_polar(x) = exp_polar(x)
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(&EXP_POLAR), vec![x]),
                    x_prime,
                )
            },
        },
        // Rounding functions - derivatives are 0 almost everywhere (piecewise constant)
        FunctionDefinition {
            name: "floor",
            arity: 1..=1,
            eval: |args| Some(args[0].floor()),
            derivative: |_, _| {
                // d/dx floor(x) = 0 almost everywhere (discontinuous at integers)
                Expr::number(0.0)
            },
        },
        FunctionDefinition {
            name: "ceil",
            arity: 1..=1,
            eval: |args| Some(args[0].ceil()),
            derivative: |_, _| {
                // d/dx ceil(x) = 0 almost everywhere (discontinuous at integers)
                Expr::number(0.0)
            },
        },
        FunctionDefinition {
            name: "round",
            arity: 1..=1,
            eval: |args| Some(args[0].round()),
            derivative: |_, _| {
                // d/dx round(x) = 0 almost everywhere (discontinuous at half-integers)
                Expr::number(0.0)
            },
        },
    ]
}
