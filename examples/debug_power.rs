use symb_anafis::diff;

fn main() {
    let result = diff("x^(1/3)".to_string(), "x".to_string(), None, None).unwrap();
    println!("x^(1/3) derivative: {}", result);
}
