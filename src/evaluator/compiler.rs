//! Expression compiler for the bytecode evaluator.
//!
//! This module compiles symbolic [`Expr`] expressions into efficient bytecode
//! ([`Instruction`]s) that can be executed by the [`CompiledEvaluator`].
//!
//! # Compilation Process
//!
//! 1. **User function expansion**: Recursively substitute function calls with their bodies
//! 2. **Constant folding**: Evaluate constant subexpressions at compile time
//! 3. **CSE (Common Subexpression Elimination)**: Cache expensive repeated subexpressions
//! 4. **Instruction emission**: Generate bytecode for the expression tree
//! 5. **Post-compilation optimization**: Fuse instruction patterns (e.g., `MulAdd`)
//!
//! # Stack Depth Tracking
//!
//! The compiler tracks stack depth throughout compilation to:
//! - Validate that the expression can be evaluated without stack overflow
//! - Pre-allocate the exact stack size needed for evaluation
//! - Guarantee that unsafe stack operations in the evaluator are safe
//!
//! # Example
//!
//! ```text
//! let mut compiler = Compiler::new(&param_ids, Some(&context));
//! compiler.compile_expr(&expr)?;
//! let (instructions, constants, max_stack, param_count, cache_size) = compiler.into_parts();
//! ```

use super::instruction::Instruction;
use crate::core::error::DiffError;
use crate::core::known_symbols::KS;
use crate::core::poly::Polynomial;
use crate::core::symbol::InternedSymbol;
use crate::core::traits::EPSILON;
use crate::core::unified_context::Context;
use crate::{Expr, ExprKind};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

/// Maximum allowed stack depth to prevent deeply nested expressions from causing issues.
///
/// This limit (1024) is chosen to:
/// - Handle realistic mathematical expressions (rarely exceed 50-100 depth)
/// - Provide a safety buffer for extreme cases
/// - Keep memory usage bounded for stack pre-allocation
pub const MAX_STACK_DEPTH: usize = 1024;

/// Internal compiler state for transforming expressions to bytecode.
///
/// The compiler performs a single pass over the expression tree, emitting
/// bytecode instructions while tracking:
/// - Stack depth (for pre-allocation and safety validation)
/// - CSE cache slots (for repeated subexpression optimization)
/// - Constant pool (for efficient constant storage)
pub struct Compiler<'ctx> {
    /// Emitted bytecode instructions
    instructions: Vec<Instruction>,
    /// Parameter IDs in evaluation order for fast linear search
    param_ids: Vec<u64>,
    /// Current stack depth during compilation
    current_stack: usize,
    /// Maximum stack depth seen during compilation
    max_stack: usize,
    /// Optional context for user function expansion
    function_context: Option<&'ctx Context>,
    /// CSE: Map from expression hash → cache slot index
    cse_cache: HashMap<u64, usize>,
    /// Number of CSE cache slots allocated
    cache_size: usize,
    /// Constant pool for numeric literals
    constants: Vec<f64>,
    /// Map from constant bit pattern → pool index (deduplication)
    const_map: HashMap<u64, u32>,
}

impl<'ctx> Compiler<'ctx> {
    /// Create a new compiler with the given parameter IDs and optional context.
    ///
    /// This is an internal function for creating expression compilers.
    /// External users should use `CompiledEvaluator::compile` instead.
    ///
    /// # Arguments
    ///
    /// * `param_ids` - Symbol IDs for each parameter, in evaluation order
    /// * `context` - Optional context for user function definitions
    pub fn new(param_ids: &[u64], context: Option<&'ctx Context>) -> Self {
        Self {
            instructions: Vec::with_capacity(64),
            param_ids: param_ids.to_vec(),
            current_stack: 0,
            max_stack: 0,
            function_context: context,
            cse_cache: HashMap::new(),
            cache_size: 0,
            constants: Vec::new(),
            const_map: HashMap::new(),
        }
    }

    /// Add a constant to the pool, deduplicating by bit pattern.
    ///
    /// Returns the index into the constant pool. Identical constants
    /// (by bit representation) share the same pool entry.
    /// Internal function for constant pool management during compilation.
    #[inline]
    pub(crate) fn add_const(&mut self, val: f64) -> u32 {
        let bits = val.to_bits();
        match self.const_map.entry(bits) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                // SAFETY: Constants pool size is bounded by expression complexity,
                // which is limited by MAX_STACK_DEPTH. Realistically < 2^16 constants.
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Constants pool size is bounded by expression complexity, realistically < 2^16 constants"
                )]
                let idx = self.constants.len() as u32;
                self.constants.push(val);
                v.insert(idx);
                idx
            }
        }
    }

    /// Track a push operation, validating stack depth.
    ///
    /// Internal function for stack management during compilation.
    ///
    /// # Errors
    ///
    /// Returns `DiffError::StackOverflow` if the stack would exceed `MAX_STACK_DEPTH`.
    #[inline]
    pub(crate) fn push(&mut self) -> Result<(), DiffError> {
        self.current_stack += 1;
        if self.current_stack > MAX_STACK_DEPTH {
            return Err(DiffError::StackOverflow {
                depth: self.current_stack,
                limit: MAX_STACK_DEPTH,
            });
        }
        self.max_stack = self.max_stack.max(self.current_stack);
        Ok(())
    }

    /// Track a pop operation.
    ///
    /// Uses saturating subtraction to handle edge cases gracefully,
    /// though correct bytecode should never underflow.
    #[inline]
    pub const fn pop(&mut self) {
        self.current_stack = self.current_stack.saturating_sub(1);
    }

    /// Emit an instruction to the bytecode stream.
    ///
    /// Internal function for instruction emission during compilation.
    #[inline]
    pub(crate) fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    /// Consume the compiler and return the compiled bytecode and metadata.
    ///
    /// Internal function for extracting compilation results.
    ///
    /// Returns a tuple of:
    /// - `instructions` - The emitted bytecode
    /// - `constants` - The constant pool
    /// - `max_stack` - Maximum stack depth required
    /// - `param_count` - Number of parameters
    /// - `cache_size` - Number of CSE cache slots
    #[inline]
    pub(crate) fn into_parts(self) -> (Vec<Instruction>, Vec<f64>, usize, usize, usize) {
        (
            self.instructions,
            self.constants,
            self.max_stack,
            self.param_ids.len(),
            self.cache_size,
        )
    }

    /// Get a reference to the emitted instructions.
    #[cfg(test)]
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Get the maximum stack depth seen during compilation.
    #[cfg(test)]
    pub const fn max_stack(&self) -> usize {
        self.max_stack
    }

    /// Determine if an expression is expensive enough to cache (CSE).
    ///
    /// This heuristic helps decide between tree-walking and compiled evaluation.
    /// Internal function for performance optimization during compilation.
    ///
    /// Caching has overhead (store/load instructions), so we only cache:
    /// - Function calls (transcendentals, special functions)
    /// - Power operations (often involve expensive `powf`)
    /// - Division (can involve NaN checks)
    /// - Large sums/products (3+ terms to amortize overhead)
    pub(crate) fn is_expensive(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::FunctionCall { .. } | ExprKind::Div(..) => true,
            ExprKind::Pow(base, exp) => {
                // Integer powers (n in range [-16, 16]) are optimized to cheap Powi/Square/etc.
                // We only cache if it's a non-integer power (likely using powf) OR if the base is complex.
                if Self::try_eval_const(exp).is_none_or(|n| (n - n.round()).abs() > EPSILON) {
                    true
                } else {
                    // If base is not a simple number or symbol, it's worth caching the result of the power
                    !matches!(base.kind, ExprKind::Number(_) | ExprKind::Symbol(_))
                }
            }

            // Cache sums/products with 3+ terms or if at least one term is expensive.
            // This avoids caching trivial 2-term operations where cache overhead
            // might exceed re-computation cost.
            ExprKind::Sum(terms) | ExprKind::Product(terms) => {
                terms.len() >= 2 || terms.iter().any(|t| Self::is_expensive(t))
            }

            _ => false,
        }
    }

    /// Try to evaluate a constant expression at compile time.
    ///
    /// Returns `Some(value)` if the expression contains only constants and
    /// known mathematical functions. Returns `None` if the expression
    /// contains variables or unsupported operations.
    ///
    /// This is a key optimization that eliminates runtime computation for
    /// constant subexpressions like `2 * pi` or `sin(0)`.
    /// Internal function for constant folding optimization.
    pub(crate) fn try_eval_const(expr: &Expr) -> Option<f64> {
        match &expr.kind {
            ExprKind::Number(n) => Some(*n),
            ExprKind::Symbol(s) => crate::core::known_symbols::get_constant_value(s.as_str()),
            ExprKind::Sum(terms) => {
                let mut sum = 0.0;
                for term in terms {
                    sum += Self::try_eval_const(term)?;
                }
                Some(sum)
            }
            ExprKind::Product(factors) => {
                let mut product = 1.0;
                for factor in factors {
                    product *= Self::try_eval_const(factor)?;
                }
                Some(product)
            }
            ExprKind::Div(num, den) => {
                let n = Self::try_eval_const(num)?;
                let d = Self::try_eval_const(den)?;
                Some(n / d)
            }
            ExprKind::Pow(base, exp) => {
                let b = Self::try_eval_const(base)?;
                let e = Self::try_eval_const(exp)?;
                Some(b.powf(e))
            }
            ExprKind::FunctionCall { name, args } => {
                // Avoid Vec allocation for common 1-2 arg functions
                match args.len() {
                    1 => {
                        let x = Self::try_eval_const(&args[0])?;
                        let id = name.id();
                        let ks = &*KS;
                        if id == ks.sin {
                            Some(x.sin())
                        } else if id == ks.cos {
                            Some(x.cos())
                        } else if id == ks.tan {
                            Some(x.tan())
                        } else if id == ks.exp {
                            Some(x.exp())
                        } else if id == ks.ln || id == ks.log {
                            Some(x.ln())
                        } else if id == ks.sqrt {
                            Some(x.sqrt())
                        } else if id == ks.abs {
                            Some(x.abs())
                        } else if id == ks.floor {
                            Some(x.floor())
                        } else if id == ks.ceil {
                            Some(x.ceil())
                        } else if id == ks.round {
                            Some(x.round())
                        } else {
                            None
                        }
                    }
                    2 => {
                        let a = Self::try_eval_const(&args[0])?;
                        let b = Self::try_eval_const(&args[1])?;
                        let id = name.id();
                        let ks = &*KS;
                        if id == ks.atan2 {
                            Some(a.atan2(b))
                        } else if id == ks.log {
                            Some(b.log(a))
                        }
                        // log(base, x)
                        else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Compile an expression to bytecode.
    ///
    /// This is the main compilation entry point for expression AST → bytecode.
    /// Internal function - external users should use `CompiledEvaluator::compile`.
    ///
    /// This is the main compilation entry point. It:
    /// 1. Handles trivial cases (numbers, symbols) in a fast path
    /// 2. Checks for CSE cache hits
    /// 3. Attempts constant folding
    /// 4. Recursively compiles subexpressions
    /// 5. Caches expensive subexpressions for CSE
    ///
    /// # Errors
    ///
    /// Returns errors for:
    /// - `UnboundVariable`: Symbol not in parameter list and not a known constant
    /// - `StackOverflow`: Expression too deeply nested
    /// - `UnsupportedFunction`: Unknown function name
    /// - `UnsupportedExpression`: Derivatives (must be simplified first)
    // Compilation handles many expression kinds; length is justified
    #[allow(
        clippy::too_many_lines,
        reason = "Compilation handles many expression kinds; length is justified"
    )]
    pub(crate) fn compile_expr(&mut self, expr: &Expr) -> Result<(), DiffError> {
        // Fast path for trivial expressions - skip CSE and constant folding checks
        match &expr.kind {
            ExprKind::Number(n) => {
                let idx = self.add_const(*n);
                self.emit(Instruction::LoadConst(idx));
                self.push()?;
                return Ok(());
            }
            ExprKind::Symbol(s) => {
                let name = s.as_str();
                let sym_id = s.id();
                // Handle known constants (pi, e, etc.)
                if let Some(value) = crate::core::known_symbols::get_constant_value(name) {
                    let idx = self.add_const(value);
                    self.emit(Instruction::LoadConst(idx));
                    self.push()?;
                } else if let Some(idx) = self.param_ids.iter().position(|&id| id == sym_id) {
                    // Truncation safe: param count bounded by realistic expression size
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "Param count bounded by realistic expression size"
                    )]
                    self.emit(Instruction::LoadParam(idx as u32));
                    self.push()?;
                } else {
                    return Err(DiffError::UnboundVariable(name.to_owned()));
                }
                return Ok(());
            }
            _ => {}
        }

        // CSE: Check if we've already compiled a structurally identical subexpression
        // Track if we should cache after compiling (for single-pass CSE)
        let cache_this = Self::is_expensive(expr);

        // Only check for expensive expressions to avoid HashMap overhead
        if cache_this && let Some(&slot) = self.cse_cache.get(&expr.hash) {
            // Cache hit! Load from cache instead of recompiling
            // Truncation safe: cache slots bounded by expression complexity
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Cache slots bounded by expression complexity"
            )]
            self.emit(Instruction::LoadCached(slot as u32));
            self.push()?;
            return Ok(());
        }

        // Try constant folding for compound expressions
        if let Some(value) = Self::try_eval_const(expr) {
            let idx = self.add_const(value);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            return Ok(());
        }

        match &expr.kind {
            // Already handled above - these are unreachable as an internal invariant
            #[allow(
                clippy::unreachable,
                reason = "Already handled above - these are unreachable as an internal invariant"
            )]
            ExprKind::Number(_) | ExprKind::Symbol(_) => unreachable!(),

            ExprKind::Sum(terms) => {
                self.compile_sum(terms)?;
            }

            ExprKind::Product(factors) => {
                self.compile_product(factors)?;
            }

            ExprKind::Div(num, den) => {
                self.compile_division(num, den)?;
            }

            ExprKind::Pow(base, exp) => {
                self.compile_power(base, exp)?;
            }

            ExprKind::FunctionCall { name, args } => {
                self.compile_function_call(name, args)?;
            }

            ExprKind::Poly(poly) => {
                self.compile_polynomial(poly)?;
            }

            ExprKind::Derivative { .. } => {
                return Err(DiffError::UnsupportedExpression(
                    "Derivatives cannot be numerically evaluated - simplify first".to_owned(),
                ));
            }
        }

        // CSE: If this subexpression should be cached, emit StoreCached and record it
        if cache_this {
            let slot = self.cache_size;
            self.cache_size += 1;
            // Truncation safe: cache slots bounded by expression complexity
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Cache slots bounded by expression complexity"
            )]
            self.emit(Instruction::StoreCached(slot as u32));
            self.cse_cache.insert(expr.hash, slot);
        }

        Ok(())
    }

    /// Compile a sum expression: `a + b + c + ...`
    #[allow(
        clippy::too_many_lines,
        reason = "Complex pattern matching logic for sum optimizations requires detailed implementation"
    )]
    fn compile_sum(&mut self, terms: &[Arc<Expr>]) -> Result<(), DiffError> {
        if terms.is_empty() {
            let idx = self.add_const(0.0);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
        } else {
            // Optimization: Patterns for Expm1 and Sub
            // We look for these even in larger sums to maximize fused ops
            let mut expm1_idx = None;
            let mut neg_one_idx = None;
            let mut sub_idx = None;

            for (i, term) in terms.iter().enumerate() {
                if let ExprKind::FunctionCall { name, args } = &term.kind {
                    if name.id() == KS.exp && args.len() == 1 {
                        expm1_idx = Some(i);
                    }
                } else if let Some(n) = Self::try_eval_const(term) {
                    if (n - -1.0).abs() < EPSILON {
                        neg_one_idx = Some(i);
                    }
                } else if let ExprKind::Product(factors) = &term.kind
                    && factors.len() == 2
                    && let Some(n) = Self::try_eval_const(&factors[0])
                    && (n - -1.0).abs() < EPSILON
                {
                    sub_idx = Some(i);
                }
            }

            // Pattern: exp(x) - 1 -> Expm1(x)
            if let (Some(e_idx), Some(n_idx)) = (expm1_idx, neg_one_idx)
                && let ExprKind::FunctionCall { args, .. } = &terms[e_idx].kind
            {
                self.compile_expr(&args[0])?;
                self.emit(Instruction::Expm1);

                // Compile remainder of the sum
                let mut remainder = Vec::with_capacity(terms.len() - 2);
                for (i, t) in terms.iter().enumerate() {
                    if i != e_idx && i != n_idx {
                        remainder.push(Arc::clone(t));
                    }
                }
                if !remainder.is_empty() {
                    self.compile_sum(&remainder)?;
                    self.emit(Instruction::Add);
                    self.pop();
                }
                return Ok(());
            }

            // Pattern: a - b -> Sub(a, b)
            if let Some(s_idx) = sub_idx {
                // Compile remainder (the "a" part)
                let mut remainder = Vec::with_capacity(terms.len() - 1);
                for (i, t) in terms.iter().enumerate() {
                    if i != s_idx {
                        remainder.push(Arc::clone(t));
                    }
                }

                if !remainder.is_empty() {
                    // Optimization: Detect MulSub (a*b - c)
                    if remainder.len() == 1
                        && let ExprKind::Product(factors) = &remainder[0].kind
                        && factors.len() == 2
                        && let ExprKind::Product(s_factors) = &terms[s_idx].kind
                    {
                        self.compile_expr(&factors[0])?;
                        self.compile_expr(&factors[1])?;
                        self.compile_expr(&s_factors[1])?;
                        self.emit(Instruction::MulSub);
                        self.pop();
                        self.pop();
                        return Ok(());
                    }

                    self.compile_sum(&remainder)?;
                    if let ExprKind::Product(factors) = &terms[s_idx].kind {
                        self.compile_expr(&factors[1])?;
                        self.emit(Instruction::Sub);
                        self.pop();
                        return Ok(());
                    }
                }
            }

            // Optimization: Detect patterns for MulAdd [a, b, c] -> a * b + c
            // We look for any term that is a product.
            let mut best_pattern = None;
            for (i, term) in terms.iter().enumerate() {
                match &term.kind {
                    ExprKind::Product(factors) if factors.len() >= 2 => {
                        best_pattern = Some(i);
                        break;
                    }
                    _ => {}
                }
            }

            if let Some(idx) = best_pattern.filter(|_| terms.len() >= 2) {
                let term = &terms[idx];
                if let ExprKind::Product(factors) = &term.kind {
                    // Check for negative MulAdd: -a*b + c -> NegMulAdd(a, b, c)
                    // Pattern: Product([-1, a, b]) + c
                    if factors.len() == 3
                        && matches!(factors[0].kind, ExprKind::Number(n) if (n - -1.0).abs() < EPSILON)
                    {
                        // -1 * a * b
                        self.compile_expr(&factors[1])?;
                        self.compile_expr(&factors[2])?;

                        // Compile remainder
                        let mut remainder = Vec::with_capacity(terms.len() - 1);
                        for (i, t) in terms.iter().enumerate() {
                            if i != idx {
                                remainder.push(Arc::clone(t));
                            }
                        }
                        self.compile_sum(&remainder)?;

                        // Stack: [a, b, c] -> -a*b + c
                        self.emit(Instruction::NegMulAdd);
                        self.pop();
                        self.pop();
                        return Ok(());
                    }

                    // Standard MulAdd: a * b * c + remainder -> (a * b) * c + remainder
                    if factors.len() == 2 {
                        self.compile_expr(&factors[0])?;
                        self.compile_expr(&factors[1])?;
                    } else {
                        // Group: (factors[0..n-1]) * factors[n-1]
                        let head = Expr::product_from_arcs(factors[0..factors.len() - 1].to_vec());
                        self.compile_expr(&head)?;
                        self.compile_expr(&factors[factors.len() - 1])?;
                    }

                    // Compile remainder
                    let mut remainder = Vec::with_capacity(terms.len() - 1);
                    for (i, t) in terms.iter().enumerate() {
                        if i != idx {
                            remainder.push(Arc::clone(t));
                        }
                    }
                    self.compile_sum(&remainder)?;

                    // Stack: [a, b, remainder_sum]
                    self.emit(Instruction::MulAdd);
                    self.pop();
                    self.pop();
                    return Ok(());
                }
            }

            // Fallback to iterative addition
            self.compile_expr(&terms[0])?;
            for term in &terms[1..] {
                self.compile_expr(term)?;
                self.emit(Instruction::Add);
                self.pop();
            }
        }
        Ok(())
    }

    /// Compile a product expression: `a * b * c * ...`
    fn compile_product(&mut self, factors: &[Arc<Expr>]) -> Result<(), DiffError> {
        if factors.is_empty() {
            let idx = self.add_const(1.0);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            return Ok(());
        }

        // Constant folding: Accumulate all constant factors
        let mut constant_acc = 1.0;
        let mut variable_factors = Vec::with_capacity(factors.len());

        for factor in factors {
            if let Some(c) = Self::try_eval_const(factor) {
                constant_acc *= c;
            } else {
                variable_factors.push(Arc::clone(factor));
            }
        }

        // Group identical variable factors to use Square/Cube/Pow4
        let mut grouped: Vec<(Arc<Expr>, usize)> = Vec::with_capacity(variable_factors.len());
        for factor in &variable_factors {
            if let Some(existing) = grouped.iter_mut().find(|(e, _)| e == factor) {
                existing.1 += 1;
            } else {
                grouped.push((Arc::clone(factor), 1));
            }
        }

        let mut first = true;
        let mut negate_at_end = false;

        // Compile variable factors first
        for (expr, count) in grouped {
            self.compile_expr_with_count(&expr, count)?;

            if !first {
                self.emit(Instruction::Mul);
                self.pop();
            }
            first = false;
        }

        // Handle the constant part at the end (enables MulConst fusion)
        if (constant_acc - 1.0).abs() > EPSILON {
            if (constant_acc - -1.0).abs() < EPSILON {
                // Defer negation to the end (saving a LoadConst + Mul)
                negate_at_end = true;
            } else {
                let idx = self.add_const(constant_acc);
                if first {
                    self.emit(Instruction::LoadConst(idx));
                    self.push()?;
                } else {
                    self.emit(Instruction::MulConst(idx));
                }
                first = false;
            }
        }

        // If we haven't emitted anything yet (e.g. factors were empty or just 1.0),
        // we must emit something.
        if first {
            let idx = self.add_const(1.0);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
        }

        if negate_at_end {
            self.emit(Instruction::Neg);
        }

        Ok(())
    }

    /// Helper to compile an expression effectively raised to a small integer power.
    fn compile_expr_with_count(&mut self, expr: &Expr, count: usize) -> Result<(), DiffError> {
        match count {
            1 => self.compile_expr(expr),
            2 => {
                self.compile_expr(expr)?;
                self.emit(Instruction::Square);
                Ok(())
            }
            3 => {
                self.compile_expr(expr)?;
                self.emit(Instruction::Cube);
                Ok(())
            }
            4 => {
                self.compile_expr(expr)?;
                self.emit(Instruction::Pow4);
                Ok(())
            }
            _ => {
                self.compile_expr(expr)?;
                // Truncation safe: we only group up to realistic powers
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "We only group up to realistic powers"
                )]
                self.emit(Instruction::Powi(count as i32));
                Ok(())
            }
        }
    }

    /// Compile a division expression with removable singularity detection.
    ///
    /// Detects patterns like:
    /// - `E/E → 1` (handles `x/x`, `sin(x)/sin(x)`, etc.)
    /// - `sin(E)/E → sinc(E)` (handles removable singularity at 0)
    /// - `E/C → E * (1/C)` (where C is a non-zero constant)
    fn compile_division(&mut self, num: &Expr, den: &Expr) -> Result<(), DiffError> {
        // Pattern 1: E/E → 1 (handles x/x, sin(x)/sin(x)`, etc.)
        if num == den {
            let idx = self.add_const(1.0);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            return Ok(());
        }

        // Pattern 2: sin(E)/E → sinc(E) (already handles E=0 → 1)
        // args[0] is Arc<Expr>, *args[0] derefs to Expr, compare with *den
        match &num.kind {
            ExprKind::FunctionCall { name, args }
                if name.id() == KS.sin && args.len() == 1 && args[0].as_ref() == den =>
            {
                self.compile_expr(den)?;
                self.emit(Instruction::Sinc);
                return Ok(());
            }
            _ => {}
        }

        // Optimization: 1 / (exp(x) - 1) -> RecipExpm1(x)
        if let ExprKind::Number(n) = num.kind
            && (n - 1.0).abs() < EPSILON
        {
            // Check if den is exp(x) - 1 or sum with exp(x) and -1
            let mut expm1_arg = None;

            if let ExprKind::Sum(terms) = &den.kind {
                // Look for exp(x) and -1.0
                let mut has_neg_one = false;
                let mut exp_arg = None;

                // Usually sum has 2 terms: exp(x) and -1
                if terms.len() == 2 {
                    for term in terms {
                        if let ExprKind::Number(term_n) = term.kind {
                            if (term_n - -1.0).abs() < EPSILON {
                                has_neg_one = true;
                            }
                        } else if let ExprKind::FunctionCall { name, args } = &term.kind
                            && name.id() == KS.exp
                            && args.len() == 1
                        {
                            exp_arg = Some(&args[0]);
                        }
                    }
                }

                if has_neg_one && exp_arg.is_some() {
                    expm1_arg = exp_arg;
                }
            }

            if let Some(arg) = expm1_arg {
                self.compile_expr(arg)?;
                self.emit(Instruction::RecipExpm1);
                return Ok(());
            }
        }

        // Optimization: num / exp(x) -> num * exp(-x)
        // Only apply if num is constant to avoid duplicating exp computation
        // (e.g. if num contains exp(x), we'd compute exp(x) and exp(-x))
        let mut exp_neg_arg = None;
        if Self::try_eval_const(num).is_some() {
            if let ExprKind::FunctionCall { name, args } = &den.kind
                && name.id() == KS.exp
                && args.len() == 1
            {
                exp_neg_arg = Some(&args[0]);
            } else if let ExprKind::Pow(base, exp) = &den.kind
                && let Some(val) = Self::try_eval_const(base)
                && (val - std::f64::consts::E).abs() < EPSILON
            {
                exp_neg_arg = Some(exp);
            }
        }

        if let Some(arg) = exp_neg_arg {
            self.compile_expr(num)?;
            self.compile_expr(arg)?;
            self.emit(Instruction::ExpNeg);
            self.emit(Instruction::Mul);
            self.pop(); // Mul pops 1 more than it pushes relative to stack growth of 2
            return Ok(());
        }

        // Optimization: E / C -> E * (1/C)
        // Multiplication is generally faster than division on most CPUs
        if let Some(val) = Self::try_eval_const(den).filter(|&v| v != 0.0) {
            self.compile_expr(num)?;
            let idx = self.add_const(1.0 / val);
            self.emit(Instruction::MulConst(idx));
            return Ok(());
        }

        // No pattern matched - compile normal division
        self.compile_expr(num)?;
        self.compile_expr(den)?;
        self.emit(Instruction::Div);
        self.pop();
        Ok(())
    }

    /// Compile a power expression with fused instruction optimization.
    ///
    /// Detects special cases for efficient evaluation:
    /// - `x^0 → 1` (constant fold)
    /// - `x^1 → x` (identity)
    /// - `x^2 → Square`
    /// - `x^3 → Cube`
    /// - `x^4 → Pow4`
    /// - `x^-1 → Recip`
    /// - `x^-2 → Square; Recip`
    /// - `x^n` for small n → `Powi(n)`
    /// - `x^0.5 → Sqrt`
    /// - `x^-0.5 → Sqrt; Recip`
    ///
    /// TODO: Root2k(k) for successive hardware sqrts (e.g. x^1/4, x^1/8).
    #[allow(
        clippy::too_many_lines,
        reason = "Handles many specific power optimization cases"
    )]
    fn compile_power(&mut self, base: &Expr, exp: &Expr) -> Result<(), DiffError> {
        // Optimization: e^x -> Exp(x)
        if let Some(base_val) = Self::try_eval_const(base)
            && (base_val - std::f64::consts::E).abs() < EPSILON
        {
            // Check for e^(-x) -> ExpNeg(x)
            // -x is represented as Product([-1, x])
            if let ExprKind::Product(factors) = &exp.kind
                && factors.len() == 2
                && let Some(c) = Self::try_eval_const(&factors[0])
                && (c - -1.0).abs() < EPSILON
            {
                self.compile_expr(&factors[1])?;
                self.emit(Instruction::ExpNeg);
                return Ok(());
            }

            self.compile_expr(exp)?;
            self.emit(Instruction::Exp);
            return Ok(());
        }

        // Check for fused instruction patterns with integer/half exponents
        // Optimization: use try_eval_const to catch rational exponents like 3/2
        if let Some(n_val) = Self::try_eval_const(exp) {
            let n_rounded = n_val.round();
            let is_integer = (n_val - n_rounded).abs() < EPSILON;

            if is_integer {
                // Truncation safe: checked by is_integer and i32 bounds below
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Checked by is_integer and i32 bounds below"
                )]
                let n_int = n_rounded as i64;
                match n_int {
                    0 => {
                        // x^0 = 1 (constant fold)
                        let idx = self.add_const(1.0);
                        self.emit(Instruction::LoadConst(idx));
                        self.push()?;
                        return Ok(());
                    }
                    1 => {
                        // x^1 = x (just compile base)
                        self.compile_expr(base)?;
                        return Ok(());
                    }
                    2 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::Square);
                        return Ok(());
                    }
                    3 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::Cube);
                        return Ok(());
                    }
                    4 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::Pow4);
                        return Ok(());
                    }
                    8 => {
                        self.compile_expr(base)?;
                        // x^8 = ((x^2)^2)^2
                        self.emit(Instruction::Square);
                        self.emit(Instruction::Square);
                        self.emit(Instruction::Square);
                        return Ok(());
                    }
                    16 => {
                        self.compile_expr(base)?;
                        // x^16 = (((x^2)^2)^2)^2
                        self.emit(Instruction::Square);
                        self.emit(Instruction::Square);
                        self.emit(Instruction::Square);
                        self.emit(Instruction::Square);
                        return Ok(());
                    }
                    -1 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::Recip);
                        return Ok(());
                    }
                    -2 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::InvSquare);
                        return Ok(());
                    }
                    -3 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::InvCube);
                        return Ok(());
                    }
                    _ => {
                        // Use powi for all other integers within i32 range
                        if let Ok(n_i32) = i32::try_from(n_int) {
                            self.compile_expr(base)?;
                            self.emit(Instruction::Powi(n_i32));
                            return Ok(());
                        }
                    }
                }
            } else if (n_val - 0.5).abs() < EPSILON {
                // x^0.5 → Sqrt
                self.compile_expr(base)?;
                self.emit(Instruction::Sqrt);
                return Ok(());
            } else if (n_val + 0.5).abs() < EPSILON {
                // x^-0.5 → InvSqrt
                self.compile_expr(base)?;
                self.emit(Instruction::InvSqrt);
                return Ok(());
            } else if (n_val - 1.5).abs() < EPSILON {
                // x^1.5 → Pow3_2
                self.compile_expr(base)?;
                self.emit(Instruction::Pow3_2);
                return Ok(());
            } else if (n_val + 1.5).abs() < EPSILON {
                // x^-1.5 → InvPow3_2
                self.compile_expr(base)?;
                self.emit(Instruction::InvPow3_2);
                return Ok(());
            } else if (n_val - (1.0 / 3.0)).abs() < EPSILON {
                // x^(1/3) → Cbrt
                self.compile_expr(base)?;
                self.emit(Instruction::Cbrt);
                return Ok(());
            }
        }

        // General case: use Pow instruction
        self.compile_expr(base)?;
        self.compile_expr(exp)?;
        self.emit(Instruction::Pow);
        self.pop();
        Ok(())
    }

    /// Compile a function call expression.
    ///
    /// Maps function names to corresponding bytecode instructions.
    /// Handles arity for multi-argument functions by popping extra operands.
    fn compile_function_call(
        &mut self,
        func_name: &InternedSymbol,
        args: &[Arc<Expr>],
    ) -> Result<(), DiffError> {
        let id = func_name.id();
        let arity = args.len();
        let ks = &*KS;

        // Optimizations: Log1p, ExpNeg, ExpSqr, ExpSqrNeg
        // These MUST run before we compile arguments to avoid leaking stack
        if arity == 1 {
            // ln(1 + x) -> Log1p(x)
            if id == ks.ln
                && let ExprKind::Sum(terms) = &args[0].kind
                && terms.len() == 2
            {
                let (t0, t1) = (&terms[0], &terms[1]);
                // Check if t0 is 1.0
                if let Some(n) = Self::try_eval_const(t0)
                    && (n - 1.0).abs() < EPSILON
                {
                    self.compile_expr(t1)?;
                    self.emit(Instruction::Log1p);
                    return Ok(());
                }
                // Check if t1 is 1.0
                if let Some(n) = Self::try_eval_const(t1)
                    && (n - 1.0).abs() < EPSILON
                {
                    self.compile_expr(t0)?;
                    self.emit(Instruction::Log1p);
                    return Ok(());
                }
            }

            // exp optimizations
            if id == ks.exp {
                // Check for -x, -x^2, or x^2
                if let ExprKind::Product(factors) = &args[0].kind {
                    let neg_idx = factors.iter().position(|f| {
                        Self::try_eval_const(f).is_some_and(|n| (n + 1.0).abs() < EPSILON)
                    });

                    if let Some(idx) = neg_idx {
                        // It's exp(- (...))
                        // If it's exp(-x^2), use ExpSqrNeg
                        if factors.len() == 2 {
                            let other = if idx == 0 { &factors[1] } else { &factors[0] };
                            if let ExprKind::Pow(base, exp) = &other.kind
                                && Self::try_eval_const(exp) == Some(2.0)
                            {
                                self.compile_expr(base)?;
                                self.emit(Instruction::ExpSqrNeg);
                                return Ok(());
                            }
                        }

                        // General exp(-product)
                        let mut remainder = factors.clone();
                        remainder.remove(idx);
                        if remainder.len() == 1 {
                            self.compile_expr(&remainder[0])?;
                        } else {
                            self.compile_product(&remainder)?;
                        }
                        self.emit(Instruction::ExpNeg);
                        return Ok(());
                    }
                } else if let ExprKind::Div(num, den) = &args[0].kind {
                    // Check if numerator is negated
                    if let ExprKind::Product(factors) = &num.kind {
                        let neg_idx = factors.iter().position(|f| {
                            Self::try_eval_const(f).is_some_and(|n| (n + 1.0).abs() < EPSILON)
                        });

                        if let Some(idx) = neg_idx {
                            // exp(-(num_rest / den))
                            let mut remainder = factors.clone();
                            remainder.remove(idx);
                            if remainder.len() == 1 {
                                self.compile_expr(&remainder[0])?;
                            } else {
                                self.compile_product(&remainder)?;
                            }
                            self.compile_expr(den)?;
                            self.emit(Instruction::Div);
                            self.pop();
                            self.emit(Instruction::ExpNeg);
                            return Ok(());
                        }
                    }
                } else if let ExprKind::Pow(base, exp) = &args[0].kind
                    && Self::try_eval_const(exp) == Some(2.0)
                {
                    // exp(x^2) -> ExpSqr(x)
                    self.compile_expr(base)?;
                    self.emit(Instruction::ExpSqr);
                    return Ok(());
                }
            }
        }

        // Compile arguments first (in order for proper stack layout)
        for arg in args {
            self.compile_expr(arg)?;
        }

        match arity {
            1 => self.compile_unary_function(func_name),
            2 => self.compile_binary_function(func_name),
            3 => self.compile_ternary_function(func_name),
            4 => self.compile_quaternary_function(func_name),
            _ => self.compile_unknown_function(func_name, arity),
        }
    }

    /// Compile a unary function call (1 argument).
    #[allow(
        clippy::too_many_lines,
        reason = "Dispatch table mapping 50+ function names to instructions"
    )]
    fn compile_unary_function(&mut self, func_name: &InternedSymbol) -> Result<(), DiffError> {
        let id = func_name.id();
        let ks = &*KS;

        let instr = if id == ks.sin {
            Instruction::Sin
        } else if id == ks.cos {
            Instruction::Cos
        } else if id == ks.tan {
            Instruction::Tan
        } else if id == ks.asin {
            Instruction::Asin
        } else if id == ks.acos {
            Instruction::Acos
        } else if id == ks.atan {
            Instruction::Atan
        } else if id == ks.cot {
            // cot(x) = 1/tan(x)
            self.emit(Instruction::Tan);
            Instruction::Recip
        } else if id == ks.sec {
            // sec(x) = 1/cos(x)
            self.emit(Instruction::Cos);
            Instruction::Recip
        } else if id == ks.csc {
            // csc(x) = 1/sin(x)
            self.emit(Instruction::Sin);
            Instruction::Recip
        } else if id == ks.acot {
            // acot(x) = atan(1/x)
            self.emit(Instruction::Recip);
            Instruction::Atan
        } else if id == ks.asec {
            // asec(x) = acos(1/x)
            self.emit(Instruction::Recip);
            Instruction::Acos
        } else if id == ks.acsc {
            // acsc(x) = asin(1/x)
            self.emit(Instruction::Recip);
            Instruction::Asin
        }
        // Hyperbolic
        else if id == ks.sinh {
            Instruction::Sinh
        } else if id == ks.cosh {
            Instruction::Cosh
        } else if id == ks.tanh {
            Instruction::Tanh
        } else if id == ks.asinh {
            Instruction::Asinh
        } else if id == ks.acosh {
            Instruction::Acosh
        } else if id == ks.atanh {
            Instruction::Atanh
        } else if id == ks.coth {
            // coth(x) = 1/tanh(x)
            self.emit(Instruction::Tanh);
            Instruction::Recip
        } else if id == ks.sech {
            // sech(x) = 1/cosh(x)
            self.emit(Instruction::Cosh);
            Instruction::Recip
        } else if id == ks.csch {
            // csch(x) = 1/sinh(x)
            self.emit(Instruction::Sinh);
            Instruction::Recip
        } else if id == ks.acoth {
            // acoth(x) = atanh(1/x)
            self.emit(Instruction::Recip);
            Instruction::Atanh
        } else if id == ks.asech {
            // asech(x) = acosh(1/x)
            self.emit(Instruction::Recip);
            Instruction::Acosh
        } else if id == ks.acsch {
            // acsch(x) = asinh(1/x)
            self.emit(Instruction::Recip);
            Instruction::Asinh
        }
        // Exponential
        else if id == ks.exp {
            Instruction::Exp
        } else if id == ks.ln {
            Instruction::Ln
        } else if id == ks.log10 {
            // log10(x) = ln(x) * (1/ln(10))
            self.emit(Instruction::Ln);
            let idx = self.add_const(std::f64::consts::LOG10_E); // 1/ln(10)
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            self.emit(Instruction::Mul);
            self.pop();
            return Ok(());
        } else if id == ks.log2 {
            // log2(x) = ln(x) * (1/ln(2))
            self.emit(Instruction::Ln);
            let idx = self.add_const(std::f64::consts::LOG2_E); // 1/ln(2)
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            self.emit(Instruction::Mul);
            self.pop();
            return Ok(());
        } else if id == ks.sqrt {
            Instruction::Sqrt
        } else if id == ks.cbrt {
            Instruction::Cbrt
        }
        // Special
        else if id == ks.abs {
            Instruction::Abs
        } else if id == ks.signum || id == ks.sign || id == ks.sgn {
            Instruction::Signum
        } else if id == ks.floor {
            Instruction::Floor
        } else if id == ks.ceil {
            Instruction::Ceil
        } else if id == ks.round {
            Instruction::Round
        } else if id == ks.erf {
            Instruction::Erf
        } else if id == ks.erfc {
            Instruction::Erfc
        } else if id == ks.gamma {
            Instruction::Gamma
        } else if id == ks.digamma {
            Instruction::Digamma
        } else if id == ks.trigamma {
            Instruction::Trigamma
        } else if id == ks.tetragamma {
            Instruction::Tetragamma
        } else if id == ks.sinc {
            Instruction::Sinc
        } else if id == ks.lambertw {
            Instruction::LambertW
        } else if id == ks.elliptic_k {
            Instruction::EllipticK
        } else if id == ks.elliptic_e {
            Instruction::EllipticE
        } else if id == ks.zeta {
            Instruction::Zeta
        } else if id == ks.exp_polar {
            Instruction::ExpPolar
        } else {
            return self.compile_unknown_function(func_name, 1);
        };

        self.emit(instr);
        Ok(())
    }

    /// Compile a binary function call (2 arguments).
    fn compile_binary_function(&mut self, func_name: &InternedSymbol) -> Result<(), DiffError> {
        let id = func_name.id();
        let ks = &*KS;

        let instr = if id == ks.log {
            self.pop();
            Instruction::Log
        } else if id == ks.atan2 {
            self.pop();
            Instruction::Atan2
        } else if id == ks.besselj {
            self.pop();
            Instruction::BesselJ
        } else if id == ks.bessely {
            self.pop();
            Instruction::BesselY
        } else if id == ks.besseli {
            self.pop();
            Instruction::BesselI
        } else if id == ks.besselk {
            self.pop();
            Instruction::BesselK
        } else if id == ks.polygamma {
            self.pop();
            Instruction::Polygamma
        } else if id == ks.beta {
            self.pop();
            Instruction::Beta
        } else if id == ks.zeta_deriv {
            self.pop();
            Instruction::ZetaDeriv
        } else if id == ks.hermite {
            self.pop();
            Instruction::Hermite
        } else {
            return self.compile_unknown_function(func_name, 2);
        };

        self.emit(instr);
        Ok(())
    }

    /// Compile a ternary function call (3 arguments).
    fn compile_ternary_function(&mut self, func_name: &InternedSymbol) -> Result<(), DiffError> {
        let id = func_name.id();
        let ks = &*KS;

        if id == ks.assoc_legendre {
            self.pop();
            self.pop();
            self.emit(Instruction::AssocLegendre);
            Ok(())
        } else {
            self.compile_unknown_function(func_name, 3)
        }
    }

    /// Compile a quaternary function call (4 arguments).
    fn compile_quaternary_function(&mut self, func_name: &InternedSymbol) -> Result<(), DiffError> {
        let id = func_name.id();
        let ks = &*KS;

        if id == ks.spherical_harmonic || id == ks.ynm {
            self.pop();
            self.pop();
            self.pop();
            self.emit(Instruction::SphericalHarmonic);
            Ok(())
        } else {
            self.compile_unknown_function(func_name, 4)
        }
    }

    /// Compiles an unknown function by checking if it exists in the user-defined function context.
    fn compile_unknown_function(
        &self,
        func_name: &InternedSymbol,
        arity: usize,
    ) -> Result<(), DiffError> {
        let name_str = func_name.as_str();
        // Check if function exists in the context
        if let Some(ctx) = self.function_context {
            let id = func_name.id();

            if let Some(user_fn) = ctx.get_user_fn_by_id(id) {
                if user_fn.arity.contains(&arity) {
                    // User function exists but has no symbolic body,
                    // so we can't evaluate it numerically.
                    return Err(DiffError::UnsupportedFunction(format!(
                        "{name_str}: user function has no body for numeric evaluation. \
                         Define a body with `with_function(.., body: Some(expr))`"
                    )));
                }
                return Err(DiffError::UnsupportedFunction(format!(
                    "{}: invalid arity (expected {:?}, got {})",
                    name_str, user_fn.arity, arity
                )));
            }
        }
        Err(DiffError::UnsupportedFunction(name_str.to_owned()))
    }

    /// Compile a polynomial using Horner's method for efficiency.
    ///
    /// Horner's method: `P(x) = a_n*x^n + ... + a_0`
    /// becomes `((a_n*x + a_{n-1})*x + ...)*x + a_0`
    ///
    /// This reduces n multiplications + n-1 additions instead of
    /// n powers + n multiplications + n-1 additions.
    fn compile_polynomial(&mut self, poly: &Polynomial) -> Result<(), DiffError> {
        let terms = poly.terms();
        if terms.is_empty() {
            let idx = self.add_const(0.0);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            return Ok(());
        }

        // Sort terms by power descending for Horner's method
        let mut sorted_terms: Vec<_> = terms.to_vec();
        sorted_terms.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        let max_pow = sorted_terms[0].0;

        // Optimization: Use PolyEval instruction for standard polynomials
        // This reduces instruction count from O(N) to O(1) and uses dense cache-friendly arrays
        if max_pow <= 64 {
            let degree = max_pow;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Constant pool size limited by memory"
            )]
            let start_idx = self.constants.len() as u32;

            // Store degree and coefficients in constant pool
            self.constants.push(f64::from(degree));

            let mut term_idx = 0;

            for current_pow in (0..=degree).rev() {
                let coeff =
                    if term_idx < sorted_terms.len() && sorted_terms[term_idx].0 == current_pow {
                        let c = sorted_terms[term_idx].1;
                        term_idx += 1;
                        c
                    } else {
                        0.0
                    };
                self.constants.push(coeff);
            }

            self.compile_expr(poly.base())?;
            self.emit(Instruction::PolyEval(start_idx));
            return Ok(());
        }

        // For simple polynomial with Symbol base, use Horner's method
        let base_param_idx = if let ExprKind::Symbol(s) = &poly.base().kind {
            let name = s.as_str();
            let sym_id = s.id();
            match name {
                _ if crate::core::known_symbols::is_known_constant(name) => None,
                _ => self.param_ids.iter().position(|&id| id == sym_id),
            }
        } else {
            None
        };

        if let Some(idx) = base_param_idx {
            // Fast path: base is a simple parameter, use Horner's method
            let initial_coeff_idx = self.add_const(sorted_terms[0].1);
            self.emit(Instruction::LoadConst(initial_coeff_idx));
            self.push()?;

            let mut term_iter = sorted_terms.iter().skip(1).peekable();

            for pow in (0..max_pow).rev() {
                // Optimized multiply-add using Horner's method:
                // current = current * x + coeff

                // Push x
                // Truncation safe: param count bounded by realistic expression size
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Param count bounded by realistic expression size"
                )]
                self.emit(Instruction::LoadParam(idx as u32));
                self.push()?;

                if let Some((_, coeff)) = term_iter.peek().filter(|t| t.0 == pow) {
                    // We have a coefficient for this power: use MulAdd
                    let const_val = *coeff;
                    let coeff_idx = self.add_const(const_val);
                    self.emit(Instruction::LoadConst(coeff_idx));
                    self.push()?;
                    self.emit(Instruction::MulAdd);
                    self.pop();
                    self.pop();
                    term_iter.next();
                } else {
                    // No coefficient: just Multiply
                    self.emit(Instruction::Mul);
                    self.pop();
                }
            }
        } else {
            // Slow path: complex base expression - expand explicitly
            self.compile_polynomial_slow_path(poly, &sorted_terms)?;
        }

        Ok(())
    }

    /// Compile a polynomial with a complex base expression.
    ///
    /// This path compiles the base once, caches its instructions,
    /// and replays them for each term to avoid recompilation overhead.
    fn compile_polynomial_slow_path(
        &mut self,
        poly: &Polynomial,
        sorted_terms: &[(u32, f64)],
    ) -> Result<(), DiffError> {
        let base = poly.base();

        // 1. Compile base once to learn instructions and stack usage
        let base_start_stack = self.current_stack;
        let base_start_instruction = self.instructions.len();

        self.compile_expr(base)?;

        let base_end_instruction = self.instructions.len();
        let base_headroom = self.max_stack.saturating_sub(base_start_stack);

        let base_instrs: Vec<Instruction> =
            self.instructions[base_start_instruction..base_end_instruction].to_vec();

        // 2. Reset state to before base compilation
        self.instructions.truncate(base_start_instruction);
        self.current_stack = base_start_stack;

        // 3. Emit polynomial expansion using the cached instructions
        let (first_pow, first_coeff) = sorted_terms[0];
        let first_coeff_idx = self.add_const(first_coeff);
        self.emit(Instruction::LoadConst(first_coeff_idx));
        self.push()?;

        // Replaying base: ensure we have enough stack space
        self.max_stack = self.max_stack.max(self.current_stack + base_headroom);

        for instr in &base_instrs {
            self.emit(*instr);
        }
        self.push()?;

        let first_pow_idx = self.add_const(f64::from(first_pow));
        self.emit(Instruction::LoadConst(first_pow_idx));
        self.push()?;
        self.emit(Instruction::Pow);
        self.pop();
        self.emit(Instruction::Mul);
        self.pop();

        // Remaining terms
        for &(pow, coeff) in &sorted_terms[1..] {
            let term_coeff_idx = self.add_const(coeff);
            self.emit(Instruction::LoadConst(term_coeff_idx));
            self.push()?;

            // Replay base again
            self.max_stack = self.max_stack.max(self.current_stack + base_headroom);
            for instr in &base_instrs {
                self.emit(*instr);
            }
            self.push()?;

            let term_pow_idx = self.add_const(f64::from(pow));
            self.emit(Instruction::LoadConst(term_pow_idx));
            self.push()?;
            self.emit(Instruction::Pow);
            self.pop();
            self.emit(Instruction::Mul);
            self.pop();
            self.emit(Instruction::Add);
            self.pop();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::many_single_char_names, reason = "Standard test relaxations")]
    use super::*;
    use crate::parser;
    use std::collections::HashSet;

    fn parse_expr(s: &str) -> Expr {
        parser::parse(s, &HashSet::new(), &HashSet::new(), None).expect("Should pass")
    }

    #[test]
    fn test_constant_folding() {
        // 2 * pi should fold to a constant
        let expr = parse_expr("2 * pi");
        let result = Compiler::try_eval_const(&expr);
        assert!(result.is_some());
        let expected = 2.0 * std::f64::consts::PI;
        assert!((result.expect("constant folding should succeed") - expected).abs() < EPSILON);
    }

    #[test]
    fn test_compile_muladd() {
        let expr = parse_expr("x * y + z");
        let x_sym = crate::symb("x");
        let y_sym = crate::symb("y");
        let z_sym = crate::symb("z");
        let param_ids = vec![x_sym.id(), y_sym.id(), z_sym.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Should find MulAdd instruction
        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::MulAdd))
        );
        assert_eq!(compiler.max_stack(), 3);
    }

    #[test]
    fn test_compile_muladd_recursive() {
        // (a * b) + (c * d) + e
        let expr = parse_expr("a * b + c * d + e");
        let a = crate::symb("a");
        let b = crate::symb("b");
        let c = crate::symb("c");
        let d = crate::symb("d");
        let e = crate::symb("e");
        let param_ids = vec![a.id(), b.id(), c.id(), d.id(), e.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Should have 2 MulAdd instructions
        let muladd_count = compiler
            .instructions()
            .iter()
            .filter(|i| matches!(i, Instruction::MulAdd))
            .count();
        assert_eq!(muladd_count, 2);
    }

    #[test]
    fn test_compile_polynomial_muladd() {
        // 2*x^2 + 3*x + 1
        let expr = parse_expr("2*x^2 + 3*x + 1");
        let x = crate::symb("x");
        let param_ids = vec![x.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Horner's method should use MulAdd
        // (2 * x + 3) * x + 1
        // -> MulAdd(2, x, 3), then MulAdd(prev, x, 1)
        // OR it uses the new PolyEval instruction if optimized
        let muladd_count = compiler
            .instructions()
            .iter()
            .filter(|i| matches!(i, Instruction::MulAdd))
            .count();
        let polyeval_count = compiler
            .instructions()
            .iter()
            .filter(|i| matches!(i, Instruction::PolyEval(_)))
            .count();

        assert!(muladd_count >= 1 || polyeval_count >= 1);
    }

    #[test]
    fn test_compile_simple() {
        let expr = parse_expr("x + 1");
        let x_sym = crate::symb("x");
        let param_ids = vec![x_sym.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Should have: LoadParam(0), LoadConst(0), Add
        assert!(compiler.instructions().len() >= 3);
        assert_eq!(compiler.max_stack(), 2); // Two values on stack at once
    }

    #[test]
    fn test_compile_muladd_extended() {
        // a * b * c + d should use MulAdd(a*b, c, d)
        let expr = parse_expr("a * b * c + d");
        let a = crate::symb("a");
        let b = crate::symb("b");
        let c = crate::symb("c");
        let d = crate::symb("d");
        let param_ids = vec![a.id(), b.id(), c.id(), d.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::MulAdd))
        );
    }

    #[test]
    fn test_compile_product_dedup() {
        // x * x should use Square
        let expr = parse_expr("x * x");
        let x = crate::symb("x");
        let param_ids = vec![x.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Square))
        );

        // x * x * x should use Cube
        let expr3 = parse_expr("x * x * x");
        let mut compiler3 = Compiler::new(&param_ids, None);
        compiler3.compile_expr(&expr3).expect("Should compile");
        assert!(
            compiler3
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Cube))
        );
    }

    #[test]
    fn test_compile_sum_exp_m1() {
        // exp(x) - 1 -> Expm1(x)
        let expr = parse_expr("exp(x) - 1");
        let x = crate::symb("x");
        let param_ids = vec![x.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Should find Expm1 instruction
        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Expm1))
        );
    }

    #[test]
    fn test_compile_sum_sub() {
        // a - b -> Sub(a, b)
        // Check for Sum([a, -b]) where -b is Product([-1, b])
        let expr = parse_expr("a - b");
        let a = crate::symb("a");
        let b = crate::symb("b");
        let param_ids = vec![a.id(), b.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Should find Sub instruction
        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Sub))
        );
    }
}
