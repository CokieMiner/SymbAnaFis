/// L1-cache optimized execution loop for the register-based evaluator.
///
/// # Safety
///
/// The caller must ensure that:
/// 1. `bytecode` is a well-formed stream of instructions terminated by a 0 (End) opcode.
/// 2. `pc` arithmetic stays within the bounds of the bytecode slice.
/// 3. All register indices in the bytecode are within the range `0..workspace_size`.
/// 4. The `arg_pool` contains valid indices for N-ary instructions.
macro_rules! dispatch_loop {
    ($bytecode:ident, $regs:ident, $arg_pool:ident, $mode:tt, $one:ident, $b1:ident, $b2:ident, $b3:ident, $b4:ident) => {
        let mut pc = $bytecode.as_ptr();

        loop {
            let opcode = *pc;
            pc = pc.add(1);

            match opcode {
                0 /* End */ => break,
                1 /* Copy */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    *($regs.add(dest)) = *($regs.add(src));
                }
                2 /* Neg */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    *($regs.add(dest)) = -*($regs.add(src));
                }
                3 /* SinCos */ => {
                    let sin_dest = *pc as usize;
                    let cos_dest = *pc.add(1) as usize;
                    let arg = *pc.add(2) as usize;
                    pc = pc.add(3);
                    let v = *($regs.add(arg));
                    let (s, c) = dispatch_loop!(@sincos v, $mode);
                    *($regs.add(sin_dest)) = s;
                    *($regs.add(cos_dest)) = c;
                }
                42 /* AsinAcos */ => {
                    let asin_dest = *pc as usize;
                    let acos_dest = *pc.add(1) as usize;
                    let arg = *pc.add(2) as usize;
                    pc = pc.add(3);
                    let v = *($regs.add(arg));
                    let (s, c) = dispatch_loop!(@asinacos v, $mode);
                    *($regs.add(asin_dest)) = s;
                    *($regs.add(acos_dest)) = c;
                }
                4 /* Add */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = *($regs.add(a)) + *($regs.add(b));
                }
                5 /* Add3 */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    pc = pc.add(4);
                    *($regs.add(dest)) = *($regs.add(a)) + *($regs.add(b)) + *($regs.add(c));
                }
                6 /* Add4 */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    let d = *pc.add(4) as usize;
                    pc = pc.add(5);
                    *($regs.add(dest)) =
                        *($regs.add(a)) + *($regs.add(b)) + *($regs.add(c)) + *($regs.add(d));
                }
                7 /* AddN */ => {
                    let dest = *pc as usize;
                    let start_idx = *pc.add(1) as usize;
                    let count = *pc.add(2) as usize;
                    pc = pc.add(3);
                    let mut pool_ptr = $arg_pool.as_ptr().add(start_idx);
                    let mut sum = *($regs.add(*pool_ptr as usize));
                    pool_ptr = pool_ptr.add(1);
                    for _ in 1..count {
                        sum += *($regs.add(*pool_ptr as usize));
                        pool_ptr = pool_ptr.add(1);
                    }
                    *($regs.add(dest)) = sum;
                }
                8 /* Mul */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = *($regs.add(a)) * *($regs.add(b));
                }
                9 /* Mul3 */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    pc = pc.add(4);
                    *($regs.add(dest)) = *($regs.add(a)) * *($regs.add(b)) * *($regs.add(c));
                }
                10 /* Mul4 */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    let d = *pc.add(4) as usize;
                    pc = pc.add(5);
                    *($regs.add(dest)) =
                        *($regs.add(a)) * *($regs.add(b)) * *($regs.add(c)) * *($regs.add(d));
                }
                11 /* MulN */ => {
                    let dest = *pc as usize;
                    let start_idx = *pc.add(1) as usize;
                    let count = *pc.add(2) as usize;
                    pc = pc.add(3);
                    let mut pool_ptr = $arg_pool.as_ptr().add(start_idx);
                    let mut prod = *($regs.add(*pool_ptr as usize));
                    pool_ptr = pool_ptr.add(1);
                    for _ in 1..count {
                        prod *= *($regs.add(*pool_ptr as usize));
                        pool_ptr = pool_ptr.add(1);
                    }
                    *($regs.add(dest)) = prod;
                }
                12 /* Sub */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = *($regs.add(a)) - *($regs.add(b));
                }
                13 /* Div */ => {
                    let dest = *pc as usize;
                    let num = *pc.add(1) as usize;
                    let den = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = *($regs.add(num)) / *($regs.add(den));
                }
                14 /* Pow */ => {
                    let dest = *pc as usize;
                    let base = *pc.add(1) as usize;
                    let exp = *pc.add(2) as usize;
                    pc = pc.add(3);
                    let b = *($regs.add(base));
                    let e = *($regs.add(exp));
                    *($regs.add(dest)) = dispatch_loop!(@pow b, e, $mode);
                }
                15 /* MulAdd */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    pc = pc.add(4);
                    let va = *($regs.add(a));
                    let vb = *($regs.add(b));
                    let vc = *($regs.add(c));
                    *($regs.add(dest)) = va.mul_add(vb, vc);
                }
                16 /* MulSub */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    pc = pc.add(4);
                    let va = *($regs.add(a));
                    let vb = *($regs.add(b));
                    let vc = *($regs.add(c));
                    *($regs.add(dest)) = va.mul_add(vb, -vc);
                }
                17 /* NegMul */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = -(*($regs.add(a)) * *($regs.add(b)));
                }
                18 /* NegMulAdd */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    pc = pc.add(4);
                    let va = *($regs.add(a));
                    let vb = *($regs.add(b));
                    let vc = *($regs.add(c));
                    *($regs.add(dest)) = (-va).mul_add(vb, vc);
                }
                19 /* NegMulSub */ => {
                    let dest = *pc as usize;
                    let a = *pc.add(1) as usize;
                    let b = *pc.add(2) as usize;
                    let c = *pc.add(3) as usize;
                    pc = pc.add(4);
                    let va = *($regs.add(a));
                    let vb = *($regs.add(b));
                    let vc = *($regs.add(c));
                    *($regs.add(dest)) = (-va).mul_add(vb, -vc);
                }
                20 /* Square */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = v * v;
                }
                21 /* Cube */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = v * v * v;
                }
                22 /* Pow4 */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    let v2 = v * v;
                    *($regs.add(dest)) = v2 * v2;
                }
                23 /* Pow3_2 */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = v * v.sqrt();
                }
                24 /* InvPow3_2 */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = $one / (v * v.sqrt());
                }
                25 /* InvSqrt */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    *($regs.add(dest)) = $one / (*($regs.add(src))).sqrt();
                }
                26 /* InvSquare */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = $one / (v * v);
                }
                27 /* InvCube */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = $one / (v * v * v);
                }
                28 /* Recip */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    *($regs.add(dest)) = $one / *($regs.add(src));
                }
                29 /* Powi */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    #[allow(
                        clippy::cast_possible_wrap,
                        reason = "Restoring two's complement i32 from flat bytecode u32"
                    )]
                    let n = *pc.add(2) as i32;
                    pc = pc.add(3);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = dispatch_loop!(@powi v, n, $mode);
                }
                30 /* Sin */ => {
                    let dest = *pc as usize;
                    let arg = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(arg));
                    *($regs.add(dest)) = dispatch_loop!(@sin v, $mode);
                }
                31 /* Cos */ => {
                    let dest = *pc as usize;
                    let arg = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(arg));
                    *($regs.add(dest)) = dispatch_loop!(@cos v, $mode);
                }
                32 /* Exp */ => {
                    let dest = *pc as usize;
                    let arg = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(arg));
                    *($regs.add(dest)) = dispatch_loop!(@exp v, $mode);
                }
                33 /* Ln */ => {
                    let dest = *pc as usize;
                    let arg = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(arg));
                    *($regs.add(dest)) = dispatch_loop!(@ln v, $mode);
                }
                34 /* Sqrt */ => {
                    let dest = *pc as usize;
                    let arg = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(arg));
                    *($regs.add(dest)) = dispatch_loop!(@sqrt v, $mode);
                }
                35 /* RecipExpm1 */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = dispatch_loop!(@recip_expm1 v, $mode, $one);
                }
                36 /* ExpSqr */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = dispatch_loop!(@exp_sqr v, $mode);
                }
                37 /* ExpSqrNeg */ => {
                    let dest = *pc as usize;
                    let src = *pc.add(1) as usize;
                    pc = pc.add(2);
                    let v = *($regs.add(src));
                    *($regs.add(dest)) = dispatch_loop!(@exp_sqr_neg v, $mode);
                }
                38 /* Builtin1 */ => {
                    let dest = *pc as usize;
                    let op: FnOp = unsafe { std::mem::transmute(*pc.add(1) as u8) };
                    let arg = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = $b1(op, *($regs.add(arg)));
                }
                39 /* Builtin2 */ => {
                    let dest = *pc as usize;
                    let op: FnOp = unsafe { std::mem::transmute(*pc.add(1) as u8) };
                    let arg1 = *pc.add(2) as usize;
                    let arg2 = *pc.add(3) as usize;
                    pc = pc.add(4);
                    *($regs.add(dest)) = $b2(op, *($regs.add(arg1)), *($regs.add(arg2)));
                }
                40 /* Builtin3 */ => {
                    let dest = *pc as usize;
                    let op: FnOp = unsafe { std::mem::transmute(*pc.add(1) as u8) };
                    let arg1 = *pc.add(2) as usize;
                    let arg2 = *pc.add(3) as usize;
                    let arg3 = *pc.add(4) as usize;
                    pc = pc.add(5);
                    *($regs.add(dest)) = $b3(
                        op,
                        *($regs.add(arg1)),
                        *($regs.add(arg2)),
                        *($regs.add(arg3)),
                    );
                }
                41 /* Builtin4 */ => {
                    let dest = *pc as usize;
                    let op: FnOp = unsafe { std::mem::transmute(*pc.add(1) as u8) };
                    let arg1 = *pc.add(2) as usize;
                    let arg2 = *pc.add(3) as usize;
                    let arg3 = *pc.add(4) as usize;
                    let arg4 = *pc.add(5) as usize;
                    pc = pc.add(6);
                    *($regs.add(dest)) = $b4(
                        op,
                        *($regs.add(arg1)),
                        *($regs.add(arg2)),
                        *($regs.add(arg3)),
                        *($regs.add(arg4)),
                    );
                }
                _ => {
                    debug_assert!(false, "invalid opcode {opcode}");
                    unsafe { std::hint::unreachable_unchecked() }
                },
            }
        }
    };

    // Internal Pow dispatch
    (@pow $b:ident, $e:ident, scalar) => { $b.powf($e) };
    (@pow $b:ident, $e:ident, simd) => { $b.pow_f64x4($e) };

    (@powi $v:ident, $n:ident, scalar) => { $v.powi($n) };
    (@powi $v:ident, $n:ident, simd) => {{
        let arr = $v.to_array();
        f64x4::new([arr[0].powi($n), arr[1].powi($n), arr[2].powi($n), arr[3].powi($n)])
    }};

    // recip_expm1(x) = 1 / (exp(x) - 1) = 1 / expm1(x)
    (@recip_expm1 $v:ident, scalar, $one:ident) => { $one / $v.exp_m1() };
    (@recip_expm1 $v:ident, simd, $one:ident) => {{
        let arr = $v.to_array();
        $one / f64x4::new([arr[0].exp_m1(), arr[1].exp_m1(), arr[2].exp_m1(), arr[3].exp_m1()])
    }};

    (@exp_sqr $v:ident, $mode:tt) => { ($v * $v).exp() };

    (@exp_sqr_neg $v:ident, $mode:tt) => { ( -($v * $v) ).exp() };

    (@sincos $v:ident, $mode:tt) => { $v.sin_cos() };

    (@asinacos $v:ident, scalar) => { ($v.asin(), $v.acos()) };
    (@asinacos $v:ident, simd) => { $v.asin_acos() };

    (@sin $v:ident, $mode:tt) => { $v.sin() };

    (@cos $v:ident, $mode:tt) => { $v.cos() };

    (@exp $v:ident, $mode:tt) => { $v.exp() };

    (@ln $v:ident, $mode:tt) => { $v.ln() };

    (@sqrt $v:ident, $mode:tt) => { $v.sqrt() };
}

/// Macro to handle the dispatch staircase for stack-allocated register files.
macro_rules! evaluate_staircase {
    ($self:ident, $params:ident, [$($size:ident),*]) => {
        $(
            if $self.workspace_size <= $size {
                return $self.evaluate_inline::<$size>($params);
            }
        )*
    };
}
