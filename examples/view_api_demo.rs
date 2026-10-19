//! Inspect logical expressions with borrowed views and a memoized DAG fold.
#![expect(
    clippy::print_stdout,
    reason = "the executable prints logical nodes and expression metadata"
)]

use core::error::Error;

use symb_anafis::{Context, Expr, ExprView, UserFunction};

#[expect(
    clippy::arithmetic_side_effects,
    reason = "symbolic arithmetic uses the library's normalized constructors"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let context = Context::new();
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let shared = x + y;
    let expression = shared.sin() + shared.cos();

    println!("Expression: {expression}");
    println!("Stored nodes: {}", expression.node_count());
    println!("Stored depth: {}", expression.max_depth());
    println!("Variables: {}", expression.variables_ordered().join(", "));
    println!("Distinct logical nodes in children-before-parent order:");
    let logical = expression.fold(Vec::new(), |mut nodes, node| {
        nodes.push(node.to_string());
        nodes
    });
    for node in &logical {
        println!("  {node}");
    }
    println!("Logical node count: {}", logical.len());

    let polynomial = x.pow(2) + x.pow(3) + 1;
    if let ExprView::Sum(terms) = polynomial.view() {
        println!("Logical terms of {polynomial}:");
        for term in terms {
            println!("  {term}");
        }
    }

    let opaque = context.register_function("opaque", UserFunction::new(1..=1))?;
    let call = opaque.call(vec![x.to_expr()])?;
    let derivative = Expr::derivative(call, x, 1);
    if let ExprView::Derivative { inner, var, order } = derivative.view() {
        println!(
            "Retained derivative: order {order}, variable {}, input {inner}",
            var.to_expr()
        );
    }

    let constant = Expr::pi();
    println!(
        "pi has a dedicated node: {}",
        matches!(constant.view(), ExprView::Pi)
    );
    Ok(())
}
