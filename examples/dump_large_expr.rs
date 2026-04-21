//! Example to dump large expressions.

use std::collections::HashSet;
use std::fmt::Write;
use std::fs::File;
use std::io::Write as IoWrite;
use symb_anafis::{CompiledEvaluator, Diff, symb};

/// Generates a complex mixed expression with N terms
fn generate_mixed_complex(n: usize) -> String {
    let mut s = String::with_capacity(n * 50);
    for i in 1..=n {
        if i > 1 {
            if i % 4 == 0 {
                write!(s, " - ").expect("Failed to write to string");
            } else {
                write!(s, " + ").expect("Failed to write to string");
            }
        }

        match i % 5 {
            0 => {
                write!(s, "{i}*x^{}", i % 10 + 1).expect("Failed to write to string");
            }
            1 => {
                write!(s, "sin({i}*x)*cos(x)").expect("Failed to write to string");
            }
            2 => {
                write!(s, "(exp(x/{i}) + sqrt(x + {i}))").expect("Failed to write to string");
            }
            3 => {
                write!(s, "(x^2 + {i})/(x + {i})").expect("Failed to write to string");
            }
            _ => {
                write!(s, "sin(exp(x) + {i})").expect("Failed to write to string");
            }
        }
    }
    s
}

#[allow(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "Example script needs to output bytecode for demonstration"
)]
fn main() {
    let expr_str = generate_mixed_complex(10);
    println!("Expression: {expr_str}");

    let x_symb = symb("x");
    let empty_set = HashSet::new();

    let parsed = symb_anafis::parse(&expr_str, &empty_set, &empty_set, None)
        .expect("Failed to parse expression");
    let diffed = Diff::new()
        .differentiate(&parsed, &x_symb)
        .expect("Failed to differentiate"); // By default does simplify

    let evaluator =
        CompiledEvaluator::compile_auto(&diffed, None).expect("Failed to compile evaluator");

    let bytecode = evaluator.disassemble();
    let mut file1 = File::create("generated_bytecode.txt").expect("Failed to create file");
    writeln!(file1, "--- Raw Bytecode (Generated) ---").unwrap();
    writeln!(file1, "{bytecode}").unwrap();

    let big_expr_str = std::fs::read_to_string("examples/symblica_exp/big_expr.txt")
        .expect("Failed to read big_expr.txt");
    let parsed_big = symb_anafis::parse(&big_expr_str, &empty_set, &empty_set, None)
        .expect("Failed to parse big expression");

    // Just compile the expression itself
    let evaluator_big = CompiledEvaluator::compile_auto(&parsed_big, None)
        .expect("Failed to compile big evaluator");

    let bytecode_big = evaluator_big.disassemble();
    let mut file2 = File::create("big_expr_bytecode.txt").expect("Failed to create file");
    writeln!(file2, "--- Raw Bytecode (big_expr.txt) ---").unwrap();
    writeln!(file2, "{bytecode_big}").unwrap();
}
