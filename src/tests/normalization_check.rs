use crate::parse;
use std::collections::HashSet;

#[test]
fn test_merging_terms() {
    let empty = HashSet::new();
    let expr = parse("y + z + 2*y", &empty, &empty, None).unwrap();
    println!("Parsed y + z + 2*y: {}", expr);

    let expr2 = parse("y * y * y", &empty, &empty, None).unwrap();
    println!("Parsed y * y * y: {}", expr2);

    let expr3 = parse("2*x + 3*x", &empty, &empty, None).unwrap();
    println!("Parsed 2*x + 3*x: {}", expr3);

    println!("Size of Expr: {} bytes", std::mem::size_of::<crate::Expr>());
    println!(
        "Size of ExprKind: {} bytes",
        std::mem::size_of::<crate::ExprKind>()
    );
}
