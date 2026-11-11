//! Parse, differentiate, simplify, apply domain properties, and evaluate a scoped expression.
#![expect(
    clippy::print_stdout,
    reason = "the executable prints symbolic expressions and numerical results"
)]

use core::error::Error;

use symb_anafis::{Context, Expr, Number, Program, Property, Workspace};

#[expect(
    clippy::arithmetic_side_effects,
    reason = "symbolic arithmetic uses the library's normalized constructors"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let expression = context.parse("x^3 + sin(x)")?;
    let derivative = Expr::derivative(expression.clone(), x, 1);
    println!("f(x) = {expression}");
    println!("f'(x) = {derivative}");
    println!("f(0.5), direct = {}", expression.evaluate([(x, 0.5_f64)])?);

    let compiled = Program::<f64>::compile(&expression, &[x])?;
    let program = compiled.with_native().unwrap_or(compiled);
    let mut workspace = Workspace::new();
    println!(
        "f(0.5), compiled (native={}) = {}",
        program.native_bytes().is_some(),
        program.eval_with(&mut workspace, &[0.5])?
    );

    let identity = x.sin().pow(2) + x.cos().pow(2);
    println!("{identity} = {}", identity.compact()?);

    x.set_properties(&[Property::Positive])?;
    println!("sqrt(x^2) for x > 0 = {}", x.pow(2).sqrt().compact()?);
    x.set_properties(&[])?;

    let exact = x.pow(2) / 3 + 1;
    let exact_compiled = Program::<Number>::compile(&exact, &[x])?;
    let mut exact_workspace = Workspace::new();
    println!(
        "({exact}) at x = 2: {}",
        exact_compiled.eval_with(&mut exact_workspace, &[Number::from_int(2)])?
    );

    let samples = [0.0, 0.5, 1.0];
    let mut output = [0.0; 3];
    program.eval_batch_with(&mut workspace, &[&samples], &mut output)?;
    for (input, result) in samples.iter().zip(output) {
        println!("f({input}) = {result}");
    }

    let wasm = program.to_wasm_bytes()?;
    println!("Standalone WASM module size: {} bytes", wasm.len());
    println!("LaTeX: {}", derivative.to_latex());
    println!("Unicode: {}", derivative.to_unicode());
    Ok(())
}
