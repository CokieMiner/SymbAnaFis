use symb_anafis::diff;

fn main() {
    let result = diff("sin(x)/x".to_string(), "x".to_string(), None, None).unwrap();
    println!("sin(x)/x derivative: {}", result);
}
