// Differentiation engine - applies calculus rules (PHASE 2 ENHANCED)
use crate::Expr;
use std::collections::HashSet;

impl Expr {
    /// Differentiate this expression with respect to a variable
    ///
    /// # Arguments
    /// * `var` - Variable to differentiate with respect to
    /// * `fixed_vars` - Set of variable names that are constants
    pub fn derive(&self, var: &str, fixed_vars: &HashSet<String>) -> Expr {
        match self {
            // Base cases
            Expr::Number(_) => Expr::Number(0.0),

            Expr::Symbol(name) => {
                // Check if this is a derivative notation like ∂^n_f(args)/∂_x^n
                if name.starts_with("∂^") && name.contains("/∂_") {
                    // Parse the derivative notation
                    // Format: ∂^order_func(args)/∂_var^order
                    let parts: Vec<&str> = name.split("/∂_").collect();
                    if parts.len() == 2 {
                        let left = parts[0]; // ∂^order_func(args)
                        let right = parts[1]; // var^order

                        // Extract order from right side
                        if let Some(order_str) = right.split('^').nth(1) {
                            if let Ok(current_order) = order_str.parse::<i32>() {
                                let new_order = current_order + 1;

                                // Extract func and args from left side
                                if let Some(func_part) = left.strip_prefix("∂^") {
                                    if let Some(order_end) = func_part.find('_') {
                                        let func_name = &func_part[order_end + 1..];
                                        // Reconstruct with new order
                                        let new_symbol = format!("∂^{}_{}/∂_{}^{}", new_order, func_name, var, new_order);
                                        return Expr::Symbol(new_symbol);
                                    }
                                }
                            }
                        }
                    }
                }

                // Standard symbol differentiation
                if name == var && !fixed_vars.contains(name) {
                    Expr::Number(1.0)
                } else {
                    Expr::Number(0.0)
                }
            }

            // Function call
            Expr::FunctionCall { name, args } => {
                // Helper to get the first argument and its derivative for single-arg functions
                let get_single_arg = || {
                    if args.len() != 1 {
                        // For now, panic or handle gracefully.
                        // Since we are in a library, maybe we should return an error expression or 0?
                        // But existing logic assumes validity.
                        if args.is_empty() {
                            panic!("Function {} expects at least 1 argument", name);
                        }
                    }
                    (&args[0], args[0].derive(var, fixed_vars))
                };

                // Check if this is a built-in function
                match name.as_str() {
                    "sin" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[sin(u)] = cos(u) * u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    "cos" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[cos(u)] = -sin(u) * u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sin".to_string(),
                                    args: vec![content.clone()],
                                }),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "ln" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[ln(u)] = (1/u) * u' = u^(-1) * u'
                        Expr::Mul(
                            Box::new(Expr::Pow(
                                Box::new(content.clone()),
                                Box::new(Expr::Number(-1.0)),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "exp" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[exp(u)] = exp(u) * u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "exp".to_string(),
                                args: vec![content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    // NEW: Hyperbolic functions
                    "sinh" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[sinh(u)] = cosh(u) * u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "cosh".to_string(),
                                args: vec![content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    "cosh" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[cosh(u)] = sinh(u) * u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "sinh".to_string(),
                                args: vec![content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    "tanh" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[tanh(u)] = (1 - tanh^2(u)) * u'
                        Expr::Mul(
                            Box::new(Expr::Sub(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(Expr::FunctionCall {
                                        name: "tanh".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    // NEW TIER 1: Trig functions
                    "tan" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[tan(u)] = sec²(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Pow(
                                Box::new(Expr::FunctionCall {
                                    name: "sec".to_string(),
                                    args: vec![content.clone()],
                                }),
                                Box::new(Expr::Number(2.0)),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "cot" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[cot(u)] = -csc²(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(Expr::FunctionCall {
                                        name: "csc".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "sec" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[sec(u)] = sec(u) · tan(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::FunctionCall {
                                    name: "sec".to_string(),
                                    args: vec![content.clone()],
                                }),
                                Box::new(Expr::FunctionCall {
                                    name: "tan".to_string(),
                                    args: vec![content.clone()],
                                }),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "csc" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[csc(u)] = -csc(u) · cot(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::Mul(
                                    Box::new(Expr::FunctionCall {
                                        name: "csc".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                    Box::new(Expr::FunctionCall {
                                        name: "cot".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    // NEW TIER 1: Inverse Trig
                    "asin" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[asin(u)] = u' / √(1-u²)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::FunctionCall {
                                name: "sqrt".to_string(),
                                args: vec![Expr::Sub(
                                    Box::new(Expr::Number(1.0)),
                                    Box::new(Expr::Pow(
                                        Box::new(content.clone()),
                                        Box::new(Expr::Number(2.0)),
                                    )),
                                )],
                            }),
                        )
                    }

                    "acos" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[acos(u)] = -u' / √(1-u²)
                        Expr::Div(
                            Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(u_prime))),
                            Box::new(Expr::FunctionCall {
                                name: "sqrt".to_string(),
                                args: vec![Expr::Sub(
                                    Box::new(Expr::Number(1.0)),
                                    Box::new(Expr::Pow(
                                        Box::new(content.clone()),
                                        Box::new(Expr::Number(2.0)),
                                    )),
                                )],
                            }),
                        )
                    }

                    "atan" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[atan(u)] = u' / (1 + u²)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Add(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                        )
                    }

                    "acot" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[acot(u)] = -u' / (1 + u²)
                        Expr::Div(
                            Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(u_prime))),
                            Box::new(Expr::Add(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                        )
                    }

                    "asec" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[asec(u)] = u' / (|u| · √(u²-1))
                        // Simplified: u' / (u · √(u²-1))
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Mul(
                                Box::new(content.clone()),
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Sub(
                                        Box::new(Expr::Pow(
                                            Box::new(content.clone()),
                                            Box::new(Expr::Number(2.0)),
                                        )),
                                        Box::new(Expr::Number(1.0)),
                                    )],
                                }),
                            )),
                        )
                    }

                    "acsc" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[acsc(u)] = -u' / (|u| · √(u²-1))
                        // Simplified: -u' / (u · √(u²-1))
                        Expr::Div(
                            Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(u_prime))),
                            Box::new(Expr::Mul(
                                Box::new(content.clone()),
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Sub(
                                        Box::new(Expr::Pow(
                                            Box::new(content.clone()),
                                            Box::new(Expr::Number(2.0)),
                                        )),
                                        Box::new(Expr::Number(1.0)),
                                    )],
                                }),
                            )),
                        )
                    }

                    // NEW TIER 1: Roots
                    "sqrt" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[√u] = u' / (2√u)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(2.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![content.clone()],
                                }),
                            )),
                        )
                    }

                    "cbrt" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[∛u] = u' / (3 · ∛(u²))
                        // = u' / (3 · u^(2/3))
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(3.0)),
                                Box::new(Expr::Pow(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Div(
                                        Box::new(Expr::Number(2.0)),
                                        Box::new(Expr::Number(3.0)),
                                    )),
                                )),
                            )),
                        )
                    }

                    // NEW TIER 2: Remaining hyperbolics
                    "coth" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[coth(u)] = -csch²(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(Expr::FunctionCall {
                                        name: "csch".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "sech" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[sech(u)] = -sech(u) · tanh(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::Mul(
                                    Box::new(Expr::FunctionCall {
                                        name: "sech".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                    Box::new(Expr::FunctionCall {
                                        name: "tanh".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "csch" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[csch(u)] = -csch(u) · coth(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::Mul(
                                    Box::new(Expr::FunctionCall {
                                        name: "csch".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                    Box::new(Expr::FunctionCall {
                                        name: "coth".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    // NEW TIER 2: Inverse Hyperbolic
                    "asinh" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[asinh(u)] = u' / √(u² + 1)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::FunctionCall {
                                name: "sqrt".to_string(),
                                args: vec![Expr::Add(
                                    Box::new(Expr::Pow(
                                        Box::new(content.clone()),
                                        Box::new(Expr::Number(2.0)),
                                    )),
                                    Box::new(Expr::Number(1.0)),
                                )],
                            }),
                        )
                    }

                    "acosh" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[acosh(u)] = u' / √(u² - 1)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::FunctionCall {
                                name: "sqrt".to_string(),
                                args: vec![Expr::Sub(
                                    Box::new(Expr::Pow(
                                        Box::new(content.clone()),
                                        Box::new(Expr::Number(2.0)),
                                    )),
                                    Box::new(Expr::Number(1.0)),
                                )],
                            }),
                        )
                    }

                    "atanh" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[atanh(u)] = u' / (1 - u²)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Sub(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                        )
                    }

                    "acoth" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[acoth(u)] = u' / (1 - u²)
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Sub(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Pow(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                        )
                    }

                    "asech" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[asech(u)] = -u' / (u · √(1 - u²))
                        Expr::Div(
                            Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(u_prime))),
                            Box::new(Expr::Mul(
                                Box::new(content.clone()),
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Sub(
                                        Box::new(Expr::Number(1.0)),
                                        Box::new(Expr::Pow(
                                            Box::new(content.clone()),
                                            Box::new(Expr::Number(2.0)),
                                        )),
                                    )],
                                }),
                            )),
                        )
                    }

                    "acsch" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[acsch(u)] = -u' / (|u| · √(1 + u²))
                        // Simplified: -u' / (u · √(1 + u²))
                        Expr::Div(
                            Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(u_prime))),
                            Box::new(Expr::Mul(
                                Box::new(content.clone()),
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Add(
                                        Box::new(Expr::Number(1.0)),
                                        Box::new(Expr::Pow(
                                            Box::new(content.clone()),
                                            Box::new(Expr::Number(2.0)),
                                        )),
                                    )],
                                }),
                            )),
                        )
                    }

                    // NEW TIER 2: Log variants
                    "log10" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[log10(u)] = u' / (u · ln(10))
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Mul(
                                Box::new(content.clone()),
                                Box::new(Expr::FunctionCall {
                                    name: "ln".to_string(),
                                    args: vec![Expr::Number(10.0)],
                                }),
                            )),
                        )
                    }

                    "log2" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[log2(u)] = u' / (u · ln(2))
                        Expr::Div(
                            Box::new(u_prime),
                            Box::new(Expr::Mul(
                                Box::new(content.clone()),
                                Box::new(Expr::FunctionCall {
                                    name: "ln".to_string(),
                                    args: vec![Expr::Number(2.0)],
                                }),
                            )),
                        )
                    }

                    "log" => {
                        // Treat log(u) as ln(u) for single argument
                        // d/dx[log(u)] = u' / u
                        // TODO: Support log(base, val)
                        let (content, u_prime) = get_single_arg();
                        Expr::Div(Box::new(u_prime), Box::new(content.clone()))
                    }

                    // NEW TIER 2/3: Special Functions
                    "sinc" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[sinc(u)] = ((cos(u)·u - sin(u)) / u²) · u'
                        // sinc(u) = sin(u)/u
                        Expr::Mul(
                            Box::new(Expr::Div(
                                Box::new(Expr::Sub(
                                    Box::new(Expr::Mul(
                                        Box::new(Expr::FunctionCall {
                                            name: "cos".to_string(),
                                            args: vec![content.clone()],
                                        }),
                                        Box::new(content.clone()),
                                    )),
                                    Box::new(Expr::FunctionCall {
                                        name: "sin".to_string(),
                                        args: vec![content.clone()],
                                    }),
                                )),
                                Box::new(Expr::Pow(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Number(2.0)),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "erf" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[erf(u)] = (2/√π) · e^(-u²) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Div(
                                    Box::new(Expr::Number(2.0)),
                                    Box::new(Expr::FunctionCall {
                                        name: "sqrt".to_string(),
                                        args: vec![Expr::Symbol("pi".to_string())], // Symbolic pi
                                    }),
                                )),
                                Box::new(Expr::FunctionCall {
                                    name: "exp".to_string(),
                                    args: vec![Expr::Mul(
                                        Box::new(Expr::Number(-1.0)),
                                        Box::new(Expr::Pow(
                                            Box::new(content.clone()),
                                            Box::new(Expr::Number(2.0)),
                                        )),
                                    )],
                                }),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "erfc" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[erfc(u)] = -(2/√π) · e^(-u²) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::Div(
                                    Box::new(Expr::Number(-2.0)),
                                    Box::new(Expr::FunctionCall {
                                        name: "sqrt".to_string(),
                                        args: vec![Expr::Symbol("pi".to_string())], // Symbolic pi
                                    }),
                                )),
                                Box::new(Expr::FunctionCall {
                                    name: "exp".to_string(),
                                    args: vec![Expr::Mul(
                                        Box::new(Expr::Number(-1.0)),
                                        Box::new(Expr::Pow(
                                            Box::new(content.clone()),
                                            Box::new(Expr::Number(2.0)),
                                        )),
                                    )],
                                }),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "gamma" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[gamma(u)] = gamma(u) · digamma(u) · u'
                        Expr::Mul(
                            Box::new(Expr::Mul(
                                Box::new(Expr::FunctionCall {
                                    name: "gamma".to_string(),
                                    args: vec![content.clone()],
                                }),
                                Box::new(Expr::FunctionCall {
                                    name: "digamma".to_string(),
                                    args: vec![content.clone()],
                                }),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    "digamma" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[digamma(u)] = trigamma(u) · u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "trigamma".to_string(),
                                args: vec![content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    "trigamma" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[trigamma(u)] = tetragamma(u) · u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "tetragamma".to_string(),
                                args: vec![content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    "tetragamma" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[tetragamma(u)] = polygamma(3, u) · u'
                        Expr::Mul(
                            Box::new(Expr::FunctionCall {
                                name: "polygamma".to_string(),
                                args: vec![Expr::Number(3.0), content.clone()],
                            }),
                            Box::new(u_prime),
                        )
                    }

                    "polygamma" => {
                        if args.len() != 2 {
                            return Expr::Symbol(format!(
                                "∂_polygamma({})/∂_{}",
                                args.iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                var
                            ));
                        }
                        let n = &args[0];
                        let x = &args[1];
                        let x_prime = x.derive(var, fixed_vars);

                        // d/dx polygamma(n, x) = polygamma(n+1, x)
                        let n_plus_1 = Expr::Add(Box::new(n.clone()), Box::new(Expr::Number(1.0)));
                        let derivative = Expr::FunctionCall {
                            name: "polygamma".to_string(),
                            args: vec![n_plus_1, x.clone()],
                        };

                        Expr::Mul(Box::new(derivative), Box::new(x_prime))
                    }

                    "LambertW" => {
                        let (content, u_prime) = get_single_arg();
                        // d/dx[W(u)] = W(u) / (u · (1 + W(u))) · u'
                        let w = Expr::FunctionCall {
                            name: "LambertW".to_string(),
                            args: vec![content.clone()],
                        };
                        Expr::Mul(
                            Box::new(Expr::Div(
                                Box::new(w.clone()),
                                Box::new(Expr::Mul(
                                    Box::new(content.clone()),
                                    Box::new(Expr::Add(Box::new(Expr::Number(1.0)), Box::new(w))),
                                )),
                            )),
                            Box::new(u_prime),
                        )
                    }

                    // Multi-argument functions
                    "beta" => {
                        if args.len() != 2 {
                            return Expr::Symbol(format!(
                                "∂_beta({})/∂_{}",
                                args.iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                var
                            ));
                        }
                        let a = &args[0];
                        let b = &args[1];
                        let a_prime = a.derive(var, fixed_vars);
                        let b_prime = b.derive(var, fixed_vars);

                        let beta_ab = Expr::FunctionCall {
                            name: "beta".to_string(),
                            args: vec![a.clone(), b.clone()],
                        };

                        let mut terms = Vec::new();

                        // ∂beta/∂a term
                        if !matches!(a_prime, Expr::Number(0.0)) {
                            let partial_a = Expr::Mul(
                                Box::new(beta_ab.clone()),
                                Box::new(Expr::Sub(
                                    Box::new(Expr::FunctionCall {
                                        name: "digamma".to_string(),
                                        args: vec![a.clone()],
                                    }),
                                    Box::new(Expr::FunctionCall {
                                        name: "digamma".to_string(),
                                        args: vec![Expr::Add(
                                            Box::new(a.clone()),
                                            Box::new(b.clone()),
                                        )],
                                    }),
                                )),
                            );
                            terms.push(Expr::Mul(Box::new(partial_a), Box::new(a_prime)));
                        }

                        // ∂beta/∂b term
                        if !matches!(b_prime, Expr::Number(0.0)) {
                            let partial_b = Expr::Mul(
                                Box::new(beta_ab.clone()),
                                Box::new(Expr::Sub(
                                    Box::new(Expr::FunctionCall {
                                        name: "digamma".to_string(),
                                        args: vec![b.clone()],
                                    }),
                                    Box::new(Expr::FunctionCall {
                                        name: "digamma".to_string(),
                                        args: vec![Expr::Add(
                                            Box::new(a.clone()),
                                            Box::new(b.clone()),
                                        )],
                                    }),
                                )),
                            );
                            terms.push(Expr::Mul(Box::new(partial_b), Box::new(b_prime)));
                        }

                        if terms.is_empty() {
                            Expr::Number(0.0)
                        } else if terms.len() == 1 {
                            terms.remove(0)
                        } else {
                            let mut result = terms.remove(0);
                            for term in terms {
                                result = Expr::Add(Box::new(result), Box::new(term));
                            }
                            result
                        }
                    }

                    "besselj" => {
                        if args.len() != 2 {
                            return Expr::Symbol(format!(
                                "∂_besselj({})/∂_{}",
                                args.iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                var
                            ));
                        }
                        let n = &args[0];
                        let x = &args[1];
                        let x_prime = x.derive(var, fixed_vars);

                        // d/dx J_n(x) = (1/2) * (J_{n-1}(x) - J_{n+1}(x))
                        let half =
                            Expr::Div(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(2.0)));
                        let n_minus_1 = Expr::Sub(Box::new(n.clone()), Box::new(Expr::Number(1.0)));
                        let n_plus_1 = Expr::Add(Box::new(n.clone()), Box::new(Expr::Number(1.0)));

                        let j_n_minus_1 = Expr::FunctionCall {
                            name: "besselj".to_string(),
                            args: vec![n_minus_1, x.clone()],
                        };
                        let j_n_plus_1 = Expr::FunctionCall {
                            name: "besselj".to_string(),
                            args: vec![n_plus_1, x.clone()],
                        };

                        let derivative = Expr::Mul(
                            Box::new(half),
                            Box::new(Expr::Sub(Box::new(j_n_minus_1), Box::new(j_n_plus_1))),
                        );

                        Expr::Mul(Box::new(derivative), Box::new(x_prime))
                    }

                    "bessely" => {
                        if args.len() != 2 {
                            return Expr::Symbol(format!(
                                "∂_bessely({})/∂_{}",
                                args.iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                var
                            ));
                        }
                        let n = &args[0];
                        let x = &args[1];
                        let x_prime = x.derive(var, fixed_vars);

                        // d/dx Y_n(x) = (1/2) * (Y_{n-1}(x) - Y_{n+1}(x))
                        let half =
                            Expr::Div(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(2.0)));
                        let n_minus_1 = Expr::Sub(Box::new(n.clone()), Box::new(Expr::Number(1.0)));
                        let n_plus_1 = Expr::Add(Box::new(n.clone()), Box::new(Expr::Number(1.0)));

                        let y_n_minus_1 = Expr::FunctionCall {
                            name: "bessely".to_string(),
                            args: vec![n_minus_1, x.clone()],
                        };
                        let y_n_plus_1 = Expr::FunctionCall {
                            name: "bessely".to_string(),
                            args: vec![n_plus_1, x.clone()],
                        };

                        let derivative = Expr::Mul(
                            Box::new(half),
                            Box::new(Expr::Sub(Box::new(y_n_minus_1), Box::new(y_n_plus_1))),
                        );

                        Expr::Mul(Box::new(derivative), Box::new(x_prime))
                    }

                    "besseli" => {
                        if args.len() != 2 {
                            return Expr::Symbol(format!(
                                "∂_besseli({})/∂_{}",
                                args.iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                var
                            ));
                        }
                        let n = &args[0];
                        let x = &args[1];
                        let x_prime = x.derive(var, fixed_vars);

                        // d/dx I_n(x) = (1/2) * (I_{n-1}(x) + I_{n+1}(x))
                        let half =
                            Expr::Div(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(2.0)));
                        let n_minus_1 = Expr::Sub(Box::new(n.clone()), Box::new(Expr::Number(1.0)));
                        let n_plus_1 = Expr::Add(Box::new(n.clone()), Box::new(Expr::Number(1.0)));

                        let i_n_minus_1 = Expr::FunctionCall {
                            name: "besseli".to_string(),
                            args: vec![n_minus_1, x.clone()],
                        };
                        let i_n_plus_1 = Expr::FunctionCall {
                            name: "besseli".to_string(),
                            args: vec![n_plus_1, x.clone()],
                        };

                        let derivative = Expr::Mul(
                            Box::new(half),
                            Box::new(Expr::Add(Box::new(i_n_minus_1), Box::new(i_n_plus_1))),
                        );

                        Expr::Mul(Box::new(derivative), Box::new(x_prime))
                    }

                    "besselk" => {
                        if args.len() != 2 {
                            return Expr::Symbol(format!(
                                "∂_besselk({})/∂_{}",
                                args.iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                var
                            ));
                        }
                        let n = &args[0];
                        let x = &args[1];
                        let x_prime = x.derive(var, fixed_vars);

                        // d/dx K_n(x) = (-1/2) * (K_{n-1}(x) + K_{n+1}(x))
                        let neg_half =
                            Expr::Div(Box::new(Expr::Number(-1.0)), Box::new(Expr::Number(2.0)));
                        let n_minus_1 = Expr::Sub(Box::new(n.clone()), Box::new(Expr::Number(1.0)));
                        let n_plus_1 = Expr::Add(Box::new(n.clone()), Box::new(Expr::Number(1.0)));

                        let k_n_minus_1 = Expr::FunctionCall {
                            name: "besselk".to_string(),
                            args: vec![n_minus_1, x.clone()],
                        };
                        let k_n_plus_1 = Expr::FunctionCall {
                            name: "besselk".to_string(),
                            args: vec![n_plus_1, x.clone()],
                        };

                        let derivative = Expr::Mul(
                            Box::new(neg_half),
                            Box::new(Expr::Add(Box::new(k_n_minus_1), Box::new(k_n_plus_1))),
                        );

                        Expr::Mul(Box::new(derivative), Box::new(x_prime))
                    }

                    _ => {
                        // Implicit/custom function - use multi-variable chain rule
                        // d/dx f(u1, u2, ...) = sum( (df/du_i) * (du_i/dx) )

                        let mut terms = Vec::new();

                        for (_i, arg) in args.iter().enumerate() {
                            let arg_prime = arg.derive(var, fixed_vars);

                            // Optimization: if derivative of argument is 0, skip this term
                            if let Expr::Number(n) = arg_prime
                                && n == 0.0
                            {
                                continue;
                            }

                            // Construct partial derivative symbol
                            // For single argument: ∂^1_f(u)/∂_x^1
                            // For multi argument: ∂^1_f(args)/∂_x^1

                            let partial_derivative = if args.len() == 1 {
                                Expr::Symbol(format!("∂^1_{}({})/∂_{}^1", name, args[0], var))
                            } else {
                                // Represent arguments as string for the symbol
                                let args_str = args
                                    .iter()
                                    .map(|a| a.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                Expr::Symbol(format!("∂^1_{}({})/∂_{}^1", name, args_str, var))
                            };

                            terms
                                .push(Expr::Mul(Box::new(partial_derivative), Box::new(arg_prime)));
                        }

                        if terms.is_empty() {
                            Expr::Number(0.0)
                        } else if terms.len() == 1 {
                            terms.remove(0)
                        } else {
                            // Sum up all terms
                            let mut result = terms.remove(0);
                            for term in terms {
                                result = Expr::Add(Box::new(result), Box::new(term));
                            }
                            result
                        }
                    }
                }
            }

            // Sum rule: (u + v)' = u' + v'
            Expr::Add(u, v) => Expr::Add(
                Box::new(u.derive(var, fixed_vars)),
                Box::new(v.derive(var, fixed_vars)),
            ),

            // Subtraction rule: (u - v)' = u' - v'
            Expr::Sub(u, v) => Expr::Sub(
                Box::new(u.derive(var, fixed_vars)),
                Box::new(v.derive(var, fixed_vars)),
            ),

            // Product rule: (u * v)' = u' * v + u * v'
            Expr::Mul(u, v) => {
                let u_prime = u.derive(var, fixed_vars);
                let v_prime = v.derive(var, fixed_vars);

                Expr::Add(
                    Box::new(Expr::Mul(Box::new(u_prime), v.clone())),
                    Box::new(Expr::Mul(u.clone(), Box::new(v_prime))),
                )
            }

            // Quotient rule: (u / v)' = (u' * v - u * v') / v^2
            Expr::Div(u, v) => {
                let u_prime = u.derive(var, fixed_vars);
                let v_prime = v.derive(var, fixed_vars);

                Expr::Div(
                    Box::new(Expr::Sub(
                        Box::new(Expr::Mul(Box::new(u_prime), v.clone())),
                        Box::new(Expr::Mul(u.clone(), Box::new(v_prime))),
                    )),
                    Box::new(Expr::Pow(v.clone(), Box::new(Expr::Number(2.0)))),
                )
            }

            // Power rule with LOGARITHMIC DIFFERENTIATION for variable exponents
            Expr::Pow(u, v) => {
                let v_prime = v.derive(var, fixed_vars);

                // Check if exponent is constant
                if matches!(v_prime, Expr::Number(0.0)) {
                    // Constant exponent - use standard power rule
                    // (u^n)' = n * u^(n-1) * u'
                    let u_prime = u.derive(var, fixed_vars);
                    let n_minus_1 = Expr::Add(v.clone(), Box::new(Expr::Number(-1.0)));

                    Expr::Mul(
                        v.clone(),
                        Box::new(Expr::Mul(
                            Box::new(Expr::Pow(u.clone(), Box::new(n_minus_1))),
                            Box::new(u_prime),
                        )),
                    )
                } else {
                    // Variable exponent - use LOGARITHMIC DIFFERENTIATION!
                    // d/dx[u^v] = u^v * (v' * ln(u) + v * u'/u)
                    let u_prime = u.derive(var, fixed_vars);

                    // Term 1: v' * ln(u)
                    let ln_u = Expr::FunctionCall {
                        name: "ln".to_string(),
                        args: vec![*u.clone()],
                    };
                    let term1 = Expr::Mul(Box::new(v_prime), Box::new(ln_u));

                    // Term 2: v * (u'/u)
                    let u_over_u_prime = Expr::Div(Box::new(u_prime), u.clone());
                    let term2 = Expr::Mul(v.clone(), Box::new(u_over_u_prime));

                    // Sum of terms
                    let sum = Expr::Add(Box::new(term1), Box::new(term2));

                    // Multiply by u^v
                    Expr::Mul(Box::new(Expr::Pow(u.clone(), v.clone())), Box::new(sum))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_sinh() {
        let expr = Expr::FunctionCall {
            name: "sinh".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        };
        let result = expr.derive("x", &HashSet::new());
        assert!(matches!(result, Expr::Mul(_, _)));
    }

    #[test]
    fn test_derive_subtraction() {
        // (x - 1)' = 1 - 0
        let expr = Expr::Sub(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(1.0)),
        );
        let result = expr.derive("x", &HashSet::new());
        assert!(matches!(result, Expr::Sub(_, _)));
    }

    #[test]
    fn test_derive_division() {
        // (x / 2)' = (1*2 - x*0) / 2^2
        let expr = Expr::Div(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0)),
        );
        let result = expr.derive("x", &HashSet::new());
        assert!(matches!(result, Expr::Div(_, _)));
    }

    #[test]
    fn test_logarithmic_differentiation() {
        // x^x should use logarithmic differentiation
        let expr = Expr::Pow(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Symbol("x".to_string())),
        );
        let result = expr.derive("x", &HashSet::new());
        // Result should be multiplication (complex expression)
        assert!(matches!(result, Expr::Mul(_, _)));
    }

    #[test]
    fn test_derivative_order_increment() {
        // Test that differentiating a derivative symbol increments the order
        let derivative_symbol = Expr::Symbol("∂^1_f(x)/∂_x^1".to_string());
        let result = derivative_symbol.derive("x", &HashSet::new());
        match result {
            Expr::Symbol(name) => assert_eq!(name, "∂^2_f(x)/∂_x^2"),
            _ => panic!("Expected symbol, got {:?}", result),
        }
    }

    #[test]
    fn test_derivative_order_increment_multi_digit() {
        // Test incrementing from 9 to 10 (single to double digit)
        let ninth_symbol = Expr::Symbol("∂^9_f(x)/∂_x^9".to_string());
        let result = ninth_symbol.derive("x", &HashSet::new());
        match result {
            Expr::Symbol(name) => assert_eq!(name, "∂^10_f(x)/∂_x^10"),
            _ => panic!("Expected symbol, got {:?}", result),
        }

        // Test incrementing from 99 to 100 (double to triple digit)
        let ninety_ninth_symbol = Expr::Symbol("∂^99_f(x)/∂_x^99".to_string());
        let result = ninety_ninth_symbol.derive("x", &HashSet::new());
        match result {
            Expr::Symbol(name) => assert_eq!(name, "∂^100_f(x)/∂_x^100"),
            _ => panic!("Expected symbol, got {:?}", result),
        }
    }
}
