//! General random-DAG differential property tests across direct evaluation,
//! VM scalar/batch/Rayon, native JIT (all `x86_64` tiers + `f32`/`f64`/`Complex`),
//! cross-architecture encoders (`aarch64`/`riscv64`), and `.wasm` execution.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "property test DAG generators use bounded indices and symbolic operator overloading"
)]

use alloc::{format, vec, vec::Vec};

use num_anafis::Complex;
use proptest::{prelude::ProptestConfig, prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr, Symbol},
    evaluator::Program,
};
#[cfg(all(native_jit_host, target_arch = "x86_64"))]
use crate::evaluator::Workspace;

use super::WasmInspector;

fn same_f64_bits(lhs: f64, rhs: f64) -> bool {
    (lhs.is_nan() && rhs.is_nan()) || lhs.to_bits() == rhs.to_bits()
}

fn same_f32_bits(lhs: f32, rhs: f32) -> bool {
    (lhs.is_nan() && rhs.is_nan()) || lhs.to_bits() == rhs.to_bits()
}

fn same_complex_bits(lhs: Complex<f64>, rhs: Complex<f64>) -> bool {
    same_f64_bits(lhs.re, rhs.re) && same_f64_bits(lhs.im, rhs.im)
}

/// Build an arbitrary shared DAG from random `(op, a, b, c)` steps over `syms`.
///
/// Every step appends its result to `pool` and may reference any earlier node,
/// generating arbitrary sharing, fan-out, register pressure, FMA candidates,
/// paired trig/hyperbolic calls, shared denominators, and long sums/products.
fn build_random_dag(syms: &[Symbol<'_>], steps: &[(u8, u16, u16, u16)], real_only: bool) -> Expr {
    let mut pool: Vec<Expr> = syms.iter().copied().map(Expr::from).collect();
    pool.push(Expr::number(2));
    pool.push(Expr::number(-3));

    for &(op, a_raw, b_raw, c_raw) in steps {
        let len = pool.len();
        let a = pool[usize::from(a_raw) % len].clone();
        let b = pool[usize::from(b_raw) % len].clone();
        let c = pool[usize::from(c_raw) % len].clone();
        let node = match op % 22 {
            0 => a + b,
            1 => a - b,
            2 => a * b,
            3 => a / (b.clone() * b + 1),
            4 => -(a * b),
            5 => a * b + c,
            6 => a * b - c,
            7 => c - a * b,
            8 => a.clone().sin() + a.cos(),
            9 => a.clone().sinh() - a.cosh(),
            10 => (a.clone() * a + 1).sqrt(),
            11 => a.abs(),
            12 => (a * 0.25).exp(),
            13 => (a.abs() + 1).ln(),
            14 => a.pow(i64::from((c_raw % 4) + 2)),
            15 => (a.abs() + 1).pow(b * 0.25),
            16 => Expr::sum(vec![
                a.clone(),
                b.clone(),
                c.clone(),
                a.clone() * b.clone(),
                b * c.clone(),
                a * c,
            ]),
            17 => Expr::product(vec![a.clone(), b, c, a + 1]),
            18 if real_only => a.floor() + b.ceil() - c.trunc(),
            19 if real_only => a.round() + b.erf(),
            20 if real_only => a.atan2(b + 2),
            21 if real_only => (a.abs() + 1).log(b.abs() + 2),
            _ => a.sin() * b.cos() + c,
        };
        pool.push(node);
    }

    // Combine the last few nodes so dead branches don't collapse the whole DAG.
    let tail = pool.len().min(4);
    Expr::sum(pool[pool.len() - tail..].to_vec())
}

#[cfg(native_jit_host)]
fn check_native_real_backends(
    prog64: &Program<f64>,
    prog32: &Program<f32>,
    row64: &[f64],
    row32: &[f32],
    vm64: f64,
    vm32: f32,
) -> Result<(), proptest::test_runner::TestCaseError> {
    use super::super::native::{AArch64Backend, NativeEmitter, NativeLayout, Riscv64Backend};

    let nat64 = prog64.with_native()?;
    let got64 = nat64.eval(row64)?;
    prop_assert!(
        same_f64_bits(got64, vm64),
        "f64 native={got64:?} vs vm={vm64:?}"
    );

    let nat32 = prog32.with_native()?;
    let got32 = nat32.eval(row32)?;
    prop_assert!(
        same_f32_bits(got32, vm32),
        "f32 native={got32:?} vs vm={vm32:?}"
    );

    // SIMD batch (17 rows = full blocks + tail) across both f64 and f32.
    let rows = 17_usize;
    let cols64: Vec<Vec<f64>> = row64
        .iter()
        .map(|&v| {
            (0_u32..17)
                .map(|r| 0.03_f64.mul_add(f64::from(r), v))
                .collect()
        })
        .collect();
    let col_refs64: Vec<&[f64]> = cols64.iter().map(Vec::as_slice).collect();
    let mut vm_batch64 = vec![0.0_f64; rows];
    let mut nat_batch64 = vec![0.0_f64; rows];
    prog64.eval_batch(&col_refs64, &mut vm_batch64)?;
    nat64.eval_batch(&col_refs64, &mut nat_batch64)?;
    for (lhs, rhs) in nat_batch64.iter().zip(&vm_batch64) {
        prop_assert!(same_f64_bits(*lhs, *rhs));
    }

    #[cfg(feature = "rayon")]
    {
        let mut par_out = vec![0.0_f64; rows];
        nat64.eval_batch_par(&col_refs64, &mut par_out)?;
        for (lhs, rhs) in par_out.iter().zip(&vm_batch64) {
            prop_assert!(same_f64_bits(*lhs, *rhs));
        }
    }

    let cols32: Vec<Vec<f32>> = row32
        .iter()
        .map(|&v| {
            (0_u16..17)
                .map(|r| 0.03_f32.mul_add(f32::from(r), v))
                .collect()
        })
        .collect();
    let col_refs32: Vec<&[f32]> = cols32.iter().map(Vec::as_slice).collect();
    let mut vm_batch32 = vec![0.0_f32; rows];
    let mut nat_batch32 = vec![0.0_f32; rows];
    prog32.eval_batch(&col_refs32, &mut vm_batch32)?;
    nat32.eval_batch(&col_refs32, &mut nat_batch32)?;
    for (lhs, rhs) in nat_batch32.iter().zip(&vm_batch32) {
        prop_assert!(same_f32_bits(*lhs, *rhs));
    }

    // Cross-arch compilation checks on the same arbitrary DAG.
    let mut arm = AArch64Backend::new();
    let (arm_code, arm_start) =
        NativeEmitter::compile(&prog64.data, NativeLayout::F64, false, &mut arm)?;
    prop_assert_eq!((arm_code.len() - arm_start) % 4, 0);

    let mut rv = Riscv64Backend::new();
    let (rv_code, rv_start) =
        NativeEmitter::compile(&prog64.data, NativeLayout::F64, false, &mut rv)?;
    prop_assert_eq!((rv_code.len() - rv_start) % 4, 0);

    Ok(())
}

#[cfg(all(native_jit_host, target_arch = "x86_64"))]
#[expect(
    unsafe_code,
    reason = "differential test executes unfused SSE2/AVX machine code directly as a bit-exact oracle for WASM"
)]
fn eval_x86_unfused_f64(prog: &Program<f64>, row: &[f64]) -> f64 {
    use core::mem::transmute;
    use std::arch::is_x86_feature_detected;

    use super::super::native::{
        ExecutableBuffer, FEAT_AVX, FEAT_SSE41, NativeEmitter, NativeLayout, Trampolines,
        X86_64Backend,
    };

    let features = if is_x86_feature_detected!("avx") {
        FEAT_SSE41 | FEAT_AVX
    } else if is_x86_feature_detected!("sse4.1") {
        FEAT_SSE41
    } else {
        0
    };
    let mut be = X86_64Backend::with_features(features);
    let (code, start) = NativeEmitter::compile(&prog.data, NativeLayout::F64, false, &mut be)
        .expect("unfused x86 compile");
    let buf = ExecutableBuffer::new(&code[start..]).expect("exec buf");
    let mut ws = Workspace::new();
    ws.prepare::<1>(&prog.data);
    ws.registers[..row.len()].copy_from_slice(row);
    let tr = Trampolines::table(NativeLayout::F64).as_ptr().cast::<()>();
    // SAFETY: `buf` holds a valid scalar f64 kernel matching `ws.registers` and `tr`.
    unsafe {
        let func: unsafe extern "C" fn(*mut f64, *const ()) -> f64 = transmute(buf.as_ptr());
        func(ws.registers.as_mut_ptr(), tr)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn random_real_dag_matches_across_vm_native_wasm_and_cross_arch(
        n_vars in 3_usize..=18,
        steps in proptest::collection::vec((0_u8..64, 0_u16..256, 0_u16..256, 0_u16..256), 1..=35),
        raw_inputs in proptest::collection::vec(-1.5_f32..=1.5, 18),
    ) {
        let context = Context::new()?;
        let syms: Vec<_> = (0..n_vars)
            .map(|i| context.symb(&format!("x{i}")).expect("sym"))
            .collect();
        let expr = build_random_dag(&syms, &steps, true);

        let prog64 = Program::<f64>::compile(&expr, &syms)?;
        let prog32 = Program::<f32>::compile(&expr, &syms)?;
        let row32 = &raw_inputs[..n_vars];
        let row64_vec: Vec<f64> = row32.iter().copied().map(f64::from).collect();
        let row64 = row64_vec.as_slice();

        let vm64 = prog64.eval(row64)?;
        let vm32 = prog32.eval(row32)?;
        let mut vm32_batch = [0.0_f32; 1];
        let row32_cols: Vec<&[f32]> = row32.iter().map(core::slice::from_ref).collect();
        prog32.eval_batch(&row32_cols, &mut vm32_batch)?;
        prop_assert!(same_f32_bits(vm32_batch[0], vm32));

        // Standalone WASM module must parse cleanly and execute on the stack machine.
        let wasm = prog64.to_wasm_bytes()?;
        let sections = WasmInspector::sections(&wasm);
        prop_assert!(!sections.is_empty());
        let wasm_val = WasmInspector::eval_f64(&wasm, row64).expect("wasm execution");

        #[cfg(all(native_jit_host, target_arch = "x86_64"))]
        {
            // WASM lowers `Fused` to unfused mul + add/sub, matching non-FMA x86_64 bit-for-bit.
            let unfused = eval_x86_unfused_f64(&prog64, row64);
            prop_assert!(
                same_f64_bits(wasm_val, unfused),
                "wasm={wasm_val:?} vs unfused_x86={unfused:?}"
            );
        }

        #[cfg(not(all(native_jit_host, target_arch = "x86_64")))]
        {
            if vm64.is_nan() {
                prop_assert!(wasm_val.is_nan());
            } else if vm64.is_infinite() {
                prop_assert_eq!(wasm_val.is_infinite(), vm64.is_infinite());
            }
        }

        #[cfg(native_jit_host)]
        check_native_real_backends(&prog64, &prog32, row64, row32, vm64, vm32)?;
    }

    #[test]
    fn random_complex_dag_matches_direct_vm_and_native_bit_exact(
        n_vars in 2_usize..=10,
        steps in proptest::collection::vec((0_u8..64, 0_u16..128, 0_u16..128, 0_u16..128), 1..=22),
        raw_re in proptest::collection::vec(-1.2_f64..=1.2, 10),
        raw_im in proptest::collection::vec(-1.2_f64..=1.2, 10),
    ) {
        let context = Context::new()?;
        let syms: Vec<_> = (0..n_vars)
            .map(|i| context.symb(&format!("z{i}")).expect("sym"))
            .collect();
        let expr = build_random_dag(&syms, &steps, false);

        let inputs: Vec<Complex<f64>> = (0..n_vars)
            .map(|i| Complex {
                re: raw_re[i],
                im: raw_im[i],
            })
            .collect();
        let bindings: Vec<_> = syms.iter().copied().zip(inputs.iter().copied()).collect();
        let direct = expr.evaluate(bindings)?;

        let prog = Program::<Complex<f64>>::compile(&expr, &syms)?;
        let vm = prog.eval(&inputs)?;
        prop_assert!(
            same_complex_bits(direct, vm),
            "complex direct={direct:?} vs vm={vm:?}"
        );

        #[cfg(native_jit_host)]
        {
            let nat = prog.with_native()?;
            let got = nat.eval(&inputs)?;
            prop_assert!(
                same_complex_bits(got, vm),
                "complex native={got:?} vs vm={vm:?}"
            );
        }
    }
}
