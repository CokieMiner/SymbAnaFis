//! Parse, differentiate, simplify, and evaluate a scoped expression.
#![expect(
    clippy::print_stdout,
    reason = "the executable prints symbolic expressions and numerical results"
)]

use core::error::Error;

use symb_anafis::{Context, Evaluator, Expr, Number, SimplificationSequence};

#[expect(
    clippy::arithmetic_side_effects,
    reason = "symbolic arithmetic uses the library's normalized constructors"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let context = Context::new();
    let x = context.symb("x")?;
    let expression = context.parse("x^3 + sin(x)")?;
    let derivative = Expr::derivative(expression.clone(), x, 1);
    println!("f(x) = {expression}");
    println!("f'(x) = {derivative}");
    println!("f(0.5), direct = {}", expression.evaluate([(x, 0.5_f64)])?);
    let mut compiled = Evaluator::<f64>::compile(&expression, &[x])?;
    println!("f(0.5), compiled = {}", compiled.evaluate(&[0.5])?);

    let identity = x.sin().pow(2) + x.cos().pow(2);
    let simplified = SimplificationSequence::compact().simplify(&identity)?;
    println!("{identity} = {}", simplified.expression());

    let exact = x.pow(2) / 3 + 1;
    let mut exact_compiled = Evaluator::<Number>::compile(&exact, &[x])?;
    println!(
        "({exact}) at x = 2: {}",
        exact_compiled.evaluate(&[Number::from_int(2)])?
    );

    let samples = [0.0, 0.5, 1.0];
    let mut output = [0.0; 3];
    compiled.evaluate_batch(&[&samples], &mut output)?;
    for (input, result) in samples.iter().zip(output) {
        println!("f({input}) = {result}");
    }

    println!("LaTeX: {}", derivative.to_latex());
    println!("Unicode: {}", derivative.to_unicode());
    Ok(())
}
