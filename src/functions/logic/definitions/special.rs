use super::super::registry::FunctionDefinition;
use crate::Expr;
use crate::core::known_symbols as ks;
use std::sync::Arc;

#[allow(clippy::too_many_lines, reason = "Static function definition list")]
pub fn get_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Special Functions
        FunctionDefinition {
            name: "abs",
            arity: 1..=1,
            eval: |args| args[0].abs(),
            derivative: |args, arg_primes| {
                // d/dx |u| = signum(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(ks::get_symbol(ks::KS.signum), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "signum",
            arity: 1..=1,
            eval: |args| args[0].signum(),
            derivative: |_, _| {
                // d/dx signum(u) = 0 almost everywhere
                Expr::number(0.0)
            },
        },
        FunctionDefinition {
            name: "erf",
            arity: 1..=1,
            eval: |args| crate::math::eval_erf(args[0]),
            derivative: |args, arg_primes| {
                // d/dx erf(u) = (2/sqrt(pi)) * exp(-u^2) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let pi = Expr::symbol("pi");
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::div_expr(
                            Expr::number(2.0),
                            Expr::func_symbol(ks::get_symbol(ks::KS.sqrt), pi),
                        ),
                        Expr::func_symbol(
                            ks::get_symbol(ks::KS.exp),
                            Expr::negate(Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0)))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "erfc",
            arity: 1..=1,
            eval: |args| crate::math::eval_erfc(args[0]),
            derivative: |args, arg_primes| {
                // d/dx erfc(u) = -d/dx erf(u)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let pi = Expr::symbol("pi");
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::div_expr(
                            Expr::number(-2.0),
                            Expr::func_symbol(ks::get_symbol(ks::KS.sqrt), pi),
                        ),
                        Expr::func_symbol(
                            ks::get_symbol(ks::KS.exp),
                            Expr::negate(Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0)))),
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
                        Expr::func_multi_from_arcs_symbol(
                            ks::get_symbol(ks::KS.gamma),
                            vec![Arc::clone(&u)],
                        ),
                        Expr::func_multi_from_arcs_symbol(ks::get_symbol(ks::KS.digamma), vec![u]),
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
                    Expr::func_multi_from_arcs_symbol(ks::get_symbol(ks::KS.trigamma), vec![u]),
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
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(ks::get_symbol(ks::KS.tetragamma), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "lgamma",
            arity: 1..=1,
            eval: |args| crate::math::eval_lgamma(args[0]),
            derivative: |args, arg_primes| {
                // d/dx lgamma(u) = digamma(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(ks::get_symbol(ks::KS.digamma), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "beta",
            arity: 2..=2,
            eval: |args| crate::math::eval_beta(args[0], args[1]),
            derivative: |args, arg_primes| {
                let a = Arc::clone(&args[0]);
                let b = Arc::clone(&args[1]);
                let a_prime = arg_primes[0].clone();
                let b_prime = arg_primes[1].clone();

                let beta_ab = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.beta),
                    vec![Arc::clone(&a), Arc::clone(&b)],
                );
                let psi_a_plus_b = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.digamma),
                    vec![Arc::new(Expr::sum_from_arcs(vec![
                        Arc::clone(&a),
                        Arc::clone(&b),
                    ]))],
                );

                let term_a = Expr::mul_expr(
                    Expr::mul_expr(
                        beta_ab.clone(),
                        Expr::sub_expr(
                            Expr::func_multi_from_arcs_symbol(
                                ks::get_symbol(ks::KS.digamma),
                                vec![a],
                            ),
                            psi_a_plus_b.clone(),
                        ),
                    ),
                    a_prime,
                );

                let term_b = Expr::mul_expr(
                    Expr::mul_expr(
                        beta_ab,
                        Expr::sub_expr(
                            Expr::func_multi_from_arcs_symbol(
                                ks::get_symbol(ks::KS.digamma),
                                vec![b],
                            ),
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
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::bessel_j(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let x_prime = arg_primes[1].clone();

                let half = Expr::number(0.5);
                let n_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(1.0))]);

                let j_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.besselj),
                    vec![Arc::new(n_minus_1), Arc::clone(&x)],
                );
                let j_next = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.besselj),
                    vec![Arc::new(n_plus_1), Arc::clone(&x)],
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
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::bessel_y(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let x_prime = arg_primes[1].clone();

                let half = Expr::number(0.5);
                let n_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(1.0))]);

                let y_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.bessely),
                    vec![Arc::new(n_minus_1), Arc::clone(&x)],
                );
                let y_next = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.bessely),
                    vec![Arc::new(n_plus_1), Arc::clone(&x)],
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
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::bessel_i(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let x_prime = arg_primes[1].clone();

                let half = Expr::number(0.5);
                let n_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(1.0))]);

                let i_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.besseli),
                    vec![Arc::new(n_minus_1), Arc::clone(&x)],
                );
                let i_next = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.besseli),
                    vec![Arc::new(n_plus_1), Arc::clone(&x)],
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
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::bessel_k(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let x_prime = arg_primes[1].clone();

                let neg_half = Expr::number(-0.5);
                let n_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(-1.0))]);
                let n_plus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(1.0))]);

                let k_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.besselk),
                    vec![Arc::new(n_minus_1), Arc::clone(&x)],
                );
                let k_next = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.besselk),
                    vec![Arc::new(n_plus_1), Arc::clone(&x)],
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
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::eval_polygamma(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let x_prime = arg_primes[1].clone();

                let n_plus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(1.0))]);
                let derivative = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.polygamma),
                    vec![Arc::new(n_plus_1), Arc::clone(&x)],
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
                    1.0 - x * x / 6.0 + x.powi(4) / 120.0
                } else {
                    x.sin() / x
                }
            },
            derivative: |args, arg_primes| {
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let lhs = Expr::div_expr(
                    Expr::sub_expr(
                        Expr::mul_from_arcs(vec![
                            Arc::new(Expr::func_multi_from_arcs_symbol(
                                ks::get_symbol(ks::KS.cos),
                                vec![Arc::clone(&u)],
                            )),
                            Arc::clone(&u),
                        ]),
                        Expr::func_multi_from_arcs_symbol(
                            ks::get_symbol(ks::KS.sin),
                            vec![Arc::clone(&u)],
                        ),
                    ),
                    Expr::pow_from_arcs(Arc::clone(&u), Arc::new(Expr::number(2.0))),
                );
                Expr::mul_expr(lhs, u_prime)
            },
        },
        FunctionDefinition {
            name: "lambertw",
            arity: 1..=1,
            eval: |args| crate::math::eval_lambert_w(args[0]),
            derivative: |args, arg_primes| {
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let w = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.lambertw),
                    vec![Arc::clone(&u)],
                );
                Expr::mul_expr(
                    Expr::div_expr(
                        w.clone(),
                        Expr::mul_from_arcs(vec![
                            Arc::clone(&u),
                            Arc::new(Expr::add_expr(Expr::number(1.0), w)),
                        ]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "elliptic_k",
            arity: 1..=1,
            eval: |args| crate::math::eval_elliptic_k(args[0]),
            derivative: |args, arg_primes| {
                let k = Arc::clone(&args[0]);
                let k_prime = arg_primes[0].clone();
                let big_k = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.elliptic_k),
                    vec![Arc::clone(&k)],
                );
                let big_e = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.elliptic_e),
                    vec![Arc::clone(&k)],
                );

                let term1 = Expr::div_expr(
                    big_e,
                    Expr::mul_from_arcs(vec![
                        Arc::clone(&k),
                        Arc::new(Expr::sub_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(Arc::clone(&k), Arc::new(Expr::number(2.0))),
                        )),
                    ]),
                );
                let term2 = Expr::div_from_arcs(Arc::new(big_k), Arc::clone(&k));

                Expr::mul_expr(Expr::sub_expr(term1, term2), k_prime)
            },
        },
        FunctionDefinition {
            name: "elliptic_e",
            arity: 1..=1,
            eval: |args| crate::math::eval_elliptic_e(args[0]),
            derivative: |args, arg_primes| {
                let k = Arc::clone(&args[0]);
                let k_prime = arg_primes[0].clone();
                let big_k = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.elliptic_k),
                    vec![Arc::clone(&k)],
                );
                let big_e = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.elliptic_e),
                    vec![Arc::clone(&k)],
                );

                Expr::mul_expr(
                    Expr::div_from_arcs(Arc::new(Expr::sub_expr(big_e, big_k)), Arc::clone(&k)),
                    k_prime,
                )
            },
        },
        FunctionDefinition {
            name: "zeta",
            arity: 1..=1,
            eval: |args| crate::math::eval_zeta_deriv(0, args[0]),
            derivative: |args, arg_primes| {
                let s = Arc::clone(&args[0]);
                let s_prime = arg_primes[0].clone();
                let zp = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.zeta_deriv),
                    vec![Arc::new(Expr::number(1.0)), Arc::clone(&s)],
                );
                Expr::mul_expr(zp, s_prime)
            },
        },
        FunctionDefinition {
            name: "zeta_deriv",
            arity: 2..=2,
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::eval_zeta_deriv(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let s = Arc::clone(&args[1]);
                let s_prime = arg_primes[1].clone();

                let n_plus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(1.0))]);
                let next_deriv = Expr::func_multi_from_arcs(
                    "zeta_deriv",
                    vec![Arc::new(n_plus_1), Arc::clone(&s)],
                );

                Expr::mul_expr(next_deriv, s_prime)
            },
        },
        FunctionDefinition {
            name: "hermite",
            arity: 2..=2,
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let n = args[0].round() as i32;
                crate::math::eval_hermite(n, args[1])
            },
            derivative: |args, arg_primes| {
                let n = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let x_prime = arg_primes[1].clone();

                let two_n = Expr::mul_from_arcs(vec![Arc::new(Expr::number(2.0)), Arc::clone(&n)]);
                let n_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&n), Arc::new(Expr::number(-1.0))]);
                let h_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.hermite),
                    vec![Arc::new(n_minus_1), Arc::clone(&x)],
                );

                Expr::mul_expr(Expr::mul_expr(two_n, h_prev), x_prime)
            },
        },
        FunctionDefinition {
            name: "assoc_legendre",
            arity: 3..=3,
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let l = args[0].round() as i32;
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let m = args[1].round() as i32;
                crate::math::eval_assoc_legendre(l, m, args[2])
            },
            derivative: |args, arg_primes| {
                let l = Arc::clone(&args[0]);
                let m = Arc::clone(&args[1]);
                let x = Arc::clone(&args[2]);
                let x_prime = arg_primes[2].clone();

                let term1 = Expr::mul_expr(
                    Expr::mul_from_arcs(vec![Arc::clone(&l), Arc::clone(&x)]),
                    Expr::func_multi_from_arcs_symbol(
                        ks::get_symbol(ks::KS.assoc_legendre),
                        vec![Arc::clone(&l), Arc::clone(&m), Arc::clone(&x)],
                    ),
                );
                let l_plus_m = Expr::sum_from_arcs(vec![Arc::clone(&l), Arc::clone(&m)]);
                let l_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&l), Arc::new(Expr::number(-1.0))]);
                let term2 = Expr::mul_expr(
                    l_plus_m,
                    Expr::func_multi_from_arcs_symbol(
                        ks::get_symbol(ks::KS.assoc_legendre),
                        vec![Arc::new(l_minus_1), Arc::clone(&m), Arc::clone(&x)],
                    ),
                );

                let numerator = Expr::sub_expr(term1, term2);
                let denominator = Expr::sub_expr(
                    Expr::pow_from_arcs(Arc::clone(&x), Arc::new(Expr::number(2.0))),
                    Expr::number(1.0),
                );

                Expr::mul_expr(Expr::div_expr(numerator, denominator), x_prime)
            },
        },
        FunctionDefinition {
            name: "spherical_harmonic",
            arity: 4..=4,
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let l = args[0].round() as i32;
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let m = args[1].round() as i32;
                crate::math::eval_spherical_harmonic(l, m, args[2], args[3])
            },
            derivative: |args, arg_primes| {
                let l = Arc::clone(&args[0]);
                let m = Arc::clone(&args[1]);
                let theta = Arc::clone(&args[2]);
                let phi = Arc::clone(&args[3]);

                let theta_prime = arg_primes[2].clone();
                let phi_prime = arg_primes[3].clone();

                let y_expr = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.spherical_harmonic),
                    vec![
                        Arc::clone(&l),
                        Arc::clone(&m),
                        Arc::clone(&theta),
                        Arc::clone(&phi),
                    ],
                );

                let m_phi = Expr::mul_from_arcs(vec![Arc::clone(&m), phi]);
                let term_phi_part = Expr::mul_from_arcs(vec![
                    Arc::new(Expr::mul_from_arcs(vec![
                        Arc::new(Expr::number(-1.0)),
                        Arc::clone(&m),
                    ])),
                    Arc::new(Expr::func_symbol(ks::get_symbol(ks::KS.tan), m_phi)),
                    Arc::new(y_expr.clone()),
                ]);
                let term_phi = Expr::mul_expr(term_phi_part, phi_prime);

                let cos_theta = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.cos),
                    vec![Arc::clone(&theta)],
                );
                let sin_theta = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.sin),
                    vec![Arc::clone(&theta)],
                );
                let l_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&l), Arc::new(Expr::number(-1.0))]);

                let p_expr = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.assoc_legendre),
                    vec![Arc::clone(&l), Arc::clone(&m), Arc::new(cos_theta.clone())],
                );
                let p_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.assoc_legendre),
                    vec![Arc::new(l_minus_1), Arc::clone(&m), Arc::new(cos_theta)],
                );

                let term_a = Expr::mul_expr(
                    y_expr.clone(),
                    Expr::mul_from_arcs(vec![
                        Arc::clone(&l),
                        Arc::new(Expr::func_multi_from_arcs_symbol(
                            ks::get_symbol(ks::KS.cot),
                            vec![Arc::clone(&theta)],
                        )),
                    ]),
                );
                let l_plus_m = Expr::sum_from_arcs(vec![Arc::clone(&l), Arc::clone(&m)]);
                let term_b = Expr::mul_expr(
                    Expr::div_expr(Expr::mul_expr(y_expr, l_plus_m), sin_theta),
                    Expr::div_expr(p_prev, p_expr),
                );

                let term_theta = Expr::mul_expr(Expr::sub_expr(term_a, term_b), theta_prime);

                Expr::add_expr(term_theta, term_phi)
            },
        },
        FunctionDefinition {
            name: "tetragamma",
            arity: 1..=1,
            eval: |args| crate::math::eval_tetragamma(args[0]),
            derivative: |args, arg_primes| {
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                let deriv = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.polygamma),
                    vec![Arc::new(Expr::number(3.0)), Arc::clone(&u)],
                );
                Expr::mul_expr(deriv, u_prime)
            },
        },
        FunctionDefinition {
            name: "ynm",
            arity: 4..=4,
            eval: |args| {
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let l = args[0].round() as i32;
                #[allow(clippy::cast_possible_truncation, reason = "Casting to f64 indices")]
                let m = args[1].round() as i32;
                crate::math::eval_spherical_harmonic(l, m, args[2], args[3])
            },
            derivative: |args, arg_primes| {
                let l = Arc::clone(&args[0]);
                let m = Arc::clone(&args[1]);
                let theta = Arc::clone(&args[2]);
                let phi = Arc::clone(&args[3]);

                let theta_prime = arg_primes[2].clone();
                let phi_prime = arg_primes[3].clone();

                let y_expr = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.ynm),
                    vec![
                        Arc::clone(&l),
                        Arc::clone(&m),
                        Arc::clone(&theta),
                        Arc::clone(&phi),
                    ],
                );

                let m_phi = Expr::mul_from_arcs(vec![Arc::clone(&m), phi]);
                let term_phi = Expr::mul_expr(
                    Expr::mul_from_arcs(vec![
                        Arc::new(Expr::mul_from_arcs(vec![
                            Arc::new(Expr::number(-1.0)),
                            Arc::clone(&m),
                        ])),
                        Arc::new(Expr::func_multi_from_arcs("tan", vec![Arc::new(m_phi)])),
                        Arc::new(y_expr.clone()),
                    ]),
                    phi_prime,
                );

                let cos_theta = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.cos),
                    vec![Arc::clone(&theta)],
                );
                let sin_theta = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.sin),
                    vec![Arc::clone(&theta)],
                );
                let l_minus_1 =
                    Expr::sum_from_arcs(vec![Arc::clone(&l), Arc::new(Expr::number(-1.0))]);

                let cos_theta_arc = Arc::new(cos_theta);
                let p_expr = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.assoc_legendre),
                    vec![Arc::clone(&l), Arc::clone(&m), Arc::clone(&cos_theta_arc)],
                );
                let p_prev = Expr::func_multi_from_arcs_symbol(
                    ks::get_symbol(ks::KS.assoc_legendre),
                    vec![
                        Arc::new(l_minus_1),
                        Arc::clone(&m),
                        Arc::clone(&cos_theta_arc),
                    ],
                );

                let term_a = Expr::mul_expr(
                    y_expr.clone(),
                    Expr::mul_from_arcs(vec![
                        Arc::clone(&l),
                        Arc::new(Expr::func_multi_from_arcs("cot", vec![Arc::clone(&theta)])),
                    ]),
                );
                let l_plus_m = Expr::sum_from_arcs(vec![Arc::clone(&l), Arc::clone(&m)]);
                let term_b = Expr::mul_expr(
                    Expr::div_expr(Expr::mul_expr(y_expr, l_plus_m), sin_theta),
                    Expr::div_expr(p_prev, p_expr),
                );

                let term_theta = Expr::mul_expr(Expr::sub_expr(term_a, term_b), theta_prime);

                Expr::add_expr(term_theta, term_phi)
            },
        },
        FunctionDefinition {
            name: "exp_polar",
            arity: 1..=1,
            eval: |args| crate::math::eval_exp_polar(args[0]),
            derivative: |args, arg_primes| {
                let x = Arc::clone(&args[0]);
                let x_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(
                        ks::get_symbol(ks::KS.exp_polar),
                        vec![Arc::clone(&x)],
                    ),
                    x_prime,
                )
            },
        },
        FunctionDefinition {
            name: "floor",
            arity: 1..=1,
            eval: |args| args[0].floor(),
            derivative: |_, _| Expr::number(0.0),
        },
        FunctionDefinition {
            name: "ceil",
            arity: 1..=1,
            eval: |args| args[0].ceil(),
            derivative: |_, _| Expr::number(0.0),
        },
        FunctionDefinition {
            name: "round",
            arity: 1..=1,
            eval: |args| args[0].round(),
            derivative: |_, _| Expr::number(0.0),
        },
    ]
}
