//! Comprehensive example showcasing *everything* exported by `num-anafis`!
//!
//! Run with: cargo run --example example --features "backend64,clifford"

#![allow(
    clippy::print_stdout,
    clippy::use_debug,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::non_ascii_literal,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Examples must print and debug-print values; convenience over strict linting"
)]

use num_anafis::{
    AnafisMathExt, FloatType, IntType, IntoScalar, NumAnafisError, Number, RationalType, Scalar, r,
    s,
};

#[cfg(feature = "clifford")]
use num_anafis::{
    CliffordNumber, FastClifford, GeneratorSet, cga_gens, ci, e_minus, e_plus, e1, e2, e3, eps,
    inf, orig, pseudo3d, pseudo5d, qi, qj, qk, sj,
};

fn title(t: &str) {
    println!("\n━━━ {} ━━━", t);
}

#[cfg(feature = "clifford")]
fn put(label: &str, val: &CliffordNumber) {
    println!("  {:<25} = {}", label, val);
}

fn main() {
    println!("============================================================");
    println!("  Welcome to num-anafis: Geometric Algebra Engine           ");
    println!("============================================================");

    // ==================================================================
    // 1. Primitive Types & Traits
    // ==================================================================
    title("1. Types & Traits (IntType, FloatType, RationalType, AnafisMathExt)");
    {
        #[cfg(feature = "backend64")]
        let my_float: FloatType = core::f64::consts::PI;
        #[cfg(feature = "backend32")]
        let my_float: FloatType = core::f32::consts::PI;
        #[cfg(feature = "backendrug")]
        let my_float: FloatType = FloatType::with_val(53, core::f64::consts::PI);

        println!(
            "  FloatType ({})         = {}",
            core::any::type_name::<FloatType>(),
            my_float
        );
        println!(
            "  my_float.lgamma()       = {} (via AnafisMathExt)",
            my_float.lgamma()
        );
    }

    #[cfg(feature = "backendrug")]
    let my_int: IntType = IntType::from(42);
    #[cfg(not(feature = "backendrug"))]
    let my_int: IntType = 42;

    println!(
        "  IntType ({})           = {}",
        core::any::type_name::<IntType>(),
        my_int
    );

    println!(
        "  RationalType ({})      = fractional backend",
        core::any::type_name::<RationalType>()
    );

    // ==================================================================
    // 2. Scalars & Conversions
    // ==================================================================
    title("2. Scalars & Conversions (Scalar, IntoScalar, s, r)");
    let sc1: Scalar = s(42);
    let sc2: Scalar = r(22, 7);

    let sc3: Scalar = 100.into_scalar();
    let sc4: Scalar = core::f64::consts::PI.into_scalar();

    println!("  Integer 's(42)'       = {}", sc1);
    println!("  Rational 'r(22, 7)'   = {}", sc2);
    println!("  100.into_scalar()     = {}", sc3);
    println!("  3.14.into_scalar()    = {}", sc4);
    println!("  Number trait (exp)    = {}", sc1.exp());

    // ==================================================================
    // 3. Error Handling
    // ==================================================================
    title("3. Error Handling (NumAnafisError)");
    #[cfg(feature = "clifford")]
    {
        let zero_mv = CliffordNumber::scalar(GeneratorSet::empty(), s(0)).unwrap();
        println!(
            "  Inverse of 0          = {:?}",
            zero_mv.geometric_inverse()
        );
    }
    #[cfg(not(feature = "clifford"))]
    println!("  (clifford feature disabled — skipping geometric algebra examples)");
    let err_example = NumAnafisError::InlineCoefficientsRequireAtMostFiveGenerators { active: 10 };
    println!("  Demonstrating Error   = {:?}", err_example);

    // ==================================================================
    // Clifford-only sections (feature gate)
    // ==================================================================
    #[cfg(feature = "clifford")]
    {
        // ==============================================================
        // 4. Generator Sets (GeneratorSet, cga_gens)
        // ==============================================================
        title("4. Generator Sets");
        let empty_gen = GeneratorSet::empty();
        let cga = cga_gens();
        println!("  Empty GeneratorSet    = {:?}", empty_gen);
        println!("  CGA GeneratorSet      = {} dimensions active", cga.len());

        // ==============================================================
        // 5. Convenience Constructors (e1..e3, ci, sj, eps, etc.)
        // ==============================================================
        title("5. Built-in Geometric Algebra Generators");
        put("Complex 'ci' (ci² = -1)", &ci());
        put("Split-Complex 'sj' (sj² = +1)", &sj());
        put("Dual 'ε' (ε² = 0)", &eps());
        put("Quaternions 'qi'", &qi());
        put("Quaternions 'qj'", &qj());
        put("Quaternions 'qk'", &qk());

        put("e1 (e1² = +1)", &e1());
        put("e2 (e2² = +1)", &e2());
        put("e3 (e3² = +1)", &e3());

        put("3D Pseudoscalar", &pseudo3d());
        put("5D Pseudoscalar", &pseudo5d());

        // ==============================================================
        // 6. Conformal Geometric Algebra (CGA) Identifiers
        // ==============================================================
        title("6. Conformal Geometric Algebra (CGA) - Cl(4,1)");
        put("Extra positive 'e_plus'", &e_plus());
        put("Extra negative 'e_minus'", &e_minus());
        put("Origin 'orig'", &orig());
        put("Infinity 'inf'", &inf());

        put("orig * inf", &(&orig() * &inf()));

        // ==============================================================
        // 7. Dynamic Multivectors (CliffordNumber)
        // ==============================================================
        title("7. Dynamic Multivectors & Spectral Math (CliffordNumber)");
        let mut mv = &e1() * &s(3) + &(&e1() * &e2()) * &s(4);
        mv = &mv + &pseudo3d();
        put("Multivector 'M'", &mv);

        put("sin(M)", &mv.sin());
        put("sin(M).chop()", &mv.sin().chop());
        put("exp(M)", &mv.exp());
        put("exp(M).chop()", &mv.exp().chop());

        // ==============================================================
        // 8. FastClifford
        // ==============================================================
        title("8. FastClifford: Zero-Allocation Compile-Time Generic Math");
        let mut fc_a = FastClifford::<3, 0, 0>::zero();
        fc_a.coeffs[0] = s(2);
        fc_a.coeffs[1] = s(3);

        let mut fc_b = FastClifford::<3, 0, 0>::zero();
        fc_b.coeffs[1] = s(5);

        println!("  FastClifford A        = {}", fc_a);
        println!("  FastClifford B        = {}", fc_b);
        println!("  A + B                 = {}", fc_a.clone() + fc_b.clone());
        println!("  A * B                 = {}", fc_a.clone() * fc_b);

        println!("  exp(A)                = {}", fc_a.exp());
        println!("  exp(A).chop()         = {}", fc_a.exp().chop());
        println!("  sin(A)                = {}", fc_a.sin());
        println!("  sin(A).chop()         = {}", fc_a.sin().chop());
    }

    println!("\n num-anafis demonstration complete!");
}
