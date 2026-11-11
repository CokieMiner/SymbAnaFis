//! Multi-output program compilation, shared CSE, VM/JIT/Rayon equivalence, and edge cases.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test expressions use small bounded indices and symbolic operator overloading"
)]

use alloc::{format, vec, vec::Vec};

use num_anafis::{Complex, Number};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr},
    errors::EvalError,
    evaluator::{MultiProgram, Program, Workspace},
};

use super::WasmInspector;

fn same_f64(lhs: f64, rhs: f64, rel_tol: f64) -> bool {
    if lhs.is_nan() && rhs.is_nan() {
        return true;
    }
    if lhs.to_bits() == rhs.to_bits() {
        return true;
    }
    let scale = lhs.abs().max(rhs.abs()).max(1.0);
    (lhs - rhs).abs() <= rel_tol * scale
}

fn same_f32(lhs: f32, rhs: f32, rel_tol: f32) -> bool {
    if lhs.is_nan() && rhs.is_nan() {
        return true;
    }
    if lhs.to_bits() == rhs.to_bits() {
        return true;
    }
    let scale = lhs.abs().max(rhs.abs()).max(1.0);
    (lhs - rhs).abs() <= rel_tol * scale
}

proptest! {
    #[test]
    fn multi_program_matches_separate_scalar_programs_across_backends_and_batches(
        raw_xs in proptest::collection::vec(-3.0_f64..=3.0, 1..40),
        raw_ys in proptest::collection::vec(-3.0_f64..=3.0, 1..40),
    ) {
        let len = raw_xs.len().min(raw_ys.len());
        let xs = &raw_xs[..len];
        let ys = &raw_ys[..len];

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let denom = x.pow(2) + y.pow(2) + 1;
        let sin_x = x.sin();
        let cos_x = x.cos();
        let prod = x * y;
        let exprs = [
            (&sin_x * (y + 1).cos() + &prod) / &denom,
            (&cos_x * (y - 1).sin() - &prod) / &denom,
            sin_x + cos_x,
            prod,
        ];
        let params = [x, y];

        let singles: Vec<Program<f64>> = exprs
            .iter()
            .map(|expr| Program::<f64>::compile(expr, &params).expect("single compile"))
            .collect();
        let multi = MultiProgram::<f64>::compile(&exprs, &params).expect("multi compile");
        prop_assert_eq!(multi.output_count(), exprs.len());

        // Single-row equivalence (VM).
        let mut ws = Workspace::new();
        let mut out_row = vec![0.0_f64; exprs.len()];
        for (&xv, &yv) in xs.iter().zip(ys) {
            multi
                .eval_with(&mut ws, &[xv, yv], &mut out_row)
                .expect("multi eval_with");
            for (idx, single) in singles.iter().enumerate() {
                let expected = single.eval(&[xv, yv]).expect("single eval");
                prop_assert!(same_f64(out_row[idx], expected, 1.0e-12));
            }
        }

        // Batch sequential + parallel equivalence (VM).
        let mut expected_cols: Vec<Vec<f64>> = vec![vec![0.0; len]; exprs.len()];
        for (single, col) in singles.iter().zip(&mut expected_cols) {
            single.eval_batch(&[xs, ys], col).expect("single batch");
        }
        let mut vm_cols: Vec<Vec<f64>> = vec![vec![0.0; len]; exprs.len()];
        {
            let mut refs: Vec<&mut [f64]> = vm_cols.iter_mut().map(Vec::as_mut_slice).collect();
            multi.eval_batch(&[xs, ys], &mut refs).expect("multi batch");
        }
        for (got, exp) in vm_cols.iter().zip(&expected_cols) {
            for (&g, &e) in got.iter().zip(exp) {
                prop_assert!(same_f64(g, e, 1.0e-12));
            }
        }

        #[cfg(feature = "rayon")]
        {
            let mut par_cols: Vec<Vec<f64>> = vec![vec![0.0; len]; exprs.len()];
            {
                let mut refs: Vec<&mut [f64]> =
                    par_cols.iter_mut().map(Vec::as_mut_slice).collect();
                multi
                    .eval_batch_par(&[xs, ys], &mut refs)
                    .expect("multi batch par");
            }
            for (got, exp) in par_cols.iter().zip(&expected_cols) {
                for (&g, &e) in got.iter().zip(exp) {
                    prop_assert!(same_f64(g, e, 1.0e-12));
                }
            }
        }

        // Native JIT scalar + batch + parallel equivalence.
        #[cfg(native_jit_host)]
        {
            let native = multi.with_native().expect("native multi compile");
            prop_assert!(native.native_bytes().is_some());
            let mut nws = Workspace::new();
            let mut nrow = vec![0.0_f64; exprs.len()];
            for (&xv, &yv) in xs.iter().zip(ys) {
                native
                    .eval_with(&mut nws, &[xv, yv], &mut nrow)
                    .expect("native eval_with");
                for (idx, single) in singles.iter().enumerate() {
                    let expected = single.eval(&[xv, yv]).expect("single eval");
                    prop_assert!(same_f64(nrow[idx], expected, 1.0e-12));
                }
            }
            let mut n_cols: Vec<Vec<f64>> = vec![vec![0.0; len]; exprs.len()];
            {
                let mut refs: Vec<&mut [f64]> = n_cols.iter_mut().map(Vec::as_mut_slice).collect();
                native
                    .eval_batch(&[xs, ys], &mut refs)
                    .expect("native batch");
            }
            for (got, exp) in n_cols.iter().zip(&expected_cols) {
                for (&g, &e) in got.iter().zip(exp) {
                    prop_assert!(same_f64(g, e, 1.0e-12));
                }
            }
            #[cfg(feature = "rayon")]
            {
                let mut n_par_cols: Vec<Vec<f64>> = vec![vec![0.0; len]; exprs.len()];
                {
                    let mut refs: Vec<&mut [f64]> =
                        n_par_cols.iter_mut().map(Vec::as_mut_slice).collect();
                    native
                        .eval_batch_par(&[xs, ys], &mut refs)
                        .expect("native batch par");
                }
                for (got, exp) in n_par_cols.iter().zip(&expected_cols) {
                    for (&g, &e) in got.iter().zip(exp) {
                        prop_assert!(same_f64(g, e, 1.0e-12));
                    }
                }
            }
        }
    }
}

#[test]
fn multi_program_shares_cse_reciprocals_and_paired_trig_across_outputs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let denom = x.pow(2) + y.pow(2) + 2;
    let e1 = (x.sin() + y) / &denom;
    let e2 = (x.cos() - y) / &denom;

    let p1 = Program::<f64>::compile(&e1, &[x, y]).expect("p1");
    let p2 = Program::<f64>::compile(&e2, &[x, y]).expect("p2");
    let joint = MultiProgram::<f64>::compile(&[e1, e2], &[x, y]).expect("joint");

    assert!(
        joint.instruction_count() < p1.instruction_count() + p2.instruction_count(),
        "expected shared CSE to reduce instruction count: {} vs {} + {}",
        joint.instruction_count(),
        p1.instruction_count(),
        p2.instruction_count()
    );
    let listing = format!("{}", joint.disassemble());
    assert!(listing.contains("sin_cos"));
    assert!(listing.contains("return r"));
    assert!(listing.contains(", r"));
}

#[test]
fn multi_program_handles_empty_single_duplicate_and_passthrough_outputs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");

    // Zero outputs.
    let empty = MultiProgram::<f64>::compile(&[], &[x, y]).expect("empty");
    assert_eq!(empty.output_count(), 0);
    assert_eq!(
        empty.eval(&[1.0, 2.0]).expect("empty eval"),
        Vec::<f64>::new()
    );
    let mut ws = Workspace::new();
    empty
        .eval_with(&mut ws, &[1.0, 2.0], &mut [])
        .expect("empty eval_with");
    empty
        .eval_batch(&[&[1.0, 2.0], &[3.0, 4.0]], &mut [])
        .expect("empty batch");
    assert!(format!("{}", empty.disassemble()).contains("return ()"));
    let empty_wasm = empty.to_wasm_bytes().expect("empty wasm");
    assert_eq!(&empty_wasm[..8], b"\0asm\x01\0\0\0");

    // Duplicate + parameter + constant + computed + subexpression reuse.
    let prod = x * y;
    let exprs = [
        x.to_expr(),
        Expr::number(5),
        prod.clone(),
        prod.clone(),
        prod + 1,
        x.sin(),
        x.cos(),
    ];
    let multi = MultiProgram::<f64>::compile(&exprs, &[x, y]).expect("mixed");
    assert_eq!(multi.output_count(), 7);
    let out = multi.eval(&[0.5, 3.0]).expect("mixed eval");
    assert!(same_f64(out[0], 0.5, 1.0e-15));
    assert!(same_f64(out[1], 5.0, 1.0e-15));
    assert!(same_f64(out[2], 1.5, 1.0e-15));
    assert!(same_f64(out[3], 1.5, 1.0e-15));
    assert!(same_f64(out[4], 2.5, 1.0e-15));
    assert!(same_f64(out[5], 0.5_f64.sin(), 1.0e-14));
    assert!(same_f64(out[6], 0.5_f64.cos(), 1.0e-14));

    // Multi-value WASM export.
    let wasm = multi.to_wasm_bytes().expect("multi wasm");
    assert_eq!(&wasm[..8], b"\0asm\x01\0\0\0");

    // Shared Workspace reuse across Program and MultiProgram.
    let single = Program::<f64>::compile(&(x + y), &[x, y]).expect("single");
    assert!(same_f64(
        single.eval_with(&mut ws, &[2.0, 3.0]).expect("single ws"),
        5.0,
        1.0e-15
    ));
    let mut buf = [0.0_f64; 7];
    multi
        .eval_with(&mut ws, &[0.5, 3.0], &mut buf)
        .expect("rebound ws");
    assert!(same_f64(buf[4], 2.5, 1.0e-15));
    assert!(same_f64(
        single
            .eval_with(&mut ws, &[4.0, 6.0])
            .expect("back to single"),
        10.0,
        1.0e-15
    ));

    // Native JIT on empty and mixed programs.
    #[cfg(native_jit_host)]
    {
        let empty_native = empty.with_native().expect("empty native");
        assert_eq!(
            empty_native.eval(&[1.0, 2.0]).expect("empty native eval"),
            Vec::<f64>::new()
        );
        empty_native
            .eval_with(&mut ws, &[1.0, 2.0], &mut [])
            .expect("empty native eval_with");
        let mixed_native = multi.with_native().expect("mixed native");
        let nout = mixed_native.eval(&[0.5, 3.0]).expect("mixed native eval");
        for (a, b) in out.iter().zip(&nout) {
            assert!(same_f64(*a, *b, 1.0e-13));
        }
    }
}

#[test]
fn multi_program_supports_f32_complex_and_exact_number_backends() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let exprs = [x + y, x * y];

    // Exact Number backend.
    let num_prog = MultiProgram::<Number>::compile(&exprs, &[x, y]).expect("number");
    let num_out = num_prog
        .eval(&[Number::from_int(3), Number::from_int(4)])
        .expect("number eval");
    assert_eq!(num_out, vec![Number::from_int(7), Number::from_int(12)]);

    // f32 backend (VM + Native JIT).
    let float32_prog = MultiProgram::<f32>::compile(&exprs, &[x, y]).expect("f32");
    let mut o1 = [0.0_f32; 5];
    let mut o2 = [0.0_f32; 5];
    let xs = [1.0_f32, 2.0, 3.0, 4.0, 5.0];
    let ys = [2.0_f32, 3.0, 4.0, 5.0, 6.0];
    float32_prog
        .eval_batch(&[&xs, &ys], &mut [&mut o1, &mut o2])
        .expect("f32 batch");
    for i in 0..5 {
        assert!(same_f32(o1[i], xs[i] + ys[i], 1.0e-6));
        assert!(same_f32(o2[i], xs[i] * ys[i], 1.0e-6));
    }

    // Complex<f64> backend (VM + Native JIT).
    let complex_prog = MultiProgram::<Complex<f64>>::compile(&exprs, &[x, y]).expect("c64");
    let z1 = Complex { re: 1.0, im: 2.0 };
    let z2 = Complex { re: 3.0, im: -1.0 };
    let c_out = complex_prog.eval(&[z1, z2]).expect("c64 eval");
    assert!(same_f64(c_out[0].re, 4.0, 1.0e-14));
    assert!(same_f64(c_out[0].im, 1.0, 1.0e-14));
    assert!(same_f64(c_out[1].re, 5.0, 1.0e-14));
    assert!(same_f64(c_out[1].im, 5.0, 1.0e-14));

    #[cfg(native_jit_host)]
    {
        let float32_native = float32_prog.with_native().expect("f32 native");
        let mut no1 = [0.0_f32; 5];
        let mut no2 = [0.0_f32; 5];
        float32_native
            .eval_batch(&[&xs, &ys], &mut [&mut no1, &mut no2])
            .expect("f32 native batch");
        for i in 0..5 {
            assert!(same_f32(no1[i], o1[i], 1.0e-6));
            assert!(same_f32(no2[i], o2[i], 1.0e-6));
        }

        let complex_native = complex_prog.with_native().expect("c64 native");
        let nc_out = complex_native.eval(&[z1, z2]).expect("c64 native eval");
        assert!(same_f64(nc_out[0].re, 4.0, 1.0e-14));
        assert!(same_f64(nc_out[0].im, 1.0, 1.0e-14));
        assert!(same_f64(nc_out[1].re, 5.0, 1.0e-14));
        assert!(same_f64(nc_out[1].im, 5.0, 1.0e-14));
    }
}

#[test]
fn multi_program_reports_explicit_validation_errors() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let multi = MultiProgram::<f64>::compile(&[x + y, x - y], &[x, y]).expect("compile");

    let mut ws = Workspace::new();
    let mut out = [0.0_f64; 1];
    assert_eq!(
        multi.eval_with(&mut ws, &[1.0, 2.0], &mut out),
        Err(EvalError::OutputCount {
            expected: 2,
            actual: 1,
        })
    );
    let mut out2 = [0.0_f64; 2];
    assert_eq!(
        multi.eval_with(&mut ws, &[1.0], &mut out2),
        Err(EvalError::ParameterCount {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        multi.eval(&[1.0]),
        Err(EvalError::ParameterCount {
            expected: 2,
            actual: 1,
        })
    );

    let mut o1 = [0.0_f64; 3];
    let mut o2 = [0.0_f64; 2];
    assert_eq!(
        multi.eval_batch(
            &[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]],
            &mut [&mut o1, &mut o2]
        ),
        Err(EvalError::OutputLength {
            output: 1,
            expected: 3,
            actual: 2,
        })
    );
    let mut o2_ok = [0.0_f64; 3];
    assert_eq!(
        multi.eval_batch(&[&[1.0, 2.0, 3.0], &[4.0, 5.0]], &mut [&mut o1, &mut o2_ok]),
        Err(EvalError::BatchLength {
            parameter: 1,
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(
        multi.eval_batch(&[&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]], &mut [&mut o1]),
        Err(EvalError::OutputCount {
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn multi_program_preserves_shared_negated_output_during_arithmetic_fusion() {
    // Regression test for `optimize_arithmetic::binary_sum`: if an output is
    // `-(x * y)` or `-x` AND another output adds `z` to it, the fusion pass
    // must NOT mutate the shared output node in place via `unnegate`.
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let z = context.symb("z").expect("z");

    let neg_prod = -(x * y);
    let neg_x = -x;
    let exprs = [neg_prod.clone(), neg_prod + z, neg_x.clone(), neg_x + z];
    let multi = MultiProgram::<f64>::compile(&exprs, &[x, y, z]).expect("compile");
    let out = multi.eval(&[2.0, 3.0, 10.0]).expect("eval");
    assert!(same_f64(out[0], -6.0, 1.0e-15));
    assert!(same_f64(out[1], 4.0, 1.0e-15));
    assert!(same_f64(out[2], -2.0, 1.0e-15));
    assert!(same_f64(out[3], 8.0, 1.0e-15));

    #[cfg(native_jit_host)]
    {
        let native = multi.with_native().expect("native");
        let nout = native.eval(&[2.0, 3.0, 10.0]).expect("native eval");
        for (a, b) in out.iter().zip(&nout) {
            assert!(same_f64(*a, *b, 1.0e-14));
        }
    }
}

#[test]
fn multi_program_emits_valid_multivalue_wasm_type_section() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");

    // 2 params -> 3 f64 results, pure inline ops (no imports).
    let multi = MultiProgram::<f64>::compile(&[x + y, x - y, x * y], &[x, y]).expect("compile");
    let wasm = multi.to_wasm_bytes().expect("wasm");
    let sections = WasmInspector::sections(&wasm);
    let (_, type_sec) = sections
        .iter()
        .find(|&&(id, _)| id == 1)
        .expect("type section");
    // 1 type: 0x60, 2 params (f64, f64), 3 results (f64, f64, f64)
    assert_eq!(type_sec, &[1, 0x60, 2, 0x7C, 0x7C, 3, 0x7C, 0x7C, 0x7C]);

    // 1 param -> 2 f32 results with math imports (sin/cos share type (1 -> 1), eval has (1 -> 2)).
    let trig = MultiProgram::<f32>::compile(&[x.sin(), x.cos()], &[x]).expect("trig");
    let trig_wasm = trig.to_wasm_bytes().expect("trig wasm");
    let trig_sections = WasmInspector::sections(&trig_wasm);
    let (_, trig_types) = trig_sections
        .iter()
        .find(|&&(id, _)| id == 1)
        .expect("trig type section");
    // 2 types: (f32)->(f32) for math imports, (f32)->(f32, f32) for eval.
    assert_eq!(
        trig_types,
        &[2, 0x60, 1, 0x7D, 1, 0x7D, 0x60, 1, 0x7D, 2, 0x7D, 0x7D]
    );
}

#[test]
fn complex_blocked_batches_and_sparse_active_parameters_match_scalar() {
    let context = Context::new().expect("context");
    let unused_a = context.symb("unused_a").expect("unused_a");
    let x = context.symb("x").expect("x");
    let unused_b = context.symb("unused_b").expect("unused_b");
    let y = context.symb("y").expect("y");
    let expr = (x * y + x.sin()) / (y + 2);

    let prog = Program::<Complex<f64>>::compile(&expr, &[unused_a, x, unused_b, y])
        .expect("compile sparse complex");
    assert_eq!(&*prog.data.active_parameters, &[1, 3]);

    // 21 rows cross the 16-lane blocked VM batch boundary (1 full block + 5 tail).
    let rows = 21_usize;
    let dummy = vec![
        Complex {
            re: 999.0,
            im: -999.0
        };
        rows
    ];
    let xs: Vec<Complex<f64>> = (0_u32..21)
        .map(|i| Complex {
            re: 0.1_f64.mul_add(f64::from(i), 0.2),
            im: (-0.05_f64).mul_add(f64::from(i), 0.1),
        })
        .collect();
    let ys: Vec<Complex<f64>> = (0_u32..21)
        .map(|i| Complex {
            re: 0.07_f64.mul_add(f64::from(i), 0.5),
            im: 0.03_f64.mul_add(f64::from(i), -0.2),
        })
        .collect();
    let mut batch_out = vec![Complex { re: 0.0, im: 0.0 }; rows];
    prog.eval_batch(&[&dummy, &xs, &dummy, &ys], &mut batch_out)
        .expect("blocked complex batch");
    for row in 0..rows {
        let scalar = prog
            .eval(&[dummy[row], xs[row], dummy[row], ys[row]])
            .expect("scalar complex");
        assert_eq!(batch_out[row].re.to_bits(), scalar.re.to_bits());
        assert_eq!(batch_out[row].im.to_bits(), scalar.im.to_bits());
    }
}
