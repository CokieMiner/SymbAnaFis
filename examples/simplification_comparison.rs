use std::convert::TryInto;
use std::env;
use symb_anafis::Diff;
use symbolica::{
    LicenseManager,
    atom::{Atom, AtomCore, Indeterminate},
    parser::ParseSettings,
    wrap_input,
};

use dotenvy::dotenv;

fn main() {
    dotenv().ok(); // Load .env file if present

    // Initialize Symbolica license key if present in environment
    if let Ok(key) = env::var("SYMBOLICA_LICENSE") {
        match LicenseManager::set_license_key(&key) {
            Ok(()) => println!("Symbolica license key applied successfully."),
            Err(e) => eprintln!("Warning: Failed to set Symbolica license key: {e}"),
        }
    }

    println!("SymbAnaFis Differentiation Output Comparison");
    println!("============================================");

    // (Name, Expression, Variable to Diff)
    let expressions = vec![
        (
            "Normal PDF",
            "exp(-(x-mu)^2/(2*sigma^2))/sqrt(2*pi*sigma^2)",
            "x",
        ),
        (
            "Gaussian 2D",
            "exp(-((x-x0)^2+(y-y0)^2)/(2*s^2))/(2*pi*s^2)",
            "x",
        ),
        (
            "Maxwell-Boltzmann",
            "4*pi*(m/(2*pi*k*T))^(3/2) * v^2 * exp(-m*v^2/(2*k*T))",
            "v",
        ),
        ("Lorentz Factor", "1/sqrt(1-v^2/c^2)", "v"),
        (
            "Lennard-Jones",
            "4*epsilon*((sigma/r)^12 - (sigma/r)^6)",
            "r",
        ),
        ("Logistic Sigmoid", "1/(1+exp(-k*(x-x0)))", "x"),
        ("Damped Oscillator", "A*exp(-gamma*t)*cos(omega*t+phi)", "t"),
        (
            "Planck Blackbody",
            "(2*h*nu^3/c^2) * (1/(exp(h*nu/(k*T))-1))",
            "nu", // frequency
        ),
    ];

    for (name, expr_str, var) in expressions {
        println!("\n--- {name} (d/d{var}) ---");
        println!("Input: {expr_str}");

        // SymbAnaFis - Raw Differentiation (No Simplification)
        match Diff::new()
            .skip_simplification(true)
            .diff_str(expr_str, var, &[])
        {
            Ok(s) => println!("SymbAnaFis (Raw):        {s}"),
            Err(e) => println!("SymbAnaFis (Raw):        Error: {e:?}"),
        }

        // SymbAnaFis - Simplified Differentiation (Default)
        match Diff::new().diff_str(expr_str, var, &[]) {
            Ok(s) => println!("SymbAnaFis (Simplified): {s}"),
            Err(e) => println!("SymbAnaFis (Simplified): Error: {e:?}"),
        }

        // Symbolica Differentiation
        let symbolica_out = {
            match Atom::parse(wrap_input!(expr_str), ParseSettings::default()) {
                Ok(atom) => match Atom::parse(wrap_input!(var), ParseSettings::default()) {
                    Ok(var_atom) => {
                        let var_indet: Result<Indeterminate, _> = var_atom.try_into();
                        match var_indet {
                            Ok(indet) => {
                                let diff_atom = atom.derivative(indet);
                                diff_atom.to_string()
                            }
                            Err(_) => format!("'{var}' is not an indeterminate"),
                        }
                    }
                    Err(e) => format!("Var Parse Error: {e}"),
                },
                Err(e) => format!("Expr Parse Error: {e}"),
            }
        };
        println!("Symbolica:               {symbolica_out}");
    }
}
