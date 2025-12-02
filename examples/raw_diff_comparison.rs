use symb_anafis::{parse, simplify_ast};
use std::collections::HashSet;
use std::time::Instant;

fn main() {
    // Normal PDF expression - complex real-world example
    let expr_str = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
    
    let mut fixed_vars: HashSet<String> = HashSet::new();
    fixed_vars.insert("mu".to_string());
    fixed_vars.insert("sigma".to_string());
    let custom_fns: HashSet<String> = HashSet::new();
    
    println!("================================================================================");
    println!("Expression: {}", expr_str);
    println!("================================================================================");
    
    match parse(expr_str, &fixed_vars, &custom_fns) {
        Ok(expr) => {
            // Benchmark raw diff
            let iterations = 1000;
            let start = Instant::now();
            let mut raw_diff = expr.derive("x", &fixed_vars);
            for _ in 1..iterations {
                raw_diff = expr.derive("x", &fixed_vars);
            }
            let raw_time = start.elapsed().as_micros() as f64 / iterations as f64;
            
            let raw_str = raw_diff.to_string();
            
            // Benchmark diff + simplify
            let start = Instant::now();
            let mut simplified_diff = simplify_ast(expr.derive("x", &fixed_vars));
            for _ in 1..iterations {
                simplified_diff = simplify_ast(expr.derive("x", &fixed_vars));
            }
            let simp_time = start.elapsed().as_micros() as f64 / iterations as f64;
            
            let simp_str = simplified_diff.to_string();
            
            println!("\nSymbAnaFis Results:");
            println!("--------------------------------------------------------------------------------");
            
            println!("\nRaw derive() ({:.2} µs):", raw_time);
            println!("  {}", raw_str);
            println!("  Length: {} chars", raw_str.len());
            
            println!("\nsimplify(derive()) ({:.2} µs):", simp_time);
            println!("  {}", simp_str);
            println!("  Length: {} chars", simp_str.len());
        }
        Err(e) => println!("Parse error: {:?}", e),
    }
}
