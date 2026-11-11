//! Position-independent `extern "C"` builtin trampoline tables for scalar,
//! complex, and multi-lane SIMD batch kernels.

#![expect(
    unsafe_code,
    reason = "C-ABI trampolines transmute typed function pointers into a uniform table and write out-pointer slots"
)]

use core::{
    mem::transmute,
    ops::{Div, Mul, Neg},
};

use num_anafis::{Complex, Numeric};

use crate::core::BuiltinFunction;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::FloatPrecision;
use super::{Fused, NativeLayout, PairedFunction};

/// Erased C-ABI function pointer entry stored in a position-independent trampoline table.
pub type RawTrampoline = unsafe extern "C" fn();

macro_rules! with_all_builtins {
    ($callback:ident $(, $($extra:tt)*)?) => {
        $callback!(
            $($($extra)*,)?
            unary: [
                Sin => sin, Cos => cos, Tan => tan, Cot => cot, Sec => sec, Csc => csc,
                Asin => asin, Acos => acos, Atan => atan, Acot => acot, Asec => asec, Acsc => acsc,
                Sinh => sinh, Cosh => cosh, Tanh => tanh, Coth => coth, Sech => sech, Csch => csch,
                Asinh => asinh, Acosh => acosh, Atanh => atanh, Acoth => acoth, Asech => asech,
                Acsch => acsch, Exp => exp, ExpM1 => exp_m1, Ln => ln, Ln1p => ln_1p,
                Log10 => log10, Log2 => log2, Sqrt => sqrt, Cbrt => cbrt, Floor => floor,
                Ceil => ceil, Round => round, Trunc => trunc, Fract => fract, Abs => abs,
                Signum => signum, Re => re, Im => im, Conj => conj, Heaviside => heaviside,
                Sinc => sinc, Erf => erf, Erfc => erfc, Gamma => gamma,
                Lgamma => lgamma, Digamma => digamma, Trigamma => trigamma,
                Tetragamma => tetragamma, Zeta => zeta, EllipticK => elliptic_k,
                EllipticE => elliptic_e,
            ],
            binary_swapped: [
                Polygamma => polygamma, BesselJ => bessel_j, BesselY => bessel_y,
                BesselI => bessel_i, BesselK => bessel_k, ZetaDeriv => zeta_deriv,
                Log => log, Hermite => hermite, LambertW => lambert_w,
            ],
            binary_direct: [
                Beta => beta, Atan2 => atan2, Hypot => hypot, Max => max, Min => min,
                Modulo => modulo, Rem => rem,
            ],
            ternary_swapped: [AssocLegendre => assoc_legendre,],
            ternary_direct: [Clamp => clamp,]
        );
    };
}

macro_rules! assign_slots {
    ($idx:expr;) => {};
    ($idx:expr; $head:ident, $($tail:ident,)*) => {
        let $head: usize = $idx;
        assign_slots!(($idx).saturating_add(1); $($tail,)*);
    };
}

macro_rules! define_builtin_lookup {
    (
        unary: [$($u_var:ident => $u_method:ident,)+],
        binary_swapped: [$($bs_var:ident => $bs_method:ident,)+],
        binary_direct: [$($bd_var:ident => $bd_method:ident,)+],
        ternary_swapped: [$($ts_var:ident => $ts_method:ident,)+],
        ternary_direct: [$($td_var:ident => $td_method:ident,)+]
    ) => {
        /// Map a [`BuiltinFunction`] to its deterministic slot index in a trampoline table.
        #[must_use]
        pub const fn builtin(function: BuiltinFunction) -> usize {
            const BUILTIN_SLOTS: usize = 0
                $(+ { let _ = BuiltinFunction::$u_var; 1 })+
                $(+ { let _ = BuiltinFunction::$bs_var; 1 })+
                $(+ { let _ = BuiltinFunction::$bd_var; 1 })+
                $(+ { let _ = BuiltinFunction::$ts_var; 1 })+
                $(+ { let _ = BuiltinFunction::$td_var; 1 })+;
            const _: () = assert!(
                BUILTIN_SLOTS == BuiltinFunction::COUNT,
                "trampoline table must list every BuiltinFunction exactly once"
            );
            assign_slots!(
                0_usize;
                $($u_method,)+
                $($bs_method,)+
                $($bd_method,)+
                $($ts_method,)+
                $($td_method,)+
            );
            match function {
                $(BuiltinFunction::$u_var => $u_method,)+
                $(BuiltinFunction::$bs_var => $bs_method,)+
                $(BuiltinFunction::$bd_var => $bd_method,)+
                $(BuiltinFunction::$ts_var => $ts_method,)+
                $(BuiltinFunction::$td_var => $td_method,)+
            }
        }
    };
}

macro_rules! real_table_impl {
    (
        $table:ident, $scalar:ty,
        unary: [$($u_var:ident => $u_method:ident,)+],
        binary_swapped: [$($bs_var:ident => $bs_method:ident,)+],
        binary_direct: [$($bd_var:ident => $bd_method:ident,)+],
        ternary_swapped: [$($ts_var:ident => $ts_method:ident,)+],
        ternary_direct: [$($td_var:ident => $td_method:ident,)+]
    ) => {
        const $table: [RawTrampoline; Trampolines::COUNT] = {
            $(unsafe extern "C" fn $u_method(x: $scalar) -> $scalar { Numeric::$u_method(&x) })+
            $(unsafe extern "C" fn $bs_method(a: $scalar, b: $scalar) -> $scalar {
                Numeric::$bs_method(&b, &a)
            })+
            $(unsafe extern "C" fn $bd_method(a: $scalar, b: $scalar) -> $scalar {
                Numeric::$bd_method(&a, &b)
            })+
            $(unsafe extern "C" fn $ts_method(d: $scalar, o: $scalar, x: $scalar) -> $scalar {
                Numeric::$ts_method(&x, &d, &o)
            })+
            $(unsafe extern "C" fn $td_method(a: $scalar, b: $scalar, c: $scalar) -> $scalar {
                Numeric::$td_method(&a, &b, &c)
            })+
            unsafe extern "C" fn pow_fn(a: $scalar, b: $scalar) -> $scalar { Numeric::pow(&a, &b) }
            unsafe extern "C" fn sincos_fn(slot: *mut $scalar) -> $scalar {
                // SAFETY: caller initializes `*slot` with the input argument.
                let x = unsafe { *slot };
                let (s, c) = Numeric::sin_cos(&x);
                // SAFETY: `slot` is a valid writable workspace register slot.
                unsafe { *slot = c; }
                s
            }
            unsafe extern "C" fn sinhcosh_fn(slot: *mut $scalar) -> $scalar {
                // SAFETY: caller initializes `*slot` with the input argument.
                let x = unsafe { *slot };
                let (s, c) = Numeric::sinh_cosh(&x);
                // SAFETY: `slot` is a valid writable workspace register slot.
                unsafe { *slot = c; }
                s
            }
            const unsafe extern "C" fn noop() {}
            const fn u(f: unsafe extern "C" fn($scalar) -> $scalar) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            const fn b(f: unsafe extern "C" fn($scalar, $scalar) -> $scalar) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            const fn t(f: unsafe extern "C" fn($scalar, $scalar, $scalar) -> $scalar) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            const fn p(f: unsafe extern "C" fn(*mut $scalar) -> $scalar) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            [
                $(u($u_method),)+ $(b($bs_method),)+ $(b($bd_method),)+
                $(t($ts_method),)+ $(t($td_method),)+
                b(pow_fn), p(sincos_fn), p(sinhcosh_fn),
                noop, noop, noop, noop, noop, noop, noop,
            ]
        };
    };
}

macro_rules! slot_table_impl {
    (
        $table:ident, $scalar:ty, $lanes:expr, $include_complex:tt,
        unary: [$($u_var:ident => $u_method:ident,)+],
        binary_swapped: [$($bs_var:ident => $bs_method:ident,)+],
        binary_direct: [$($bd_var:ident => $bd_method:ident,)+],
        ternary_swapped: [$($ts_var:ident => $ts_method:ident,)+],
        ternary_direct: [$($td_var:ident => $td_method:ident,)+]
    ) => {
        const $table: [RawTrampoline; Trampolines::COUNT] = {
            $(unsafe extern "C" fn $u_method(out: *mut $scalar, a: *const $scalar) {
                // SAFETY: caller passes valid workspace lanes; reading before writing is alias-safe.
                unsafe {
                    for i in 0..$lanes {
                        let x = *a.add(i);
                        *out.add(i) = Numeric::$u_method(&x);
                    }
                }
            })+
            $(unsafe extern "C" fn $bs_method(out: *mut $scalar, a: *const $scalar, b: *const $scalar) {
                // SAFETY: caller passes valid workspace lanes; reading before writing is alias-safe.
                unsafe {
                    for i in 0..$lanes {
                        let (lhs, rhs) = (*a.add(i), *b.add(i));
                        *out.add(i) = Numeric::$bs_method(&rhs, &lhs);
                    }
                }
            })+
            $(unsafe extern "C" fn $bd_method(out: *mut $scalar, a: *const $scalar, b: *const $scalar) {
                // SAFETY: caller passes valid workspace lanes; reading before writing is alias-safe.
                unsafe {
                    for i in 0..$lanes {
                        let (lhs, rhs) = (*a.add(i), *b.add(i));
                        *out.add(i) = Numeric::$bd_method(&lhs, &rhs);
                    }
                }
            })+
            $(unsafe extern "C" fn $ts_method(
                out: *mut $scalar, d: *const $scalar, o: *const $scalar, x: *const $scalar,
            ) {
                // SAFETY: caller passes valid workspace lanes; reading before writing is alias-safe.
                unsafe {
                    for i in 0..$lanes {
                        let (deg, ord, val) = (*d.add(i), *o.add(i), *x.add(i));
                        *out.add(i) = Numeric::$ts_method(&val, &deg, &ord);
                    }
                }
            })+
            $(unsafe extern "C" fn $td_method(
                out: *mut $scalar, a: *const $scalar, b: *const $scalar, c: *const $scalar,
            ) {
                // SAFETY: caller passes valid workspace lanes; reading before writing is alias-safe.
                unsafe {
                    for i in 0..$lanes {
                        let (first, second, third) = (*a.add(i), *b.add(i), *c.add(i));
                        *out.add(i) = Numeric::$td_method(&first, &second, &third);
                    }
                }
            })+
            unsafe extern "C" fn pow_fn(out: *mut $scalar, a: *const $scalar, b: *const $scalar) {
                // SAFETY: caller passes valid workspace lanes; reading before writing is alias-safe.
                unsafe {
                    for i in 0..$lanes {
                        let (lhs, rhs) = (*a.add(i), *b.add(i));
                        *out.add(i) = Numeric::pow(&lhs, &rhs);
                    }
                }
            }
            unsafe extern "C" fn sincos_fn(out1: *mut $scalar, out2: *mut $scalar, a: *const $scalar) {
                // SAFETY: caller passes valid workspace lanes (`out2 != a` and `out2 != out1`).
                unsafe {
                    for i in 0..$lanes {
                        let (s, c) = Numeric::sin_cos(&*a.add(i));
                        *out1.add(i) = s;
                        *out2.add(i) = c;
                    }
                }
            }
            unsafe extern "C" fn sinhcosh_fn(out1: *mut $scalar, out2: *mut $scalar, a: *const $scalar) {
                // SAFETY: caller passes valid workspace lanes (`out2 != a` and `out2 != out1`).
                unsafe {
                    for i in 0..$lanes {
                        let (s, c) = Numeric::sinh_cosh(&*a.add(i));
                        *out1.add(i) = s;
                        *out2.add(i) = c;
                    }
                }
            }
            const fn u(f: unsafe extern "C" fn(*mut $scalar, *const $scalar)) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            const fn b(
                f: unsafe extern "C" fn(*mut $scalar, *const $scalar, *const $scalar),
            ) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            const fn p(
                f: unsafe extern "C" fn(*mut $scalar, *mut $scalar, *const $scalar),
            ) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            const fn t(
                f: unsafe extern "C" fn(*mut $scalar, *const $scalar, *const $scalar, *const $scalar),
            ) -> RawTrampoline {
                // SAFETY: all `extern "C"` function pointers share one representation.
                unsafe { transmute(f) }
            }
            slot_table_tail!(
                $include_complex, $scalar, b, t,
                [$(u($u_method),)+ $(b($bs_method),)+ $(b($bd_method),)+
                 $(t($ts_method),)+ $(t($td_method),)+
                 b(pow_fn), p(sincos_fn), p(sinhcosh_fn)]
            )
        };
    };
}

macro_rules! slot_table_tail {
    (false, $scalar:ty, $b:ident, $t:ident, [$($head:expr),+]) => {{
        const unsafe extern "C" fn noop() {}
        [$($head,)+ noop, noop, noop, noop, noop, noop, noop]
    }};
    (true, $scalar:ty, $b:ident, $t:ident, [$($head:expr),+]) => {{
        unsafe extern "C" fn mul_fn(out: *mut $scalar, a: *const $scalar, b: *const $scalar) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Mul::mul(*a, *b); }
        }
        unsafe extern "C" fn div_fn(out: *mut $scalar, a: *const $scalar, b: *const $scalar) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Div::div(*a, *b); }
        }
        unsafe extern "C" fn neg_mul_fn(out: *mut $scalar, a: *const $scalar, b: *const $scalar) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Neg::neg(Mul::mul(*a, *b)); }
        }
        unsafe extern "C" fn fused_add_fn(
            out: *mut $scalar, a: *const $scalar, b: *const $scalar, c: *const $scalar,
        ) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Numeric::mul_add(&*a, &*b, &*c); }
        }
        unsafe extern "C" fn fused_sub_fn(
            out: *mut $scalar, a: *const $scalar, b: *const $scalar, c: *const $scalar,
        ) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Numeric::mul_add(&*a, &*b, &Neg::neg(*c)); }
        }
        unsafe extern "C" fn neg_fused_add_fn(
            out: *mut $scalar, a: *const $scalar, b: *const $scalar, c: *const $scalar,
        ) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Numeric::mul_add(&Neg::neg(*a), &*b, &*c); }
        }
        unsafe extern "C" fn neg_fused_sub_fn(
            out: *mut $scalar, a: *const $scalar, b: *const $scalar, c: *const $scalar,
        ) {
            // SAFETY: caller passes valid aligned workspace slots.
            unsafe { *out = Numeric::mul_add(&Neg::neg(*a), &*b, &Neg::neg(*c)); }
        }
        [
            $($head,)+
            $b(mul_fn), $b(div_fn), $b(neg_mul_fn),
            $t(fused_add_fn), $t(fused_sub_fn), $t(neg_fused_add_fn), $t(neg_fused_sub_fn),
        ]
    }};
}

/// Namespace for position-independent native trampoline indices and static tables.
pub struct Trampolines;

impl Trampolines {
    pub const POWER: usize = BuiltinFunction::COUNT;
    pub const SINCOS: usize = BuiltinFunction::COUNT + 1;
    pub const SINHCOSH: usize = BuiltinFunction::COUNT + 2;
    pub const COMPLEX_MUL: usize = BuiltinFunction::COUNT + 3;
    pub const COMPLEX_DIV: usize = BuiltinFunction::COUNT + 4;
    pub const COMPLEX_NEG_MUL: usize = BuiltinFunction::COUNT + 5;
    pub const COMPLEX_FUSED_ADD: usize = BuiltinFunction::COUNT + 6;
    pub const COMPLEX_FUSED_SUB: usize = BuiltinFunction::COUNT + 7;
    pub const COMPLEX_NEG_FUSED_ADD: usize = BuiltinFunction::COUNT + 8;
    pub const COMPLEX_NEG_FUSED_SUB: usize = BuiltinFunction::COUNT + 9;
    pub const COUNT: usize = BuiltinFunction::COUNT + 10;

    #[must_use]
    pub const fn table(layout: NativeLayout) -> &'static [RawTrampoline; Self::COUNT] {
        match layout {
            NativeLayout::F64 => &TABLE_F64,
            NativeLayout::F32 => &TABLE_F32,
            NativeLayout::ComplexF64 => &TABLE_C64,
            NativeLayout::ComplexF32 => &TABLE_C32,
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub const fn vector_table(
        precision: FloatPrecision,
        lanes: usize,
    ) -> Option<&'static [RawTrampoline; Self::COUNT]> {
        match (precision, lanes) {
            (FloatPrecision::F64, 4) => Some(&TABLE_F64X4),
            (FloatPrecision::F64, 8) => Some(&TABLE_F64X8),
            (FloatPrecision::F32, 8) => Some(&TABLE_F32X8),
            (FloatPrecision::F32, 16) => Some(&TABLE_F32X16),
            (FloatPrecision::F64 | FloatPrecision::F32, _) => None,
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[must_use]
    pub const fn vector_table(
        precision: FloatPrecision,
        lanes: usize,
    ) -> Option<&'static [RawTrampoline; Self::COUNT]> {
        match (precision, lanes) {
            (FloatPrecision::F64, 2) => Some(&TABLE_F64X2),
            (FloatPrecision::F32, 4) => Some(&TABLE_F32X4),
            (FloatPrecision::F64 | FloatPrecision::F32, _) => None,
        }
    }

    with_all_builtins!(define_builtin_lookup);

    #[must_use]
    pub const fn paired(paired: PairedFunction) -> usize {
        match paired {
            PairedFunction::SinCos => Self::SINCOS,
            PairedFunction::SinhCosh => Self::SINHCOSH,
        }
    }

    #[must_use]
    pub const fn complex_fused(fused: Fused) -> usize {
        match (fused.negate_product, fused.negate_addend) {
            (false, false) => Self::COMPLEX_FUSED_ADD,
            (false, true) => Self::COMPLEX_FUSED_SUB,
            (true, false) => Self::COMPLEX_NEG_FUSED_ADD,
            (true, true) => Self::COMPLEX_NEG_FUSED_SUB,
        }
    }
}

with_all_builtins!(real_table_impl, TABLE_F64, f64);
with_all_builtins!(real_table_impl, TABLE_F32, f32);
with_all_builtins!(slot_table_impl, TABLE_C64, Complex<f64>, 1, true);
with_all_builtins!(slot_table_impl, TABLE_C32, Complex<f32>, 1, true);
#[cfg(target_arch = "aarch64")]
with_all_builtins!(slot_table_impl, TABLE_F64X2, f64, 2, false);
#[cfg(target_arch = "aarch64")]
with_all_builtins!(slot_table_impl, TABLE_F32X4, f32, 4, false);
#[cfg(target_arch = "x86_64")]
with_all_builtins!(slot_table_impl, TABLE_F64X4, f64, 4, false);
#[cfg(target_arch = "x86_64")]
with_all_builtins!(slot_table_impl, TABLE_F64X8, f64, 8, false);
#[cfg(target_arch = "x86_64")]
with_all_builtins!(slot_table_impl, TABLE_F32X8, f32, 8, false);
#[cfg(target_arch = "x86_64")]
with_all_builtins!(slot_table_impl, TABLE_F32X16, f32, 16, false);
