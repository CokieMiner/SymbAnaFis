use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Calculus Applications");
    println!("================================");

    println!("\n1. RELATED RATES - Ladder Sliding Problem");
    println!("----------------------------------------");

    // Ladder leaning against wall: x² + y² = L²
    // Differentiate implicitly: 2x dx/dt + 2y dy/dt = 0
    let constraint = "x^2 + y^2";
    let rate_equation = diff(constraint.to_string(), "t".to_string(), None, None).unwrap();

    println!("Constraint: x² + y² = L²");
    println!("Rate:      2x dx/dt + 2y dy/dt = {}", rate_equation);

    println!("\n2. OPTIMIZATION - Maximum Volume Box");
    println!("-----------------------------------");

    // Box volume: V = x * y * z, with constraint x + 2y + 2z = constant
    let volume = "x * y * z";
    let volume_grad_x = diff(
        volume.to_string(),
        "x".to_string(),
        Some(&["y".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let volume_grad_y = diff(
        volume.to_string(),
        "y".to_string(),
        Some(&["x".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let volume_grad_z = diff(
        volume.to_string(),
        "z".to_string(),
        Some(&["x".to_string(), "y".to_string()]),
        None,
    )
    .unwrap();

    println!("Volume: V = {}", volume);
    println!("∂V/∂x = {}", volume_grad_x);
    println!("∂V/∂y = {}", volume_grad_y);
    println!("∂V/∂z = {}", volume_grad_z);

    println!("\n3. TAYLOR SERIES - Approximation");
    println!("------------------------------");

    // f(x) ≈ f(a) + f'(a)(x-a) + f''(a)(x-a)²/2! + f'''(a)(x-a)³/3!
    let function = "sin(x)";
    let f1 = diff(function.to_string(), "x".to_string(), None, None).unwrap();
    let f2 = diff("cos(x)".to_string(), "x".to_string(), None, None).unwrap();

    println!("Function: f(x) = {}", function);
    println!("f(a) = sin(a)");
    println!("f'(a) = {}", f1.replace("x", "a"));
    println!("f''(a) = {}", f2.replace("x", "a"));
    println!("f'''(a) = -{}", f1.replace("x", "a")); // Third derivative of sin(x) is -cos(x)

    println!("\n4. DIFFERENTIAL EQUATIONS - Homogeneous Solution");
    println!("----------------------------------------------");

    // Second-order DE: y'' + p(x)y' + q(x)y = 0
    // For y = e^(rx), we get: r²e^(rx) + p(x)re^(rx) + q(x)e^(rx) = 0
    let trial_solution = "exp(r * x)";
    let y_prime = diff(
        trial_solution.to_string(),
        "x".to_string(),
        Some(&["r".to_string()]),
        None,
    )
    .unwrap();
    let y_double_prime = diff(
        y_prime.clone(),
        "x".to_string(),
        Some(&["r".to_string()]),
        None,
    )
    .unwrap();

    println!("Trial solution: y = {}", trial_solution);
    println!("y' = {}", y_prime);
    println!("y'' = {}", y_double_prime);

    println!("\n5. VECTOR CALCULUS - Gradient");
    println!("----------------------------");

    // Scalar field: f(x,y,z) = x² + y² + z²
    let scalar_field = "x^2 + y^2 + z^2";
    let grad_x = diff(
        scalar_field.to_string(),
        "x".to_string(),
        Some(&["y".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let grad_y = diff(
        scalar_field.to_string(),
        "y".to_string(),
        Some(&["x".to_string(), "z".to_string()]),
        None,
    )
    .unwrap();
    let grad_z = diff(
        scalar_field.to_string(),
        "z".to_string(),
        Some(&["x".to_string(), "y".to_string()]),
        None,
    )
    .unwrap();

    println!("Scalar field: f(x,y,z) = {}", scalar_field);
    println!("∇f = ({}, {}, {})", grad_x, grad_y, grad_z);

    println!("\n6. LINEAR APPROXIMATION - Error Analysis");
    println!("--------------------------------------");

    // f(x) ≈ f(a) + f'(a)(x-a)
    let function = "sqrt(x)";
    let approximation_point = "4"; // a = 4, f(4) = 2
    let derivative = diff(function.to_string(), "x".to_string(), None, None).unwrap();

    println!("Function: f(x) = {}", function);
    println!(
        "Linear approximation at x = {}: f(x) ≈ f({}) + f'({})(x - {})",
        approximation_point, approximation_point, approximation_point, approximation_point
    );
    println!(
        "f'({}) = {}",
        approximation_point,
        derivative.replace("x", approximation_point)
    );

    println!("\n7. MEAN VALUE THEOREM - Rolle's Theorem");
    println!("-------------------------------------");

    // If f(a) = f(b) and f is continuous and differentiable on [a,b],
    // then there exists c in (a,b) such that f'(c) = 0
    let function = "x^3 - 3*x";
    let derivative = diff(function.to_string(), "x".to_string(), None, None).unwrap();

    println!("Function: f(x) = {}", function);
    println!("f'(x) = {}", derivative);
    println!(
        "Critical points where f'(x) = 0: solve {}",
        derivative.replace("x", "c")
    );

    println!("\n8. L'HÔPITAL'S RULE - Indeterminate Forms");
    println!("----------------------------------------");

    // 0/0 form: lim(x→0) sin(x)/x = 1
    let numerator = "sin(x)";
    let denominator = "x";
    let num_deriv = diff(numerator.to_string(), "x".to_string(), None, None).unwrap();
    let den_deriv = diff(denominator.to_string(), "x".to_string(), None, None).unwrap();

    println!("Limit: lim(x→0) {}/{}", numerator, denominator);
    println!("L'Hôpital: lim(x→0) {}/{} = {}", num_deriv, den_deriv, "1");

    println!("\n9. ARC LENGTH - Parametric Curves");
    println!("--------------------------------");

    // Arc length: L = ∫√[(dx/dt)² + (dy/dt)²] dt
    let x_parametric = "cos(t)";
    let y_parametric = "sin(t)"; // Circle: x=cos(t), y=sin(t)
    let dx_dt = diff(x_parametric.to_string(), "t".to_string(), None, None).unwrap();
    let dy_dt = diff(y_parametric.to_string(), "t".to_string(), None, None).unwrap();

    println!(
        "Parametric curve: x(t) = {}, y(t) = {}",
        x_parametric, y_parametric
    );
    println!("dx/dt = {}", dx_dt);
    println!("dy/dt = {}", dy_dt);
    println!("Arc length element: ds = √[({})² + ({})²] dt", dx_dt, dy_dt);

    println!("\n10. SURFACE AREA - Solids of Revolution");
    println!("-------------------------------------");

    // Surface area: S = ∫ 2πy √[1 + (dy/dx)²] dx
    let curve = "x^2"; // y = x²
    let dy_dx = diff(curve.to_string(), "x".to_string(), None, None).unwrap();

    println!("Curve: y = {}", curve);
    println!("dy/dx = {}", dy_dx);
    println!("Surface area element: dS = 2πy √[1 + ({})²] dx", dy_dx);
}
