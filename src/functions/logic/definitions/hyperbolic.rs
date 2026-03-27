use super::FunctionDefinition;
use crate::Expr;
use crate::core::known_symbols::{KS, get_symbol};
use std::sync::Arc;

#[allow(clippy::too_many_lines, reason = "Static function definition list")]
pub fn get_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Hyperbolic
        FunctionDefinition {
            name: "sinh",
            arity: 1..=1,
            eval: |args| args[0].sinh(),
            derivative: |args, arg_primes| {
                // d/dx sinh(u) = cosh(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(KS.cosh), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cosh",
            arity: 1..=1,
            eval: |args| args[0].cosh(),
            derivative: |args, arg_primes| {
                // d/dx cosh(u) = sinh(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(KS.sinh), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "tanh",
            arity: 1..=1,
            eval: |args| args[0].tanh(),
            derivative: |args, arg_primes| {
                // d/dx tanh(u) = (1 - tanh^2(u)) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::sub_expr(
                        Expr::number(1.0),
                        Expr::pow(
                            Expr::func_multi_from_arcs_symbol(get_symbol(KS.tanh), vec![u]),
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
            eval: |args| 1.0_f64 / args[0].tanh(),
            derivative: |args, arg_primes| {
                // d/dx coth(u) = -csch^2(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::pow(
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.csch), vec![u]),
                        Expr::number(2.0),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "sech",
            arity: 1..=1,
            eval: |args| 1.0_f64 / args[0].cosh(),
            derivative: |args, arg_primes| {
                // d/dx sech(u) = -sech(u)tanh(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(
                            get_symbol(KS.sech),
                            vec![Arc::clone(&u)],
                        ),
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.tanh), vec![u]),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "csch",
            arity: 1..=1,
            eval: |args| 1.0_f64 / args[0].sinh(),
            derivative: |args, arg_primes| {
                // d/dx csch(u) = -csch(u)coth(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(
                            get_symbol(KS.csch),
                            vec![Arc::clone(&u)],
                        ),
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.coth), vec![u]),
                    )),
                    u_prime,
                )
            },
        },
    ]
}
