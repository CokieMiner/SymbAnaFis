use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Physics & Engineering Applications");
    println!("==============================================");

    println!("\n1. KINEMATICS - Position, Velocity, Acceleration");
    println!("----------------------------------------------");

    // Position as function of time: x(t) = 5t² + 3t + 10
    let position = "5*t^2 + 3*t + 10";
    let velocity = diff(position.to_string(), "t".to_string(), None, None).unwrap();
    let acceleration = diff(velocity.clone(), "t".to_string(), None, None).unwrap();

    println!("Position:     x(t) = {}", position);
    println!("Velocity:     v(t) = dx/dt = {}", velocity);
    println!("Acceleration: a(t) = dv/dt = {}", acceleration);

    println!("\n2. ELECTRICITY - RC Circuit Voltage");
    println!("----------------------------------");

    // Voltage in RC circuit: V(t) = V₀ * e^(-t/(R*C))
    let voltage = "V0 * exp(-t / (R * C))";
    let current = diff(
        voltage.to_string(),
        "t".to_string(),
        Some(&["V0".to_string(), "R".to_string(), "C".to_string()]),
        None,
    )
    .unwrap();

    println!("Voltage: V(t) = {}", voltage);
    println!("Current: I(t) = dV/dt = {}", current);

    println!("\n3. THERMODYNAMICS - Heat Conduction");
    println!("----------------------------------");

    // Temperature in 1D heat equation solution: T(x,t) = T₀ * erf(x/(2*sqrt(α*t)))
    let temperature = "T0 * erf(x / (2 * sqrt(alpha * t)))";
    let heat_flux = diff(
        temperature.to_string(),
        "x".to_string(),
        Some(&["T0".to_string(), "alpha".to_string(), "t".to_string()]),
        None,
    )
    .unwrap();

    println!("Temperature: T(x,t) = {}", temperature);
    println!("Heat flux:   q(x,t) = -k*dT/dx = -k*{}", heat_flux);

    println!("\n4. QUANTUM MECHANICS - Wave Function");
    println!("-----------------------------------");

    // Gaussian wave packet: ψ(x,t) = (1/√(σ√π)) * exp(-x²/(4σ²)) * exp(i*k*x - i*E*t/ℏ)
    // For simplicity, just the real part
    let wave_function = "exp(-x^2 / (4 * sigma^2)) / sqrt(sigma * sqrt(pi))";
    let probability_density = format!("({})^2", wave_function);
    let prob_derivative = diff(
        probability_density.clone(),
        "x".to_string(),
        Some(&["sigma".to_string()]),
        None,
    )
    .unwrap();

    println!("Wave function: ψ(x) = {}", wave_function);
    println!("Probability:   |ψ|² = {}", probability_density);
    println!("d|ψ|²/dx = {}", prob_derivative);

    println!("\n5. FLUID DYNAMICS - Velocity Profile (Poiseuille Flow)");
    println!("----------------------------------------------------");

    // Velocity in pipe flow: u(r) = (ΔP/(4μL)) * (R² - r²)
    let velocity = "deltaP / (4 * mu * L) * (R^2 - r^2)";
    let shear_stress = diff(
        velocity.to_string(),
        "r".to_string(),
        Some(&[
            "deltaP".to_string(),
            "mu".to_string(),
            "L".to_string(),
            "R".to_string(),
        ]),
        None,
    )
    .unwrap();

    println!("Velocity:    u(r) = {}", velocity);
    println!("Shear rate: du/dr = {}", shear_stress);

    println!("\n6. OPTICS - Lens Formula");
    println!("-----------------------");

    // Lens equation: 1/f = 1/do + 1/di
    // Differentiate to find magnification sensitivity
    let magnification = "di / do"; // M = hi/ho = -di/do for thin lenses
    let mag_sensitivity = diff(
        magnification.to_string(),
        "do".to_string(),
        Some(&["di".to_string()]),
        None,
    )
    .unwrap();

    println!("Magnification: M = {}", magnification);
    println!("dM/ddo = {}", mag_sensitivity);

    println!("\n7. CONTROL SYSTEMS - Transfer Function");
    println!("------------------------------------");

    // Second-order system: G(s) = ωₙ² / (s² + 2ζωₙs + ωₙ²)
    // For simplicity, just the denominator derivative
    let denominator = "s^2 + 2*zeta*omega_n*s + omega_n^2";
    let denom_derivative = diff(
        denominator.to_string(),
        "s".to_string(),
        Some(&["zeta".to_string(), "omega_n".to_string()]),
        None,
    )
    .unwrap();

    println!("Characteristic equation: {}", denominator);
    println!("d(denom)/ds = {}", denom_derivative);

    println!("\n8. STATISTICS - Normal Distribution");
    println!("---------------------------------");

    // Normal PDF: f(x) = (1/√(2πσ²)) * exp(-(x-μ)²/(2σ²))
    let normal_pdf = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
    let pdf_derivative = diff(
        normal_pdf.to_string(),
        "x".to_string(),
        Some(&["mu".to_string(), "sigma".to_string()]),
        None,
    )
    .unwrap();

    println!("Normal PDF: f(x) = {}", normal_pdf);
    println!("df/dx = {}", pdf_derivative);

    println!("\n9. CHEMICAL KINETICS - Reaction Rate");
    println!("----------------------------------");

    // First-order reaction: [A] = [A]₀ * e^(-kt)
    let concentration = "A0 * exp(-k * t)";
    let reaction_rate = diff(
        concentration.to_string(),
        "t".to_string(),
        Some(&["A0".to_string(), "k".to_string()]),
        None,
    )
    .unwrap();

    println!("Concentration: [A](t) = {}", concentration);
    println!("Rate:         -d[A]/dt = -{}", reaction_rate);

    println!("\n10. ACOUSTICS - Sound Wave");
    println!("-------------------------");

    // Pressure variation: p(x,t) = p₀ * sin(kx - ωt)
    let pressure = "p0 * sin(k * x - omega * t)";
    let particle_velocity = diff(
        pressure.to_string(),
        "x".to_string(),
        Some(&[
            "p0".to_string(),
            "k".to_string(),
            "omega".to_string(),
            "t".to_string(),
        ]),
        None,
    )
    .unwrap();

    println!("Pressure:     p(x,t) = {}", pressure);
    println!(
        "Velocity:     u(x,t) = (1/ρ) * dp/dx = (1/ρ) * {}",
        particle_velocity
    );
}
