use super::FunctionDefinition;
use crate::core::Expr;
use crate::core::known_symbols::{KS, get_symbol};
use std::sync::Arc;

#[allow(clippy::too_many_lines, reason = "Static function definition list")]
pub fn get_exponential_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Roots / Exp / Log
        FunctionDefinition {
            name: "exp",
            arity: 1..=1,
            eval: |args| args[0].exp(),
            derivative: |args, arg_primes| {
                // d/dx exp(u) = exp(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(KS.exp), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "ln",
            arity: 1..=1,
            eval: |args| args[0].ln(),
            derivative: |args, arg_primes| {
                // d/dx ln(u) = u' / u
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::pow_from_arcs(u, Arc::new(Expr::number(-1.0))),
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
                // Exact comparison for base == 1.0 is mathematically intentional
                #[allow(clippy::float_cmp, reason = "Exact comparison for log base == 1.0")]
                let invalid = base <= 0.0 || base == 1.0 || x <= 0.0;
                if invalid { f64::NAN } else { x.log(base) }
            },
            derivative: |args, arg_primes| {
                // log_b(x) = ln(x) / ln(b)
                // d/dt log_b(x) = (1/(x*ln(b))) * x' - (ln(x)/(b*ln(b)^2)) * b'
                //               = x'/(x*ln(b)) - b'*ln(x)/(b*ln(b)^2)
                let b = Arc::clone(&args[0]);
                let x = Arc::clone(&args[1]);
                let b_prime = arg_primes[0].clone();
                let x_prime = arg_primes[1].clone();

                // Term 1: x' / (x * ln(b))
                let ln_b =
                    Expr::func_multi_from_arcs_symbol(get_symbol(KS.ln), vec![Arc::clone(&b)]);
                let term1 = Expr::div_expr(
                    x_prime,
                    Expr::mul_from_arcs(vec![Arc::clone(&x), Arc::new(ln_b.clone())]),
                );

                // Term 2: -b' * ln(x) / (b * ln(b)^2)
                let ln_x = Expr::func_multi_from_arcs_symbol(get_symbol(KS.ln), vec![x]);
                let ln_b_sq = Expr::pow(ln_b, Expr::number(2.0));
                let term2 = Expr::negate(Expr::div_expr(
                    Expr::mul_expr(b_prime, ln_x),
                    Expr::mul_from_arcs(vec![b, Arc::new(ln_b_sq)]),
                ));

                Expr::add_expr(term1, term2)
            },
        },
        FunctionDefinition {
            name: "log10",
            arity: 1..=1,
            eval: |args| args[0].log10(),
            derivative: |args, arg_primes| {
                // d/dx log10(u) = u' / (u * ln(10))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            u,
                            Arc::new(Expr::func_symbol(get_symbol(KS.ln), Expr::number(10.0))),
                        ]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "log2",
            arity: 1..=1,
            eval: |args| args[0].log2(),
            derivative: |args, arg_primes| {
                // d/dx log2(u) = u' / (u * ln(2))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_from_arcs(vec![
                            u,
                            Arc::new(Expr::func_symbol(get_symbol(KS.ln), Expr::number(2.0))),
                        ]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "sqrt",
            arity: 1..=1,
            eval: |args| args[0].sqrt(),
            derivative: |args, arg_primes| {
                // d/dx sqrt(u) = u' / (2 * sqrt(u))
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::div_expr(
                        Expr::number(1.0),
                        Expr::mul_expr(
                            Expr::number(2.0),
                            Expr::func_multi_from_arcs_symbol(get_symbol(KS.sqrt), vec![u]),
                        ),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cbrt",
            arity: 1..=1,
            eval: |args| args[0].cbrt(),
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
                                u,
                                Arc::new(Expr::div_expr(Expr::number(2.0), Expr::number(3.0))),
                            ),
                        ),
                    ),
                    u_prime,
                )
            },
        },
    ]
}
