//! Shared test expressions for benchmarks
//!
//! Medium-sized, realistic expressions from physics, statistics, and ML.

/// Normal probability density function
/// ~30 nodes, includes exp, sqrt, division
pub const NORMAL_PDF: &str = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
pub const NORMAL_PDF_VAR: &str = "x";
pub const NORMAL_PDF_FIXED: &[&str] = &["mu", "sigma", "pi"];

/// 2D Gaussian distribution
/// ~40 nodes, two variables
pub const GAUSSIAN_2D: &str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)";
pub const GAUSSIAN_2D_VAR: &str = "x";
pub const GAUSSIAN_2D_FIXED: &[&str] = &["x0", "y0", "s", "pi"];

/// Maxwell-Boltzmann speed distribution
/// ~50 nodes, complex physics expression
pub const MAXWELL_BOLTZMANN: &str =
    "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))";
pub const MAXWELL_BOLTZMANN_VAR: &str = "v";
pub const MAXWELL_BOLTZMANN_FIXED: &[&str] = &["m", "k", "T", "pi"];

/// Lorentz factor (special relativity)
/// ~15 nodes, sqrt and division
pub const LORENTZ_FACTOR: &str = "1 / sqrt(1 - v^2 / c^2)";
pub const LORENTZ_FACTOR_VAR: &str = "v";
pub const LORENTZ_FACTOR_FIXED: &[&str] = &["c"];

/// Lennard-Jones potential
/// ~25 nodes, power functions
pub const LENNARD_JONES: &str = "4 * epsilon * ((sigma / r)^12 - (sigma / r)^6)";
pub const LENNARD_JONES_VAR: &str = "r";
pub const LENNARD_JONES_FIXED: &[&str] = &["epsilon", "sigma"];

/// Logistic sigmoid function
/// ~15 nodes, common in ML
pub const LOGISTIC_SIGMOID: &str = "1 / (1 + exp(-k * (x - x0)))";
pub const LOGISTIC_SIGMOID_VAR: &str = "x";
pub const LOGISTIC_SIGMOID_FIXED: &[&str] = &["k", "x0"];

/// Damped harmonic oscillator
/// ~25 nodes, exp and trig
pub const DAMPED_OSCILLATOR: &str = "A * exp(-gamma * t) * cos(omega * t + phi)";
pub const DAMPED_OSCILLATOR_VAR: &str = "t";
pub const DAMPED_OSCILLATOR_FIXED: &[&str] = &["A", "gamma", "omega", "phi"];

/// Planck blackbody radiation law
/// ~35 nodes, physics
pub const PLANCK_BLACKBODY: &str = "2 * h * nu^3 / c^2 * 1 / (exp(h * nu / (k * T)) - 1)";
pub const PLANCK_BLACKBODY_VAR: &str = "nu";
pub const PLANCK_BLACKBODY_FIXED: &[&str] = &["h", "c", "k", "T"];

/// Bessel function example (first kind)
/// ~20 nodes, special function
pub const BESSEL_WAVE: &str = "A * besselj(0, k * r) * exp(-alpha * r)";
pub const BESSEL_WAVE_VAR: &str = "r";
pub const BESSEL_WAVE_FIXED: &[&str] = &["A", "k", "alpha"];

/// All expressions as a slice for iteration
pub const ALL_EXPRESSIONS: &[(&str, &str, &str, &[&str])] = &[
    ("Normal PDF", NORMAL_PDF, NORMAL_PDF_VAR, NORMAL_PDF_FIXED),
    (
        "Gaussian 2D",
        GAUSSIAN_2D,
        GAUSSIAN_2D_VAR,
        GAUSSIAN_2D_FIXED,
    ),
    (
        "Maxwell-Boltzmann",
        MAXWELL_BOLTZMANN,
        MAXWELL_BOLTZMANN_VAR,
        MAXWELL_BOLTZMANN_FIXED,
    ),
    (
        "Lorentz Factor",
        LORENTZ_FACTOR,
        LORENTZ_FACTOR_VAR,
        LORENTZ_FACTOR_FIXED,
    ),
    (
        "Lennard-Jones",
        LENNARD_JONES,
        LENNARD_JONES_VAR,
        LENNARD_JONES_FIXED,
    ),
    (
        "Logistic Sigmoid",
        LOGISTIC_SIGMOID,
        LOGISTIC_SIGMOID_VAR,
        LOGISTIC_SIGMOID_FIXED,
    ),
    (
        "Damped Oscillator",
        DAMPED_OSCILLATOR,
        DAMPED_OSCILLATOR_VAR,
        DAMPED_OSCILLATOR_FIXED,
    ),
    (
        "Planck Blackbody",
        PLANCK_BLACKBODY,
        PLANCK_BLACKBODY_VAR,
        PLANCK_BLACKBODY_FIXED,
    ),
    (
        "Bessel Wave",
        BESSEL_WAVE,
        BESSEL_WAVE_VAR,
        BESSEL_WAVE_FIXED,
    ),
];
