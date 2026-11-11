//! Pattern matching, rewrite rules, user functions, substitution, and custom simplification.
#![expect(
    clippy::print_stdout,
    reason = "the executable prints symbolic transformations and results"
)]

use core::error::Error;

use symb_anafis::{
    Context, DomainSafety, Pattern, RewriteRule, SimplificationSequence, Substitution, UserFunction,
};

#[expect(
    clippy::arithmetic_side_effects,
    reason = "symbolic arithmetic uses the library's normalized constructors"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;

    // Concrete substitution.
    let poly = x.pow(2) + 2 * x + 1;
    let sub = Substitution::new(x.to_expr(), y + 1);
    let substituted = sub.apply_all(&poly);
    println!("Original:    {poly}");
    println!("x -> y + 1:  {substituted}");
    println!("Expanded:    {}", substituted.expand()?);
    println!("Factored:    {}", substituted.expand()?.factor()?);

    // Custom user function and pattern rewrite rule: f(sqrt(a)) -> exp(a).
    let f = context.register_function("f", UserFunction::new(1..=1))?;
    let a = Pattern::capture("a", None);
    let rule = RewriteRule::new(Pattern::apply(f.clone(), vec![a.sqrt()])?, a.exp())?;

    let input = f.call(vec![(x + 1).sqrt()])? + f.call(vec![y.sqrt()])?;
    let rewritten = rule.apply_all(&input, usize::MAX, None)?;
    println!("\nBefore rule: {input}");
    println!("After rule:  {rewritten}");

    // Configurable simplification sequence comparing domain safety modes.
    let ratio = (x.pow(2) - 1) / (x - 1);
    let preserving = SimplificationSequence::compact()
        .domain_safety(DomainSafety::Preserving)
        .simplify(&ratio)?;
    let assumed = SimplificationSequence::compact()
        .domain_safety(DomainSafety::AssumptionDependent)
        .simplify(&ratio)?;
    println!("\nRatio:                 {ratio}");
    println!("Preserving:            {}", preserving.into_expression());
    println!("AssumptionDependent:   {}", assumed.into_expression());
    Ok(())
}
