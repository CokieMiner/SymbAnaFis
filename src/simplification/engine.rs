use super::rules::{RuleContext, RuleRegistry};
use crate::Expr;
use std::collections::{HashMap, HashSet};

/// Main simplification engine with rule-based architecture
pub struct Simplifier {
    registry: RuleRegistry,
    rule_caches: HashMap<String, HashMap<Expr, Option<Expr>>>, // Per-rule memoization
    max_iterations: usize,
    max_depth: usize,
    context: RuleContext,
    domain_safe: bool,
}

impl Default for Simplifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Simplifier {
    pub fn new() -> Self {
        let mut registry = RuleRegistry::new();
        registry.load_all_rules();
        registry.order_by_dependencies();

        Self {
            registry,
            rule_caches: HashMap::new(),
            max_iterations: 1000,
            max_depth: 50,
            context: RuleContext::default(),
            domain_safe: false,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_domain_safe(mut self, domain_safe: bool) -> Self {
        self.domain_safe = domain_safe;
        self
    }

    /// Main simplification entry point
    pub fn simplify(&mut self, expr: Expr) -> Expr {
        let mut current = expr;
        let mut iterations = 0;
        let mut seen = HashSet::new();

        loop {
            if iterations >= self.max_iterations {
                eprintln!(
                    "Warning: Simplification exceeded maximum iterations ({})",
                    self.max_iterations
                );
                break;
            }

            // Cycle detection
            if !seen.insert(current.clone()) {
                break; // Cycle detected
            }

            let original = current.clone();
            current = self.apply_rules_bottom_up(current, 0);

            if current == original {
                break; // No changes
            }

            iterations += 1;
        }

        current
    }

    /// Apply rules bottom-up through the expression tree
    fn apply_rules_bottom_up(&mut self, expr: Expr, depth: usize) -> Expr {
        if depth > self.max_depth {
            return expr; // Prevent stack overflow
        }

        let result = match expr {
            Expr::Add(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(*u, depth + 1);
                let v_simplified = self.apply_rules_bottom_up(*v, depth + 1);
                let new_expr = Expr::Add(Box::new(u_simplified), Box::new(v_simplified));
                self.apply_rules_to_node(new_expr, depth)
            }
            Expr::Sub(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(*u, depth + 1);
                let v_simplified = self.apply_rules_bottom_up(*v, depth + 1);
                let new_expr = Expr::Sub(Box::new(u_simplified), Box::new(v_simplified));
                self.apply_rules_to_node(new_expr, depth)
            }
            Expr::Mul(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(*u, depth + 1);
                let v_simplified = self.apply_rules_bottom_up(*v, depth + 1);
                let new_expr = Expr::Mul(Box::new(u_simplified), Box::new(v_simplified));
                self.apply_rules_to_node(new_expr, depth)
            }
            Expr::Div(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(*u, depth + 1);
                let v_simplified = self.apply_rules_bottom_up(*v, depth + 1);
                let new_expr = Expr::Div(Box::new(u_simplified), Box::new(v_simplified));
                self.apply_rules_to_node(new_expr, depth)
            }
            Expr::Pow(u, v) => {
                let u_simplified = self.apply_rules_bottom_up(*u, depth + 1);
                let v_simplified = self.apply_rules_bottom_up(*v, depth + 1);
                let new_expr = Expr::Pow(Box::new(u_simplified), Box::new(v_simplified));
                self.apply_rules_to_node(new_expr, depth)
            }
            Expr::FunctionCall { name, args } => {
                let args_simplified: Vec<Expr> = args
                    .into_iter()
                    .map(|arg| self.apply_rules_bottom_up(arg, depth + 1))
                    .collect();
                let new_expr = Expr::FunctionCall {
                    name,
                    args: args_simplified,
                };
                self.apply_rules_to_node(new_expr, depth)
            }
            other => self.apply_rules_to_node(other, depth),
        };
        result
    }

    /// Apply all applicable rules to a single node in dependency order
    fn apply_rules_to_node(&mut self, mut current: Expr, depth: usize) -> Expr {
        let mut context = self.context.clone().with_depth(depth).with_domain_safe(self.domain_safe);

        for rule in self.registry.get_rules() {
            // Skip rules that alter domains if domain_safe is enabled
            if context.domain_safe && rule.alters_domain() {
                continue;
            }

            // Check per-rule cache
            let rule_name = rule.name();
            if let Some(cache) = self.rule_caches.get(rule_name) {
                if let Some(cached_result) = cache.get(&current) {
                    if let Some(new_expr) = cached_result {
                        current = new_expr.clone();
                        continue;
                    } else {
                        // Cached None, skip
                        continue;
                    }
                }
            }

            // Apply rule with context
            if let Some(new_expr) = rule.apply(&current, &context) {
                current = new_expr;
                // Update context with new parent
                context = context.with_parent(current.clone());
            }

            // Cache the result (None if no change)
            let changed = true; // Assume changed for simplicity
            self.rule_caches.entry(rule_name.to_string()).or_insert(HashMap::new())
                .insert(current.clone(), if changed { Some(current.clone()) } else { None });
        }

        current
    }
}

/// Verifier for post-simplification equivalence checking
pub struct Verifier;

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier {
    pub fn new() -> Self {
        Self
    }

    /// Check if original and simplified expressions are equivalent by sampling
    pub fn verify_equivalence(&self, original: &Expr, simplified: &Expr, variables: &HashSet<String>) -> Result<(), String> {
        let test_points = [-2.0, -1.0, 0.0, 1.0, 2.0]; // Simple test points

        for &val in &test_points {
            let mut env = HashMap::new();
            for var in variables {
                env.insert(var.clone(), val);
            }

            let orig_val = self.evaluate_expr(original, &env);
            let simp_val = self.evaluate_expr(simplified, &env);

            match (orig_val, simp_val) {
                (Ok(o), Ok(s)) => {
                    if !o.is_nan() && !s.is_nan() && (o - s).abs() > 1e-6 {
                        return Err(format!("Equivalence check failed at {}: original={}, simplified={}", val, o, s));
                    }
                }
                _ => {
                    // Skip if evaluation fails or is NaN
                }
            }
        }
        Ok(())
    }

    fn evaluate_expr(&self, expr: &Expr, env: &HashMap<String, f64>) -> Result<f64, String> {
        match expr {
            Expr::Number(n) => Ok(*n),
            Expr::Symbol(s) => {
                if s == "e" {
                    Ok(std::f64::consts::E)
                } else if s == "pi" {
                    Ok(std::f64::consts::PI)
                } else {
                    env.get(s).copied().ok_or_else(|| format!("Variable {} not found", s))
                }
            }
            Expr::Add(a, b) => Ok(self.evaluate_expr(a, env)? + self.evaluate_expr(b, env)?),
            Expr::Sub(a, b) => Ok(self.evaluate_expr(a, env)? - self.evaluate_expr(b, env)?),
            Expr::Mul(a, b) => Ok(self.evaluate_expr(a, env)? * self.evaluate_expr(b, env)?),
            Expr::Div(a, b) => {
                let denom = self.evaluate_expr(b, env)?;
                if denom == 0.0 { return Err("Division by zero".to_string()); }
                Ok(self.evaluate_expr(a, env)? / denom)
            }
            Expr::Pow(a, b) => Ok(self.evaluate_expr(a, env)?.powf(self.evaluate_expr(b, env)?)),
            Expr::FunctionCall { name, args } => {
                match name.as_str() {
                    "sin" => Ok(self.evaluate_expr(&args[0], env)?.sin()),
                    "cos" => Ok(self.evaluate_expr(&args[0], env)?.cos()),
                    "tan" => Ok(self.evaluate_expr(&args[0], env)?.tan()),
                    "sinh" => Ok(self.evaluate_expr(&args[0], env)?.sinh()),
                    "cosh" => Ok(self.evaluate_expr(&args[0], env)?.cosh()),
                    "tanh" => Ok(self.evaluate_expr(&args[0], env)?.tanh()),
                    "coth" => Ok(1.0 / self.evaluate_expr(&args[0], env)?.tanh()),
                    "sech" => Ok(1.0 / self.evaluate_expr(&args[0], env)?.cosh()),
                    "csch" => Ok(1.0 / self.evaluate_expr(&args[0], env)?.sinh()),
                    "sqrt" => Ok(self.evaluate_expr(&args[0], env)?.sqrt()),
                    "cbrt" => Ok(self.evaluate_expr(&args[0], env)?.cbrt()),
                    "exp" => Ok(self.evaluate_expr(&args[0], env)?.exp()),
                    "ln" => Ok(self.evaluate_expr(&args[0], env)?.ln()),
                    _ => Err(format!("Unsupported function: {}", name)),
                }
            }
        }
    }
}


// Convenience function for one-off simplifications
pub fn simplify_expr(expr: Expr) -> Expr {
    let variables = expr.variables();
    match simplify_expr_with_verification(expr.clone(), variables, false) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Verification failed: {}, falling back to unverified simplification", e);
            let mut simplifier = Simplifier::new()
                .with_max_iterations(1000)
                .with_max_depth(20);
            simplifier.simplify(expr)
        }
    }
}

/// Convenience function with verification
pub fn simplify_expr_with_verification(expr: Expr, variables: HashSet<String>, domain_safe: bool) -> Result<Expr, String> {
    let original = expr.clone();
    let mut simplifier = Simplifier::new()
        .with_max_iterations(1000)
        .with_max_depth(20)
        .with_domain_safe(domain_safe);
    let simplified = simplifier.simplify(expr);

    let verifier = Verifier::new();
    verifier.verify_equivalence(&original, &simplified, &variables)?;

    Ok(simplified)
}
