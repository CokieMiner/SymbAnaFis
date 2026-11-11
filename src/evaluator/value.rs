//! Value policy for direct and compiled numerical evaluation.

use num_anafis::{Complex, Number, Numeric, Real};

/// A value type the evaluator computes with.
///
/// Arithmetic, literal conversion, and functions come from [`Numeric`]; this
/// trait adds the float and batching policy of compiled programs. Every
/// `num_anafis` value type implements it. Another commutative scalar implements
/// `Numeric` and opts in with `impl EvaluationValue for MyValue {}`.
/// Noncommutative algebras do not fit, since products are commutative.
pub trait EvaluationValue: Numeric {
    /// Whether every value of this type is strictly real (`im(x) == 0`,
    /// `re(x) == conj(x) == x`, and `clamp(x, lo, hi) == min(max(x, lo), hi)`).
    const IS_REAL: bool = false;
    /// Whether algebraic reassociation is permitted for native floating execution.
    const OPTIMIZE_FLOATS: bool = false;
    /// Whether sequential and parallel batch evaluation in the portable VM
    /// packs independent rows into fixed-width blocks for SIMD vectorization
    /// and dispatch amortization. Blocking performs no algebraic reassociation.
    const BATCH_BLOCKED: bool = Self::OPTIMIZE_FLOATS;
    /// Whether compilation may emit fused operations for this build.
    const FUSE: bool = false;
    /// Whether runtime `x86`/`x86_64` FMA detection may enable fused operations
    /// when [`Self::FUSE`] is statically `false`.
    const RUNTIME_FMA: bool = false;

    /// Recognize the native negative unit for sign lowering.
    #[must_use]
    fn is_negative_one(_value: &Self) -> bool {
        false
    }

    /// Admit a finite nonzero constant reciprocal for native strength reduction.
    /// Other values retain division and return `None`.
    #[must_use]
    fn constant_reciprocal(_value: &Self) -> Option<Self> {
        None
    }

    /// Return a bit-exact deduplication key for native floating-point constants.
    /// Types returning `None` keep separate constant slots per distinct expression.
    #[must_use]
    fn constant_bits(_value: &Self) -> Option<u64> {
        None
    }
}

impl EvaluationValue for Number {}

impl EvaluationValue for Real {
    const IS_REAL: bool = true;
}

impl EvaluationValue for Complex {}

impl EvaluationValue for Complex<f64> {
    const BATCH_BLOCKED: bool = true;
}

impl EvaluationValue for Complex<f32> {
    const BATCH_BLOCKED: bool = true;
}

/// Native binary formats permit reassociation, sign lowering, and constant reciprocals.
macro_rules! native_float_policy {
    ($($float:ty => $to_u64:expr),* $(,)?) => { $(
        impl EvaluationValue for $float {
            const IS_REAL: bool = true;
            const OPTIMIZE_FLOATS: bool = true;
            const BATCH_BLOCKED: bool = true;
            /// Fused operations are emitted unconditionally when `std` is
            /// enabled and the target compiles the FMA instruction inline;
            /// `no_std` falls back to software emulation. On `x86` and
            /// `x86_64`, runtime feature detection in the VM also enables FMA
            /// when `std` is active and the processor supports FMA.
            const FUSE: bool = cfg!(all(
                feature = "std",
                any(
                    target_feature = "fma",
                    target_arch = "aarch64",
                    target_arch = "arm64ec",
                    target_arch = "powerpc",
                    target_arch = "powerpc64",
                    target_arch = "s390x",
                    all(target_arch = "arm", target_feature = "vfp4"),
                    all(target_arch = "loongarch64", target_feature = "d"),
                    all(
                        any(target_arch = "riscv32", target_arch = "riscv64"),
                        target_feature = "d",
                    ),
                )
            ));

            const RUNTIME_FMA: bool = cfg!(feature = "std");

            fn is_negative_one(value: &Self) -> bool {
                let negative_one: $float = -1.0;
                value.to_bits() == negative_one.to_bits()
            }

            fn constant_reciprocal(value: &Self) -> Option<Self> {
                if !value.is_finite() || *value == 0.0 {
                    return None;
                }
                let reciprocal = 1.0 / value;
                (reciprocal.is_finite() && reciprocal != 0.0).then_some(reciprocal)
            }

            fn constant_bits(value: &Self) -> Option<u64> {
                Some(($to_u64)(value.to_bits()))
            }
        }
    )* };
}

native_float_policy!(f32 => u64::from, f64 => |bits| bits);
