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
    /// Parameter names in order (for mapping `HashMap` → array)
    pub param_names: Box<[String]>,
    /// Required stack depth for evaluation
    pub register_count: usize,
    /// Number of parameters expected
    pub param_count: usize,
    /// Lazily cached SIMD-splatted constants (one f64x4 per scalar constant).
    simd_constants: OnceLock<Arc<[f64x4]>>,
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
        let (instructions, mut constants, max_stack, param_count) = compiler.into_parts();

        // Post-compilation optimization pass: fuse instructions
        let optimized_instructions = Self::optimize_instructions(instructions, &mut constants);

        Ok(Self {
            instructions: optimized_instructions.into_boxed_slice(),
            constants: constants.into_boxed_slice(),
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
    /// - `MulAdd` fusion: `[Mul, Add]` → `MulAdd`
    /// - `MulSub` fusion: `[Mul, Sub]` → `MulSub`
    /// - `NegMulAdd` fusion: `[Mul, Sub]` → `NegMulAdd`
    /// - `ExpNeg` fusion: `[Neg, Exp]` → `ExpNeg`
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
        _constants: &mut Vec<f64>,
    ) -> Vec<Instruction> {
        use instruction::FnOp;
        use rustc_hash::FxHashMap;

        if instructions.len() < 2 {
            return instructions;
        }

        // Count uses of each register
        let mut use_count: FxHashMap<u32, usize> = FxHashMap::default();
        for instr in &instructions {
            match instr {
                Instruction::LoadConst { .. } => {}
                Instruction::Copy { src, .. }
                | Instruction::Neg { src, .. }
                | Instruction::Square { src, .. }
                | Instruction::Cube { src, .. }
                | Instruction::Pow4 { src, .. }
                | Instruction::Pow3_2 { src, .. }
                | Instruction::InvPow3_2 { src, .. }
                | Instruction::InvSqrt { src, .. }
                | Instruction::InvSquare { src, .. }
                | Instruction::InvCube { src, .. }
                | Instruction::Recip { src, .. }
                | Instruction::Powi { src, .. }
                | Instruction::RecipExpm1 { src, .. }
                | Instruction::ExpSqr { src, .. }
                | Instruction::ExpSqrNeg { src, .. } => {
                    *use_count.entry(*src).or_insert(0) += 1;
                }
                Instruction::Add { a, b, .. }
                | Instruction::Mul { a, b, .. }
                | Instruction::Sub { a, b, .. }
                | Instruction::Div { num: a, den: b, .. }
                | Instruction::Pow {
                    base: a, exp: b, ..
                } => {
                    *use_count.entry(*a).or_insert(0) += 1;
                    *use_count.entry(*b).or_insert(0) += 1;
                }
                Instruction::Builtin1 { arg, .. } => {
                    *use_count.entry(*arg).or_insert(0) += 1;
                }
                Instruction::Builtin2 { arg1, arg2, .. } => {
                    *use_count.entry(*arg1).or_insert(0) += 1;
                    *use_count.entry(*arg2).or_insert(0) += 1;
                }
                Instruction::Builtin3 {
                    arg1, arg2, arg3, ..
                } => {
                    *use_count.entry(*arg1).or_insert(0) += 1;
                    *use_count.entry(*arg2).or_insert(0) += 1;
                    *use_count.entry(*arg3).or_insert(0) += 1;
                }
                Instruction::Builtin4 {
                    arg1,
                    arg2,
                    arg3,
                    arg4,
                    ..
                } => {
                    *use_count.entry(*arg1).or_insert(0) += 1;
                    *use_count.entry(*arg2).or_insert(0) += 1;
                    *use_count.entry(*arg3).or_insert(0) += 1;
                    *use_count.entry(*arg4).or_insert(0) += 1;
                }
                Instruction::MulAdd { a, b, c, .. }
                | Instruction::MulSub { a, b, c, .. }
                | Instruction::NegMulAdd { a, b, c, .. } => {
                    *use_count.entry(*a).or_insert(0) += 1;
                    *use_count.entry(*b).or_insert(0) += 1;
                    *use_count.entry(*c).or_insert(0) += 1;
                }
                Instruction::PolyEval { x, .. } => {
                    *use_count.entry(*x).or_insert(0) += 1;
                }
            }
        }

        let single_use = |r: &u32| use_count.get(r).copied().unwrap_or(0) == 1;

        let n = instructions.len();
        let mut out = Vec::with_capacity(n);
        let mut i = 0;

        while i < n {
            if i + 1 < n {
                match (&instructions[i], &instructions[i + 1]) {
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
                    // Mul{t,a,b}, Sub{d, c, t} -> NegMulAdd{d, a, b, c}
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
                        out.push(Instruction::NegMulAdd {
                            dest: *sub_dest,
                            a: *mul_a,
                            b: *mul_b,
                            c: *sub_a,
                        });
                        i += 2;
                        continue;
                    }
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
                    _ => {}
                }
            }
            out.push(instructions[i].clone());
            i += 1;
        }
        out
    }

    /// Deduplicate constant pool to reduce memory usage and improve cache hits.
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
                    .collect::<Arc<[_]>>()
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
            .field("register_count", &self.register_count)
            .field("constant_count", &self.constants.len())
            .field(
                "simd_constants_cached",
                &self.simd_constants.get().is_some(),
            )
            .finish()
    }
}
