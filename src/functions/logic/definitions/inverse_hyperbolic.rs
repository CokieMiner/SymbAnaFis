use super::super::registry::FunctionDefinition;
use crate::Expr;
use crate::core::known_symbols as ks;
use std::sync::Arc;

#[allow(clippy::too_many_lines, reason = "Static function definition list")]
pub fn get_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Inverse Hyperbolic
        FunctionDefinition {
            name: "asinh",
            arity: 1..=1,
            eval: |args| args[0].asinh(),
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
                                Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
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
            eval: |args| args[0].acosh(),
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
                                Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
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
            eval: |args| args[0].atanh(),
            derivative: |args, arg_primes| {
                // d/dx atanh(u) = u' / (1 - u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::sub_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "acoth",
            arity: 1..=1,
            eval: |args| 0.5 * ((args[0] + 1.0) / (args[0] - 1.0)).ln(),
            derivative: |args, arg_primes| {
                // d/dx acoth(u) = u' / (1 - u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::sub_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "asech",
            arity: 1..=1,
            eval: |args| (1.0_f64 / args[0]).acosh(),
            derivative: |args, arg_primes| {
                // d/dx asech(u) = -u' / (u * sqrt(1 - u^2)) = -1/(u*sqrt(1-u^2)) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            Arc::clone(&u),
                            Arc::new(Expr::func_symbol(
                                ks::get_symbol(ks::KS.sqrt),
                                Expr::sub_expr(
                                    Expr::number(1.0),
                                    Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
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
                    f64::NAN
                } else {
                    (1.0_f64 / args[0]).asinh()
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
                                ks::get_symbol(ks::KS.abs),
                                vec![Arc::clone(&u)],
                            )),
                            Arc::new(Expr::func_symbol(
                                ks::get_symbol(ks::KS.sqrt),
                                Expr::sub_expr(
                                    Expr::number(1.0),
                                    Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
                                ),
                            )),
                        ]),
                    )),
                    u_prime,
                )
            },
        },
    ]
}
