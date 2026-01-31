// Example app requirements: unwrap for logic, stdout for output, similar names for math, long main functions
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "Example app requirements: unwrap for logic, stdout for output, similar names for math, long main functions"
)]
//! Physics & Engineering Applications using `SymbAnaFis`.

//!
//! Run with: cargo run --example applications
use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Physics & Engineering Applications");
    println!("==============================================");

    println!("\n1. KINEMATICS - Position, Velocity, Acceleration");
    println!("----------------------------------------------");

    // Position as function of time: x(t) = 5t² + 3t + 10
    let position = "5*t^2 + 3*t + 10";
    let velocity_expr = diff(position, "t", &[], None).unwrap();
    let acceleration_expr = diff(&velocity_expr, "t", &[], None).unwrap();

    println!("Position:     x(t) = {position}");
    println!("Velocity:     v(t) = dx/dt = {velocity_expr}");
    println!("Acceleration: a(t) = dv/dt = {acceleration_expr}");

    println!("\n2. ELECTRICITY - RC Circuit Voltage");
    println!("----------------------------------");

    // Voltage in RC circuit: V(t) = V₀ * e^(-t/(R*C))
    let voltage = "V0 * exp(-t / (R * C))";
    let current = diff(voltage, "t", &["V0", "R", "C"], None).unwrap();

    println!("Voltage: V(t) = {voltage}");
    println!("Current: I(t) = dV/dt = {current}");

    println!("\n3. THERMODYNAMICS - Heat Conduction");
    println!("----------------------------------");

    // Temperature in 1D heat equation solution: T(x,t) = T₀ * erf(x/(2*sqrt(α*t)))
    let temperature = "T0 * erf(x / (2 * sqrt(alpha * t)))";
    let heat_flux = diff(temperature, "x", &["T0", "alpha", "t"], None).unwrap();

    println!("Temperature: T(x,t) = {temperature}");
    println!("Heat flux:   q(x,t) = -k*dT/dx = -k*{heat_flux}");

    println!("\n4. QUANTUM MECHANICS - Wave Function");
    println!("-----------------------------------");

    // Gaussian wave packet: ψ(x,t) = (1/√(σ√π)) * exp(-x²/(4σ²)) * exp(i*k*x - i*E*t/ℏ)
    // For simplicity, just the real part
    let wave_function = "exp(-x^2 / (4 * sigma^2)) / sqrt(sigma * sqrt(pi))";
    let probability_density = format!("({wave_function})^2");
    let prob_derivative = diff(&probability_density, "x", &["sigma"], None).unwrap();

    println!("Wave function: \u{3c8}(x) = {wave_function}");
    println!("Probability:   |\u{3c8}|\u{b2} = {probability_density}");
    println!("d|\u{3c8}|\u{b2}/dx = {prob_derivative}");

    println!("\n5. FLUID DYNAMICS - Velocity Profile (Poiseuille Flow)");
    println!("----------------------------------------------------");

    // Velocity in pipe flow: u(r) = (ΔP/(4μL)) * (R² - r²)
    let velocity = "deltaP / (4 * mu * L) * (R^2 - r^2)";
    let shear_stress = diff(velocity, "r", &["deltaP", "mu", "L", "R"], None).unwrap();

    println!("Velocity:    u(r) = {velocity}");
    println!("Shear rate: du/dr = {shear_stress}");

    println!("\n6. OPTICS - Lens Formula");
    println!("-----------------------");

    // Lens equation: 1/f = 1/do + 1/di
    // Differentiate to find magnification sensitivity
    let magnification = "di / do"; // M = hi/ho = -di/do for thin lenses
    let mag_sensitivity = diff(magnification, "do", &["di"], None).unwrap();

    println!("Magnification: M = {magnification}");
    println!("dM/ddo = {mag_sensitivity}");

    println!("\n7. CONTROL SYSTEMS - Transfer Function");
    println!("------------------------------------");

    // Second-order system: G(s) = ωₙ² / (s² + 2ζωₙs + ωₙ²)
    // For simplicity, just the denominator derivative
    let denominator = "s^2 + 2*zeta*omega_n*s + omega_n^2";
    let denom_derivative = diff(denominator, "s", &["zeta", "omega_n"], None).unwrap();

    println!("Characteristic equation: {denominator}");
    println!("d(denom)/ds = {denom_derivative}");

    println!("\n8. STATISTICS - Normal Distribution");
    println!("---------------------------------");

    // Normal PDF: f(x) = (1/√(2πσ²)) * exp(-(x-μ)²/(2σ²))
    let normal_pdf = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
    let pdf_derivative = diff(normal_pdf, "x", &["mu", "sigma"], None).unwrap();

    println!("Normal PDF: f(x) = {normal_pdf}");
    println!("df/dx = {pdf_derivative}");

    println!("\n9. CHEMICAL KINETICS - Reaction Rate");
    println!("----------------------------------");

    // First-order reaction: [A] = [A]₀ * e^(-kt)
    let concentration = "A0 * exp(-k * t)";
    let reaction_rate = diff(concentration, "t", &["A0", "k"], None).unwrap();

    // Negate the derivative to get -d[A]/dt (rate of consumption)
    let negated_rate = format!("-1 * ({reaction_rate})");
    let consumption_rate = symb_anafis::simplify(&negated_rate, &["A0", "k"], None).unwrap();

    println!("Concentration: [A](t) = {concentration}");
    println!("Rate:         -d[A]/dt = {consumption_rate}");

    println!("\n10. ACOUSTICS - Sound Wave");
    println!("-------------------------");

    // Pressure variation: p(x,t) = p₀ * sin(kx - ωt)
    let pressure = "p0 * sin(k * x - omega * t)";
    let particle_velocity = diff(pressure, "x", &["p0", "k", "omega", "t"], None).unwrap();

    println!("Pressure:     p(x,t) = {pressure}");
    println!("Velocity:     u(x,t) = (1/\u{3c1}) * dp/dx = (1/\u{3c1}) * {particle_velocity}");

    println!("\n11. CALCULUS - Related Rates");
    println!("---------------------------");

    // Ladder leaning against wall: x² + y² = L²
    // Differentiate implicitly: 2x dx/dt + 2y dy/dt = 0
    let constraint = "x^2 + y^2";
    let rate_equation = diff(constraint, "t", &[], None).unwrap();

    println!("Constraint: x\u{b2} + y\u{b2} = L\u{b2}");
    println!("Rate:      2x dx/dt + 2y dy/dt = {rate_equation}");

    println!("\n12. OPTIMIZATION - Maximum Volume");
    println!("-------------------------------");

    // Box volume: V = x * y * z, with constraint x + 2y + 2z = constant
    let volume = "x * y * z";
    let volume_grad_x = diff(volume, "x", &["y", "z"], None).unwrap();

    println!("Volume: V = {volume}");
    println!("\u{2202}V/\u{2202}x = {volume_grad_x}");

    println!("\n13. TAYLOR SERIES");
    println!("----------------");

    // f(x) ≈ f(a) + f'(a)(x-a)
    let function = "sin(x)";
    let f1 = diff(function, "x", &[], None).unwrap();

    println!("Function: f(x) = {function}");
    println!("f'(x) = {f1}");

    println!("\n14. ARC LENGTH");
    println!("-------------");

    // Arc length: L = ∫√[(dx/dt)² + (dy/dt)²] dt
    let x_parametric = "cos(t)";
    let y_parametric = "sin(t)";
    let dx_dt = diff(x_parametric, "t", &[], None).unwrap();
    let dy_dt = diff(y_parametric, "t", &[], None).unwrap();

    println!("Parametric: x(t) = {x_parametric}, y(t) = {y_parametric}");
    println!("dx/dt = {dx_dt}");
    println!("dy/dt = {dy_dt}");
    let ds_squared = format!("({dx_dt})^2 + ({dy_dt})^2");
    let ds_inner = symb_anafis::simplify(&ds_squared, &[], None).unwrap();
    let ds = format!("sqrt({ds_inner})");
    let ds_simplified = symb_anafis::simplify(&ds, &[], None).unwrap();
    println!("Arc length element: ds = {ds_simplified} dt");
    println!("\n15. MECHANICS - Harmonic Oscillator");
    println!("----------------------------------");

    // Simple harmonic motion: x(t) = A*cos(ωt + φ)
    let displacement = "A * cos(omega * t + phi)";
    let velocity_shm = diff(displacement, "t", &["A", "omega", "phi"], None).unwrap();
    let acceleration_shm = diff(&velocity_shm, "t", &["A", "omega", "phi"], None).unwrap();

    println!("Displacement: x(t) = {displacement}");
    println!("Velocity:     v(t) = {velocity_shm}");
    println!("Acceleration: a(t) = {acceleration_shm}");

    println!("\n16. ELECTROMAGNETISM - Maxwell's Equations");
    println!("-----------------------------------------");

    // Faraday's law: ∇ × E = -∂B/∂t
    // For a simple case, B = B0*exp(-t/τ)
    let magnetic_field = "B0 * exp(-t / tau)";
    let emf = diff(magnetic_field, "t", &["B0", "tau"], None).unwrap();

    // Negate for Faraday's law: EMF = -dB/dt
    let negated_emf = format!("-1 * ({emf})");
    let induced_emf = symb_anafis::simplify(&negated_emf, &["B0", "tau"], None).unwrap();

    println!("Magnetic field: B(t) = {magnetic_field}");
    println!("EMF:           \u{3b5} = -dB/dt = {induced_emf}");

    println!("\n17. THERMODYNAMICS - Entropy");
    println!("---------------------------");

    // Entropy change: dS = dQ_rev/T
    // For ideal gas: S = C_v*ln(T) + R*ln(V) + const
    let entropy = "Cv * ln(T) + R * ln(V)";
    let entropy_temp_deriv = diff(entropy, "T", &["Cv", "R", "V"], None).unwrap();

    println!("Entropy: S = {entropy}");
    println!("\u{2202}S/\u{2202}T = {entropy_temp_deriv}");

    println!("\n18. QUANTUM PHYSICS - Schr\u{f6}dinger Equation");
    println!("-----------------------------------------");

    // Time-dependent wave function: ψ(x,t) = ψ0(x)*exp(-iEt/ℏ)
    let wave_function_td = "psi0 * exp(-i * E * t / hbar)";
    let time_derivative = diff(wave_function_td, "t", &["psi0", "E", "hbar"], None).unwrap();

    println!("Wave function: \u{3c8}(x,t) = {wave_function_td}");
    println!("\u{2202}\u{3c8}/\u{2202}t = {time_derivative}");

    println!("\n19. RELATIVITY - Time Dilation");
    println!("-----------------------------");

    // Proper time: τ = t * sqrt(1 - v²/c²)
    let proper_time = "t * sqrt(1 - v^2 / c^2)";
    let time_dilation_deriv = diff(proper_time, "v", &["t", "c"], None).unwrap();

    println!("Proper time: \u{3c4} = {proper_time}");
    println!("d\u{3c4}/dv = {time_dilation_deriv}");

    println!("\n20. ASTROPHYSICS - Orbital Mechanics");
    println!("-----------------------------------");

    // Kepler's second law: dA/dt = constant
    // For elliptical orbit: r = a(1-ecc²)/(1 + ecc*cos(θ))
    let radius = "a * (1 - ecc^2) / (1 + ecc * cos(theta))";
    let angular_momentum = diff(radius, "theta", &["a", "ecc"], None).unwrap();

    println!("Orbital radius: r(\u{3b8}) = {radius}");
    println!("dr/d\u{3b8} = {angular_momentum}");
}
