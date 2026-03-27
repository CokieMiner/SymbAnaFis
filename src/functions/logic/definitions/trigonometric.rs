use super::FunctionDefinition;
use crate::Expr;
use crate::core::known_symbols::{KS, get_symbol};
use std::sync::Arc;

#[allow(clippy::too_many_lines, reason = "Static function definition list")]
pub fn get_definitions() -> Vec<FunctionDefinition> {
    vec![
        // Trigonometric
        FunctionDefinition {
            name: "sin",
            arity: 1..=1,
            eval: |args| args[0].sin(),
            derivative: |args, arg_primes| {
                // d/dx sin(u) = cos(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::func_multi_from_arcs_symbol(get_symbol(KS.cos), vec![u]),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cos",
            arity: 1..=1,
            eval: |args| args[0].cos(),
            derivative: |args, arg_primes| {
                // d/dx cos(u) = -sin(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::func_multi_from_arcs_symbol(
                        get_symbol(KS.sin),
                        vec![u],
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "tan",
            arity: 1..=1,
            eval: |args| args[0].tan(),
            derivative: |args, arg_primes| {
                // d/dx tan(u) = sec^2(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::pow(
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.sec), vec![u]),
                        Expr::number(2.0),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "cot",
            arity: 1..=1,
            eval: |args| 1.0 / args[0].tan(),
            derivative: |args, arg_primes| {
                // d/dx cot(u) = -csc^2(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::pow(
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.csc), vec![u]),
                        Expr::number(2.0),
                    )),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "sec",
            arity: 1..=1,
            eval: |args| 1.0 / args[0].cos(),
            derivative: |args, arg_primes| {
                // d/dx sec(u) = sec(u)tan(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.sec), vec![Arc::clone(&u)]),
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.tan), vec![u]),
                    ),
                    u_prime,
                )
            },
        },
        FunctionDefinition {
            name: "csc",
            arity: 1..=1,
            eval: |args| 1.0 / args[0].sin(),
            derivative: |args, arg_primes| {
                // d/dx csc(u) = -csc(u)cot(u) * u'
                let u = Arc::clone(&args[0]);
                let u_prime = arg_primes[0].clone();
                Expr::mul_expr(
                    Expr::negate(Expr::mul_expr(
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.csc), vec![Arc::clone(&u)]),
                        Expr::func_multi_from_arcs_symbol(get_symbol(KS.cot), vec![u]),
                    )),
                    u_prime,
                )
            },
        },
    ]
}
