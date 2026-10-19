//! Kinematics, volume uncertainty, and a polar-coordinate Jacobian.
//!
//! Coordinates and measured lengths are real. Volume uncertainties are
//! independent standard uncertainties, expressed in the same units as lengths.
//! Covariance propagation uses a first-order local approximation.
//! Polar coordinates use \(r>0\) and an explicitly chosen angular interval.
#![expect(
    clippy::print_stdout,
    reason = "the executable prints scientific models and their computed quantities"
)]

use core::error::Error;

use symb_anafis::{Context, CovarianceMatrix, Expr, Number, Uncertainty};

#[expect(
    clippy::arithmetic_side_effects,
    reason = "symbolic arithmetic uses the library's normalized constructors"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let context = Context::new();
    let t = context.symb("t")?;
    let position = 5 * t.pow(2) + 3 * t + 10;
    let velocity = Expr::derivative(position.clone(), t, 1);
    let acceleration = Expr::derivative(position.clone(), t, 2);
    println!("Position: {position}");
    println!("Velocity: {velocity}");
    println!("Acceleration: {acceleration}");
    println!(
        "Velocity at t = 2: {}",
        velocity.evaluate([(t, Number::from_int(2))])?
    );

    let length = context.symb("L")?;
    let width = context.symb("W")?;
    let height = context.symb("H")?;
    let volume = length * width * height;
    let standard_uncertainty = Expr::number(1) / 100;
    let covariance = CovarianceMatrix::diagonal(vec![standard_uncertainty.pow(2); 3])?;
    let propagated = Uncertainty::propagate(&volume, &[length, width, height], &covariance)?;
    let measured = [
        (length, Number::from_int(2)),
        (width, Number::from_int(3)),
        (height, Number::from_int(4)),
    ];
    println!("\nVolume: {volume}");
    for (variable, sensitivity) in [length, width, height]
        .iter()
        .zip(propagated.sensitivities())
    {
        println!("Sensitivity to {}: {sensitivity}", variable.to_expr());
    }
    println!(
        "Volume at L = 2, W = 3, H = 4: {}",
        volume.evaluate(measured.clone())?
    );
    println!("Combined variance: {}", propagated.variance());
    println!(
        "Combined variance at the measured lengths: {}",
        propagated.variance().evaluate(measured.clone())?
    );
    println!(
        "Combined standard uncertainty: {}",
        propagated.standard_deviation().evaluate(measured)?
    );

    let radius = context.symb("r")?;
    let theta = context.symb("theta")?;
    let cartesian = [radius * theta.cos(), radius * theta.sin()];
    let [cartesian_x, cartesian_y] = &cartesian;
    let jacobian = Expr::jacobian(&cartesian, &[radius, theta]);
    println!("\nPolar map: x = {cartesian_x}, y = {cartesian_y}");
    println!("Jacobian rows; columns are r, theta:");
    for row in jacobian {
        let entries: Vec<_> = row.iter().map(ToString::to_string).collect();
        println!("  [{}]", entries.join(", "));
    }
    Ok(())
}
