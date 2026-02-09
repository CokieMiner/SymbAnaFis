// View API Demo: allow for example code patterns
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::use_debug,
    reason = "Demo script: unwrap for setup, stdout for output, debug for examples"
)]
//! View API Demo - Pattern Matching on Expression Structure
//!
//! Demonstrates the View API for inspecting expression structure
//! without exposing internal implementation details.
//!
//! Run with: `cargo run --example view_api_demo`

use symb_anafis::visitor::ExprView;
use symb_anafis::{Expr, Symbol, symb};

/// Recursively print expression structure using View API
#[allow(
    clippy::too_many_lines,
    reason = "Demo function showing multiple examples"
)]
fn print_structure(expr: &Expr, indent: usize) {
    let view = expr.view();
    let prefix = "  ".repeat(indent);

    match view {
        ExprView::Number(n) => {
            println!("{prefix}Number: {n}");
        }
        ExprView::Symbol(name) => {
            println!("{prefix}Symbol: {name}");
        }
        ExprView::Sum(terms) => {
            println!("{prefix}Sum ({} terms):", terms.len());
            for term in terms.iter() {
                print_structure(term, indent + 1);
            }
        }
        ExprView::Product(factors) => {
            println!("{prefix}Product ({} factors):", factors.len());
            for factor in factors.iter() {
                print_structure(factor, indent + 1);
            }
        }
        ExprView::Div(num, den) => {
            println!("{prefix}Division:");
            println!("{prefix}  Numerator:");
            print_structure(num, indent + 2);
            println!("{prefix}  Denominator:");
            print_structure(den, indent + 2);
        }
        ExprView::Pow(base, exp) => {
            println!("{prefix}Power:");
            println!("{prefix}  Base:");
            print_structure(base, indent + 2);
            println!("{prefix}  Exponent:");
            print_structure(exp, indent + 2);
        }
        ExprView::Function { name, args } => {
            println!("{prefix}Function: {name} ({} args)", args.len());
            for (i, arg) in args.iter().enumerate() {
                println!("{prefix}  Arg {i}:");
                print_structure(arg, indent + 2);
            }
        }
        ExprView::Derivative { inner, var, order } => {
            println!("{prefix}Derivative: d^{order}/d{var}^{order}");
            print_structure(inner, indent + 1);
        }
    }
}

/// Convert expression to JSON-like string representation
fn to_json_like(expr: &Expr) -> String {
    match expr.view() {
        ExprView::Number(n) => {
            format!(r#"{{"kind": "Number", "value": {n}}}"#)
        }
        ExprView::Symbol(name) => {
            format!(r#"{{"kind": "Symbol", "name": "{name}"}}"#)
        }
        ExprView::Sum(terms) => {
            let children: Vec<String> = terms.iter().map(|t| to_json_like(t)).collect();
            format!(
                r#"{{"kind": "Sum", "children": [{}]}}"#,
                children.join(", ")
            )
        }
        ExprView::Product(factors) => {
            let children: Vec<String> = factors.iter().map(|f| to_json_like(f)).collect();
            format!(
                r#"{{"kind": "Product", "children": [{}]}}"#,
                children.join(", ")
            )
        }
        ExprView::Div(num, den) => {
            format!(
                r#"{{"kind": "Div", "left": {}, "right": {}}}"#,
                to_json_like(num),
                to_json_like(den)
            )
        }
        ExprView::Pow(base, exp) => {
            format!(
                r#"{{"kind": "Pow", "left": {}, "right": {}}}"#,
                to_json_like(base),
                to_json_like(exp)
            )
        }
        ExprView::Function { name, args } => {
            let children: Vec<String> = args.iter().map(|a| to_json_like(a)).collect();
            format!(
                r#"{{"kind": "Function", "name": "{name}", "args": [{}]}}"#,
                children.join(", ")
            )
        }
        ExprView::Derivative { inner, var, order } => {
            format!(
                r#"{{"kind": "Derivative", "var": "{var}", "order": {order}, "inner": {}}}"#,
                to_json_like(inner)
            )
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Demo function showing multiple examples"
)]
fn main() {
    println!("{}", "=".repeat(70));
    println!("VIEW API DEMO - Expression Structure Inspection");
    println!("{}", "=".repeat(70));

    // Example 1: Polynomial (might be optimized internally as Poly)
    println!("\n1. POLYNOMIAL: x^2 + 2*x + 1");
    println!("{}", "-".repeat(70));
    let x = symb("x");
    let poly = x.pow(2.0) + 2.0 * x.to_expr() + 1.0;
    println!("Expression: {poly}");
    println!(
        "View kind: {:?}",
        match poly.view() {
            ExprView::Number(_) => "Number",
            ExprView::Symbol(_) => "Symbol",
            ExprView::Sum(_) => "Sum",
            ExprView::Product(_) => "Product",
            ExprView::Div(_, _) => "Div",
            ExprView::Pow(_, _) => "Pow",
            ExprView::Function { .. } => "Function",
            ExprView::Derivative { .. } => "Derivative",
        }
    );
    println!("\nStructure:");
    print_structure(&poly, 0);

    // Example 2: Trigonometric expression
    println!("\n\n2. TRIGONOMETRIC: sin(x)^2 + cos(x)^2");
    println!("{}", "-".repeat(70));
    let trig = x.sin().pow(2.0) + x.cos().pow(2.0);
    println!("Expression: {trig}");
    println!(
        "View kind: {:?}",
        match trig.view() {
            ExprView::Sum(_) => "Sum",
            _ => "Other",
        }
    );
    println!("\nStructure:");
    print_structure(&trig, 0);

    // Example 3: Rational function
    println!("\n\n3. RATIONAL FUNCTION: (x + 1) / (x - 1)");
    println!("{}", "-".repeat(70));
    let rational = (x.to_expr() + 1.0) / (x.to_expr() - 1.0);
    println!("Expression: {rational}");
    println!(
        "View kind: {:?}",
        match rational.view() {
            ExprView::Div(_, _) => "Div",
            _ => "Other",
        }
    );
    println!("\nStructure:");
    print_structure(&rational, 0);

    // Example 4: View API properties
    println!("\n\n4. VIEW API PROPERTIES");
    println!("{}", "-".repeat(70));
    let expr4 = x.pow(2.0) + 3.0 * x.to_expr() + 5.0;
    let view4 = expr4.view();

    println!("Expression: {expr4}");
    println!(
        "Kind:       {:?}",
        match &view4 {
            ExprView::Sum(_) => "Sum",
            _ => "Other",
        }
    );

    if let ExprView::Sum(terms) = &view4 {
        println!("Is Sum:     true");
        println!("Is Product: false");
        println!("# Children: {}", terms.len());
        println!("First term: {}", terms[0]);
        println!("Last term:  {}", terms[terms.len() - 1]);
    }

    // Example 5: Converting to custom format
    println!("\n\n5. CUSTOM CONVERSION (Example: to JSON-like)");
    println!("{}", "-".repeat(70));
    let expr5 = x.sin() + x.pow(2.0);
    println!("Expression: {expr5}");
    println!("As JSON:    {}", to_json_like(&expr5));

    // Example 6: Anonymous symbols
    println!("\n\n6. ANONYMOUS SYMBOLS");
    println!("{}", "-".repeat(70));
    let anon = Symbol::anon();
    let expr6 = anon.to_expr() + 1.0;
    let view6 = expr6.view();

    println!("Expression:   {expr6}");
    println!(
        "View kind:    {:?}",
        match &view6 {
            ExprView::Sum(_) => "Sum",
            _ => "Other",
        }
    );

    if let ExprView::Sum(terms) = &view6 {
        println!("# Children:   {}", terms.len());

        // Find the symbol child (not the number)
        for term in terms.iter() {
            if let ExprView::Symbol(name) = term.view() {
                println!("Symbol child: {term}");
                println!("Symbol name:  {name}");
                break;
            }
        }
        println!("             (Note: anonymous symbols show as '$ID')");
    }

    // Example 7: Helper methods
    println!("\n\n7. VIEW API HELPER METHODS");
    println!("{}", "-".repeat(70));
    let expr7 = x.pow(2.0) + 3.0;
    let view7 = expr7.view();

    println!("Expression: {expr7}");
    println!("is_number:  {}", view7.is_number());
    println!("is_symbol:  {}", view7.is_symbol());
    println!("is_sum:     {}", view7.is_sum());
    println!("is_product: {}", view7.is_product());

    // Pattern matching for other types
    match &view7 {
        ExprView::Div(_, _) => println!("is_div:     true"),
        _ => println!("is_div:     false"),
    }
    match &view7 {
        ExprView::Pow(_, _) => println!("is_pow:     true"),
        _ => println!("is_pow:     false"),
    }

    println!("\n{}", "=".repeat(70));
    println!("Demo complete! View API allows safe pattern matching on expressions.");
    println!("{}", "=".repeat(70));
}
