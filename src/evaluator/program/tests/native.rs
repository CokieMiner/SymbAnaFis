//! Differential, determinism, high-pressure register allocation, cross-architecture
//! encoding, and SIMD batch tests for `Program::with_native`.

#![cfg_attr(
    native_jit_host,
    expect(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        reason = "test expressions use small bounded indices and symbolic operator overloading"
    )
)]

#[cfg(not(native_jit_host))]
#[test]
fn native_compilation_returns_unavailable_without_jit_backend() {
    use crate::{core::Context, errors::EvalError, evaluator::Program};

    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let expr = x.pow(2) + 1;
    let program = Program::<f64>::compile(&expr, &[x]).expect("compile");
    assert!(program.native_bytes().is_none());
    assert_eq!(
        program.with_native().expect_err("jit disabled"),
        EvalError::NativeCompilationUnavailable
    );
    let fallback = program.with_native().unwrap_or(program);
    assert_eq!(fallback.eval(&[3.0]).expect("fallback eval"), 10.0);
}

#[cfg(native_jit_host)]
mod host {
    use core::ptr::eq;

    use alloc::{boxed::Box, format, vec, vec::Vec};

    use num_anafis::Complex;

    use crate::{
        core::{Context, Expr},
        evaluator::{Program, Workspace},
    };

    fn next_rng(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    fn random_pair_tree(state: &mut u64, vars: &[Expr], depth: u32) -> Expr {
        if depth == 0 {
            return if next_rng(state) % 3 == 0 {
                let lit = i64::try_from(next_rng(state) % 5).expect("lit") + 2;
                Expr::number(lit)
            } else {
                let idx = usize::try_from(next_rng(state)).expect("idx") % vars.len();
                vars[idx].clone()
            };
        }
        let lhs = random_pair_tree(state, vars, depth - 1);
        let rhs = random_pair_tree(state, vars, depth - 1);
        match next_rng(state) % 4 {
            0 => lhs + rhs,
            1 => lhs * rhs,
            2 => lhs - rhs,
            _ => {
                let arg = lhs * rhs + 1;
                arg.clone().sin() * 3 + arg.cos()
            }
        }
    }

    #[test]
    fn native_f64_and_f32_match_vm_across_fused_folds_pairs_and_builtins() {
        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let fused_variants = (x * y + z) * (x * y - z) + (z - x * y) - (-(x * y) - z);
        let fold_sum = x + y + z + (x * 2) + (y * 3) + (z * 4);
        let fold_prod = x * y * z * (x + 1) * (y + 2);
        let fold_dot = x * y + y * z + z * x + (x + 1) * (y + 1);
        let paired = x.sin() + x.cos() + y.sinh() + y.cosh();
        let inlines = (x * x + 0.5).sqrt()
            + (x - y).abs()
            + (x * 1.7).floor()
            + (y * 1.3).ceil()
            + (z * 2.9).trunc()
            + (x * 2.5).round();
        let specials = y.atan2(x) + x.pow(y) + x.erf() + y.erfc() + x.log(y + 2);

        let full = fused_variants + fold_sum + fold_prod + fold_dot + paired + inlines + specials;

        let prog_f64 = Program::<f64>::compile(&full, &[x, y, z]).expect("f64 compile");
        assert!(prog_f64.native_bytes().is_none());
        let nat_f64 = prog_f64.with_native().expect("f64 native");
        let idempotent = nat_f64.with_native().expect("idempotent with_native");
        let nat_clone = nat_f64.clone();
        assert_eq!(nat_f64.workspace_size(), prog_f64.workspace_size());
        assert_eq!(nat_f64.instruction_count(), prog_f64.instruction_count());
        assert_eq!(nat_f64.parameters().count(), 3);
        let bytes = nat_f64.native_bytes().expect("native bytes");
        assert_ne!(bytes, []);
        assert_eq!(idempotent.native_bytes(), Some(bytes));
        assert_eq!(nat_clone.native_bytes(), Some(bytes));
        assert!(eq(
            bytes.as_ptr(),
            idempotent.native_bytes().expect("same buffer").as_ptr()
        ));

        let inputs_f64 = [0.6_f64, 0.8, -0.4];
        let vm_res64 = prog_f64.eval(&inputs_f64).expect("vm eval f64");
        let nat_res64 = nat_clone.eval(&inputs_f64).expect("native eval f64");
        assert!(vm_res64.is_finite(), "expected finite f64 result");
        assert_eq!(nat_res64.to_bits(), vm_res64.to_bits());

        let prog_f32 = Program::<f32>::compile(&full, &[x, y, z]).expect("f32 compile");
        let nat_f32 = prog_f32.with_native().expect("f32 native");
        let inputs_f32 = [0.6_f32, 0.8, -0.4];
        let vm_res32 = prog_f32.eval(&inputs_f32).expect("vm eval f32");
        let nat_res32 = nat_f32.eval(&inputs_f32).expect("native eval f32");
        assert!(vm_res32.is_finite(), "expected finite f32 result");
        assert_eq!(nat_res32.to_bits(), vm_res32.to_bits());

        let id_prog = Program::<f64>::compile(&Expr::from(x), &[x]).expect("id compile");
        let id_nat = id_prog.with_native().expect("id native");
        assert_eq!(
            id_nat.eval(&[3.5]).expect("id eval").to_bits(),
            3.5_f64.to_bits()
        );
        let const_prog = Program::<f64>::compile(&Expr::number(7), &[x]).expect("const compile");
        let const_nat = const_prog.with_native().expect("const native");
        assert_eq!(
            const_nat.eval(&[1.0]).expect("const eval").to_bits(),
            7.0_f64.to_bits()
        );

        // Exercise ternary trampoline dispatch separately so its NaN return does not mask `full`.
        let tern_prog =
            Program::<f64>::compile(&x.cos().assoc_legendre(2, 1), &[x]).expect("ternary compile");
        let tern_nat = tern_prog.with_native().expect("ternary native");
        assert_eq!(
            tern_nat.eval(&[0.6]).expect("ternary nat").to_bits(),
            tern_prog.eval(&[0.6]).expect("ternary vm").to_bits()
        );
    }

    #[test]
    fn native_simd_batch_matches_vm_across_block_sizes_and_tails() {
        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let full = (x * y + z) * (x * y - z)
            + (z - x * y)
            + (x.sin() + x.cos() + y.sinh() + y.cosh())
            + (x * x + 0.5).sqrt()
            + (x - y).abs()
            + (x * 1.7).floor()
            + (y * 1.3).ceil()
            + (z * 2.9).trunc()
            + y.atan2(x)
            + x.pow(y);

        let prog_f64 = Program::<f64>::compile(&full, &[x, y, z]).expect("f64 compile");
        let nat_f64 = prog_f64.with_native().expect("f64 native");
        let prog_f32 = Program::<f32>::compile(&full, &[x, y, z]).expect("f32 compile");
        let nat_f32 = prog_f32.with_native().expect("f32 native");

        let mut ws64 = Workspace::new();
        let mut ws32 = Workspace::new();
        for len in [1_usize, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 25] {
            let col_x: Vec<f64> = (0..len)
                .map(|i| 0.04_f64.mul_add(f64::from(u32::try_from(i).expect("i")), 0.25))
                .collect();
            let col_y: Vec<f64> = (0..len)
                .map(|i| 0.03_f64.mul_add(f64::from(u32::try_from(i).expect("i")), 0.45))
                .collect();
            let col_z: Vec<f64> = (0..len)
                .map(|i| 0.05_f64.mul_add(f64::from(u32::try_from(i).expect("i")), -0.3))
                .collect();
            let mut vm_batch = vec![0.0_f64; len];
            let mut nat_batch = vec![0.0_f64; len];
            prog_f64
                .eval_batch(&[&col_x, &col_y, &col_z], &mut vm_batch)
                .expect("vm batch f64");
            nat_f64
                .eval_batch_with(&mut ws64, &[&col_x, &col_y, &col_z], &mut nat_batch)
                .expect("native batch f64");
            for (lhs, rhs) in nat_batch.iter().zip(&vm_batch) {
                assert!(rhs.is_finite(), "expected finite f64 batch entry");
                assert_eq!(lhs.to_bits(), rhs.to_bits());
            }

            let xs32: Vec<f32> = (0..len)
                .map(|i| 0.04_f32.mul_add(f32::from(u16::try_from(i).expect("i")), 0.25_f32))
                .collect();
            let ys32: Vec<f32> = (0..len)
                .map(|i| 0.03_f32.mul_add(f32::from(u16::try_from(i).expect("i")), 0.45_f32))
                .collect();
            let zs32: Vec<f32> = (0..len)
                .map(|i| 0.05_f32.mul_add(f32::from(u16::try_from(i).expect("i")), -0.3_f32))
                .collect();
            let mut vm_batch32 = vec![0.0_f32; len];
            let mut nat_batch32 = vec![0.0_f32; len];
            prog_f32
                .eval_batch(&[&xs32, &ys32, &zs32], &mut vm_batch32)
                .expect("vm batch f32");
            nat_f32
                .eval_batch_with(&mut ws32, &[&xs32, &ys32, &zs32], &mut nat_batch32)
                .expect("native batch f32");
            for (lhs, rhs) in nat_batch32.iter().zip(&vm_batch32) {
                assert!(rhs.is_finite(), "expected finite f32 batch entry");
                assert_eq!(lhs.to_bits(), rhs.to_bits());
            }
        }

        #[cfg(feature = "rayon")]
        {
            let col_x: Vec<f64> = (0_u32..67)
                .map(|i| 0.01_f64.mul_add(f64::from(i), 0.25))
                .collect();
            let col_y: Vec<f64> = (0_u32..67)
                .map(|i| 0.01_f64.mul_add(f64::from(i), 0.45))
                .collect();
            let col_z: Vec<f64> = (0_u32..67)
                .map(|i| 0.01_f64.mul_add(f64::from(i), -0.3))
                .collect();
            let mut vm_batch = vec![0.0_f64; 67];
            let mut nat_par = vec![0.0_f64; 67];
            prog_f64
                .eval_batch(&[&col_x, &col_y, &col_z], &mut vm_batch)
                .expect("vm batch par");
            nat_f64
                .eval_batch_par(&[&col_x, &col_y, &col_z], &mut nat_par)
                .expect("native par batch");
            for (lhs, rhs) in nat_par.iter().zip(&vm_batch) {
                assert_eq!(lhs.to_bits(), rhs.to_bits());
            }
        }
    }

    #[test]
    fn native_complex_f64_and_f32_match_vm_across_fused_dot_and_folds() {
        let context = Context::new().expect("context");
        let sym_x = context.symb("x").expect("x");
        let sym_y = context.symb("y").expect("y");
        let sym_z = context.symb("z").expect("z");

        let c_fused = (sym_x * sym_y + sym_z)
            + (sym_x * sym_y - sym_z)
            + (sym_z - sym_x * sym_y)
            + (-(sym_x * sym_y) - sym_z);
        let c_dot = (sym_x + 1) * (sym_y + 1)
            + (sym_y + 2) * (sym_z + 2)
            + (sym_z + 3) * (sym_x + 3)
            + (sym_x + 4) * (sym_y + 5)
            + (sym_y + 6) * (sym_z + 7)
            + (sym_z + 8) * (sym_x + 9)
            + (sym_x + 10) * (sym_y + 11)
            + (sym_y + 12) * (sym_z + 13);
        let c_expr = (sym_x * sym_y + sym_x / sym_y - sym_x).sin()
            + sym_x.cos()
            + sym_y.pow(2)
            + (sym_x + sym_y + (sym_x * 2) + (sym_y * 3))
            + (sym_x + 1) * (sym_y + 2) * (sym_x + sym_y)
            + c_fused
            + c_dot;

        let prog_c64 =
            Program::<Complex<f64>>::compile(&c_expr, &[sym_x, sym_y, sym_z]).expect("c64 compile");
        let nat_c64 = prog_c64.with_native().expect("c64 native");
        let in_c64 = [
            Complex {
                re: 0.7_f64,
                im: -0.3,
            },
            Complex { re: 1.2, im: 0.5 },
            Complex { re: -0.4, im: 0.9 },
        ];
        let vm_c64 = prog_c64.eval(&in_c64).expect("vm c64");
        let res_c64 = nat_c64.eval(&in_c64).expect("nat c64");
        assert_eq!(res_c64.re.to_bits(), vm_c64.re.to_bits());
        assert_eq!(res_c64.im.to_bits(), vm_c64.im.to_bits());

        let prog_c32 =
            Program::<Complex<f32>>::compile(&c_expr, &[sym_x, sym_y, sym_z]).expect("c32 compile");
        let nat_c32 = prog_c32.with_native().expect("c32 native");
        let in_c32 = [
            Complex {
                re: 0.7_f32,
                im: -0.3,
            },
            Complex { re: 1.2, im: 0.5 },
            Complex { re: -0.4, im: 0.9 },
        ];
        let vm_c32 = prog_c32.eval(&in_c32).expect("vm c32");
        let res_c32 = nat_c32.eval(&in_c32).expect("nat c32");
        assert_eq!(res_c32.re.to_bits(), vm_c32.re.to_bits());
        assert_eq!(res_c32.im.to_bits(), vm_c32.im.to_bits());
    }

    #[test]
    fn native_high_pressure_evex_spilling_and_simd_batch_match_vm() {
        let context = Context::new().expect("context");
        // 40 symbols force >32 simultaneously live values, exercising EVEX xmm16..xmm31 + Belady spills.
        let mut symbols = Vec::with_capacity(40);
        let mut inputs = Vec::with_capacity(40);
        for idx in 0_u32..40 {
            let sym = context.symb(&format!("v{idx}")).expect("symbol creation");
            symbols.push(sym);
            inputs.push(0.05_f64 * f64::from(idx + 1));
        }
        let mut subterms = Vec::with_capacity(40);
        for idx in 0..40 {
            let first_sym = symbols[idx];
            let second_sym = symbols[(idx + 1) % 40];
            let third_sym = symbols[(idx + 13) % 40];
            subterms.push(
                (first_sym * second_sym + third_sym) / (first_sym + 2)
                    + (second_sym - third_sym) * (first_sym + third_sym)
                    - (first_sym * third_sym),
            );
        }
        let mut combined = Expr::number(1);
        for term in &subterms {
            combined = combined * term.clone() + term.clone();
        }
        for term in &subterms {
            combined = combined + term.clone() * term.clone();
        }
        let high_pressure_prog =
            Program::<f64>::compile(&combined, &symbols).expect("high pressure");
        let high_pressure_nat = high_pressure_prog
            .with_native()
            .expect("high pressure native");
        let expected = high_pressure_prog.eval(&inputs).expect("vm high pressure");
        let actual = high_pressure_nat.eval(&inputs).expect("nat high pressure");
        assert_eq!(actual.to_bits(), expected.to_bits());

        // Also verify leaf SIMD batch kernel under >32-register pressure across 19 rows (2×8 + 3 tail).
        let cols: Vec<Vec<f64>> = (0..40)
            .map(|p| {
                let base = 0.05 * f64::from(u32::try_from(p + 1).expect("p"));
                (0..19)
                    .map(|r| 0.01_f64.mul_add(f64::from(u32::try_from(r).expect("r")), base))
                    .collect()
            })
            .collect();
        let col_refs: Vec<&[f64]> = cols.iter().map(Vec::as_slice).collect();
        let mut vm_out = [0.0_f64; 19];
        let mut nat_out = [0.0_f64; 19];
        high_pressure_prog
            .eval_batch(&col_refs, &mut vm_out)
            .expect("vm high pressure batch");
        high_pressure_nat
            .eval_batch(&col_refs, &mut nat_out)
            .expect("nat high pressure batch");
        for (lhs, rhs) in nat_out.iter().zip(&vm_out) {
            assert_eq!(lhs.to_bits(), rhs.to_bits());
        }

        // Randomized differential test over nested sin/cos pairs and reused result slots.
        let sym_x = context.symb("x").expect("x");
        let sym_y = context.symb("y").expect("y");
        let sym_z = context.symb("z").expect("z");
        let vars = [Expr::from(sym_x), Expr::from(sym_y), Expr::from(sym_z)];
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        let row = [0.37_f64, -0.81, 1.13];
        for _ in 0..500 {
            let depth = 2 + u32::try_from(next_rng(&mut seed) % 3).expect("depth");
            let expr = random_pair_tree(&mut seed, &vars, depth)
                + random_pair_tree(&mut seed, &vars, depth);
            let program = Program::<f64>::compile(&expr, &[sym_x, sym_y, sym_z])
                .expect("random pair compile");
            let vm = program.eval(&row).expect("vm random pair");
            let nat = program
                .with_native()
                .expect("native random pair")
                .eval(&row)
                .expect("native eval random pair");
            assert_eq!(nat.to_bits(), vm.to_bits());
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn x86_tier_expr(context: &Context) -> (Expr, [crate::core::Symbol<'_>; 3]) {
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");
        let expr = (x * y + z) * (x * y - z) + (z - x * y) - (-(x * y) - z)
            + (x.sin() + x.cos() + y.sinh() + y.cosh())
            + (x * x + 0.5).sqrt()
            + (x - y).abs()
            + (x * 1.7).floor()
            + (y * 1.3).ceil()
            + (z * 2.9).trunc()
            + y.atan2(x)
            + x.pow(y);
        (expr, [x, y, z])
    }

    #[cfg(target_arch = "x86_64")]
    #[expect(
        unsafe_code,
        reason = "test helpers invoke compiled x86_64 scalar and SIMD batch kernels from ExecutableBuffer"
    )]
    mod x86_exec {
        use core::mem::transmute;

        use super::super::super::super::native::ExecutableBuffer;

        pub fn scalar_f64(buf: &ExecutableBuffer, regs: &mut [f64], tr: *const ()) -> f64 {
            // SAFETY: caller compiled `buf` as a valid scalar f64 kernel matching `regs` and `tr`.
            unsafe {
                let func: unsafe extern "C" fn(*mut f64, *const ()) -> f64 =
                    transmute(buf.as_ptr());
                func(regs.as_mut_ptr(), tr)
            }
        }

        pub fn scalar_f32(buf: &ExecutableBuffer, regs: &mut [f32], tr: *const ()) -> f32 {
            // SAFETY: caller compiled `buf` as a valid scalar f32 kernel matching `regs` and `tr`.
            unsafe {
                let func: unsafe extern "C" fn(*mut f32, *const ()) -> f32 =
                    transmute(buf.as_ptr());
                func(regs.as_mut_ptr(), tr)
            }
        }

        pub fn vec_f64(buf: &ExecutableBuffer, regs: *mut f64, tr: *const ()) {
            // SAFETY: caller compiled `buf` as a valid SIMD f64 kernel matching `regs` and `tr`.
            unsafe {
                let func: unsafe extern "C" fn(*mut f64, *const ()) = transmute(buf.as_ptr());
                func(regs, tr);
            }
        }

        pub fn vec_f32(buf: &ExecutableBuffer, regs: *mut f32, tr: *const ()) {
            // SAFETY: caller compiled `buf` as a valid SIMD f32 kernel matching `regs` and `tr`.
            unsafe {
                let func: unsafe extern "C" fn(*mut f32, *const ()) = transmute(buf.as_ptr());
                func(regs, tr);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_lower_isa_tiers_match_vm() {
        use std::arch::is_x86_feature_detected;

        use super::super::super::native::{
            ExecutableBuffer, FEAT_AVX, FEAT_AVX2, FEAT_AVX512, FEAT_FMA, FEAT_SSE41,
            NativeEmitter, NativeLayout, Trampolines, X86_64Backend,
        };

        let context = Context::new().expect("context");
        let (expr, syms) = x86_tier_expr(&context);
        let prog64 = Program::<f64>::compile(&expr, &syms).expect("f64 compile");
        let prog32 = Program::<f32>::compile(&expr, &syms).expect("f32 compile");
        let inputs64 = [0.6_f64, 0.8, -0.4];
        let inputs32 = [0.6_f32, 0.8, -0.4];
        let vm64 = prog64.eval(&inputs64).expect("vm f64");
        let vm32 = prog32.eval(&inputs32).expect("vm f32");
        assert!(vm64.is_finite(), "expected finite f64 oracle, got {vm64}");
        assert!(vm32.is_finite(), "expected finite f32 oracle, got {vm32}");

        let mut tiers: Vec<(u8, bool)> = vec![(0, false)];
        if is_x86_feature_detected!("sse4.1") {
            tiers.push((FEAT_SSE41, false));
        }
        if is_x86_feature_detected!("avx") {
            tiers.push((FEAT_SSE41 | FEAT_AVX, false));
        }
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            tiers.push((FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA, true));
        }
        if is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512vl")
            && is_x86_feature_detected!("avx512dq")
            && is_x86_feature_detected!("fma")
        {
            tiers.push((
                FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA | FEAT_AVX512,
                true,
            ));
        }

        for (features, has_fma) in tiers {
            let mut be64 = X86_64Backend::with_features(features);
            let (code64, start64) =
                NativeEmitter::compile(&prog64.data, NativeLayout::F64, false, &mut be64)
                    .expect("compile f64 tier");
            let buf64 = ExecutableBuffer::new(&code64[start64..]).expect("exec buf f64");
            let mut ws64 = Workspace::new();
            ws64.prepare::<1>(&prog64.data);
            ws64.registers[..3].copy_from_slice(&inputs64);
            let tr64 = Trampolines::table(NativeLayout::F64).as_ptr().cast::<()>();
            let got64 = x86_exec::scalar_f64(&buf64, &mut ws64.registers, tr64);
            if has_fma {
                assert_eq!(got64.to_bits(), vm64.to_bits(), "f64 tier 0x{features:02X}");
            } else {
                assert!((got64 - vm64).abs() <= 1.0e-12 * vm64.abs().max(1.0));
            }

            let mut be32 = X86_64Backend::with_features(features);
            let (code32, start32) =
                NativeEmitter::compile(&prog32.data, NativeLayout::F32, false, &mut be32)
                    .expect("compile f32 tier");
            let buf32 = ExecutableBuffer::new(&code32[start32..]).expect("exec buf f32");
            let mut ws32 = Workspace::new();
            ws32.prepare::<1>(&prog32.data);
            ws32.registers[..3].copy_from_slice(&inputs32);
            let tr32 = Trampolines::table(NativeLayout::F32).as_ptr().cast::<()>();
            let got32 = x86_exec::scalar_f32(&buf32, &mut ws32.registers, tr32);
            if has_fma {
                assert_eq!(got32.to_bits(), vm32.to_bits(), "f32 tier 0x{features:02X}");
            } else {
                assert!((got32 - vm32).abs() <= 1.0e-5 * vm32.abs().max(1.0));
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_vec256_batch_and_vzeroupper_match_vm() {
        use std::arch::is_x86_feature_detected;

        use super::super::super::native::{
            ExecutableBuffer, FEAT_AVX, FEAT_AVX2, FEAT_FMA, FEAT_SSE41, FloatPrecision,
            NativeEmitter, Trampolines, X86_64Backend,
        };

        if !is_x86_feature_detected!("avx2") || !is_x86_feature_detected!("fma") {
            return;
        }
        let context = Context::new().expect("context");
        let (expr, syms) = x86_tier_expr(&context);
        let avx2_feats = FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA;

        let prog64 = Program::<f64>::compile(&expr, &syms).expect("f64 compile");
        let (mut be_v64, lanes64) = X86_64Backend::with_features(avx2_feats)
            .into_batch(FloatPrecision::F64)
            .expect("vec256 f64");
        assert_eq!(lanes64, 4);
        let (code_v64, start_v64) =
            NativeEmitter::compile_vector(&prog64.data, FloatPrecision::F64, lanes64, &mut be_v64)
                .expect("compile vec256 f64");
        let slice_v64 = &code_v64[start_v64..];
        assert!(
            slice_v64
                .windows(3)
                .filter(|w| *w == [0xC5, 0xF8, 0x77])
                .count()
                >= 2
        );
        let buf_v64 = ExecutableBuffer::new(slice_v64).expect("exec buf vec256 f64");
        let mut ws_v64 = Workspace::new();
        let regs_v64 = ws_v64.prepare::<4>(&prog64.data);
        let xs = [0.25_f64, 0.4, 0.55, 0.7];
        let ys = [0.35_f64, 0.5, 0.65, 0.8];
        let zs = [-0.2_f64, -0.3, -0.4, -0.5];
        regs_v64[0] = xs;
        regs_v64[1] = ys;
        regs_v64[2] = zs;
        let tr_v64 = Trampolines::vector_table(FloatPrecision::F64, 4)
            .expect("table f64x4")
            .as_ptr()
            .cast::<()>();
        x86_exec::vec_f64(&buf_v64, regs_v64.as_mut_ptr().cast::<f64>(), tr_v64);
        let mut expected = [0.0_f64; 4];
        prog64
            .eval_batch(&[&xs, &ys, &zs], &mut expected)
            .expect("vm batch 4");
        let res64_idx = usize::try_from(prog64.data.results[0]).expect("result idx");
        for lane in 0..4 {
            assert_eq!(
                regs_v64[res64_idx][lane].to_bits(),
                expected[lane].to_bits()
            );
        }

        let prog32 = Program::<f32>::compile(&expr, &syms).expect("f32 compile");
        let (mut be_v32, lanes32) = X86_64Backend::with_features(avx2_feats)
            .into_batch(FloatPrecision::F32)
            .expect("vec256 f32");
        assert_eq!(lanes32, 8);
        let (code_v32, start_v32) =
            NativeEmitter::compile_vector(&prog32.data, FloatPrecision::F32, lanes32, &mut be_v32)
                .expect("compile vec256 f32");
        let buf_v32 = ExecutableBuffer::new(&code_v32[start_v32..]).expect("exec buf vec256");
        let mut ws_v32 = Workspace::new();
        let regs_v32 = ws_v32.prepare::<8>(&prog32.data);
        let xs32 = [0.25_f32, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6];
        let ys32 = [0.35_f32, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7];
        let zs32 = [-0.1_f32, -0.15, -0.2, -0.25, -0.3, -0.35, -0.4, -0.45];
        regs_v32[0] = xs32;
        regs_v32[1] = ys32;
        regs_v32[2] = zs32;
        let tr_v32 = Trampolines::vector_table(FloatPrecision::F32, 8)
            .expect("table f32x8")
            .as_ptr()
            .cast::<()>();
        x86_exec::vec_f32(&buf_v32, regs_v32.as_mut_ptr().cast::<f32>(), tr_v32);
        let mut expected32 = [0.0_f32; 8];
        prog32
            .eval_batch(&[&xs32, &ys32, &zs32], &mut expected32)
            .expect("vm batch 8");
        let res32_idx = usize::try_from(prog32.data.results[0]).expect("result idx");
        for lane in 0..8 {
            assert_eq!(
                regs_v32[res32_idx][lane].to_bits(),
                expected32[lane].to_bits()
            );
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_liveness_duplicate_operand_and_slot_bounds() {
        use super::super::super::{
            Fold, Instruction, OperandSpan, Operation, ProgramData,
            native::{ArchBackend, LivenessPlan, X86_64Backend},
        };

        let dup_prog = ProgramData::<f64> {
            instructions: vec![Instruction {
                output: 2,
                operation: Operation::Fold(Fold::Sum, OperandSpan { start: 0, end: 3 }),
            }]
            .into_boxed_slice(),
            operands: vec![0, 1, 0].into_boxed_slice(),
            parameters: Box::new([]),
            active_parameters: Box::new([]),
            results: vec![2].into_boxed_slice(),
            constants: vec![1.0, 2.0].into_boxed_slice(),
            register_count: 3,
            initial: 0.0,
        };
        let plan = LivenessPlan::analyze(&dup_prog, &X86_64Backend::new());
        assert_eq!(plan.use_next, [0, u32::MAX, u32::MAX]);

        let mut be = X86_64Backend::new();
        let mut dummy = Vec::new();
        assert!(
            be.emit_call_slots(&mut dummy, 0, &[0, 8, 16, 24, 32])
                .is_err()
        );
    }

    #[test]
    fn cross_arch_aarch64_and_riscv64_encoders_emit_valid_frames_and_guard_bounds() {
        use super::super::super::{
            Fused,
            native::{
                AArch64Backend, ArchBackend, FloatPrecision, HwReg, NativeEmitter, NativeLayout,
                Riscv64Backend,
            },
        };

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let leaf_expr = (x * y + z) * (x - y) + z.sqrt();
        let call_expr = (x * y + z).sin() + x.cos() + x.pow(y) + (x + y).abs();
        let leaf_prog = Program::<f64>::compile(&leaf_expr, &[x, y, z]).expect("leaf");
        let call_prog = Program::<f64>::compile(&call_expr, &[x, y, z]).expect("call");

        // AArch64 leaf and call compilation + exact encoding spot checks.
        let mut arm_leaf = AArch64Backend::new();
        let (arm_leaf_code, arm_leaf_start) =
            NativeEmitter::compile(&leaf_prog.data, NativeLayout::F64, false, &mut arm_leaf)
                .expect("arm leaf compile");
        let arm_leaf_bytes = &arm_leaf_code[arm_leaf_start..];
        assert_eq!(arm_leaf_bytes.len() % 4, 0);
        // Pure leaf kernel ends with `ret` (`0xD65F_03C0`) and does not allocate a stack frame.
        assert_eq!(
            &arm_leaf_bytes[arm_leaf_bytes.len() - 4..],
            &0xD65F_03C0_u32.to_le_bytes()
        );
        assert!(
            !arm_leaf_bytes
                .as_chunks::<4>()
                .0
                .iter()
                .any(|w| *w == 0xA9BA_7BFD_u32.to_le_bytes())
        );

        let mut arm_call = AArch64Backend::new();
        let (arm_call_code, arm_call_start) =
            NativeEmitter::compile(&call_prog.data, NativeLayout::F64, false, &mut arm_call)
                .expect("arm call compile");
        let arm_call_bytes = &arm_call_code[arm_call_start..];
        assert_eq!(arm_call_bytes.len() % 4, 0);
        // Non-leaf prologue starts with `stp x29, x30, [sp, #-96]!; mov x29, sp`.
        assert_eq!(&arm_call_bytes[0..4], &0xA9BA_7BFD_u32.to_le_bytes());
        assert_eq!(&arm_call_bytes[4..8], &0x9100_03FD_u32.to_le_bytes());

        // AArch64 FMA variants + large offset + bounds checks.
        let mut arm_unit = AArch64Backend::new();
        let mut arm_buf = Vec::new();
        arm_unit.emit_fused(
            &mut arm_buf,
            FloatPrecision::F64,
            Fused {
                negate_product: false,
                negate_addend: true,
            },
            HwReg(0),
            [HwReg(1), HwReg(2), HwReg(3)],
        );
        assert_eq!(arm_buf, 0x1F62_8C20_u32.to_le_bytes());
        assert!(arm_unit.emit_call_reg(&mut arm_buf, 0x1000).is_err());
        assert!(arm_unit.emit_call_slots(&mut arm_buf, 0, &[0; 9]).is_err());

        // RISC-V 64 leaf and call compilation + exact encoding spot checks.
        let mut rv_leaf = Riscv64Backend::new();
        let (rv_leaf_code, rv_leaf_start) =
            NativeEmitter::compile(&leaf_prog.data, NativeLayout::F64, false, &mut rv_leaf)
                .expect("rv leaf compile");
        let rv_leaf_bytes = &rv_leaf_code[rv_leaf_start..];
        assert_eq!(rv_leaf_bytes.len() % 4, 0);
        assert_eq!(
            &rv_leaf_bytes[rv_leaf_bytes.len() - 4..],
            &0x0000_8067_u32.to_le_bytes()
        );

        let mut rv_call = Riscv64Backend::new();
        let (rv_call_code, rv_call_start) =
            NativeEmitter::compile(&call_prog.data, NativeLayout::F64, false, &mut rv_call)
                .expect("rv call compile");
        let rv_call_bytes = &rv_call_code[rv_call_start..];
        assert_eq!(rv_call_bytes.len() % 4, 0);
        assert_eq!(
            &rv_call_bytes[rv_call_bytes.len() - 4..],
            &0x0000_8067_u32.to_le_bytes()
        );

        let mut rv_unit = Riscv64Backend::new();
        let mut rv_buf = Vec::new();
        assert!(rv_unit.emit_call_slots(&mut rv_buf, 0, &[0; 9]).is_err());
        assert!(
            rv_unit
                .emit_load(&mut rv_buf, FloatPrecision::F64, HwReg(0), 0x8000_0000)
                .is_err()
        );
    }

    #[test]
    fn aarch64_neon_batch_encoder_emits_aligned_simd_frames() {
        use super::super::super::native::{AArch64Backend, FloatPrecision, NativeEmitter};

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");
        let expr = (x * y + z) * (x - y) + x.sin() + x.cos() + z.sqrt() + (x + y).abs();

        let prog64 = Program::<f64>::compile(&expr, &[x, y, z]).expect("f64 compile");
        let (mut arm_vec64, lanes64) = AArch64Backend::new().into_batch(FloatPrecision::F64);
        assert_eq!(lanes64, 2);
        let (code64, start64) = NativeEmitter::compile_vector(
            &prog64.data,
            FloatPrecision::F64,
            lanes64,
            &mut arm_vec64,
        )
        .expect("arm neon f64x2 compile");
        assert_eq!((code64.len() - start64) % 4, 0);

        let prog32 = Program::<f32>::compile(&expr, &[x, y, z]).expect("f32 compile");
        let (mut arm_vec32, lanes32) = AArch64Backend::new().into_batch(FloatPrecision::F32);
        assert_eq!(lanes32, 4);
        let (code32, start32) = NativeEmitter::compile_vector(
            &prog32.data,
            FloatPrecision::F32,
            lanes32,
            &mut arm_vec32,
        )
        .expect("arm neon f32x4 compile");
        assert_eq!((code32.len() - start32) % 4, 0);
    }

    #[test]
    fn riscv64_base_bias_and_hi20_cache() {
        use super::super::super::native::{ArchBackend, FloatPrecision, HwReg, Riscv64Backend};

        // RISC-V `base_bias = 2040` keeps offsets up to 4087 B in a single `fld` (4 B),
        // and caches `hi20` in `t0` across same-page large loads until invalidated by a call.
        let mut rv_unit = Riscv64Backend::new();
        let mut rv_buf = Vec::new();
        rv_unit.configure_frame(true, 8192);
        rv_unit
            .emit_load(&mut rv_buf, FloatPrecision::F64, HwReg(1), 4080)
            .expect("direct biased load");
        assert_eq!(rv_buf.len(), 4);
        rv_buf.clear();
        rv_unit
            .emit_load(&mut rv_buf, FloatPrecision::F64, HwReg(1), 4088)
            .expect("first large-page load");
        assert_eq!(rv_buf.len(), 12); // lui + add + fld
        rv_buf.clear();
        rv_unit
            .emit_load(&mut rv_buf, FloatPrecision::F64, HwReg(2), 4096)
            .expect("cached large-page load");
        assert_eq!(rv_buf.len(), 4); // reuses cached `t0`
        rv_unit
            .emit_call_reg(&mut rv_buf, 0)
            .expect("call invalidates hi20");
        rv_buf.clear();
        rv_unit
            .emit_load(&mut rv_buf, FloatPrecision::F64, HwReg(3), 4096)
            .expect("reloads hi20 after call");
        assert_eq!(rv_buf.len(), 12);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_avx512_high_pressure_scalar_and_vec512_match_vm() {
        use std::arch::is_x86_feature_detected;

        use super::super::super::native::{
            FEAT_AVX, FEAT_AVX2, FEAT_AVX512, FEAT_FMA, FEAT_SSE41, FloatPrecision, NativeEmitter,
            X86_64Backend,
        };

        if !is_x86_feature_detected!("avx512f")
            || !is_x86_feature_detected!("avx512vl")
            || !is_x86_feature_detected!("avx512dq")
        {
            return;
        }
        let context = Context::new().expect("context");
        let syms: Vec<_> = (0_u8..24)
            .map(|i| context.symb(&format!("p{i}")).expect("sym"))
            .collect();
        // High register pressure across sqrt/abs/neg/floor/ceil/trunc/fma/div exercises
        // Belady spilling and EVEX register expansion (`xmm0..xmm31` / `zmm0..zmm31`).
        let mut nodes = Vec::with_capacity(22);
        for i in 0..22 {
            let a = &syms[i];
            let b = &syms[(i + 1) % 24];
            let c = &syms[(i + 2) % 24];
            let base = (a * b + c + 2).abs().sqrt();
            let rounded = base.floor() - b.ceil() + c.trunc();
            nodes.push(-base + rounded / (a.abs() + 2));
        }
        let mut total = Expr::number(0);
        for i in 0..22 {
            let partner = &nodes[(i + 7) % 22];
            total = total + &nodes[i] * partner + (&nodes[i] - partner) / (partner.abs() + 3);
        }

        let prog64 = Program::<f64>::compile(&total, &syms).expect("f64 compile");
        let avx512_feats = FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA | FEAT_AVX512;
        let (mut be_vec, lanes) = X86_64Backend::with_features(avx512_feats)
            .into_batch(FloatPrecision::F64)
            .expect("vec512 f64");
        assert_eq!(lanes, 8);
        let (vec_code, vec_start) =
            NativeEmitter::compile_vector(&prog64.data, FloatPrecision::F64, lanes, &mut be_vec)
                .expect("vec512 compile");
        assert!(vec_code[vec_start..].contains(&0x62));

        let inputs64: Vec<f64> = (0_u8..24)
            .map(|i| {
                let sign = if i % 2 == 0 { 0.17_f64 } else { -0.17 };
                f64::from(i).mul_add(sign, 0.35)
            })
            .collect();
        let vm64 = prog64.eval(&inputs64).expect("vm f64");
        let native64 = prog64.with_native().expect("native f64");
        let jit64 = native64.eval(&inputs64).expect("jit f64");
        assert!(jit64.is_finite());
        assert_eq!(jit64.to_bits(), vm64.to_bits());

        let prog32 = Program::<f32>::compile(&total, &syms).expect("f32 compile");
        let inputs32: Vec<f32> = (0_u8..24)
            .map(|i| {
                let sign = if i % 2 == 0 { 0.17_f32 } else { -0.17 };
                f32::from(i).mul_add(sign, 0.35)
            })
            .collect();
        let vm32 = prog32.eval(&inputs32).expect("vm f32");
        let native32 = prog32.with_native().expect("native f32");
        let jit32 = native32.eval(&inputs32).expect("jit f32");
        assert!(jit32.is_finite());
        assert_eq!(jit32.to_bits(), vm32.to_bits());
    }

    #[test]
    fn native_scalar_complex_and_simd_match_vm_for_new_component_step_and_selection_builtins() {
        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let real_expr = x.re()
            + y.im()
            + z.conj()
            + x.heaviside()
            + x.hypot(y)
            + x.max(y)
            + y.min(z)
            + x.modulo(y)
            + x.rem(y)
            + x.clamp(y, z);

        // Scalar f64 and f32 JIT vs VM.
        let prog64 = Program::<f64>::compile(&real_expr, &[x, y, z]).expect("f64 compile");
        let nat64 = prog64.with_native().expect("f64 native");
        let pt64 = [-7.25_f64, 1.5, 4.0];
        let vm64 = prog64.eval(&pt64).expect("vm f64");
        let jit64 = nat64.eval(&pt64).expect("jit f64");
        assert!(vm64.is_finite());
        assert_eq!(jit64.to_bits(), vm64.to_bits());

        let prog32 = Program::<f32>::compile(&real_expr, &[x, y, z]).expect("f32 compile");
        let nat32 = prog32.with_native().expect("f32 native");
        let pt32 = [-7.25_f32, 1.5, 4.0];
        let vm32 = prog32.eval(&pt32).expect("vm f32");
        let jit32 = nat32.eval(&pt32).expect("jit f32");
        assert!(vm32.is_finite());
        assert_eq!(jit32.to_bits(), vm32.to_bits());

        // SIMD batch f64 and f32 across 17 lanes (covers AVX-512 8/16, AVX2 4/8, and scalar tail).
        let xs: Vec<f64> = (0_u32..17)
            .map(|i| 0.6_f64.mul_add(f64::from(i), -4.5))
            .collect();
        let ys: Vec<f64> = (0_u32..17)
            .map(|i| 0.1_f64.mul_add(f64::from(i), 1.2))
            .collect();
        let zs: Vec<f64> = (0_u32..17)
            .map(|i| 0.2_f64.mul_add(f64::from(i), 2.5))
            .collect();
        let mut vm_batch = vec![0.0_f64; 17];
        let mut nat_batch = vec![0.0_f64; 17];
        prog64
            .eval_batch(&[&xs, &ys, &zs], &mut vm_batch)
            .expect("vm batch");
        nat64
            .eval_batch(&[&xs, &ys, &zs], &mut nat_batch)
            .expect("nat batch");
        for (actual, expected) in nat_batch.iter().zip(&vm_batch) {
            assert!(expected.is_finite());
            assert_eq!(actual.to_bits(), expected.to_bits());
        }

        // Complex f64 and f32 JIT vs VM for re, im, conj, hypot, plus real-axis clamp/mod/rem.
        let complex_expr = x.re() + y.im() + z.conj() + x.hypot(y);
        let cprog64 =
            Program::<Complex<f64>>::compile(&complex_expr, &[x, y, z]).expect("c64 compile");
        let cnat64 = cprog64.with_native().expect("c64 native");
        let cpt64 = [
            Complex {
                re: 1.5_f64,
                im: -2.25,
            },
            Complex { re: -0.75, im: 3.0 },
            Complex { re: 2.0, im: -1.0 },
        ];
        let cvm64 = cprog64.eval(&cpt64).expect("cvm64");
        let cjit64 = cnat64.eval(&cpt64).expect("cjit64");
        assert_eq!(cjit64.re.to_bits(), cvm64.re.to_bits());
        assert_eq!(cjit64.im.to_bits(), cvm64.im.to_bits());

        let clamp_cprog =
            Program::<Complex<f64>>::compile(&x.clamp(y, z), &[x, y, z]).expect("c64 clamp");
        let clamp_cnat = clamp_cprog.with_native().expect("c64 clamp native");
        let axis_pt = [
            Complex {
                re: 5.0_f64,
                im: 0.0,
            },
            Complex { re: 0.0, im: 0.0 },
            Complex { re: 2.0, im: 0.0 },
        ];
        assert_eq!(
            clamp_cnat.eval(&axis_pt).expect("clamp nat"),
            Complex { re: 2.0, im: 0.0 }
        );
    }

    #[test]
    fn real_lowering_elides_re_im_conj_and_complex_jit_inlines_them_as_leaf_kernels() {
        use num_anafis::Real;

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        // Real types lower `re`, `conj`, and `im` to zero instructions.
        for expr in [x.re(), x.conj(), x.im()] {
            assert_eq!(
                Program::<f64>::compile(&expr, &[x])
                    .expect("f64")
                    .instruction_count(),
                0
            );
            assert_eq!(
                Program::<f32>::compile(&expr, &[x])
                    .expect("f32")
                    .instruction_count(),
                0
            );
            assert_eq!(
                Program::<Real>::compile(&expr, &[x])
                    .expect("real")
                    .instruction_count(),
                0
            );
        }
        assert_eq!(
            Program::<f64>::compile(&x.im(), &[x])
                .expect("im f64")
                .eval(&[f64::NAN])
                .expect("im eval")
                .to_bits(),
            0.0_f64.to_bits()
        );

        // Complex `re`, `im`, `conj`, `+`, `-` emit leaf kernels without trampoline calls.
        let cexpr = x.re() + y.im() - z.conj();
        let cprog64 = Program::<Complex<f64>>::compile(&cexpr, &[x, y, z]).expect("c64");
        let cnat64 = cprog64.with_native().expect("c64 native");
        let cprog32 = Program::<Complex<f32>>::compile(&cexpr, &[x, y, z]).expect("c32");
        let cnat32 = cprog32.with_native().expect("c32 native");

        for inputs in [
            [
                Complex {
                    re: 1.5_f64,
                    im: -2.25,
                },
                Complex { re: -0.75, im: 3.0 },
                Complex { re: 2.0, im: -1.0 },
            ],
            [
                Complex { re: -0.0, im: -0.0 },
                Complex { re: 4.0, im: -0.0 },
                Complex { re: -0.0, im: 0.0 },
            ],
            [
                Complex {
                    re: 1.0,
                    im: f64::NAN,
                },
                Complex {
                    re: f64::NAN,
                    im: -2.5,
                },
                Complex { re: 3.0, im: -4.0 },
            ],
        ] {
            let vm64 = cprog64.eval(&inputs).expect("cvm64");
            let jit64 = cnat64.eval(&inputs).expect("cjit64");
            assert_eq!(jit64.re.to_bits(), vm64.re.to_bits());
            assert_eq!(jit64.im.to_bits(), vm64.im.to_bits());
        }
        for inputs32 in [
            [
                Complex {
                    re: 1.5_f32,
                    im: -2.25,
                },
                Complex { re: -0.75, im: 3.0 },
                Complex { re: 2.0, im: -1.0 },
            ],
            [
                Complex { re: -0.0, im: -0.0 },
                Complex { re: 4.0, im: -0.0 },
                Complex { re: -0.0, im: 0.0 },
            ],
            [
                Complex {
                    re: 1.0,
                    im: f32::NAN,
                },
                Complex {
                    re: f32::NAN,
                    im: -2.5,
                },
                Complex { re: 3.0, im: -4.0 },
            ],
        ] {
            let vm32 = cprog32.eval(&inputs32).expect("cvm32");
            let jit32 = cnat32.eval(&inputs32).expect("cjit32");
            assert_eq!(jit32.re.to_bits(), vm32.re.to_bits());
            assert_eq!(jit32.im.to_bits(), vm32.im.to_bits());
        }
    }

    #[test]
    fn aarch64_and_riscv64_inline_binary_and_zero_encodings() {
        use super::super::super::{
            InlineBinary,
            native::{
                AArch64Backend, ArchBackend, FloatPrecision, HwReg, LivenessPlan, NativeEmitter,
                NativeLayout, Riscv64Backend,
            },
        };

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let max_prog = Program::<f64>::compile(&x.clamp(y, z), &[x, y, z]).expect("clamp");
        let mut arm = AArch64Backend::new();
        assert!(!LivenessPlan::analyze(&max_prog.data, &arm).has_calls);
        let (arm_code, arm_start) =
            NativeEmitter::compile(&max_prog.data, NativeLayout::F64, false, &mut arm)
                .expect("arm clamp");
        assert_eq!((arm_code.len() - arm_start) % 4, 0);

        let mut arm_buf = Vec::new();
        arm.emit_zero(&mut arm_buf, FloatPrecision::F64, HwReg(1));
        assert_eq!(arm_buf, 0x6F00_E401_u32.to_le_bytes());
        arm_buf.clear();
        arm.emit_inline_binary(
            &mut arm_buf,
            FloatPrecision::F64,
            InlineBinary::Max,
            HwReg(0),
            HwReg(0),
            HwReg(1),
        )
        .expect("arm fmax");
        assert_eq!(arm_buf, 0x1E61_4800_u32.to_le_bytes());
        arm_buf.clear();
        arm.emit_inline_binary(
            &mut arm_buf,
            FloatPrecision::F64,
            InlineBinary::Min,
            HwReg(0),
            HwReg(0),
            HwReg(1),
        )
        .expect("arm fmin");
        assert_eq!(arm_buf, 0x1E61_5800_u32.to_le_bytes());

        let (mut arm_vec, _) = AArch64Backend::new().into_batch(FloatPrecision::F64);
        arm_buf.clear();
        arm_vec
            .emit_inline_binary(
                &mut arm_buf,
                FloatPrecision::F64,
                InlineBinary::Max,
                HwReg(0),
                HwReg(0),
                HwReg(1),
            )
            .expect("neon fmax");
        assert_eq!(arm_buf, 0x4E61_F400_u32.to_le_bytes());
        arm_buf.clear();
        arm_vec
            .emit_inline_binary(
                &mut arm_buf,
                FloatPrecision::F64,
                InlineBinary::Min,
                HwReg(0),
                HwReg(0),
                HwReg(1),
            )
            .expect("neon fmin");
        assert_eq!(arm_buf, 0x4EE1_F400_u32.to_le_bytes());

        let mut rv = Riscv64Backend::new();
        assert!(!rv.supports_inline_binary(InlineBinary::Max));
        assert!(!rv.supports_inline_binary(InlineBinary::Min));
        assert!(LivenessPlan::analyze(&max_prog.data, &rv).has_calls);
        let mut rv_buf = Vec::new();
        rv.emit_zero(&mut rv_buf, FloatPrecision::F64, HwReg(11));
        assert_eq!(rv_buf, 0xF200_05D3_u32.to_le_bytes());
        rv_buf.clear();
        rv.emit_zero(&mut rv_buf, FloatPrecision::F32, HwReg(11));
        assert_eq!(rv_buf, 0xF000_05D3_u32.to_le_bytes());
    }

    #[cfg(target_arch = "x86_64")]
    const IEEE_EDGE_ROWS_F64: [[f64; 3]; 12] = [
        [-0.0, 0.0, 1.0],
        [0.0, -0.0, 1.0],
        [-0.0, -0.0, 0.0],
        [0.0, 0.0, -0.0],
        [f64::NAN, 0.0, 1.0],
        [0.0, f64::NAN, 1.0],
        [f64::NAN, 1.5, 2.0],
        [1.5, f64::NAN, 2.0],
        [f64::NAN, f64::NAN, 1.0],
        [f64::MAX, f64::MAX, f64::MAX],
        [f64::NEG_INFINITY, f64::INFINITY, 0.0],
        [-3.5, -1.25, 2.75],
    ];

    #[cfg(target_arch = "x86_64")]
    const IEEE_EDGE_ROWS_F32: [[f32; 3]; 12] = [
        [-0.0, 0.0, 1.0],
        [0.0, -0.0, 1.0],
        [-0.0, -0.0, 0.0],
        [0.0, 0.0, -0.0],
        [f32::NAN, 0.0, 1.0],
        [0.0, f32::NAN, 1.0],
        [f32::NAN, 1.5, 2.0],
        [1.5, f32::NAN, 2.0],
        [f32::NAN, f32::NAN, 1.0],
        [f32::MAX, f32::MAX, f32::MAX],
        [f32::NEG_INFINITY, f32::INFINITY, 0.0],
        [-3.5, -1.25, 2.75],
    ];

    #[cfg(target_arch = "x86_64")]
    fn assert_f64_ieee(actual: f64, expected: f64, label: &str) {
        if expected.is_nan() {
            assert!(actual.is_nan(), "{label}: expected NaN, got {actual}");
        } else {
            assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn assert_f32_ieee(actual: f32, expected: f32, label: &str) {
        if expected.is_nan() {
            assert!(actual.is_nan(), "{label}: expected NaN, got {actual}");
        } else {
            assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_inline_max_min_clamp_edge_cases_across_all_isa_tiers() {
        use std::arch::is_x86_feature_detected;

        use super::super::super::native::{
            ExecutableBuffer, FEAT_AVX, FEAT_AVX2, FEAT_AVX512, FEAT_FMA, FEAT_SSE41, LivenessPlan,
            NativeEmitter, NativeLayout, Trampolines, X86_64Backend,
        };

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let mut tiers: Vec<u8> = vec![0];
        if is_x86_feature_detected!("avx") {
            tiers.push(FEAT_SSE41 | FEAT_AVX);
        }
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            tiers.push(FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA);
        }
        if is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512vl")
            && is_x86_feature_detected!("avx512dq")
            && is_x86_feature_detected!("fma")
        {
            tiers.push(FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA | FEAT_AVX512);
        }

        let exprs = [
            x.max(y),
            x.min(y),
            x.clamp(y, z),
            (-x).max(y.abs()) + (-z) + x.min(y).abs(),
            (x + y).max(x - y) + (x + y).min(y),
        ];

        for expr in &exprs {
            let prog64 = Program::<f64>::compile(expr, &[x, y, z]).expect("f64");
            let prog32 = Program::<f32>::compile(expr, &[x, y, z]).expect("f32");
            for &features in &tiers {
                let mut be64 = X86_64Backend::with_features(features);
                assert!(!LivenessPlan::analyze(&prog64.data, &be64).has_calls);
                let (code64, start64) =
                    NativeEmitter::compile(&prog64.data, NativeLayout::F64, false, &mut be64)
                        .expect("compile f64");
                let buf64 = ExecutableBuffer::new(&code64[start64..]).expect("exec f64");
                let tr64 = Trampolines::table(NativeLayout::F64).as_ptr().cast::<()>();
                let mut ws64 = Workspace::new();
                ws64.prepare::<1>(&prog64.data);
                for row in IEEE_EDGE_ROWS_F64 {
                    ws64.registers[..3].copy_from_slice(&row);
                    let jit64 = x86_exec::scalar_f64(&buf64, &mut ws64.registers, tr64);
                    assert_f64_ieee(jit64, prog64.eval(&row).expect("vm64"), "scalar f64");
                }

                let mut be32 = X86_64Backend::with_features(features);
                assert!(!LivenessPlan::analyze(&prog32.data, &be32).has_calls);
                let (code32, start32) =
                    NativeEmitter::compile(&prog32.data, NativeLayout::F32, false, &mut be32)
                        .expect("compile f32");
                let buf32 = ExecutableBuffer::new(&code32[start32..]).expect("exec f32");
                let tr32 = Trampolines::table(NativeLayout::F32).as_ptr().cast::<()>();
                let mut ws32 = Workspace::new();
                ws32.prepare::<1>(&prog32.data);
                for row32 in IEEE_EDGE_ROWS_F32 {
                    ws32.registers[..3].copy_from_slice(&row32);
                    let jit32 = x86_exec::scalar_f32(&buf32, &mut ws32.registers, tr32);
                    assert_f32_ieee(jit32, prog32.eval(&row32).expect("vm32"), "scalar f32");
                }
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_inline_max_min_clamp_simd_batch_edge_cases() {
        use std::arch::is_x86_feature_detected;

        use super::super::super::native::{
            ExecutableBuffer, FEAT_AVX, FEAT_AVX2, FEAT_FMA, FEAT_SSE41, FloatPrecision,
            LivenessPlan, NativeEmitter, Trampolines, X86_64Backend,
        };

        let context = Context::new().expect("context");
        let x = context.symb("x").expect("x");
        let y = context.symb("y").expect("y");
        let z = context.symb("z").expect("z");

        let batch_expr = x.max(y) + x.min(y) + x.clamp(y, z);
        let bprog64 = Program::<f64>::compile(&batch_expr, &[x, y, z]).expect("bf64");
        let bnat64 = bprog64.with_native().expect("bnat64");
        let cols64: [Vec<f64>; 3] = core::array::from_fn(|col| {
            IEEE_EDGE_ROWS_F64
                .iter()
                .cycle()
                .take(17)
                .map(|r| r[col])
                .collect()
        });
        let (mut vm_b64, mut nat_b64) = (vec![0.0_f64; 17], vec![0.0_f64; 17]);
        bprog64
            .eval_batch(&[&cols64[0], &cols64[1], &cols64[2]], &mut vm_b64)
            .expect("vm b64");
        bnat64
            .eval_batch(&[&cols64[0], &cols64[1], &cols64[2]], &mut nat_b64)
            .expect("nat b64");
        for (&got, &exp) in nat_b64.iter().zip(&vm_b64) {
            assert_f64_ieee(got, exp, "batch f64");
        }

        let bprog32 = Program::<f32>::compile(&batch_expr, &[x, y, z]).expect("bf32");
        let bnat32 = bprog32.with_native().expect("bnat32");
        let cols32: [Vec<f32>; 3] = core::array::from_fn(|col| {
            IEEE_EDGE_ROWS_F32
                .iter()
                .cycle()
                .take(17)
                .map(|r| r[col])
                .collect()
        });
        let (mut vm_b32, mut nat_b32) = (vec![0.0_f32; 17], vec![0.0_f32; 17]);
        bprog32
            .eval_batch(&[&cols32[0], &cols32[1], &cols32[2]], &mut vm_b32)
            .expect("vm b32");
        bnat32
            .eval_batch(&[&cols32[0], &cols32[1], &cols32[2]], &mut nat_b32)
            .expect("nat b32");
        for (&got, &exp) in nat_b32.iter().zip(&vm_b32) {
            assert_f32_ieee(got, exp, "batch f32");
        }

        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            let avx2_feats = FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA;
            for expr in [x.max(y), x.min(y)] {
                let prog = Program::<f64>::compile(&expr, &[x, y]).expect("v256");
                let (mut backend, lanes) = X86_64Backend::with_features(avx2_feats)
                    .into_batch(FloatPrecision::F64)
                    .expect("vec256");
                assert_eq!(lanes, 4);
                assert!(!LivenessPlan::analyze(&prog.data, &backend).has_calls);
                let (code, start) = NativeEmitter::compile_vector(
                    &prog.data,
                    FloatPrecision::F64,
                    lanes,
                    &mut backend,
                )
                .expect("vec256 compile");
                let buf = ExecutableBuffer::new(&code[start..]).expect("exec v256");
                let mut ws = Workspace::new();
                let regs = ws.prepare::<4>(&prog.data);
                let lhs = [-0.0_f64, 0.0, f64::NAN, 3.5];
                let rhs = [0.0_f64, -0.0, 0.0, -2.0];
                regs[0] = lhs;
                regs[1] = rhs;
                let tr = Trampolines::vector_table(FloatPrecision::F64, 4)
                    .expect("table")
                    .as_ptr()
                    .cast::<()>();
                x86_exec::vec_f64(&buf, regs.as_mut_ptr().cast::<f64>(), tr);
                let mut exp = [0.0_f64; 4];
                prog.eval_batch(&[&lhs, &rhs], &mut exp).expect("vm v256");
                let res_idx = usize::try_from(prog.data.results[0]).expect("res idx");
                for lane in 0..4 {
                    assert_f64_ieee(regs[res_idx][lane], exp[lane], "v256 lane");
                }
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn x86_64_inline_unary_mem_folding_matches_vm_across_isa_tiers() {
        use std::arch::is_x86_feature_detected;

        use super::super::super::native::{
            ExecutableBuffer, FEAT_AVX, FEAT_AVX2, FEAT_AVX512, FEAT_FMA, FEAT_SSE41,
            FloatPrecision, NativeEmitter, NativeLayout, Trampolines, X86_64Backend,
        };

        let context = Context::new().expect("context");
        let syms = ["a", "b", "c", "d", "e"].map(|name| context.symb(name).expect("sym"));
        // Single-use parameters fold directly from memory for `sqrt`/`floor`/`ceil`/`trunc`,
        // while `abs` stays register-only (avoids 16-byte scalar over-read).
        let expr =
            syms[0].sqrt() + syms[1].floor() + syms[2].ceil() + syms[3].trunc() + syms[4].abs();
        let prog64 = Program::<f64>::compile(&expr, &syms).expect("f64");
        let prog32 = Program::<f32>::compile(&expr, &syms).expect("f32");
        let row64 = [2.25_f64, -1.3, 2.1, -3.9, -4.5];
        let row32 = [2.25_f32, -1.3, 2.1, -3.9, -4.5];
        let vm64 = prog64.eval(&row64).expect("vm64");
        let vm32 = prog32.eval(&row32).expect("vm32");

        let mut tiers: Vec<u8> = vec![0];
        if is_x86_feature_detected!("sse4.1") {
            tiers.push(FEAT_SSE41);
        }
        if is_x86_feature_detected!("avx") {
            tiers.push(FEAT_SSE41 | FEAT_AVX);
        }
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            tiers.push(FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA);
        }
        if is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512vl")
            && is_x86_feature_detected!("avx512dq")
            && is_x86_feature_detected!("fma")
        {
            tiers.push(FEAT_SSE41 | FEAT_AVX | FEAT_AVX2 | FEAT_FMA | FEAT_AVX512);
        }

        for features in tiers {
            let mut be64 = X86_64Backend::with_features(features);
            let (code64, start64) =
                NativeEmitter::compile(&prog64.data, NativeLayout::F64, false, &mut be64)
                    .expect("f64 tier");
            let buf64 = ExecutableBuffer::new(&code64[start64..]).expect("exec64");
            let mut ws64 = Workspace::new();
            ws64.prepare::<1>(&prog64.data);
            ws64.registers[..5].copy_from_slice(&row64);
            let tr64 = Trampolines::table(NativeLayout::F64).as_ptr().cast::<()>();
            assert_eq!(
                x86_exec::scalar_f64(&buf64, &mut ws64.registers, tr64).to_bits(),
                vm64.to_bits()
            );

            let mut be32 = X86_64Backend::with_features(features);
            let (code32, start32) =
                NativeEmitter::compile(&prog32.data, NativeLayout::F32, false, &mut be32)
                    .expect("f32 tier");
            let buf32 = ExecutableBuffer::new(&code32[start32..]).expect("exec32");
            let mut ws32 = Workspace::new();
            ws32.prepare::<1>(&prog32.data);
            ws32.registers[..5].copy_from_slice(&row32);
            let tr32 = Trampolines::table(NativeLayout::F32).as_ptr().cast::<()>();
            assert_eq!(
                x86_exec::scalar_f32(&buf32, &mut ws32.registers, tr32).to_bits(),
                vm32.to_bits()
            );

            if let Some((mut vbe, lanes)) =
                X86_64Backend::with_features(features).into_batch(FloatPrecision::F64)
            {
                let (vcode, vstart) = NativeEmitter::compile_vector(
                    &prog64.data,
                    FloatPrecision::F64,
                    lanes,
                    &mut vbe,
                )
                .expect("vec f64 tier");
                let vbuf = ExecutableBuffer::new(&vcode[vstart..]).expect("exec vec64");
                let trv = Trampolines::vector_table(FloatPrecision::F64, lanes)
                    .expect("vec table")
                    .as_ptr()
                    .cast::<()>();
                let mut vws = Workspace::new();
                let res_idx = usize::try_from(prog64.data.results[0]).expect("res");
                if lanes == 4 {
                    let regs = vws.prepare::<4>(&prog64.data);
                    for (slot, val) in regs[..5].iter_mut().zip(row64) {
                        *slot = [val; 4];
                    }
                    x86_exec::vec_f64(&vbuf, regs.as_mut_ptr().cast::<f64>(), trv);
                    for lane in regs[res_idx] {
                        assert_eq!(lane.to_bits(), vm64.to_bits());
                    }
                } else {
                    let regs = vws.prepare::<8>(&prog64.data);
                    for (slot, val) in regs[..5].iter_mut().zip(row64) {
                        *slot = [val; 8];
                    }
                    x86_exec::vec_f64(&vbuf, regs.as_mut_ptr().cast::<f64>(), trv);
                    for lane in regs[res_idx] {
                        assert_eq!(lane.to_bits(), vm64.to_bits());
                    }
                }
            }
        }
    }
}
