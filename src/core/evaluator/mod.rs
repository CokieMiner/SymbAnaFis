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
//! This module uses unsafe code in performance-critical stack operations.
//! Safety is guaranteed by the [`Compiler`] which validates stack depth at
//! compile time. See [`stack`] module documentation for details.
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
//! - [`stack`]: Stack operations with safety documentation
//! - [`execution`]: Scalar evaluation implementation
//! - [`simd`]: SIMD batch evaluation implementation

// Allow unsafe code in this module - safety is guaranteed by compile-time stack validation
#![allow(unsafe_code)]

// Internal submodules - visibility is controlled by parent module (not exported from crate root)
mod compiler;
mod execution;
mod instruction;
mod simd;
mod stack;

#[cfg(test)]
mod tests;

// Re-exports for sibling modules within evaluator
pub(super) use compiler::Compiler;
pub(super) use instruction::Instruction;

use crate::core::error::DiffError;
use crate::core::unified_context::Context;
use crate::{Expr, ExprKind, Symbol};
use std::sync::Arc;

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
pub struct CompiledEvaluator {
    /// Bytecode instructions (immutable after compilation)
    pub instructions: Arc<[Instruction]>,
    /// Required stack depth for evaluation
    pub stack_size: usize,
    /// Parameter names in order (for mapping `HashMap` → array)
    pub param_names: Arc<[String]>,
    /// Number of parameters expected
    pub param_count: usize,
    /// Number of CSE cache slots required
    pub cache_size: usize,
    /// Constant pool for numeric literals
    pub constants: Arc<[f64]>,
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
    /// - `StackOverflow`: Expression too deeply nested (> 1024 depth)
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

        let mut compiler = Compiler::new(&param_ids, context);

        // Single-pass compilation with CSE
        compiler.compile_expr(&expanded_expr)?;

        // Extract compilation results
        let (instructions, constants, max_stack, param_count, cache_size) = compiler.into_parts();

        // Post-compilation optimization pass: fuse instructions
        let optimized_instructions = Self::optimize_instructions(instructions);

        Ok(Self {
            instructions: optimized_instructions.into(),
            stack_size: max_stack,
            param_names: param_names.into_iter().collect(),
            param_count,
            cache_size,
            constants: constants.into(),
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
        let vars = expr.variables();
        let mut param_order: Vec<String> = vars
            .into_iter()
            .filter(|v| !crate::core::known_symbols::is_known_constant(v.as_str()))
            .collect();
        param_order.sort(); // Consistent ordering

        Self::compile(expr, &param_order, context)
    }

    /// Post-compilation optimization pass that fuses instruction patterns.
    ///
    /// Currently detects:
    /// - `MulAdd` fusion: `[Mul, LoadX, Add]` → `[LoadX, MulAdd]`
    //
    // Allow needless_pass_by_value: Takes ownership to match call site pattern where
    // caller builds Vec and passes it directly without needing it afterwards.
    #[allow(clippy::needless_pass_by_value)]
    fn optimize_instructions(instructions: Vec<Instruction>) -> Vec<Instruction> {
        Self::fuse_muladd(&instructions)
    }

    /// Fuse `a * b + c` patterns into `MulAdd` instruction.
    ///
    /// The `MulAdd` instruction uses hardware FMA (fused multiply-add) when available,
    /// which is both faster and more accurate than separate multiply and add.
    fn fuse_muladd(instructions: &[Instruction]) -> Vec<Instruction> {
        use Instruction::{Add, LoadCached, LoadConst, LoadParam, Mul, MulAdd};

        let mut result = Vec::with_capacity(instructions.len());
        let mut i = 0;

        while i < instructions.len() {
            // Look for pattern: ..., Mul, LoadX, Add
            if i + 2 < instructions.len()
                && let (Mul, load_instr, Add) =
                    (&instructions[i], &instructions[i + 1], &instructions[i + 2])
                && matches!(load_instr, LoadParam(_) | LoadConst(_) | LoadCached(_))
            {
                // Fuse: emit Load then MulAdd
                result.push(*load_instr);
                result.push(MulAdd);
                i += 3;
                continue;
            }

            result.push(instructions[i]);
            i += 1;
        }

        result
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
        expanding: &mut std::collections::HashSet<String>,
        depth: usize,
    ) -> Expr {
        const MAX_EXPANSION_DEPTH: usize = 100;

        if depth > MAX_EXPANSION_DEPTH {
            // Return unexpanded to prevent stack overflow
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

                let fn_name = name.as_str().to_owned();

                // Check for recursion and if this is a user function with a body
                if !expanding.contains(&fn_name)
                    && let Some(user_fn) = ctx.get_user_fn(&fn_name)
                    && user_fn.accepts_arity(expanded_args.len())
                    && let Some(body_fn) = &user_fn.body
                {
                    // Mark as expanding to prevent infinite recursion
                    expanding.insert(fn_name.clone());

                    let arc_args: Vec<Arc<Expr>> =
                        expanded_args.iter().map(|a| Arc::new(a.clone())).collect();
                    let body_expr = body_fn(&arc_args);
                    let result = Self::expand_user_functions(&body_expr, ctx, expanding, depth + 1);

                    expanding.remove(&fn_name);
                    return result;
                }

                // Not expandable - return as-is with expanded args
                Expr::func_multi(name.as_str(), expanded_args)
            }

            ExprKind::Poly(poly) => {
                let poly_expr = poly.to_expr();
                Self::expand_user_functions(&poly_expr, ctx, expanding, depth + 1)
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

    /// Get the required stack size for this expression.
    #[must_use]
    pub const fn stack_size(&self) -> usize {
        self.stack_size
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
}

impl std::fmt::Debug for CompiledEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledEvaluator")
            .field("param_names", &self.param_names)
            .field("param_count", &self.param_count)
            .field("instruction_count", &self.instructions.len())
            .field("stack_size", &self.stack_size)
            .field("cache_size", &self.cache_size)
            .field("constant_count", &self.constants.len())
            .finish()
    }
}
