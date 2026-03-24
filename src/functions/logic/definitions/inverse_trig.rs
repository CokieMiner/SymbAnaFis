use super::super::registry::FunctionDefinition;
use crate::Expr;
use crate::core::known_symbols as ks;
use std::sync::Arc;

#[allow(clippy::too_many_lines, reason = "Static function definition list")]
pub fn get_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Inverse Trigonometric
        FunctionDefinition {
            name: "asin",
            arity: 1..=1,
            eval: |args| args[0].asin(),
            derivative: |args, arg_primes| {
                // d/dx asin(u) = u' / sqrt(1 - u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::func_symbol(
                            ks::get_symbol(ks::KS.sqrt),
                            Expr::sub_expr(
                                Expr::number(1.0),
                                Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
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
            eval: |args| args[0].acos(),
            derivative: |args, arg_primes| {
                // d/dx acos(u) = -u' / sqrt(1 - u^2) = -1/sqrt(1-u^2) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::div_expr(
                        Expr::number(1.0),
                        Expr::func_symbol(
                            ks::get_symbol(ks::KS.sqrt),
                            Expr::sub_expr(
                                Expr::number(1.0),
                                Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
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
            eval: |args| args[0].atan(),
            derivative: |args, arg_primes| {
                // d/dx atan(u) = u' / (1 + u^2)
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::add_expr(
                            Expr::number(1.0),
                            Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "atan2",
            arity: 2..=2,
            eval: |args| args[0].atan2(args[1]),
            derivative: |args, arg_primes| {
                // d/dx atan2(y, x) = (x*y' - y*x') / (x^2 + y^2)
                let y = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let y_prime = arg_primes[0].clone();
                let x_prime = arg_primes[1].clone();

                let numerator = Expr::sub_expr(
                    Expr::mul_expr(Expr::unwrap_arc(Arc::clone(&x)), y_prime),
                    Expr::mul_expr(Expr::unwrap_arc(Arc::clone(&y)), x_prime),
                );
                let denominator = Expr::add_expr(
                    Expr::pow_from_arcs(x, Arc::new(Expr::number(2.0))),
                    Expr::pow_from_arcs(y, Arc::new(Expr::number(2.0))),
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
                    std::f64::consts::PI / 2.0
                } else if x > 0.0 {
                    (1.0_f64 / x).atan()
                } else {
                    (1.0_f64 / x).atan() + std::f64::consts::PI
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
                            Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
                        ),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "asec",
            arity: 1..=1,
            eval: |args| (1.0_f64 / args[0]).acos(),
            derivative: |args, arg_primes| {
                // d/dx asec(u) = u' / (|u| * sqrt(u^2 - 1))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_expr(
                            Expr::func_multi_from_arcs_symbol(
                                ks::get_symbol(ks::KS.abs),
                                vec![Arc::clone(&u)],
                            ),
                            Expr::func_symbol(
                                ks::get_symbol(ks::KS.sqrt),
                                Expr::sub_expr(
                                    Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
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
            eval: |args| (1.0_f64 / args[0]).asin(),
            derivative: |args, arg_primes| {
                // d/dx acsc(u) = -u' / (|u| * sqrt(u^2 - 1)) = -1/(|u|*sqrt(u^2-1)) * u'
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
                                    Expr::pow_from_arcs(u, Arc::new(Expr::number(2.0))),
                                    Expr::number(1.0),
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
