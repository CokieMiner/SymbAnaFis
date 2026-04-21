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
                    let mut sum = *($regs.add(*$arg_pool.get_unchecked(start_idx) as usize));
                    for i in 1..count {
                        sum += *($regs.add(*$arg_pool.get_unchecked(start_idx + i) as usize));
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
                    let mut prod = *($regs.add(*$arg_pool.get_unchecked(start_idx) as usize));
                    for i in 1..count {
                        prod *= *($regs.add(*$arg_pool.get_unchecked(start_idx + i) as usize));
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
                    let op = unsafe { std::mem::transmute_copy::<u32, FnOp>(&*pc.add(1)) };
                    let arg = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = $b1(op, *($regs.add(arg)));
                }
                39 /* Builtin2 */ => {
                    let dest = *pc as usize;
                    let op = unsafe { std::mem::transmute_copy::<u32, FnOp>(&*pc.add(1)) };
                    let arg1 = *pc.add(2) as usize;
                    let arg2 = *pc.add(3) as usize;
                    pc = pc.add(4);
                    *($regs.add(dest)) = $b2(op, *($regs.add(arg1)), *($regs.add(arg2)));
                }
                40 /* Builtin3 */ => {
                    let dest = *pc as usize;
                    let op = unsafe { std::mem::transmute_copy::<u32, FnOp>(&*pc.add(1)) };
                    let start_idx = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = $b3(
                        op,
                        *($regs.add(*$arg_pool.get_unchecked(start_idx) as usize)),
                        *($regs.add(*$arg_pool.get_unchecked(start_idx + 1) as usize)),
                        *($regs.add(*$arg_pool.get_unchecked(start_idx + 2) as usize)),
                    );
                }
                41 /* Builtin4 */ => {
                    let dest = *pc as usize;
                    let op = unsafe { std::mem::transmute_copy::<u32, FnOp>(&*pc.add(1)) };
                    let start_idx = *pc.add(2) as usize;
                    pc = pc.add(3);
                    *($regs.add(dest)) = $b4(
                        op,
                        *($regs.add(*$arg_pool.get_unchecked(start_idx) as usize)),
                        *($regs.add(*$arg_pool.get_unchecked(start_idx + 1) as usize)),
                        *($regs.add(*$arg_pool.get_unchecked(start_idx + 2) as usize)),
                        *($regs.add(*$arg_pool.get_unchecked(start_idx + 3) as usize)),
                    );
                }
                _ => unsafe { std::hint::unreachable_unchecked() },
            }
        }
    };

    // Internal Pow dispatch
    (@pow $b:ident, $e:ident, scalar) => { $b.powf($e) };
    (@pow $b:ident, $e:ident, simd) => {
        {
            let arr_b = $b.to_array();
            let arr_e = $e.to_array();
            f64x4::from([
                arr_b[0].powf(arr_e[0]),
                arr_b[1].powf(arr_e[1]),
                arr_b[2].powf(arr_e[2]),
                arr_b[3].powf(arr_e[3]),
            ])
        }
    };

    (@powi $v:ident, $n:ident, scalar) => { $v.powi($n) };
    (@powi $v:ident, $n:ident, simd) => { f64x4::from($v.to_array().map(|v| v.powi($n))) };

    (@recip_expm1 $v:ident, scalar, $one:ident) => { $one / $v.exp_m1() };
    (@recip_expm1 $v:ident, simd, $one:ident) => { $one / f64x4::from($v.to_array().map(f64::exp_m1)) };

    (@exp_sqr $v:ident, scalar) => { ($v * $v).exp() };
    (@exp_sqr $v:ident, simd) => { f64x4::from(($v * $v).to_array().map(f64::exp)) };

    (@exp_sqr_neg $v:ident, scalar) => { ( -($v * $v) ).exp() };
    (@exp_sqr_neg $v:ident, simd) => { f64x4::from((-($v * $v)).to_array().map(f64::exp)) };

    (@sincos $v:ident, scalar) => { $v.sin_cos() };
    (@sincos $v:ident, simd) => {
        {
            let arr = $v.to_array();
            let (s0, c0) = arr[0].sin_cos();
            let (s1, c1) = arr[1].sin_cos();
            let (s2, c2) = arr[2].sin_cos();
            let (s3, c3) = arr[3].sin_cos();
            (f64x4::from([s0, s1, s2, s3]), f64x4::from([c0, c1, c2, c3]))
        }
    };

    (@sin $v:ident, scalar) => { $v.sin() };
    (@sin $v:ident, simd) => { f64x4::from($v.to_array().map(f64::sin)) };

    (@cos $v:ident, scalar) => { $v.cos() };
    (@cos $v:ident, simd) => { f64x4::from($v.to_array().map(f64::cos)) };

    (@exp $v:ident, scalar) => { $v.exp() };
    (@exp $v:ident, simd) => { f64x4::from($v.to_array().map(f64::exp)) };

    (@ln $v:ident, scalar) => { $v.ln() };
    (@ln $v:ident, simd) => { f64x4::from($v.to_array().map(f64::ln)) };

    (@sqrt $v:ident, scalar) => { $v.sqrt() };
    (@sqrt $v:ident, simd) => { $v.sqrt() };
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
