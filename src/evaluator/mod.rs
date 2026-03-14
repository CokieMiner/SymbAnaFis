//! Compiled expression evaluator for fast numerical evaluation.
//!
//! This module converts expression trees into flat bytecode that can be evaluated
//! efficiently without tree traversal. The evaluator is thread-safe for parallel
//! evaluation and uses SIMD vectorization for batch operations.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐    ┌────────────┐    ┌─────────────────────┐
//! │    Expr     │ -> │  Compiler  │ -> │  CompiledEvaluator  │
//! │ (AST Tree)  │    │ (Bytecode) │    │   (Stack Machine)   │
//! └─────────────┘    └────────────┘    └─────────────────────┘
//!                                              │
//!                          ┌───────────────────┼───────────────────┐
//!                          ▼                   ▼                   ▼
//!                    ┌──────────┐       ┌──────────┐       ┌──────────┐
//!                    │ evaluate │       │eval_batch│       │ parallel │
//!                    │ (scalar) │       │  (SIMD)  │       │  (Rayon) │
//!                    └──────────┘       └──────────┘       └──────────┘
//! ```
//!
//! # Safety Model
//!
//! This module uses unsafe code in performance-critical evaluation loops.
//! Safety is guaranteed by the [`Compiler`] and by register allocation that
//! ensures all indices are in-bounds for the allocated register file. See
//! [`execution`] and [`simd`] for the invariants upheld around unsafe access.
//!
//! # Example
//!
//! ```
//! use symb_anafis::parse;
//! use std::collections::HashSet;
//!
//! let expr = parse("sin(x) * cos(x) + x^2", &HashSet::new(), &HashSet::new(), None)
//!     .expect("Should parse");
//! let evaluator = expr.compile().expect("Should compile");
//!
//! // Evaluate at x = 0.5
//! let result = evaluator.evaluate(&[0.5]);
//! assert!((result - (0.5_f64.sin() * 0.5_f64.cos() + 0.25)).abs() < 1e-10);
//! ```
//!
//! # Modules
//!
//! - [`instruction`]: Bytecode instruction definitions
//! - [`compiler`]: Expression-to-bytecode compilation
//! - [`execution`]: Scalar evaluation implementation
//! - [`simd`]: SIMD batch evaluation implementation
//! - [`eval_math`]: Helper math functions for evaluator-only special cases

// Allow unsafe code in this module - safety is guaranteed by register allocation.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    reason = "Safety is guaranteed by register allocation and validated indices"
)]

// Internal submodules - visibility is controlled by parent module (not exported from crate root)
mod compiler;
mod eval_math;
mod execution;
mod instruction;
mod simd;

#[cfg(test)]
mod tests;

// Re-exports for sibling modules within evaluator
pub use compiler::Compiler;
pub use instruction::Instruction;

use crate::core::error::DiffError;
use crate::core::unified_context::Context;
use crate::{Expr, ExprKind, Symbol};
use rustc_hash::FxHashSet;
use std::sync::{Arc, OnceLock};
use wide::f64x4;

// =============================================================================
// ToParamName trait - allows compile methods to accept strings or symbols
// =============================================================================

/// Trait for types that can be used as parameter names in compile methods.
///
/// This allows `compile` to accept `&[&str]`, `&[&Symbol]`, or mixed types.
///
/// # Example
///
/// ```
/// use symb_anafis::{symb, parse, CompiledEvaluator};
/// use std::collections::HashSet;
///
/// let expr = parse("x + y", &HashSet::new(), &HashSet::new(), None).expect("Should parse");
/// let x = symb("x");
/// let y = symb("y");
///
/// // Using strings
/// let c1 = CompiledEvaluator::compile(&expr, &["x", "y"], None).expect("Should compile");
///
/// // Using symbols
/// let c2 = CompiledEvaluator::compile(&expr, &[&x, &y], None).expect("Should compile");
/// ```
pub trait ToParamName {
    /// Get the parameter as a symbol ID (for fast lookup) and name (for storage/error messages).
    fn to_param_id_and_name(&self) -> (u64, String);
}

// Blanket impl for anything that can convert to &str
impl<T: AsRef<str>> ToParamName for T {
    fn to_param_id_and_name(&self) -> (u64, String) {
        let s = self.as_ref();
        let sym = crate::symb(s);
        (sym.id(), s.to_owned())
    }
}

impl ToParamName for Symbol {
    fn to_param_id_and_name(&self) -> (u64, String) {
        (
            self.id(),
            self.name().unwrap_or_else(|| format!("${}", self.id())),
        )
    }
}

impl ToParamName for &Symbol {
    fn to_param_id_and_name(&self) -> (u64, String) {
        (
            self.id(),
            self.name().unwrap_or_else(|| format!("${}", self.id())),
        )
    }
}

// =============================================================================
// CompiledEvaluator - The main public interface
// =============================================================================

/// Compiled expression evaluator - thread-safe, reusable.
///
/// The evaluator holds immutable bytecode that can be shared across threads.
/// Each call to `evaluate` uses a thread-local or per-call stack.
///
/// # Thread Safety
///
/// `CompiledEvaluator` is `Send + Sync` because:
/// - All data is immutable after construction
/// - Each evaluation uses its own stack (no shared mutable state)
///
/// # Performance Characteristics
///
/// | Method | Use Case | Performance |
/// |--------|----------|-------------|
/// | `evaluate` | Single point | ~100ns for simple expressions |
/// | `eval_batch` | Multiple points | ~25ns/point with SIMD |
/// | `eval_batch_parallel` | Large datasets | Scales with cores |
#[derive(Clone)]
#[allow(
    clippy::partial_pub_fields,
    reason = "Public evaluator metadata is API surface; internal SIMD cache should remain private"
)]
pub struct CompiledEvaluator {
    /// Bytecode instructions (immutable after compilation)
    pub instructions: Box<[Instruction]>,
    /// Constant pool for numeric literals
    pub constants: Box<[f64]>,
    /// Argument pool for N-ary instructions (`AddN`, `MulN`)
    pub arg_pool: Box<[u32]>,
    /// Parameter names in order (for mapping `HashMap` → array)
    pub param_names: Box<[String]>,
    /// Required stack depth for evaluation
    pub register_count: usize,
    /// Number of parameters expected
    pub param_count: usize,
    /// Lazily cached SIMD-splatted constants (one f64x4 per scalar constant).
    simd_constants: OnceLock<Box<[f64x4]>>,
}

impl CompiledEvaluator {
    /// Compile an expression to bytecode.
    ///
    /// # Arguments
    ///
    /// * `expr` - The expression to compile
    /// * `param_order` - Parameters in evaluation order (accepts `&[&str]` or `&[&Symbol]`)
    /// * `context` - Optional context for custom function definitions
    ///
    /// # Example
    ///
    /// ```
    /// use symb_anafis::{symb, CompiledEvaluator};
    ///
    /// let x = symb("x");
    /// let y = symb("y");
    /// let expr = x.pow(2.0) + y;
    ///
    /// // Using strings
    /// let compiled = CompiledEvaluator::compile(&expr, &["x", "y"], None)
    ///     .expect("Should compile");
    ///
    /// // Using symbols
    /// let compiled = CompiledEvaluator::compile(&expr, &[&x, &y], None)
    ///     .expect("Should compile");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `DiffError` if:
    /// - `UnboundVariable`: Symbol not in parameter list and not a known constant
    /// - (Previously `StackOverflow`, now removed — the evaluator stack is heap-allocated)
    /// - `UnsupportedFunction`: Unknown function name
    /// - `UnsupportedExpression`: Unevaluated derivatives
    pub fn compile<P: ToParamName>(
        expr: &Expr,
        param_order: &[P],
        context: Option<&Context>,
    ) -> Result<Self, DiffError> {
        // Get symbol IDs and names for each parameter
        let params: Vec<(u64, String)> = param_order
            .iter()
            .map(ToParamName::to_param_id_and_name)
            .collect();
        let (param_ids, param_names): (Vec<u64>, Vec<String>) = params.into_iter().unzip();

        // Expand user function calls with their body expressions
        let expanded_expr = context.map_or_else(
            || expr.clone(),
            |ctx| {
                let mut expanding = std::collections::HashSet::new();
                Self::expand_user_functions(expr, ctx, &mut expanding, 0)
            },
        );

        let mut compiler = Compiler::new(&param_ids);

        // Single-pass compilation with CSE
        compiler.compile_expr(&expanded_expr)?;

        // Extract compilation results
        let (instructions, mut constants, mut arg_pool, _max_stack, param_count) =
            compiler.into_parts();

        // Post-compilation optimization pass: fuse instructions and compact constants
        let (optimized_instructions, max_stack) =
            Self::optimize_instructions(instructions, &mut constants, &mut arg_pool, param_count);

        Ok(Self {
            instructions: optimized_instructions.into_boxed_slice(),
            constants: constants.into_boxed_slice(),
            arg_pool: arg_pool.into_boxed_slice(),
            param_names: param_names.into_boxed_slice(),
            register_count: max_stack,
            param_count,
            simd_constants: OnceLock::new(),
        })
    }

    /// Compile an expression, automatically determining parameter order from variables.
    ///
    /// Variables are sorted alphabetically for consistent ordering.
    ///
    /// # Arguments
    ///
    /// * `expr` - The expression to compile
    /// * `context` - Optional context for custom function definitions
    ///
    /// # Example
    ///
    /// ```
    /// use symb_anafis::{symb, CompiledEvaluator};
    ///
    /// let x = symb("x");
    /// let expr = x.pow(2.0) + x.sin();
    ///
    /// // Auto-detect variables (will be sorted: ["x"])
    /// let compiled = CompiledEvaluator::compile_auto(&expr, None)
    ///     .expect("Should compile");
    /// let result = compiled.evaluate(&[2.0]);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `DiffError` if compilation fails.
    pub fn compile_auto(expr: &Expr, context: Option<&Context>) -> Result<Self, DiffError> {
        let vars = expr.variables_ordered();
        let mut param_order: Vec<String> = vars
            .into_iter()
            .filter(|v| {
                let id = crate::core::symbol::symb_interned(v.as_str()).id();
                !crate::core::known_symbols::is_known_constant_by_id(id)
            })
            .collect();

        // Sort alphabetically for consistent ordering (required by tests and documentation)
        param_order.sort();

        Self::compile(expr, &param_order, context)
    }

    /// Post-compilation optimization pass that fuses instruction patterns.
    ///
    /// Currently detects:
    /// - **FMA Fusions**: `[Mul, Add]` → `MulAdd`, `[Mul, Sub]` → `MulSub`, `[Mul, Sub(rev)]` → `NegMulAdd`
    /// - **Inverse Fusions**: `[Sqrt, Recip]` → `InvSqrt`, `[Square, Recip]` → `InvSquare`, `[Cube, Recip]` → `InvCube`
    /// - **Power Fusions**: `[Square, Mul]` → `Cube`, `[Pow4, Recip]` → `Square + InvSquare` (for $x^{-4}$)
    /// - **Exponential Fusions**: `[Neg, Exp]` → `ExpNeg`
    ///
    /// The pass also performs copy forwarding and constant pool compaction.
    //
    // Allow needless_pass_by_value: Takes ownership to match call site pattern where
    // caller builds Vec and passes it directly without needing it afterwards.
    #[allow(
        clippy::needless_pass_by_value,
        clippy::too_many_lines,
        reason = "Peephole optimizer with multiple patterns"
    )]
    fn optimize_instructions(
        instructions: Vec<Instruction>,
        constants: &mut Vec<f64>,
        arg_pool: &mut [u32],
        param_count: usize,
    ) -> (Vec<Instruction>, usize) {
        use instruction::FnOp;

        if instructions.is_empty() {
            let rc = param_count + constants.len();
            return (instructions, rc);
        }

        // Find max register index to use a Vec instead of HashMap
        let mut max_reg_idx = 0;
        for instr in &instructions {
            instr.for_each_reg(|r| max_reg_idx = max_reg_idx.max(r));
        }
        let old_const_count = constants.len();

        // Count uses of each register
        let mut use_count = vec![0_usize; (max_reg_idx + 1) as usize];
        for instr in &instructions {
            instr.for_each_read(|r| use_count[r as usize] += 1);
            let (start_idx, count) = match *instr {
                Instruction::AddN {
                    start_idx, count, ..
                }
                | Instruction::MulN {
                    start_idx, count, ..
                } => (start_idx, count),
                Instruction::Builtin3 { start_idx, .. } => (start_idx, 3),
                Instruction::Builtin4 { start_idx, .. } => (start_idx, 4),
                _ => {
                    continue;
                }
            };
            for j in 0..count {
                let r = arg_pool[(start_idx + j) as usize];
                use_count[r as usize] += 1;
            }
        }

        let single_use = |r: &u32| use_count[*r as usize] == 1;

        let n = instructions.len();
        let mut out = Vec::with_capacity(n);
        let mut i = 0;

        while i < n {
            if i + 1 < n {
                match (&instructions[i], &instructions[i + 1]) {
                    // LoadConst{t, c}, Add{d, t, s} or Add{d, s, t} -> AddConst{d, s, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::Add {
                            dest: add_dest,
                            a: add_a,
                            b: add_b,
                        },
                    ) if single_use(ld_dest) => {
                        if add_a == ld_dest {
                            out.push(Instruction::AddConst {
                                dest: *add_dest,
                                src: *add_b,
                                const_idx: *ld_c,
                            });
                            i += 2;
                            continue;
                        } else if add_b == ld_dest {
                            out.push(Instruction::AddConst {
                                dest: *add_dest,
                                src: *add_a,
                                const_idx: *ld_c,
                            });
                            i += 2;
                            continue;
                        }
                    }
                    // LoadConst{t, c}, MulAdd{d, a, b, t} -> MulAddConst{d, a, b, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::MulAdd {
                            dest: add_dest,
                            a: add_a,
                            b: add_b,
                            c: add_c,
                        },
                    ) if *add_c == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::MulAddConst {
                            dest: *add_dest,
                            a: *add_a,
                            b: *add_b,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, MulSub{d, a, b, t} -> MulSubConst{d, a, b, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::MulSub {
                            dest: sub_dest,
                            a: sub_a,
                            b: sub_b,
                            c: sub_c,
                        },
                    ) if *sub_c == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::MulSubConst {
                            dest: *sub_dest,
                            a: *sub_a,
                            b: *sub_b,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, MulSubRev{d, a, b, t} -> MulSubRevConst{d, a, b, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::MulSubRev {
                            dest: sub_dest,
                            a: sub_a,
                            b: sub_b,
                            c: sub_c,
                        },
                    ) if *sub_c == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::MulSubRevConst {
                            dest: *sub_dest,
                            a: *sub_a,
                            b: *sub_b,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, NegMulAdd{d, a, b, t} -> NegMulAddConst{d, a, b, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::NegMulAdd {
                            dest: add_dest,
                            a: add_a,
                            b: add_b,
                            c: add_c,
                        },
                    ) if *add_c == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::NegMulAddConst {
                            dest: *add_dest,
                            a: *add_a,
                            b: *add_b,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, Mul{d, t, s} or Mul{d, s, t} -> MulConst{d, s, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                    ) if single_use(ld_dest) => {
                        if mul_a == ld_dest {
                            out.push(Instruction::MulConst {
                                dest: *mul_dest,
                                src: *mul_b,
                                const_idx: *ld_c,
                            });
                            i += 2;
                            continue;
                        } else if mul_b == ld_dest {
                            out.push(Instruction::MulConst {
                                dest: *mul_dest,
                                src: *mul_a,
                                const_idx: *ld_c,
                            });
                            i += 2;
                            continue;
                        }
                    }
                    // LoadConst{t, c}, Sub{d, s, t} -> SubConst{d, s, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::Sub {
                            dest: sub_dest,
                            a: sub_a,
                            b: sub_b,
                        },
                    ) if *sub_b == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::SubConst {
                            dest: *sub_dest,
                            src: *sub_a,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, Sub{d, t, s} -> ConstSub{d, s, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::Sub {
                            dest: sub_dest,
                            a: sub_a,
                            b: sub_b,
                        },
                    ) if *sub_a == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::ConstSub {
                            dest: *sub_dest,
                            src: *sub_b,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, Div{d, s, t} -> DivConst{d, s, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::Div {
                            dest: div_dest,
                            num: div_num,
                            den: div_den,
                        },
                    ) if *div_den == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::DivConst {
                            dest: *div_dest,
                            src: *div_num,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // LoadConst{t, c}, Div{d, t, s} -> ConstDiv{d, s, c}
                    (
                        Instruction::LoadConst {
                            dest: ld_dest,
                            const_idx: ld_c,
                        },
                        Instruction::Div {
                            dest: div_dest,
                            num: div_num,
                            den: div_den,
                        },
                    ) if *div_num == *ld_dest && single_use(ld_dest) => {
                        out.push(Instruction::ConstDiv {
                            dest: *div_dest,
                            src: *div_den,
                            const_idx: *ld_c,
                        });
                        i += 2;
                        continue;
                    }
                    // Mul{t,a,b}, Add{d, t, c} -> MulAdd{d, a, b, c}
                    (
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                        Instruction::Add {
                            dest: add_dest,
                            a: add_a,
                            b: add_b,
                        },
                    ) if single_use(mul_dest) => {
                        if add_a == mul_dest {
                            out.push(Instruction::MulAdd {
                                dest: *add_dest,
                                a: *mul_a,
                                b: *mul_b,
                                c: *add_b,
                            });
                            i += 2;
                            continue;
                        } else if add_b == mul_dest {
                            out.push(Instruction::MulAdd {
                                dest: *add_dest,
                                a: *mul_a,
                                b: *mul_b,
                                c: *add_a,
                            });
                            i += 2;
                            continue;
                        }
                    }
                    // Mul{t,a,b}, Sub{d, t, c} -> MulSub{d, a, b, c}
                    (
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                        Instruction::Sub {
                            dest: sub_dest,
                            a: sub_a,
                            b: sub_b,
                        },
                    ) if sub_a == mul_dest && single_use(mul_dest) => {
                        out.push(Instruction::MulSub {
                            dest: *sub_dest,
                            a: *mul_a,
                            b: *mul_b,
                            c: *sub_b,
                        });
                        i += 2;
                        continue;
                    }
                    // Mul{t,a,b}, Sub{d, c, t} -> MulSubRev{d, a, b, c}
                    (
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                        Instruction::Sub {
                            dest: sub_dest,
                            a: sub_a,
                            b: sub_b,
                        },
                    ) if sub_b == mul_dest && single_use(mul_dest) => {
                        out.push(Instruction::MulSubRev {
                            dest: *sub_dest,
                            a: *mul_a,
                            b: *mul_b,
                            c: *sub_a,
                        });
                        i += 2;
                        continue;
                    }
                    // NegMulAdd{t,a,b,c}, Sub{d, c', t} -> MulSubRev{d, a, b, c' - c} -- wait, too complex.
                    // Just basic ones for now.
                    // Neg{t, s}, Exp{d, t} -> ExpNeg{d, s}
                    (
                        Instruction::Neg {
                            dest: neg_dest,
                            src: neg_src,
                        },
                        Instruction::Builtin1 {
                            dest: exp_dest,
                            op: FnOp::Exp,
                            arg: exp_arg,
                        },
                    ) if exp_arg == neg_dest && single_use(neg_dest) => {
                        out.push(Instruction::Builtin1 {
                            dest: *exp_dest,
                            op: FnOp::ExpNeg,
                            arg: *neg_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Sqrt{t, s}, Recip{d, t} -> InvSqrt{d, s}
                    (
                        Instruction::Builtin1 {
                            dest: sqrt_dest,
                            op: FnOp::Sqrt,
                            arg: sqrt_src,
                        },
                        Instruction::Recip {
                            dest: recip_dest,
                            src: recip_src,
                        },
                    ) if recip_src == sqrt_dest && single_use(sqrt_dest) => {
                        out.push(Instruction::InvSqrt {
                            dest: *recip_dest,
                            src: *sqrt_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Square{t, s}, Recip{d, t} -> InvSquare{d, s}
                    (
                        Instruction::Square {
                            dest: sq_dest,
                            src: sq_src,
                        },
                        Instruction::Recip {
                            dest: recip_dest,
                            src: recip_src,
                        },
                    ) if recip_src == sq_dest && single_use(sq_dest) => {
                        out.push(Instruction::InvSquare {
                            dest: *recip_dest,
                            src: *sq_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Cube{t, s}, Recip{d, t} -> InvCube{d, s}
                    (
                        Instruction::Cube {
                            dest: cube_dest,
                            src: cube_src,
                        },
                        Instruction::Recip {
                            dest: recip_dest,
                            src: recip_src,
                        },
                    ) if recip_src == cube_dest && single_use(cube_dest) => {
                        out.push(Instruction::InvCube {
                            dest: *recip_dest,
                            src: *cube_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Pow3_2{t, s}, Recip{d, t} -> InvPow3_2{d, s}
                    (
                        Instruction::Pow3_2 {
                            dest: p32_dest,
                            src: p32_src,
                        },
                        Instruction::Recip {
                            dest: recip_dest,
                            src: recip_src,
                        },
                    ) if recip_src == p32_dest && single_use(p32_dest) => {
                        out.push(Instruction::InvPow3_2 {
                            dest: *recip_dest,
                            src: *p32_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Pow4{t, s}, Recip{d, t} -> InvSquare{d, t'} where t' = Square(s)
                    // Note: We can reuse the temp register from Pow4 for the intermediate Square.
                    (
                        Instruction::Pow4 {
                            dest: p4_dest,
                            src: p4_src,
                        },
                        Instruction::Recip {
                            dest: recip_dest,
                            src: recip_src,
                        },
                    ) if recip_src == p4_dest && single_use(p4_dest) => {
                        // Reuse p4_dest as a temporary for x^2
                        out.push(Instruction::Square {
                            dest: *p4_dest,
                            src: *p4_src,
                        });
                        out.push(Instruction::InvSquare {
                            dest: *recip_dest,
                            src: *p4_dest,
                        });
                        i += 2;
                        continue;
                    }
                    // Square{t, s}, Mul{d, t, s} or Mul{d, s, t} -> Cube{d, s}
                    (
                        Instruction::Square {
                            dest: sq_dest,
                            src: sq_src,
                        },
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                    ) if single_use(sq_dest)
                        && ((*mul_a == *sq_dest && *mul_b == *sq_src)
                            || (*mul_a == *sq_src && *mul_b == *sq_dest)) =>
                    {
                        out.push(Instruction::Cube {
                            dest: *mul_dest,
                            src: *sq_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Cube{t, s}, Mul{d, t, s} or Mul{d, s, t} -> Pow4{d, s}
                    (
                        Instruction::Cube {
                            dest: cube_dest,
                            src: cube_src,
                        },
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                    ) if single_use(cube_dest)
                        && ((*mul_a == *cube_dest && *mul_b == *cube_src)
                            || (*mul_a == *cube_src && *mul_b == *cube_dest)) =>
                    {
                        out.push(Instruction::Pow4 {
                            dest: *mul_dest,
                            src: *cube_src,
                        });
                        i += 2;
                        continue;
                    }
                    // Square{t1, s}, Square{t2, s}, Mul{d, t1, t2} -> Pow4{d, s}
                    // Note: This is less common but can happen after CSE.
                    (
                        Instruction::Square {
                            dest: sq1_dest,
                            src: sq1_src,
                        },
                        Instruction::Mul {
                            dest: mul_dest,
                            a: mul_a,
                            b: mul_b,
                        },
                    ) if single_use(sq1_dest) => {
                        // We need to look back or ahead? This is hard in a single pass.
                        // Let's just do the ones that are adjacent or easily detectable.
                    }
                    _ => {}
                }
            }
            out.push(instructions[i]);
            i += 1;
        }

        // --- Copy forwarding pass ---
        // Recompute use_count after fusion
        use_count.fill(0);
        for instr in &out {
            instr.for_each_read(|r| use_count[r as usize] += 1);
            let (start_idx, count) = match *instr {
                Instruction::AddN {
                    start_idx, count, ..
                }
                | Instruction::MulN {
                    start_idx, count, ..
                } => (start_idx, count),
                Instruction::Builtin3 { start_idx, .. } => (start_idx, 3),
                Instruction::Builtin4 { start_idx, .. } => (start_idx, 4),
                _ => {
                    continue;
                }
            };
            for j in 0..count {
                let r = arg_pool[(start_idx + j) as usize];
                use_count[r as usize] += 1;
            }
        }

        // Build a forwarding table: copy_of[r] = the register r was copied from (or r itself)
        let mut copy_of: Vec<u32> = (0..=max_reg_idx).collect();

        // First pass: record all Copy sources
        for instr in &out {
            if let Instruction::Copy { dest, src } = *instr {
                // Only forward if dest is written exactly once (this Copy is its sole definition)
                // and dest != 0 (never forward away from the result register).
                // SAFETY: We only forward if src is a Parameter or Constant (immutables).
                // Temporaries (src >= param_count + const_count) can be overwritten.
                if dest != 0
                    && use_count[dest as usize] <= 1
                    && src < (param_count + old_const_count) as u32
                {
                    copy_of[dest as usize] = copy_of[src as usize];
                }
            }
        }

        // Second pass: rewrite all reads through the forwarding table, then drop dead Copies
        for instr in &mut out {
            instr.map_reads(|r| copy_of[r as usize]);
            let (start_idx, count) = match *instr {
                Instruction::AddN {
                    start_idx, count, ..
                }
                | Instruction::MulN {
                    start_idx, count, ..
                } => (start_idx, count),
                Instruction::Builtin3 { start_idx, .. } => (start_idx, 3),
                Instruction::Builtin4 { start_idx, .. } => (start_idx, 4),
                _ => {
                    continue;
                }
            };
            for j in 0..count {
                let r_ref = &mut arg_pool[(start_idx + j) as usize];
                *r_ref = copy_of[*r_ref as usize];
            }
        }
        out.retain(|instr| {
            if let Instruction::Copy { dest, .. } = *instr {
                // Keep if dest was NOT forwarded (i.e. it's still needed as a named register)
                copy_of[dest as usize] == dest
            } else {
                true
            }
        });

        // --- Dead Constant Elimination & Pool Compaction ---
        let mut used_pool_indices = FxHashSet::default();
        for instr in &out {
            match instr {
                Instruction::LoadConst { const_idx, .. }
                | Instruction::MulAddConst { const_idx, .. }
                | Instruction::MulSubConst { const_idx, .. }
                | Instruction::NegMulAddConst { const_idx, .. }
                | Instruction::AddConst { const_idx, .. }
                | Instruction::MulConst { const_idx, .. }
                | Instruction::SubConst { const_idx, .. }
                | Instruction::ConstSub { const_idx, .. }
                | Instruction::DivConst { const_idx, .. }
                | Instruction::ConstDiv { const_idx, .. } => {
                    used_pool_indices.insert(*const_idx);
                }
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Degree and polynomial length fit in u32 and are strictly positive"
                )]
                Instruction::PolyEval { const_idx, .. } => {
                    let start = *const_idx as usize;
                    let degree = constants[start].to_bits() as usize;
                    for j in 0..=(degree + 1) {
                        used_pool_indices.insert((start + j) as u32);
                    }
                }
                _ => {}
            }
        }

        // A constant might also be "used" if it's referenced as a register directly
        // in instructions like Add { a, b, dest }.
        let mut used_reg_indices = FxHashSet::default();
        for instr in &out {
            instr.for_each_read(|r| {
                if r >= param_count as u32 && r < (param_count + old_const_count) as u32 {
                    used_reg_indices.insert(r - param_count as u32);
                }
            });
            let (start_idx, count) = match *instr {
                Instruction::AddN {
                    start_idx, count, ..
                }
                | Instruction::MulN {
                    start_idx, count, ..
                } => (start_idx, count),
                Instruction::Builtin3 { start_idx, .. } => (start_idx, 3),
                Instruction::Builtin4 { start_idx, .. } => (start_idx, 4),
                _ => {
                    continue;
                }
            };
            for j in 0..count {
                let r = arg_pool[(start_idx + j) as usize];
                if r >= param_count as u32 && r < (param_count + old_const_count) as u32 {
                    used_reg_indices.insert(r - param_count as u32);
                }
            }
        }

        let all_used_indices: FxHashSet<u32> = used_pool_indices
            .union(&used_reg_indices)
            .copied()
            .collect();

        if all_used_indices.len() == old_const_count {
            let mut final_max_reg = 0;
            for instr in &out {
                instr.for_each_reg(|r| final_max_reg = final_max_reg.max(r));
            }
            return (
                out,
                (final_max_reg as usize + 1).max(param_count + old_const_count),
            );
        }

        // Compact constants pool
        let mut new_constants = Vec::with_capacity(all_used_indices.len());
        let mut index_map = rustc_hash::FxHashMap::default();
        for (old_idx, &val) in constants.iter().enumerate() {
            if all_used_indices.contains(&(old_idx as u32)) {
                index_map.insert(old_idx as u32, new_constants.len() as u32);
                new_constants.push(val);
            }
        }
        let new_const_count = new_constants.len();
        *constants = new_constants;

        // Update instructions: pool indices AND register indices
        let shift = old_const_count as u32 - new_const_count as u32;
        for instr in &mut out {
            // Update pool indices
            match instr {
                Instruction::LoadConst { const_idx, .. }
                | Instruction::MulAddConst { const_idx, .. }
                | Instruction::MulSubConst { const_idx, .. }
                | Instruction::NegMulAddConst { const_idx, .. }
                | Instruction::AddConst { const_idx, .. }
                | Instruction::MulConst { const_idx, .. }
                | Instruction::SubConst { const_idx, .. }
                | Instruction::ConstSub { const_idx, .. }
                | Instruction::DivConst { const_idx, .. }
                | Instruction::ConstDiv { const_idx, .. }
                | Instruction::PolyEval { const_idx, .. } => {
                    if let Some(&new_idx) = index_map.get(const_idx) {
                        *const_idx = new_idx;
                    }
                }
                _ => {}
            }

            // Update register indices
            let mut remap = |r: u32| {
                if r < param_count as u32 {
                    r
                } else if r < (param_count + old_const_count) as u32 {
                    let old_c_idx = r - param_count as u32;
                    param_count as u32 + *index_map.get(&old_c_idx).unwrap_or(&0)
                } else {
                    // Temp register: shift down
                    r - shift
                }
            };
            instr.map_regs(&mut remap);
            let (start_idx, count) = match *instr {
                Instruction::AddN {
                    start_idx, count, ..
                }
                | Instruction::MulN {
                    start_idx, count, ..
                } => (start_idx, count),
                Instruction::Builtin3 { start_idx, .. } => (start_idx, 3),
                Instruction::Builtin4 { start_idx, .. } => (start_idx, 4),
                _ => {
                    continue;
                }
            };
            for j in 0..count {
                let r_ref = &mut arg_pool[(start_idx + j) as usize];
                *r_ref = remap(*r_ref);
            }
        }

        // --- Final Constant Optimization & Instruction Mapping ---
        // Convert DivConst to MulConst where possible, and other strength reductions
        for instr in &mut out {
            match *instr {
                Instruction::DivConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    let divisor = constants[const_idx as usize];
                    if (divisor - 1.0).abs() < f64::EPSILON {
                        *instr = Instruction::Copy { dest, src };
                    } else if divisor != 0.0 && divisor.is_finite() {
                        let recip = 1.0 / divisor;
                        // Check if reciprocal already exists
                        let mut found_idx = None;
                        for (idx, &c) in constants.iter().enumerate() {
                            if (c - recip).abs() < f64::EPSILON {
                                found_idx = Some(idx as u32);
                                break;
                            }
                        }

                        if let Some(new_idx) = found_idx {
                            *instr = Instruction::MulConst {
                                dest,
                                src,
                                const_idx: new_idx,
                            };
                        } else {
                            // Add reciprocal to constants pool
                            let new_idx = constants.len() as u32;
                            constants.push(recip);
                            *instr = Instruction::MulConst {
                                dest,
                                src,
                                const_idx: new_idx,
                            };
                        }
                    }
                }
                Instruction::ConstDiv {
                    dest,
                    src,
                    const_idx,
                } => {
                    let c = constants[const_idx as usize];
                    if (c - 1.0).abs() < f64::EPSILON {
                        *instr = Instruction::Recip { dest, src };
                    }
                }
                Instruction::AddConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    let c = constants[const_idx as usize];
                    if c.abs() < f64::EPSILON {
                        *instr = Instruction::Copy { dest, src };
                    }
                }
                Instruction::MulConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    let c = constants[const_idx as usize];
                    if (c - 1.0).abs() < f64::EPSILON {
                        *instr = Instruction::Copy { dest, src };
                    } else if (c + 1.0).abs() < f64::EPSILON {
                        *instr = Instruction::Neg { dest, src };
                    }
                }
                Instruction::SubConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    let c = constants[const_idx as usize];
                    if c.abs() < f64::EPSILON {
                        *instr = Instruction::Copy { dest, src };
                    } else {
                        let neg_c = -c;
                        let mut found_idx = None;
                        for (idx, &val) in constants.iter().enumerate() {
                            if (val - neg_c).abs() < f64::EPSILON {
                                found_idx = Some(idx as u32);
                                break;
                            }
                        }
                        if let Some(new_idx) = found_idx {
                            *instr = Instruction::AddConst {
                                dest,
                                src,
                                const_idx: new_idx,
                            };
                        } else {
                            let new_idx = constants.len() as u32;
                            constants.push(neg_c);
                            *instr = Instruction::AddConst {
                                dest,
                                src,
                                const_idx: new_idx,
                            };
                        }
                    }
                }
                Instruction::ConstSub {
                    dest,
                    src,
                    const_idx,
                } => {
                    let c = constants[const_idx as usize];
                    if c.abs() < f64::EPSILON {
                        *instr = Instruction::Neg { dest, src };
                    }
                }
                Instruction::Powi {
                    dest,
                    src,
                    n: pow_n,
                } => match pow_n {
                    2 => *instr = Instruction::Square { dest, src },
                    3 => *instr = Instruction::Cube { dest, src },
                    4 => *instr = Instruction::Pow4 { dest, src },
                    -1 => *instr = Instruction::Recip { dest, src },
                    _ => {}
                },
                _ => {}
            }
        }

        let mut final_max_reg = 0;
        for instr in &out {
            instr.for_each_reg(|r| final_max_reg = final_max_reg.max(r));
            let (start_idx, count) = match *instr {
                Instruction::AddN {
                    start_idx, count, ..
                }
                | Instruction::MulN {
                    start_idx, count, ..
                } => (start_idx, count),
                Instruction::Builtin3 { start_idx, .. } => (start_idx, 3),
                Instruction::Builtin4 { start_idx, .. } => (start_idx, 4),
                _ => {
                    continue;
                }
            };
            for j in 0..count {
                let r = arg_pool[(start_idx + j) as usize];
                final_max_reg = final_max_reg.max(r);
            }
        }

        (
            out,
            (final_max_reg as usize + 1).max(param_count + constants.len()),
        )
    }

    /// Recursively expand user function calls with their body expressions.
    ///
    /// This substitutes `f(arg1, arg2, ...)` with the body expression where
    /// formal parameters are replaced by the actual argument expressions.
    ///
    /// # Recursion Protection
    ///
    /// - The `expanding` set tracks functions currently being expanded to prevent
    ///   infinite recursion from self-referential or mutually recursive functions.
    /// - The `depth` parameter limits recursion depth to prevent stack overflow.
    fn expand_user_functions(
        expr: &Expr,
        ctx: &Context,
        expanding: &mut std::collections::HashSet<u64>,
        depth: usize,
    ) -> Expr {
        const MAX_EXPANSION_DEPTH: usize = 100;

        if depth > MAX_EXPANSION_DEPTH {
            // Return unexpanded to prevent stack overflow
            return expr.clone();
        }

        // Fast path: if the context has no user-defined functions with bodies,
        // there's nothing to expand — skip the entire recursive traversal.
        if depth == 0 && !ctx.has_expandable_functions() {
            return expr.clone();
        }

        match &expr.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => expr.clone(),

            ExprKind::Sum(terms) => {
                let expanded: Vec<Expr> = terms
                    .iter()
                    .map(|t| Self::expand_user_functions(t, ctx, expanding, depth + 1))
                    .collect();
                Expr::sum(expanded)
            }

            ExprKind::Product(factors) => {
                let expanded: Vec<Expr> = factors
                    .iter()
                    .map(|f| Self::expand_user_functions(f, ctx, expanding, depth + 1))
                    .collect();
                Expr::product(expanded)
            }

            ExprKind::Div(num, den) => {
                let num_exp = Self::expand_user_functions(num, ctx, expanding, depth + 1);
                let den_exp = Self::expand_user_functions(den, ctx, expanding, depth + 1);
                Expr::div_expr(num_exp, den_exp)
            }

            ExprKind::Pow(base, exp) => {
                let base_exp = Self::expand_user_functions(base, ctx, expanding, depth + 1);
                let exp_exp = Self::expand_user_functions(exp, ctx, expanding, depth + 1);
                Expr::pow_static(base_exp, exp_exp)
            }

            ExprKind::FunctionCall { name, args } => {
                // First expand arguments
                let expanded_args: Vec<Expr> = args
                    .iter()
                    .map(|a| Self::expand_user_functions(a, ctx, expanding, depth + 1))
                    .collect();

                let fn_id = name.id();

                // Check for recursion and if this is a user function with a body
                if !expanding.contains(&fn_id)
                    && let Some(user_fn) = ctx.get_user_fn_by_id(fn_id)
                    && user_fn.accepts_arity(expanded_args.len())
                    && let Some(body_fn) = &user_fn.body
                {
                    // Mark as expanding to prevent infinite recursion
                    expanding.insert(fn_id);

                    let arc_args: Vec<Arc<Expr>> =
                        expanded_args.iter().map(|a| Arc::new(a.clone())).collect();
                    let body_expr = body_fn(&arc_args);
                    let result = Self::expand_user_functions(&body_expr, ctx, expanding, depth + 1);

                    expanding.remove(&fn_id);
                    return result;
                }

                // Not expandable - return as-is with expanded args
                Expr::func_multi_symbol(name.clone(), expanded_args)
            }

            ExprKind::Poly(poly) => {
                // Expand user functions in the base expression only.
                // The Poly node itself is preserved so compile_polynomial()
                // can emit a single PolyEval (Horner) instruction.
                // Expanding to to_expr() would flatten into Sum-of-Products,
                // causing individual high-power terms to overflow f64 to ±∞.
                let expanded_base =
                    Self::expand_user_functions(poly.base(), ctx, expanding, depth + 1);

                if expanded_base == **poly.base() {
                    expr.clone() // base unchanged, no need to rebuild
                } else {
                    Expr::poly(poly.with_base(Arc::new(expanded_base)))
                }
            }

            ExprKind::Derivative { inner, var, order } => {
                let expanded_inner = Self::expand_user_functions(inner, ctx, expanding, depth + 1);
                Expr::derivative_interned(expanded_inner, var.clone(), *order)
            }
        }
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the required register count for this expression.
    #[must_use]
    pub const fn register_count(&self) -> usize {
        self.register_count
    }

    /// Get parameter names in order.
    #[inline]
    #[must_use]
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }

    /// Get number of parameters.
    #[inline]
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.param_names.len()
    }

    /// Get number of bytecode instructions (for debugging/profiling).
    #[inline]
    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Get constants as SIMD vectors, computed once and cached.
    #[inline]
    pub(crate) fn simd_constants(&self) -> &[f64x4] {
        self.simd_constants
            .get_or_init(|| {
                self.constants
                    .iter()
                    .map(|&c| f64x4::splat(c))
                    .collect::<Box<[_]>>()
            })
            .as_ref()
    }
}

impl std::fmt::Debug for CompiledEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledEvaluator")
            .field("param_names", &self.param_names)
            .field("param_count", &self.param_count)
            .field("instruction_count", &self.instructions.len())
            .field("arg_pool_count", &self.arg_pool.len())
            .field("register_count", &self.register_count)
            .field("constant_count", &self.constants.len())
            .field(
                "simd_constants_cached",
                &self.simd_constants.get().is_some(),
            )
            .finish()
    }
}
