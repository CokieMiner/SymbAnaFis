use symb_anafis::diff;

fn main() {
    let result = diff("asin(x)".to_string(), "x".to_string(), None, None).unwrap();
    println!("asin(x) derivative: {}", result);
    
    let result2 = diff("atan(x)".to_string(), "x".to_string(), None, None).unwrap();
    println!("atan(x) derivative: {}", result2);
}
