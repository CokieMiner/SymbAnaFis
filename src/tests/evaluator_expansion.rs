#[cfg(test)]
mod tests {
    use crate::Expr;
    use crate::core::known_symbols as ks;
    use crate::evaluator::CompiledEvaluator;
    use crate::parser;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn parse_expr(s: &str) -> crate::Expr {
        parser::parse(s, &HashSet::new(), &HashSet::new(), None).unwrap()
    }

    #[test]
    fn test_spherical_harmonic() {
        // Y_0^0 = 1/2 * sqrt(1/pi)
        let expr_str = "spherical_harmonic(0, 0, 0, 0)";
        let expr = parse_expr(expr_str);

        let eval = CompiledEvaluator::compile_auto(&expr, None).unwrap();
        let result = eval.evaluate(&[]);

        let expected = 0.5 * (1.0 / std::f64::consts::PI).sqrt();
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_ynm_alias() {
        // ynm is alias for spherical_harmonic
        let expr_str = "ynm(0, 0, 0, 0)";
        let expr = parse_expr(expr_str);

        let eval = CompiledEvaluator::compile_auto(&expr, None).unwrap();
        let result = eval.evaluate(&[]);

        let expected = 0.5 * (1.0 / std::f64::consts::PI).sqrt();
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_exp_polar() {
        // exp_polar(x) = exp(x)
        let expr_str = "exp_polar(2.0)";
        let expr = parse_expr(expr_str);

        let eval = CompiledEvaluator::compile_auto(&expr, None).unwrap();
        let result = eval.evaluate(&[]);

        let expected = (2.0_f64).exp();
        assert!((result - expected).abs() < 1e-10);
    }
    #[test]
    fn test_atan2() {
        let evaluator = CompiledEvaluator::compile_auto(
            &Expr::func_multi_from_arcs_symbol(
                ks::get_symbol(ks::KS.atan2),
                vec![
                    Arc::new(Expr::number(1.0)), // y
                    Arc::new(Expr::number(0.0)), // x (y/x -> +infinity -> pi/2)
                ],
            ),
            None,
        )
        .unwrap();

        let result = evaluator.evaluate(&[]);
        assert!((result - std::f64::consts::PI / 2.0).abs() < 1e-10);
    }
}
