//! Scalar and blocked VM instruction interpreter.

#![expect(
    unsafe_code,
    reason = "validated programs prove every register index and pooled operand span before execution"
)]

use core::{
    array::from_fn,
    mem::ManuallyDrop,
    ops::{Add, Div, Mul, Neg, Sub},
    ptr::read,
};
#[cfg(all(
    feature = "std",
    not(miri),
    target_arch = "aarch64",
    not(target_feature = "sve")
))]
use std::arch::is_aarch64_feature_detected;
#[cfg(all(
    feature = "std",
    not(miri),
    target_arch = "loongarch64",
    not(target_feature = "lasx")
))]
use std::arch::is_loongarch_feature_detected;
#[cfg(all(
    feature = "std",
    not(miri),
    target_arch = "s390x",
    not(target_feature = "vector-enhancements-1")
))]
use std::arch::is_s390x_feature_detected;
#[cfg(all(
    feature = "std",
    not(miri),
    any(target_arch = "x86", target_arch = "x86_64"),
    not(all(target_feature = "avx512f", target_feature = "fma"))
))]
use std::arch::is_x86_feature_detected;

use crate::core::{BuiltinFunction, Kernel};

use super::{
    Combine, EvaluationValue, Fold, InlineBinary, InlineUnary, Instruction, Operation,
    PairedFunction, ProgramData,
};

/// Generate target-feature-specialized hot execution loops and cold handlers.
macro_rules! specialized_tiers {
    ($(
        #[cfg($pred:meta)]
        ($execute:ident, $operate:ident, $feature:literal, $runtime_fma:tt);
    )*) => {
        $(
            #[cfg($pred)]
            #[target_feature(enable = $feature)]
            unsafe fn $execute<const LANES: usize>(&self, registers: &mut [[Value; LANES]]) {
                specialized_tiers!(@dispatch self, registers, LANES, Self::$operate, $runtime_fma);
            }

            #[cfg($pred)]
            #[inline(never)]
            #[target_feature(enable = $feature)]
            unsafe fn $operate<const LANES: usize>(
                instruction: &Instruction,
                registers: &mut [[Value; LANES]],
            ) {
                // SAFETY: caller verified `$feature` and upholds `operate`'s contract.
                unsafe {
                    Self::operate_inner(instruction, registers);
                }
            }
        )*
    };
    (@dispatch $self:ident, $registers:ident, $lanes:ident, $op:path, true) => {
        if Value::FUSE || Value::RUNTIME_FMA {
            $self.execute_inner::<$lanes, true>($registers, $op);
        } else {
            $self.execute_inner::<$lanes, false>($registers, $op);
        }
    };
    (@dispatch $self:ident, $registers:ident, $lanes:ident, $op:path, false) => {
        if Value::FUSE {
            $self.execute_inner::<$lanes, true>($registers, $op);
        } else {
            $self.execute_inner::<$lanes, false>($registers, $op);
        }
    };
}

macro_rules! lanes {
    ($registers:ident, [$first:ident $(, $input:ident)*] => owned $operator:path) => {{
        let $first = $registers.get_unchecked(usize::try_from(*$first).unwrap_or(0));
        $(let $input = $registers.get_unchecked(usize::try_from(*$input).unwrap_or(0));)*
        from_fn(|lane| $operator($first.get_unchecked(lane).clone() $(, $input.get_unchecked(lane))*))
    }};
    ($registers:ident, [$($input:ident),+] => $kernel:expr) => {{
        $(let $input = $registers.get_unchecked(usize::try_from(*$input).unwrap_or(0));)+
        from_fn(|lane| $kernel($($input.get_unchecked(lane)),+))
    }};
}

macro_rules! fold {
    ($registers:ident, $operands:expr, $span:ident, $operation:path) => {{
        let start = usize::try_from($span.start).unwrap_or(0);
        let end = usize::try_from($span.end).unwrap_or(0);
        let operands = $operands.get_unchecked(start..end);
        let first_reg = usize::try_from(*operands.get_unchecked(0)).unwrap_or(0);
        let mut folded = ManuallyDrop::new($registers.get_unchecked(first_reg).clone());
        for &source in operands.get_unchecked(1..) {
            let values = $registers.get_unchecked(usize::try_from(source).unwrap_or(0));
            folded = ManuallyDrop::new(from_fn(|lane| {
                $operation(read(folded.get_unchecked(lane)), values.get_unchecked(lane))
            }));
        }
        ManuallyDrop::into_inner(folded)
    }};
}

macro_rules! dot {
    ($registers:ident, $operands:expr, $span:ident, $fused:ident) => {{
        let start = usize::try_from($span.start).unwrap_or(0);
        let end = usize::try_from($span.end).unwrap_or(0);
        let (pairs, _) = $operands.get_unchecked(start..end).as_chunks::<2>();
        let [left, right] = pairs.get_unchecked(0);
        let mut total = ManuallyDrop::new(lanes!($registers, [left, right] => owned Mul::mul));
        if $fused {
            for [left, right] in pairs.get_unchecked(1..) {
                let left = $registers.get_unchecked(usize::try_from(*left).unwrap_or(0));
                let right = $registers.get_unchecked(usize::try_from(*right).unwrap_or(0));
                total = ManuallyDrop::new(from_fn(|lane| {
                    let acc = read(total.get_unchecked(lane));
                    Value::mul_add(left.get_unchecked(lane), right.get_unchecked(lane), &acc)
                }));
            }
        } else {
            for [left, right] in pairs.get_unchecked(1..) {
                let left = $registers.get_unchecked(usize::try_from(*left).unwrap_or(0));
                let right = $registers.get_unchecked(usize::try_from(*right).unwrap_or(0));
                total = ManuallyDrop::new(from_fn(|lane| {
                    Add::add(
                        read(total.get_unchecked(lane)),
                        Mul::mul(left.get_unchecked(lane).clone(), right.get_unchecked(lane)),
                    )
                }));
            }
        }
        ManuallyDrop::into_inner(total)
    }};
}

impl<Value: EvaluationValue> ProgramData<Value> {
    /// Whether the active target executes fused multiply-add instructions inline for `Value`.
    #[cfg(all(
        feature = "std",
        not(miri),
        any(target_arch = "x86", target_arch = "x86_64"),
        not(target_feature = "fma")
    ))]
    #[must_use]
    pub fn fuse() -> bool {
        if Value::RUNTIME_FMA {
            return Value::FUSE || is_x86_feature_detected!("fma");
        }
        Value::FUSE
    }

    /// Whether the active target executes fused multiply-add instructions inline for `Value`.
    #[cfg(not(all(
        feature = "std",
        not(miri),
        any(target_arch = "x86", target_arch = "x86_64"),
        not(target_feature = "fma")
    )))]
    #[must_use]
    pub const fn fuse() -> bool {
        Value::FUSE
    }

    /// Execute every instruction on `LANES` independent rows, populating all
    /// compiled result slots in `registers`.
    pub fn execute<const LANES: usize>(&self, registers: &mut [[Value; LANES]]) {
        #[cfg(all(
            feature = "std",
            not(miri),
            any(target_arch = "x86", target_arch = "x86_64"),
            not(all(target_feature = "avx512f", target_feature = "fma"))
        ))]
        if Value::BATCH_BLOCKED
            && LANES > 1
            && is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("fma")
        {
            // SAFETY: runtime detection verified AVX-512F and FMA support.
            return unsafe { self.execute_wide8_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            any(target_arch = "x86", target_arch = "x86_64"),
            not(all(target_feature = "avx2", target_feature = "fma"))
        ))]
        if Value::BATCH_BLOCKED
            && LANES > 1
            && is_x86_feature_detected!("avx2")
            && is_x86_feature_detected!("fma")
        {
            // SAFETY: runtime detection verified AVX2 and FMA support.
            return unsafe { self.execute_wide_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            any(target_arch = "x86", target_arch = "x86_64"),
            not(target_feature = "avx2")
        ))]
        if Value::BATCH_BLOCKED && LANES > 1 && is_x86_feature_detected!("avx2") {
            // SAFETY: runtime detection verified AVX2 support.
            return unsafe { self.execute_wide(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            any(target_arch = "x86", target_arch = "x86_64"),
            not(target_feature = "fma")
        ))]
        if Value::RUNTIME_FMA && is_x86_feature_detected!("fma") {
            // SAFETY: runtime detection verified FMA support.
            return unsafe { self.execute_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            target_arch = "aarch64",
            not(target_feature = "sve")
        ))]
        if Value::BATCH_BLOCKED && LANES > 1 && is_aarch64_feature_detected!("sve") {
            // SAFETY: runtime detection verified SVE support.
            return unsafe { self.execute_wide_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            target_arch = "loongarch64",
            not(target_feature = "lasx")
        ))]
        if Value::BATCH_BLOCKED && LANES > 1 && is_loongarch_feature_detected!("lasx") {
            // SAFETY: runtime detection verified LASX support.
            return unsafe { self.execute_wide_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            target_arch = "loongarch64",
            not(target_feature = "lsx")
        ))]
        if Value::BATCH_BLOCKED && LANES > 1 && is_loongarch_feature_detected!("lsx") {
            // SAFETY: runtime detection verified LSX support.
            return unsafe { self.execute_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            target_arch = "s390x",
            not(target_feature = "vector-enhancements-1")
        ))]
        if Value::BATCH_BLOCKED
            && LANES > 1
            && is_s390x_feature_detected!("vector-enhancements-1")
            && is_s390x_feature_detected!("vector")
        {
            // SAFETY: runtime detection verified vector-enhancements-1 support.
            return unsafe { self.execute_wide_fused(registers) };
        }
        #[cfg(all(
            feature = "std",
            not(miri),
            target_arch = "s390x",
            not(target_feature = "vector")
        ))]
        if Value::BATCH_BLOCKED && LANES > 1 && is_s390x_feature_detected!("vector") {
            // SAFETY: runtime detection verified vector support.
            return unsafe { self.execute_fused(registers) };
        }
        if Value::FUSE {
            self.execute_inner::<LANES, true>(registers, Self::operate);
        } else {
            self.execute_inner::<LANES, false>(registers, Self::operate);
        }
    }

    specialized_tiers! {
        #[cfg(all(feature = "std", not(miri), any(target_arch = "x86", target_arch = "x86_64"), not(all(target_feature = "avx512f", target_feature = "fma"))))]
        (execute_wide8_fused, operate_wide8_fused, "avx512f,fma", true);
        #[cfg(all(feature = "std", not(miri), any(target_arch = "x86", target_arch = "x86_64"), not(all(target_feature = "avx2", target_feature = "fma"))))]
        (execute_wide_fused, operate_wide_fused, "avx2,fma", true);
        #[cfg(all(feature = "std", not(miri), any(target_arch = "x86", target_arch = "x86_64"), not(target_feature = "avx2")))]
        (execute_wide, operate_wide, "avx2", false);
        #[cfg(all(feature = "std", not(miri), any(target_arch = "x86", target_arch = "x86_64"), not(target_feature = "fma")))]
        (execute_fused, operate_fused, "fma", true);
        #[cfg(all(feature = "std", not(miri), target_arch = "aarch64", not(target_feature = "sve")))]
        (execute_wide_fused, operate_wide_fused, "sve", true);
        #[cfg(all(feature = "std", not(miri), target_arch = "loongarch64", not(target_feature = "lasx")))]
        (execute_wide_fused, operate_wide_fused, "lasx", true);
        #[cfg(all(feature = "std", not(miri), target_arch = "loongarch64", not(target_feature = "lsx")))]
        (execute_fused, operate_fused, "lsx", true);
        #[cfg(all(feature = "std", not(miri), target_arch = "s390x", not(target_feature = "vector-enhancements-1")))]
        (execute_wide_fused, operate_wide_fused, "vector,vector-enhancements-1", true);
        #[cfg(all(feature = "std", not(miri), target_arch = "s390x", not(target_feature = "vector")))]
        (execute_fused, operate_fused, "vector", true);
    }

    #[inline]
    fn execute_inner<const LANES: usize, const FUSED: bool>(
        &self,
        registers: &mut [[Value; LANES]],
        cold: unsafe fn(&Instruction, &mut [[Value; LANES]]),
    ) {
        // SAFETY: `validate` proved every pooled span and register index in bounds.
        unsafe {
            for instruction in &self.instructions {
                let output = usize::try_from(instruction.output).unwrap_or(0);
                let operation = &instruction.operation;
                if let Operation::Combine(kind, [left, right]) = operation {
                    match kind {
                        Combine::Multiply => {
                            *registers.get_unchecked_mut(output) =
                                lanes!(registers, [left, right] => owned Mul::mul);
                        }
                        Combine::Add => {
                            *registers.get_unchecked_mut(output) =
                                lanes!(registers, [left, right] => owned Add::add);
                        }
                        Combine::Subtract => {
                            *registers.get_unchecked_mut(output) =
                                lanes!(registers, [left, right] => owned Sub::sub);
                        }
                    }
                } else if let Operation::Fold(kind, span) = operation {
                    match kind {
                        Fold::Sum => {
                            *registers.get_unchecked_mut(output) =
                                fold!(registers, self.operands, span, Add::add);
                        }
                        Fold::Product => {
                            *registers.get_unchecked_mut(output) =
                                fold!(registers, self.operands, span, Mul::mul);
                        }
                        Fold::Dot => {
                            *registers.get_unchecked_mut(output) =
                                dot!(registers, self.operands, span, FUSED);
                        }
                    }
                } else if FUSED && let Operation::Fused(fused, [left, right, addend]) = operation {
                    if fused.negate_product {
                        if fused.negate_addend {
                            *registers.get_unchecked_mut(output) = lanes!(registers, [left, right, addend] => |x: &Value, y, z: &Value| {
                                Value::mul_add(&x.clone().neg(), y, &z.clone().neg())
                            });
                        } else {
                            *registers.get_unchecked_mut(output) = lanes!(registers, [left, right, addend] => |x: &Value, y, z| {
                                Value::mul_add(&x.clone().neg(), y, z)
                            });
                        }
                    } else if fused.negate_addend {
                        *registers.get_unchecked_mut(output) = lanes!(registers, [left, right, addend] => |x, y, z: &Value| {
                            Value::mul_add(x, y, &z.clone().neg())
                        });
                    } else {
                        *registers.get_unchecked_mut(output) =
                            lanes!(registers, [left, right, addend] => Value::mul_add);
                    }
                } else {
                    cold(instruction, registers);
                }
            }
        }
    }

    #[inline(never)]
    unsafe fn operate<const LANES: usize>(
        instruction: &Instruction,
        registers: &mut [[Value; LANES]],
    ) {
        // SAFETY: by the caller's contract.
        unsafe {
            Self::operate_inner(instruction, registers);
        }
    }

    #[inline]
    unsafe fn operate_inner<const LANES: usize>(
        instruction: &Instruction,
        registers: &mut [[Value; LANES]],
    ) {
        // SAFETY: `validate` proved every register index below `register_count`,
        // matching builtin arities, and disjoint paired output slots.
        unsafe {
            let output = usize::try_from(instruction.output).unwrap_or(0);
            *registers.get_unchecked_mut(output) = match &instruction.operation {
                Operation::Combine(..) | Operation::Fold(..) => return,
                Operation::Fused(fused, [left, right, addend]) => {
                    lanes!(registers, [left, right, addend] => |x: &Value, y: &Value, z: &Value| {
                        let prod = if fused.negate_product { x.clone().neg() } else { x.clone() };
                        if fused.negate_addend {
                            Value::mul_add(&prod, y, &z.clone().neg())
                        } else {
                            Value::mul_add(&prod, y, z)
                        }
                    })
                }
                Operation::Negate([input]) => lanes!(registers, [input] => owned Neg::neg),
                Operation::NegatedMultiply([left, right]) => {
                    lanes!(registers, [left, right] => owned Mul::mul).map(Neg::neg)
                }
                Operation::Divide([left, right]) => {
                    lanes!(registers, [left, right] => owned Div::div)
                }
                Operation::Power([base, exponent]) => {
                    lanes!(registers, [base, exponent] => Value::pow)
                }
                Operation::Unary(builtin, [input]) => match InlineUnary::of(*builtin) {
                    Some(InlineUnary::Sqrt) => lanes!(registers, [input] => Value::sqrt),
                    Some(InlineUnary::Abs) => lanes!(registers, [input] => Value::abs),
                    Some(InlineUnary::Floor) => lanes!(registers, [input] => Value::floor),
                    Some(InlineUnary::Ceil) => lanes!(registers, [input] => Value::ceil),
                    Some(InlineUnary::Trunc) => lanes!(registers, [input] => Value::trunc),
                    Some(InlineUnary::Round) => lanes!(registers, [input] => Value::round),
                    None if *builtin == BuiltinFunction::Re => {
                        lanes!(registers, [input] => Value::re)
                    }
                    None if *builtin == BuiltinFunction::Im => {
                        lanes!(registers, [input] => Value::im)
                    }
                    None if *builtin == BuiltinFunction::Conj => {
                        lanes!(registers, [input] => Value::conj)
                    }
                    None if *builtin == BuiltinFunction::Heaviside => {
                        lanes!(registers, [input] => Value::heaviside)
                    }
                    None if *builtin == BuiltinFunction::Signum => {
                        lanes!(registers, [input] => Value::signum)
                    }
                    None => {
                        let Kernel::Unary(kernel) = Kernel::<Value>::of(*builtin) else {
                            return;
                        };
                        lanes!(registers, [input] => kernel)
                    }
                },
                Operation::Pair(kind, [input], second) => {
                    let in_idx = usize::try_from(*input).unwrap_or(0);
                    let sec_idx = usize::try_from(*second).unwrap_or(0);
                    let [arguments, cosines] =
                        registers.get_disjoint_unchecked_mut([in_idx, sec_idx]);
                    match kind {
                        PairedFunction::SinCos => from_fn(|lane| {
                            let (sine, cosine) = Value::sin_cos(arguments.get_unchecked(lane));
                            *cosines.get_unchecked_mut(lane) = cosine;
                            sine
                        }),
                        PairedFunction::SinhCosh => from_fn(|lane| {
                            let (sinh, cosh) = Value::sinh_cosh(arguments.get_unchecked(lane));
                            *cosines.get_unchecked_mut(lane) = cosh;
                            sinh
                        }),
                    }
                }
                Operation::Binary(builtin, [left, right]) => match InlineBinary::of(*builtin) {
                    Some(InlineBinary::Max) => lanes!(registers, [left, right] => Value::max),
                    Some(InlineBinary::Min) => lanes!(registers, [left, right] => Value::min),
                    None => {
                        let Kernel::Binary(kernel) = Kernel::<Value>::of(*builtin) else {
                            return;
                        };
                        lanes!(registers, [left, right] => kernel)
                    }
                },
                Operation::Ternary(builtin, [first, second, third]) => {
                    if *builtin == BuiltinFunction::Clamp {
                        lanes!(registers, [first, second, third] => Value::clamp)
                    } else {
                        let Kernel::Ternary(kernel) = Kernel::<Value>::of(*builtin) else {
                            return;
                        };
                        lanes!(registers, [first, second, third] => kernel)
                    }
                }
            };
        }
    }
}
