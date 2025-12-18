#[cfg(test)]
mod tests {
    use crate::{ExprKind, diff, parser};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_zeta_closure() {
        // d/ds zeta(s) -> zeta_deriv(1, s)
        // d^2/ds^2 zeta(s) -> zeta_deriv(2, s)
        let d1 = diff("zeta(s)", "s", None, None).unwrap();
        println!("d1 string: {}", d1);
        assert!(d1.contains("zeta_deriv"));

        // Differentiate result again
        let d2 = diff(&d1, "s", None, None).unwrap();
        println!("d2 string: {}", d2);

        // Evaluate d2 at s=3.0 (safe region)
        // zeta_deriv(2, 3.0) should be defined
        let fixed = HashSet::new();
        let custom = HashSet::new();

        // Actually, let's just create a simplified reproduction in the test
        let tokens_debug = crate::parser::parse(&d2, &fixed, &custom, None);
        if let Err(e) = &tokens_debug {
            println!("Parse error: {:?}", e);
        }
        let expr = parser::parse(&d2, &fixed, &custom, None).unwrap();
        let mut vars = HashMap::new();
        vars.insert("s", 3.0);

        let val = expr.evaluate(&vars);
        if let ExprKind::Number(n) = val.kind {
            assert!(n.is_finite());
        } else {
            panic!("Evaluation failed to produce a number: {:?}", val);
        }
    }

    #[test]
    fn test_gamma_polygamma_closure() {
        // gamma -> gamma*digamma -> ... -> polygamma
        let d1 = diff("gamma(x)", "x", None, None).unwrap();
        let d2 = diff(&d1, "x", None, None).unwrap();
        let d3 = diff(&d2, "x", None, None).unwrap();

        // Should eventually involve polygamma
        // d/dx digamma -> trigamma -> tetragamma -> polygamma(3, x)
        // d3 should contain polygamma or tetra/trigamma names
        assert!(
            d3.contains("polygamma") || d3.contains("tetragamma") || d3.contains("trigamma"),
            "Higher derivatives of gamma should stay within closed set of polygamma functions. Got: {}",
            d3
        );
    }
}
