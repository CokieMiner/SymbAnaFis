#![allow(
    clippy::print_stdout,
    reason = "Examples must print values to demonstrate behavior"
)]
//! Deep capabilities showcase for `num-anafis`.
//!
//! Run with:
//! `cargo run -p num-anafis --example deep_capabilities`

use core::array::from_fn;

use num_anafis::{
    CliffordNumber, Evaluate, Interval, NumAnafisError, Number, Signature, Vector, e1, e2, e3, eps,
    i, j, r, s,
};

fn print_title(title: &str) {
    println!("\n=== {title} ===");
}

fn print_clifford(label: &str, value: &CliffordNumber) {
    let sig = value.signature();
    println!(
        "{label} in Cl({}, {}, {}), n = {}",
        sig.p,
        sig.q,
        sig.r,
        value.active_generators()
    );

    for (blade, coeff) in value.nonzero_blades() {
        println!("  blade {:>2}: {}", blade, coeff);
    }
}

fn main() -> Result<(), NumAnafisError> {
    print_title("Number: exact + approximate arithmetic");
    let a = s(42);
    let b = r(22, 7);
    let c = Number::float(2.5);
    let c_exact = r(5, 2);

    println!("a = {a}, b = {b}, c = {c}");
    println!("a + b = {}", &a + &b);
    println!("a - b = {}", &a - &b);
    println!("b * c (float path) = {}", &b * &c);
    println!("b * r(5, 2) (exact path) = {}", &b * &c_exact);
    println!("a / b = {}", &a / &b);

    println!("abs(-b) = {}", (-&b).abs());
    println!("fract(b) = {}", b.fract());
    println!("pow_i64(b, 3) = {:?}", b.pow_i64(3));
    println!("sqrt(2) ~= {}", s(2).sqrt());
    println!("cbrt(27) = {}", s(27).cbrt());
    println!("round(7/3) = {}", r(7, 3).round());

    let theta = r(355, 113);
    println!("sin(theta) = {}", theta.sin());
    println!("cos(theta) = {}", theta.cos());
    println!("exp(1) = {}", s(1).exp());
    println!("ln(5) = {}", s(5).ln());
    println!("atan(1) = {}", s(1).atan());
    println!("atan2(1, -1) = {}", s(1).atan2(&s(-1)));

    let tol = r(1, 1_000_000);
    let close_1 = s(1.0);
    let close_2 = s(1.0000005);
    println!(
        "approx_eq_number(1.0, 1.0000005, 1e-6) = {}",
        close_1.approx_eq_number(&close_2, &tol)
    );

    println!("to_i64_exact(42) = {:?}", a.to_i64_exact());
    println!("to_u32_exact(42) = {:?}", a.to_u32_exact());
    println!("to_f64_lossy(22/7) = {}", b.to_f64_lossy());

    let inf = s(f64::INFINITY);
    println!("is_finite(infinity) = {}", inf.is_finite());

    print_title("Interval arithmetic");
    let i1 = Interval::new(s(-2), s(5));
    let i2 = Interval::new(r(1, 3), s(4));

    println!("i1 = [{}, {}]", i1.lo, i1.hi);
    println!("i2 = [{}, {}]", i2.lo, i2.hi);
    println!("width(i1) = {}", i1.width());
    println!("midpoint(i1) = {}", i1.midpoint());
    println!("contains(i1, 0) = {}", i1.contains(&s(0)));
    println!("i1 + i2 = [{}, {}]", (&i1 + &i2).lo, (&i1 + &i2).hi);
    println!("i1 - i2 = [{}, {}]", (&i1 - &i2).lo, (&i1 - &i2).hi);
    println!("i1 * i2 = [{}, {}]", (&i1 * &i2).lo, (&i1 * &i2).hi);
    println!("i1 / i2 = [{}, {}]", (&i1 / &i2).lo, (&i1 / &i2).hi);

    print_title("Vector<T>: generic component-wise algebra");
    let v1 = Vector::new(s(1), s(2), s(3));
    let v2 = Vector::new(r(1, 2), s(-3), s(4));
    let scale = s(2);

    let v_add = &v1 + &v2;
    let v_sub = &v1 - &v2;
    let v_neg = -&v2;
    let v_scaled = &v1 * &scale;

    println!("v1 = {:?}", v1.components);
    println!("v2 = {:?}", v2.components);
    println!("v1 + v2 = {:?}", v_add.components);
    println!("v1 - v2 = {:?}", v_sub.components);
    println!("-v2 = {:?}", v_neg.components);
    println!("2 * v1 (via v1 * 2) = {:?}", v_scaled.components);

    print_title("CliffordNumber: constructors and products");
    let sig_r3 = Signature::new(3, 0, 0).expect("R3 signature is statically valid");

    let e1 = e1();
    let e2 = e2();
    let e3 = e3();

    let biv12 = &e1 * &e2;
    let triv123 = &biv12 * &e3;
    let rotor_like = CliffordNumber::scalar(sig_r3, 3, s(1))? + (&biv12 * r(1, 2));

    print_clifford("e1", &e1);
    print_clifford("e2", &e2);
    print_clifford("e1 * e2", &biv12);
    print_clifford("(e1 * e2) * e3", &triv123);
    print_clifford("1 + 1/2 e12", &rotor_like);

    let anticom = &(&e1 * &e2) + &(&e2 * &e1);
    print_clifford("e1e2 + e2e1 (should be 0 in Euclidean metric)", &anticom);

    print_title("CliffordNumber: special algebras + Evaluate trait");
    let z = s(2) + (s(3) * i());
    let d = s(5) + (r(1, 2) * eps());
    let split_num = s(1) + (s(2) * j());

    print_clifford("complex z = 2 + 3i", &z);
    print_clifford("dual d = 5 + 1/2 eps", &d);
    print_clifford("split s = 1 + 2j", &split_num);

    print_clifford("sin(z)", &z.sin());
    print_clifford("exp(z)", &z.exp());
    print_clifford("ln(d)", &d.ln());
    print_clifford("cos(s)", &split_num.cos());
    let split_sqrt = split_num.sqrt();
    print_clifford("sqrt(s)", &split_sqrt);
    println!(
        "sqrt(1 + 2j) finite scalar? {} (false here means domain has no real split-complex root)",
        split_sqrt.coeff(0).is_finite()
    );

    let unsupported = &e1 + &e2;
    let unsupported_eval = unsupported.sin();
    print_clifford("sin(e1 + e2) -> unsupported fallback", &unsupported_eval);
    println!(
        "unsupported fallback scalar finite? {}",
        unsupported_eval.coeff(0).is_finite()
    );

    print_title("Clifford coefficients: inline and heap representations");
    let inline =
        CliffordNumber::from_coeffs(sig_r3, 3, from_fn(|idx| if idx == 0 { s(1) } else { s(0) }))?;

    println!("inline storage? {}", !inline.is_heap_allocated());

    let sig5 = Signature::new(5, 0, 0).expect("Cl(5,0,0) signature is statically valid");

    let mut dense = vec![s(0); 1_usize << 5];
    dense[0] = s(7);
    dense[1] = s(1);
    dense[31] = r(3, 2);

    let heap = CliffordNumber::from_dense_coeffs(sig5, 5, dense)?;
    println!("heap storage? {}", heap.is_heap_allocated());
    print_clifford("heap CliffordNumber", &heap);

    print_title("Done");

    Ok(())
}
