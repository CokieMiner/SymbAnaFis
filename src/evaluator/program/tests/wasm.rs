//! Standalone WebAssembly (`.wasm`) binary export, stack scheduling, local compaction,
//! and differential bytecode execution against the register VM.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test expressions and stack-machine interpreter use bounded indices and symbolic operators"
)]

use alloc::vec::Vec;

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr},
    evaluator::Program,
};

/// Binary section parser and stack-machine interpreter for generated `.wasm` modules.
pub struct WasmInspector;

impl WasmInspector {
    pub fn sections(bytes: &[u8]) -> Vec<(u8, &[u8])> {
        assert_eq!(bytes.get(..8), Some(&b"\0asm\x01\0\0\0"[..]));
        let mut pos = 8_usize;
        let mut sections = Vec::new();
        while pos < bytes.len() {
            let id = bytes[pos];
            pos += 1;
            let mut len = 0_usize;
            let mut shift = 0_u32;
            loop {
                let byte = bytes[pos];
                pos += 1;
                len |= usize::from(byte & 0x7F) << shift;
                if (byte & 0x80) == 0 {
                    break;
                }
                shift += 7;
            }
            sections.push((id, &bytes[pos..pos + len]));
            pos += len;
        }
        assert_eq!(pos, bytes.len());
        sections
    }

    fn read_leb_u32(bytes: &[u8], pos: &mut usize) -> u32 {
        let mut val = 0_u32;
        let mut shift = 0_u32;
        loop {
            let byte = bytes[*pos];
            *pos += 1;
            val |= u32::from(byte & 0x7F) << shift;
            if (byte & 0x80) == 0 {
                return val;
            }
            shift += 7;
        }
    }

    fn read_name<'bytes>(bytes: &'bytes [u8], pos: &mut usize) -> &'bytes str {
        let len = usize::try_from(Self::read_leb_u32(bytes, pos)).expect("name len");
        let slice = &bytes[*pos..*pos + len];
        *pos += len;
        core::str::from_utf8(slice).expect("utf8 name")
    }

    fn eval_math_import(name: &str, stack: &mut Vec<f64>) -> Option<f64> {
        use num_anafis::Numeric;

        Some(match name {
            "sin" => Numeric::sin(&stack.pop()?),
            "cos" => Numeric::cos(&stack.pop()?),
            "tan" => Numeric::tan(&stack.pop()?),
            "sinh" => Numeric::sinh(&stack.pop()?),
            "cosh" => Numeric::cosh(&stack.pop()?),
            "tanh" => Numeric::tanh(&stack.pop()?),
            "exp" => Numeric::exp(&stack.pop()?),
            "ln" => Numeric::ln(&stack.pop()?),
            "log10" => Numeric::log10(&stack.pop()?),
            "round" => Numeric::round(&stack.pop()?),
            "erf" => Numeric::erf(&stack.pop()?),
            "erfc" => Numeric::erfc(&stack.pop()?),
            "log2" => Numeric::log2(&stack.pop()?),
            "pow" => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                Numeric::pow(&a, &b)
            }
            "atan2" => {
                let x = stack.pop()?;
                let y = stack.pop()?;
                Numeric::atan2(&y, &x)
            }
            "log" => {
                let val = stack.pop()?;
                let base = stack.pop()?;
                Numeric::log(&val, &base)
            }
            "assoc_legendre" => {
                let val = stack.pop()?;
                let order = stack.pop()?;
                let degree = stack.pop()?;
                Numeric::assoc_legendre(&val, &degree, &order)
            }
            _ => return None,
        })
    }

    pub fn eval_f64(bytes: &[u8], args: &[f64]) -> Option<f64> {
        let sections = Self::sections(bytes);
        let mut imports: Vec<&str> = Vec::new();
        let mut code_body: &[u8] = &[];
        for (id, payload) in sections {
            if id == 2 {
                let mut pos = 0_usize;
                for _ in 0..Self::read_leb_u32(payload, &mut pos) {
                    assert_eq!(Self::read_name(payload, &mut pos), "math");
                    let name = Self::read_name(payload, &mut pos);
                    assert_eq!(payload[pos], 0x00);
                    pos += 1;
                    let _type_idx = Self::read_leb_u32(payload, &mut pos);
                    imports.push(name);
                }
                assert_eq!(pos, payload.len());
            } else if id == 10 {
                let mut pos = 0_usize;
                assert_eq!(Self::read_leb_u32(payload, &mut pos), 1);
                let body_len =
                    usize::try_from(Self::read_leb_u32(payload, &mut pos)).expect("body len");
                code_body = &payload[pos..pos + body_len];
                assert_eq!(pos + body_len, payload.len());
            }
        }
        let mut pos = 0_usize;
        let mut locals = args.to_vec();
        for _ in 0..Self::read_leb_u32(code_body, &mut pos) {
            let count =
                usize::try_from(Self::read_leb_u32(code_body, &mut pos)).expect("local count");
            assert_eq!(code_body[pos], 0x7C);
            pos += 1;
            locals.resize(locals.len() + count, 0.0_f64);
        }
        let mut stack: Vec<f64> = Vec::new();
        while pos < code_body.len() {
            let op = code_body[pos];
            pos += 1;
            match op {
                0x0B => break,
                0x10 => {
                    let idx =
                        usize::try_from(Self::read_leb_u32(code_body, &mut pos)).expect("fn idx");
                    let res = Self::eval_math_import(imports[idx], &mut stack)?;
                    stack.push(res);
                }
                0x1A => {
                    let _ = stack.pop()?;
                }
                0x20 => {
                    let idx = usize::try_from(Self::read_leb_u32(code_body, &mut pos))
                        .expect("local idx");
                    stack.push(locals[idx]);
                }
                0x21 => {
                    let idx = usize::try_from(Self::read_leb_u32(code_body, &mut pos))
                        .expect("local idx");
                    locals[idx] = stack.pop()?;
                }
                0x22 => {
                    let idx = usize::try_from(Self::read_leb_u32(code_body, &mut pos))
                        .expect("local idx");
                    locals[idx] = *stack.last()?;
                }
                0x44 => {
                    let mut raw = [0_u8; 8];
                    raw.copy_from_slice(&code_body[pos..pos + 8]);
                    pos += 8;
                    stack.push(f64::from_bits(u64::from_le_bytes(raw)));
                }
                0x99..=0x9D | 0x9F => {
                    let v = stack.pop()?;
                    stack.push(match op {
                        0x99 => v.abs(),
                        0x9A => -v,
                        0x9B => v.ceil(),
                        0x9C => v.floor(),
                        0x9D => v.trunc(),
                        _ => v.sqrt(),
                    });
                }
                0xA0..=0xA5 => {
                    use num_anafis::Numeric;

                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(match op {
                        0xA0 => a + b,
                        0xA1 => a - b,
                        0xA2 => a * b,
                        0xA3 => a / b,
                        0xA4 => Numeric::min(&a, &b),
                        _ => Numeric::max(&a, &b),
                    });
                }
                _ => return None,
            }
        }
        assert_eq!(pos, code_body.len());
        assert_eq!(stack.len(), 1);
        stack.pop()
    }
}

proptest! {
    #[test]
    fn wasm_binary_export_produces_valid_header_and_sections(
        a in -5_i32..=5,
        b in -5_i32..=5,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let expr = (x * a + y * b).sin() + (x / (y * y + 2)).sqrt() + x.cos();
        let program = Program::<f64>::compile(&expr, &[x, y])?;
        let wasm = program.to_wasm_bytes()?;
        let sections = WasmInspector::sections(&wasm);
        // Type section must declare eval as `(param f64 f64) (result f64)` (`60 02 7C 7C 01 7C`).
        prop_assert_eq!(sections[0].0, 1);
        prop_assert!(sections[0].1.windows(6).any(|w| w == [0x60, 0x02, 0x7C, 0x7C, 0x01, 0x7C]));
        prop_assert!(!wasm.windows(6).any(|w| w == b"memory"));

        #[cfg(native_jit_host)]
        {
            let native1 = program.with_native()?;
            let native2 = program.with_native()?;
            prop_assert_eq!(native1.native_bytes(), native2.native_bytes());
            let vm_val = program.eval(&[1.25, -0.75])?;
            let nat_val = native1.eval(&[1.25, -0.75])?;
            prop_assert_eq!(nat_val.to_bits(), vm_val.to_bits());
        }
    }
}

#[test]
fn wasm_export_inlines_constants_and_omits_imports_for_pure_arithmetic() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let poly = x * x + 3 * y - 1;

    let wasm64 = Program::<f64>::compile(&poly, &[x, y])
        .expect("poly f64")
        .to_wasm_bytes()
        .expect("wasm f64");
    let secs64 = WasmInspector::sections(&wasm64);
    let ids64: Vec<u8> = secs64.iter().map(|&(id, _)| id).collect();
    assert_eq!(ids64, [1, 3, 7, 10]);
    assert_eq!(secs64[0].1, [0x01, 0x60, 0x02, 0x7C, 0x7C, 0x01, 0x7C]);
    let mut const3_f64 = [0_u8; 9];
    const3_f64[0] = 0x44;
    const3_f64[1..].copy_from_slice(&3.0_f64.to_bits().to_le_bytes());
    assert!(secs64[3].1.windows(9).any(|w| w == const3_f64));
    assert_eq!(WasmInspector::eval_f64(&wasm64, &[2.0, 5.0]), Some(18.0));

    let wasm32 = Program::<f32>::compile(&poly, &[x, y])
        .expect("poly f32")
        .to_wasm_bytes()
        .expect("wasm f32");
    let secs32 = WasmInspector::sections(&wasm32);
    assert_eq!(secs32[0].1, [0x01, 0x60, 0x02, 0x7D, 0x7D, 0x01, 0x7D]);
    let mut const3_f32 = [0_u8; 5];
    const3_f32[0] = 0x43;
    const3_f32[1..].copy_from_slice(&3.0_f32.to_bits().to_le_bytes());
    assert!(secs32[3].1.windows(5).any(|w| w == const3_f32));

    let id_wasm = Program::<f64>::compile(&Expr::from(x), &[x])
        .expect("id")
        .to_wasm_bytes()
        .expect("id wasm");
    let id_secs = WasmInspector::sections(&id_wasm);
    assert_eq!(id_secs[3].1, [0x01, 0x04, 0x00, 0x20, 0x00, 0x0B]);

    let const_wasm = Program::<f64>::compile(&Expr::number(7), &[])
        .expect("const")
        .to_wasm_bytes()
        .expect("const wasm");
    let const_secs = WasmInspector::sections(&const_wasm);
    assert_eq!(const_secs[0].1, [0x01, 0x60, 0x00, 0x01, 0x7C]);
    let mut expected_const_code = Vec::from([0x01, 0x0B, 0x00, 0x44]);
    expected_const_code.extend_from_slice(&7.0_f64.to_bits().to_le_bytes());
    expected_const_code.push(0x0B);
    assert_eq!(const_secs[3].1, expected_const_code.as_slice());

    let trig_wasm = Program::<f64>::compile(&(x.sin() + x.cos() + y.atan2(x)), &[x, y])
        .expect("trig")
        .to_wasm_bytes()
        .expect("trig wasm");
    let trig_secs = WasmInspector::sections(&trig_wasm);
    let trig_ids: Vec<u8> = trig_secs.iter().map(|&(id, _)| id).collect();
    assert_eq!(trig_ids, [1, 2, 3, 7, 10]);
    assert!(trig_secs[1].1.windows(4).any(|w| w == b"math"));
    assert!(trig_secs[1].1.windows(5).any(|w| w == b"atan2"));
    assert!(!trig_secs[1].1.windows(6).any(|w| w == b"memory"));
}

#[test]
fn wasm_bytecode_execution_matches_vm_across_fused_folds_pairs_and_imports() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let z = context.symb("z").expect("z");

    let fused_variants = (x * y + z) * (x * y - z) + (z - x * y) - (-(x * y) - z);
    let fold_dot = (x + 1) * (y + 1)
        + (y + 2) * (z + 2)
        + (z + 3) * (x + 3)
        + (x + 4) * (y + 5)
        + (y + 6) * (z + 7)
        + (z + 8) * (x + 9)
        + (x + 10) * (y + 11)
        + (y + 12) * (z + 13);
    let paired = x.sin() + x.cos() + y.sinh() + y.cosh();
    let inlines = (x * x + 0.5).sqrt()
        + (x - y).abs()
        + (x * 1.7).floor()
        + (y * 1.3).ceil()
        + (z * 2.9).trunc()
        + (x * 2.5).round();
    let imports = y.atan2(x) + x.pow(y) + x.erf() + x.log(Expr::number(2)) + x.log(y + 2);

    let full = fused_variants + fold_dot + paired + inlines + imports;
    let prog = Program::<f64>::compile(&full, &[x, y, z]).expect("compile");
    let wasm = prog.to_wasm_bytes().expect("wasm bytes");
    let inputs = [0.6_f64, 0.8, -0.4];
    let vm_val = prog.eval(&inputs).expect("vm eval");
    assert!(vm_val.is_finite(), "expected finite vm value, got {vm_val}");
    let wasm_val = WasmInspector::eval_f64(&wasm, &inputs).expect("wasm eval");
    assert!(
        (wasm_val - vm_val).abs() <= 1.0e-12 * vm_val.abs().max(1.0),
        "wasm={wasm_val} vs vm={vm_val}"
    );

    // Shared subexpression read immediately and again later exercises `local.tee` (`0x22`).
    let shared = x.sin() + y;
    let tee_expr = shared.clone() * 3 + shared.clone() * shared;
    let tee_prog = Program::<f64>::compile(&tee_expr, &[x, y]).expect("tee compile");
    let tee_wasm = tee_prog.to_wasm_bytes().expect("tee wasm");
    assert!(tee_wasm.contains(&0x22), "expected local.tee opcode");
    let tee_vm = tee_prog.eval(&[0.5, 1.25]).expect("tee vm");
    let tee_got = WasmInspector::eval_f64(&tee_wasm, &[0.5, 1.25]).expect("tee eval");
    assert!((tee_got - tee_vm).abs() <= 1.0e-13);

    // Separately exercise ternary `"assoc_legendre"` import emission and execution.
    let ternary_prog =
        Program::<f64>::compile(&x.cos().assoc_legendre(2, 1), &[x]).expect("ternary compile");
    let ternary_wasm = ternary_prog.to_wasm_bytes().expect("ternary wasm");
    let ternary_vm = ternary_prog.eval(&[0.6]).expect("ternary vm");
    let ternary_got = WasmInspector::eval_f64(&ternary_wasm, &[0.6]).expect("ternary wasm eval");
    assert_eq!(ternary_got.is_nan(), ternary_vm.is_nan());
}

#[test]
fn wasm_inlines_real_components_max_min_and_clamp_with_zero_imports() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let z = context.symb("z").expect("z");

    let combined = x.re() + y.im() + z.conj() + x.max(y) + y.min(z) + x.clamp(y, z);
    let prog64 = Program::<f64>::compile(&combined, &[x, y, z]).expect("f64 compile");
    let wasm64 = prog64.to_wasm_bytes().expect("f64 wasm");
    let ids64: Vec<u8> = WasmInspector::sections(&wasm64)
        .iter()
        .map(|&(id, _)| id)
        .collect();
    assert_eq!(ids64, [1, 3, 7, 10], "expected zero math imports for f64");
    let inputs = [-2.5_f64, -1.0, 3.25];
    let vm64 = prog64.eval(&inputs).expect("vm64");
    let got64 = WasmInspector::eval_f64(&wasm64, &inputs).expect("wasm64");
    assert_eq!(got64.to_bits(), vm64.to_bits());

    let prog32 = Program::<f32>::compile(&combined, &[x, y, z]).expect("f32 compile");
    let wasm32 = prog32.to_wasm_bytes().expect("f32 wasm");
    let ids32: Vec<u8> = WasmInspector::sections(&wasm32)
        .iter()
        .map(|&(id, _)| id)
        .collect();
    assert_eq!(ids32, [1, 3, 7, 10], "expected zero math imports for f32");

    // Verify IEEE 754-2019 signed-zero and NaN semantics for inline max/min/clamp.
    let max_prog = Program::<f64>::compile(&x.max(y), &[x, y]).expect("max");
    let min_prog = Program::<f64>::compile(&x.min(y), &[x, y]).expect("min");
    let max_wasm = max_prog.to_wasm_bytes().expect("max wasm");
    let min_wasm = min_prog.to_wasm_bytes().expect("min wasm");
    for pair in [
        [-0.0_f64, 0.0],
        [0.0, -0.0],
        [-0.0, -0.0],
        [0.0, 0.0],
        [f64::NAN, 1.0],
        [1.0, f64::NAN],
        [f64::NAN, f64::NAN],
        [f64::NEG_INFINITY, f64::INFINITY],
    ] {
        for (prog, wasm) in [(&max_prog, &max_wasm), (&min_prog, &min_wasm)] {
            let expected = prog.eval(&pair).expect("vm");
            let actual = WasmInspector::eval_f64(wasm, &pair).expect("wasm");
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
    }
}
