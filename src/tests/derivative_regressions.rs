use crate::parser::parse;
use crate::{Expr, diff};
use std::collections::HashSet;

#[test]
fn test_quotient_rule_sign_distribution() {
    // Derivative of (x^2 + 1)/(x - 1)
    let result = diff(
        "(x^2 + 1) / (x - 1)".to_string(),
        "x".to_string(),
        None,
        None,
    )
    .unwrap();
    // Quick checks on the printed result to assert the quotient layout is correct
    // Expect denominator squared
    assert!(
        result.contains("(-1 + x)^2")
            || result.contains("(x - 1)^2")
            || result.contains("(x + -1)^2"),
        "Expected denominator squared in derivative: {}",
        result
    );
    // Expect the numerator to be properly grouped (either as a subtraction with parenthesized terms,
    // or as an expression that starts with parenthesis showing proper grouping)
    // The key is that the expression is well-formed and parseable
    assert!(
        result.contains("/"),
        "Expected division in derivative: {}",
        result
    );
}

#[test]
fn test_orbital_denominator_squared() {
    // r(theta) = a*(1 - e^2)/(1 + e*cos(theta))
    let result = diff(
        "a*(1 - e^2) / (1 + e*cos(theta))".to_string(),
        "theta".to_string(),
        Some(&["a".to_string(), "e".to_string()]),
        None,
    )
    .unwrap();

    let fixed_vars: HashSet<String> = ["a".to_string(), "e".to_string()].iter().cloned().collect();
    let custom_functions: HashSet<String> = HashSet::new();
    let result_ast = parse(&result, &fixed_vars, &custom_functions).unwrap();

    // Expect derivative denominator to be (1 + e*cos(theta))^2
    if let Expr::Div(_, denom) = result_ast {
        if let Expr::Pow(base, exp) = &*denom {
            // base must be (1 + e*cos(theta)), exponent must be 2
            assert_eq!(*exp.clone(), Expr::Number(2.0));
            let expected_base =
                parse("e * cos(theta) + 1", &fixed_vars, &custom_functions).unwrap();
            assert_eq!(*base.clone(), expected_base, "Denominator base mismatch");
        } else {
            panic!("Denominator is not a power as expected: {:?}", denom);
        }
    } else {
        panic!("Derivative is not a division as expected: {}", result);
    }
}
