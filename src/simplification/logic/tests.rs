#[allow(
    clippy::unwrap_used,
    clippy::items_after_statements,
    reason = "Standard test relaxations"
)]
mod rule_registry_tests {
    use super::super::engine::global_registry;
    use super::super::rules::{ExprKind, RuleCategory};

    #[test]
    fn test_rule_registry_loads_all_categories() {
        let registry = global_registry();

        let mut category_counts = std::collections::HashMap::new();
        for rule in &registry.rules {
            *category_counts.entry(rule.category()).or_insert(0) += 1;
        }

        assert!(category_counts.contains_key(&RuleCategory::Numeric));
        assert!(category_counts.contains_key(&RuleCategory::Algebraic));
        assert!(category_counts.contains_key(&RuleCategory::Trigonometric));
        assert!(category_counts.contains_key(&RuleCategory::Exponential));
        assert!(category_counts.contains_key(&RuleCategory::Root));
        assert!(category_counts.contains_key(&RuleCategory::Hyperbolic));

        assert!(
            registry.rules.len() >= 100,
            "Expected at least 100 rules, got {}",
            registry.rules.len()
        );
    }

    #[test]
    fn test_rules_sorted_by_priority() {
        let registry = global_registry();

        for i in 0..(registry.rules.len() - 1) {
            let current_priority = registry.rules[i].priority();
            let next_priority = registry.rules[i + 1].priority();
            assert!(
                current_priority >= next_priority,
                "Rules should be sorted by priority descending. Rule '{}' (priority {}) comes before '{}' (priority {})",
                registry.rules[i].name(),
                current_priority,
                registry.rules[i + 1].name(),
                next_priority
            );
        }
    }

    #[test]
    fn test_rule_names_are_unique() {
        let registry = global_registry();
        let mut seen_names = std::collections::HashSet::new();

        for rule in &registry.rules {
            let name = rule.name();
            assert!(
                seen_names.insert(name),
                "Duplicate rule name found: '{name}'"
            );
        }
    }

    #[test]
    fn test_priority_ranges_follow_convention() {
        let registry = global_registry();

        for rule in &registry.rules {
            let priority = rule.priority();
            assert!(
                (1..=100).contains(&priority),
                "Rule '{}' has priority {} outside expected range [1, 100]",
                rule.name(),
                priority
            );
        }
    }

    #[test]
    fn test_kind_indexing_works() {
        let registry = global_registry();

        let sum_rules = registry.get_rules_for_kind(ExprKind::Sum);
        let product_rules = registry.get_rules_for_kind(ExprKind::Product);
        let pow_rules = registry.get_rules_for_kind(ExprKind::Pow);

        assert!(!sum_rules.is_empty(), "Sum should have rules");
        assert!(!product_rules.is_empty(), "Product should have rules");
        assert!(!pow_rules.is_empty(), "Pow should have rules");
        assert!(sum_rules.len() < registry.rules.len());
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::items_after_statements,
    reason = "Standard test relaxations"
)]
mod debug_factoring_logic_tests {
    use super::super::helpers;
    use crate::{Expr, ExprKind};

    #[test]
    fn debug_perfect_square_logic() {
        let x = Expr::symbol("x");
        let term1 = Expr::product(vec![Expr::number(4.0), Expr::pow(x, Expr::number(2.0))]);
        let term2 = Expr::product(vec![Expr::number(4.0), Expr::symbol("x")]);
        let term3 = Expr::number(1.0);

        let expr = Expr::sum(vec![term1, term2, term3]);

        let expr_terms: Vec<Expr> = match &expr.kind {
            ExprKind::Sum(ts) if ts.len() == 3 => ts.iter().map(|t| (**t).clone()).collect(),
            ExprKind::Sum(ts) if ts.len() == 2 => {
                let mut flat = Vec::new();
                for t in ts {
                    if let ExprKind::Poly(poly) = &t.kind {
                        flat.extend(poly.to_expr_terms());
                    } else {
                        flat.push((**t).clone());
                    }
                }
                if flat.len() != 3 {
                    return;
                }
                flat
            }
            ExprKind::Sum(_) => return,
            ExprKind::Poly(poly) => {
                assert!(!poly.terms().is_empty(), "Expected at least 1 term in Poly");
                return;
            }
            _ => vec![expr.clone()],
        };

        for term in &expr_terms {
            let _unused = helpers::extract_coeff(term);
        }

        let mut square_terms = Vec::new();
        let mut linear_terms = Vec::new();
        let mut constants = Vec::new();

        for term in &expr_terms {
            match &term.kind {
                ExprKind::Pow(base, exp) => {
                    if let ExprKind::Number(n) = &exp.kind
                        && (*n - 2.0).abs() < 1e-10
                    {
                        square_terms.push((1.0, base.as_ref().clone()));
                        continue;
                    }
                    linear_terms.push((1.0, term.clone(), Expr::number(1.0)));
                }
                ExprKind::Number(n) => {
                    constants.push(*n);
                }
                ExprKind::Product(_) => {
                    let (coeff, non_numeric) = extract_coeff_and_factors_debug(term);

                    if non_numeric.len() == 1 {
                        if let ExprKind::Pow(base, exp) = &non_numeric[0].kind
                            && let ExprKind::Number(n) = &exp.kind
                            && (*n - 2.0).abs() < 1e-10
                        {
                            square_terms.push((coeff, base.as_ref().clone()));
                            continue;
                        }
                        linear_terms.push((coeff, non_numeric[0].clone(), Expr::number(1.0)));
                    } else if non_numeric.len() == 2 {
                        linear_terms.push((coeff, non_numeric[0].clone(), non_numeric[1].clone()));
                    }
                }
                _ => {
                    linear_terms.push((1.0, term.clone(), Expr::number(1.0)));
                }
            }
        }

        assert_eq!(square_terms.len(), 1, "Expected 1 square term");
        assert_eq!(linear_terms.len(), 1, "Expected 1 linear term");
        assert_eq!(constants.len(), 1, "Expected 1 constant");
    }

    fn extract_coeff_and_factors_debug(term: &Expr) -> (f64, Vec<Expr>) {
        let factors: Vec<Expr> = match &term.kind {
            ExprKind::Product(fs) => fs.iter().map(|f| (**f).clone()).collect(),
            _ => vec![term.clone()],
        };
        let mut coeff = 1.0;
        let mut non_numeric: Vec<Expr> = Vec::new();

        for f in factors {
            if let ExprKind::Number(n) = &f.kind {
                coeff *= n;
            } else {
                non_numeric.push(f);
            }
        }
        (coeff, non_numeric)
    }
}
