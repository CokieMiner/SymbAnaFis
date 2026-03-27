//! High-level evaluator API: types, builders, traits, and free functions.
//!
//! # Public Surface
//! - [`EvaluatorBuilder`] — builder for [`CompiledEvaluator`]
//! - [`CompiledEvaluator`] — compiled, thread-safe expression evaluator
//! - [`ToParamName`] — trait for types usable as parameter names
//! - [`eval_f64`] — parallel batch evaluation over multiple expressions (requires `parallel` feature)

use std::fmt::{Debug, Formatter, Result as FmtResult};

pub use super::logic::VarLookup;
pub use super::logic::{Compiler, Instruction, expand_user_functions};
#[cfg(feature = "parallel")]
pub use super::logic::{EvalResult, ExprInput, SKIP, Value, VarInput, evaluate_parallel};

#[cfg(feature = "parallel")]
use super::logic::eval_single_expr_chunked;
#[cfg(all(feature = "parallel", feature = "python"))]
pub use super::logic::evaluate_parallel_with_hint;

use crate::{
    Expr, Symbol,
    core::{Context, error::DiffError, known_symbols::is_known_constant_by_id, symb_interned},
    symb,
};
#[cfg(feature = "parallel")]
use std::sync::OnceLock;
#[cfg(feature = "parallel")]
use wide::f64x4;

// ============================================================================
// EvaluatorBuilder
// ============================================================================

/// Builder for `CompiledEvaluator` to handle complex operations with optional parameters.
///
/// # Example
///
/// ```
/// use symb_anafis::{symb, EvaluatorBuilder};
///
/// let x = symb("x");
/// let y = symb("y");
/// let expr = x.pow(2.0) + y;
///
/// let compiled = EvaluatorBuilder::new(&expr)
///     .params(&["x", "y"])
///     .build()
///     .expect("Should compile");
/// ```
pub struct EvaluatorBuilder<'ctx> {
    pub(crate) expr: &'ctx Expr,
    pub(crate) param_order: Option<Vec<String>>,
    pub(crate) context: Option<&'ctx Context>,
}

impl<'ctx> EvaluatorBuilder<'ctx> {
    /// Create a new builder for the given expression.
    #[inline]
    #[must_use]
    pub const fn new(expr: &'ctx Expr) -> Self {
        Self {
            expr,
            param_order: None,
            context: None,
        }
    }

    /// Set the parameter order. If not set, parameters are automatically extracted and sorted.
    #[inline]
    #[must_use]
    pub fn params<I, P>(mut self, params: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: ToParamName,
    {
        self.param_order = Some(
            params
                .into_iter()
                .map(|p| p.to_param_id_and_name().1)
                .collect(),
        );
        self
    }

    /// Set the context for custom function definitions.
    #[inline]
    #[must_use]
    pub const fn context(mut self, ctx: &'ctx Context) -> Self {
        self.context = Some(ctx);
        self
    }

    /// Build the `CompiledEvaluator`.
    ///
    /// # Errors
    ///
    /// Returns `DiffError` if compilation fails.
    pub fn build(self) -> Result<CompiledEvaluator, DiffError> {
        if let Some(params) = self.param_order {
            CompiledEvaluator::compile(self.expr, &params, self.context)
        } else {
            CompiledEvaluator::compile_auto(self.expr, self.context)
        }
    }
}

// ============================================================================
// CompiledEvaluator
// ============================================================================

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

#[derive(Clone)]
pub struct CompiledEvaluator {
    /// Bytecode instructions (immutable after compilation)
    pub(crate) instructions: Box<[Instruction]>,
    /// Constant pool for numeric literals
    pub(crate) constants: Box<[f64]>,
    /// Argument pool for N-ary instructions (`AddN`, `MulN`)
    pub(crate) arg_pool: Box<[u32]>,
    /// Parameter names in order (for mapping `HashMap` -> array)
    pub(crate) param_names: Box<[String]>,
    /// Required workspace size for evaluation
    pub(crate) workspace_size: usize,
    /// Number of parameters expected
    pub(crate) param_count: usize,
    /// Lazily cached SIMD-splatted constants (one f64x4 per scalar constant).
    #[cfg(feature = "parallel")]
    pub(crate) simd_constants_cache: OnceLock<Box<[f64x4]>>,
    /// Pre-loaded register template with constants filled in.
    /// Used to avoid repeated constant copying in scalar evaluation.
    pub(crate) registers_template: Box<[f64]>,
}

impl CompiledEvaluator {
    /// Create a new `EvaluatorBuilder` for complex compilations.
    #[inline]
    #[must_use]
    pub const fn builder(expr: &Expr) -> EvaluatorBuilder<'_> {
        EvaluatorBuilder::new(expr)
    }

    /// Get constants as SIMD vectors, computed once and cached.
    #[cfg(feature = "parallel")]
    #[inline]
    pub(crate) fn simd_constants(&self) -> &[f64x4] {
        self.simd_constants_cache
            .get_or_init(|| {
                self.constants
                    .iter()
                    .map(|&c| f64x4::splat(c))
                    .collect::<Box<[_]>>()
            })
            .as_ref()
    }

    /// Get the compiled evaluator parameter names in order.
    #[inline]
    #[must_use]
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }

    /// Get the number of parameters expected by this evaluator.
    #[inline]
    #[must_use]
    pub const fn param_count(&self) -> usize {
        self.param_count
    }

    /// Get the workspace size required for evaluation.
    #[inline]
    #[must_use]
    pub const fn workspace_size(&self) -> usize {
        self.workspace_size
    }

    /// Get the number of compiled instructions.
    #[inline]
    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Get the number of constants in the constant pool.
    #[inline]
    #[must_use]
    pub fn constant_count(&self) -> usize {
        self.constants.len()
    }
}

impl Debug for CompiledEvaluator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut s = f.debug_struct("CompiledEvaluator");
        s.field("param_names", &self.param_names)
            .field("param_count", &self.param_count)
            .field("instruction_count", &self.instructions.len())
            .field("arg_pool_count", &self.arg_pool.len())
            .field("workspace_size", &self.workspace_size)
            .field("constant_count", &self.constants.len());

        #[cfg(feature = "parallel")]
        s.field(
            "simd_constants_cached",
            &self.simd_constants_cache.get().is_some(),
        );

        s.field(
            "has_registers_template",
            &!self.registers_template.is_empty(),
        )
        .finish()
    }
}

// ============================================================================
// ToParamName trait
// ============================================================================

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
        let sym = symb(s);
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

// ============================================================================
// Free functions
// ============================================================================

#[cfg(feature = "parallel")]
/// High-performance parallel batch evaluation for pure numeric workloads.
///
/// # Errors
///
/// Returns `DiffError` if `exprs`, `var_names`, and `data` differ in length,
/// or if any individual expression fails to compile or evaluate.
pub fn eval_f64<V: ToParamName + Sync>(
    exprs: &[&Expr],
    var_names: &[&[V]],
    data: &[&[&[f64]]],
) -> Result<Vec<Vec<f64>>, DiffError> {
    use rayon::prelude::*;

    if exprs.len() != var_names.len() || exprs.len() != data.len() {
        return Err(DiffError::invalid_syntax(
            "exprs, var_names, and data must have the same length",
        ));
    }

    (0..exprs.len())
        .into_par_iter()
        .map(|expr_idx| {
            eval_single_expr_chunked(
                exprs[expr_idx],
                var_names[expr_idx],
                data[expr_idx],
                expr_idx,
            )
        })
        .collect()
}
// ============================================================================
// Compilation entry-points (impl on CompiledEvaluator)
// ============================================================================

impl CompiledEvaluator {
    /// Compile an expression to bytecode.
    ///
    /// * `param_order` — Parameters in evaluation order. Accepts `&[&str]` or `&[&Symbol]`.
    /// * `context` — Optional context for custom function definitions.
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
    /// let compiled = CompiledEvaluator::compile(&expr, &["x", "y"], None)
    ///     .expect("Should compile");
    ///
    /// let compiled = CompiledEvaluator::compile(&expr, &[&x, &y], None)
    ///     .expect("Should compile");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `DiffError` if:
    /// - `UnboundVariable`: Symbol not in parameter list and not a known constant
    /// - `UnsupportedFunction`: Unknown function name
    /// - `UnsupportedExpression`: Unevaluated derivatives
    pub fn compile<P: ToParamName>(
        expr: &Expr,
        param_order: &[P],
        context: Option<&Context>,
    ) -> Result<Self, DiffError> {
        let params: Vec<(u64, String)> = param_order
            .iter()
            .map(ToParamName::to_param_id_and_name)
            .collect();
        let (param_ids, param_names): (Vec<u64>, Vec<String>) = params.into_iter().unzip();

        let expanded_expr =
            context.map_or_else(|| expr.clone(), |ctx| expand_user_functions(expr, ctx));

        let mut compiler = Compiler::new(&param_ids);
        compiler.compile_expr(&expanded_expr)?;

        let (instructions, mut constants, mut arg_pool, _max_stack, param_count) =
            compiler.into_parts();

        let (optimized_instructions, max_stack) =
            Self::optimize_instructions(instructions, &mut constants, &mut arg_pool, param_count)?;

        let mut registers_template = vec![0.0; max_stack];
        if !constants.is_empty() {
            registers_template[param_count..(param_count + constants.len())]
                .copy_from_slice(&constants);
        }

        Ok(Self {
            instructions: Box::from(optimized_instructions),
            constants: Box::from(constants),
            arg_pool: Box::from(arg_pool),
            param_names: Box::from(param_names),
            workspace_size: max_stack,
            param_count,
            #[cfg(feature = "parallel")]
            simd_constants_cache: OnceLock::new(),
            registers_template: Box::from(registers_template),
        })
    }

    /// Compile an expression, automatically determining parameter order from variables.
    ///
    /// Variables are sorted alphabetically for consistent ordering.
    ///
    /// # Errors
    ///
    /// Returns `DiffError` if compilation fails.
    pub fn compile_auto(expr: &Expr, context: Option<&Context>) -> Result<Self, DiffError> {
        let vars = expr.variables_ordered();
        let mut param_order: Vec<String> = vars
            .into_iter()
            .filter(|v| {
                let id = symb_interned(v.as_str()).id();
                !is_known_constant_by_id(id)
            })
            .collect();

        param_order.sort();
        Self::compile(expr, &param_order, context)
    }
}
