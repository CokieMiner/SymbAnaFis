use crate::{Diff, Expr, Simplify, symb};

/// Helper macro to ensure String API and Object API produce identical results.
/// This enforces that every feature available via parsing is also available via method chaining.
macro_rules! assert_api_parity {
    ($name:ident, $string_expr:expr, $object_expr:expr) => {
        #[test]
        fn $name() {
            let s_api = crate::parse(
                $string_expr,
                &std::collections::HashSet::new(),
                &std::collections::HashSet::new(),
                None,
            )
            .expect("String Parse Failed");
            let o_api = $object_expr;

            assert_eq!(
                s_api, o_api,
                "String API vs Object API mismatch for {}",
                $string_expr
            );
        }
    };
}

// 1. Core Function Parity
assert_api_parity!(parity_sin, "sin(x)", symb("x").sin());
assert_api_parity!(parity_pow, "x^2", symb("x").pow(2.0));
assert_api_parity!(parity_add, "x + y", symb("x") + symb("y"));
assert_api_parity!(parity_log, "log(10, x)", symb("x").log(10.0));

// 2. Builder Parity: Fixed Variables
#[test]
fn parity_fixed_vars() {
    let x = symb("x");
    let a = symb("a");

    // String API (implicit in diff args or explicit in builder?)
    // The string API `diff("a*x", "x", &["a"])` uses the string slice for known symbols.
    // The Object API `Diff::new().fixed_var(&a)` uses the symbol reference.

    let str_res = crate::diff("a*x", "x", &["a"], None).expect("String diff failed");

    let obj_res = Diff::new()
        .fixed_var(&a)
        .differentiate(&(a * x), &x)
        .expect("Object diff failed");

    assert_eq!(
        str_res,
        format!("{}", obj_res),
        "Diff Builder Parity Failed"
    );
}

// 3. Builder Parity: Simplify with Fixed Vars
#[test]
fn parity_simplify_fixed() {
    let x = symb("x");
    let k = symb("k");

    let str_res = crate::simplify("k*x + k*x", &["k"], None).expect("String simplify failed");

    let obj_res = Simplify::new()
        .fixed_var(&k)
        .simplify(&(k * x + k * x))
        .expect("Object simplify failed");

    assert_eq!(
        str_res,
        format!("{}", obj_res),
        "Simplify Builder Parity Failed"
    );
}

// 4. Custom Function Parity
// Ensuring that if we can parse "f(x)", we can also build it manually
#[test]
fn parity_custom_functions() {
    let x = symb("x");

    // String API: parse "f(x)" with known function "f"
    let mut funcs = std::collections::HashSet::new();
    funcs.insert("f".to_string());
    let mut funcs = std::collections::HashSet::new();
    funcs.insert("f".to_string());
    let str_expr = crate::parse("f(x)", &std::collections::HashSet::new(), &funcs, None).unwrap();

    // Object API: Expr::func("f", x)
    let obj_expr = Expr::func("f", x.to_expr());

    assert_eq!(str_expr, obj_expr, "Custom Function API Parity Failed");
}

// 5. Evaluation Parity (Flexible Inputs)
// Ensure eval_f64 works with both Strings and Symbols in the variable list
#[cfg(feature = "parallel")]
#[test]
fn parity_eval_inputs() {
    use crate::eval_f64;

    let x = symb("x");
    let expr = x.pow(2.0);
    let data = vec![2.0];

    // Case A: Using Strings
    let res_str = eval_f64(&[&expr], &[&["x"]], &[&[&data]]).unwrap();

    // Case B: Using Symbols
    let res_sym = eval_f64(&[&expr], &[&[&x]], &[&[&data]]).unwrap();

    assert_eq!(res_str, res_sym, "eval_f64 input parity failed");
}

#[test]
fn ensure_string_and_typesafe_apis_match() {
    use crate::{Symbol, parser::parse, symb};

    let x = symb("x");
    let y = symb("y");

    let p = |s: &str| {
        parse(
            s,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            None,
        )
        .unwrap()
    };

    macro_rules! check_unary {
        ($($method:ident),*) => {
            $(
                let str_form = format!("{}(x)", stringify!($method));
                let parsed = p(&str_form);
                let typed = x.$method();
                assert_eq!(parsed, typed, "Mismatch for unary function {}", stringify!($method));
            )*
        };
    }

    // Macro para funções binárias "normais" (simétricas ou ordem direta)
    // Ex: atan2(y, x) -> y.atan2(x)
    macro_rules! check_binary_direct {
        ($($method:ident),*) => {
            $(
                let str_form = format!("{}(x, y)", stringify!($method));
                let parsed = p(&str_form);
                let typed = x.$method(y);
                assert_eq!(parsed, typed, "Mismatch for binary function {}", stringify!($method));
            )*
        };
    }

    // --- 1. Funções Unárias (Tudo igual) ---
    check_unary!(
        sin, cos, tan, cot, sec, csc, asin, acos, atan, acot, asec, acsc, sinh, cosh, tanh, coth,
        sech, csch, asinh, acosh, atanh, acoth, asech, acsch, exp, ln, log10, log2, sqrt, cbrt,
        abs, signum, floor, ceil, round, erf, erfc, gamma, digamma, trigamma, zeta, sinc, lambertw,
        exp_polar
    );

    // --- 2. Funções Binárias Diretas ---
    // atan2(y, x) e beta(a, b) seguem a ordem direta
    check_binary_direct!(atan2, beta);

    // --- 3. Funções Paramétricas (Ordem Invertida no Método) ---
    // O método é x.func(n), mas o parser lê func(n, x).
    // Logo, x.func(y) deve ser igual a parse("func(y, x)")

    type ParametricFn = (&'static str, fn(Symbol, Symbol) -> Expr);
    let funcs: [ParametricFn; 8] = [
        ("polygamma", |a, b| a.polygamma(b)),
        ("besselj", |a, b| a.besselj(b)),
        ("bessely", |a, b| a.bessely(b)),
        ("besseli", |a, b| a.besseli(b)),
        ("besselk", |a, b| a.besselk(b)),
        ("hermite", |a, b| a.hermite(b)),
        ("zeta_deriv", |a, b| a.zeta_deriv(b)),
        ("log", |a, b| a.log(b)), // x.log(base) -> log(base, x)
    ];

    for (name, method) in funcs {
        // Método: x.func(y)
        let typed = method(x, y);
        // Parser: func(y, x)  <-- NOTA A TROCA DE ORDEM AQUI
        let str_form = format!("{}(y, x)", name);
        let parsed = p(&str_form);

        assert_eq!(parsed, typed, "Mismatch for parametric function {}", name);
    }
}
