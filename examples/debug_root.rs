use symb_anafis::diff;

fn main() {
    let result = diff("cbrt(x)".to_string(), "x".to_string(), None, None).unwrap();
    println!("cbrt(x) derivative: {}", result);
}
