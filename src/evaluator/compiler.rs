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
    #[inline]
    pub fn add_const(&mut self, val: f64) -> u32 {
        let bits = val.to_bits();
        match self.const_map.entry(bits) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                // SAFETY: Constants pool size is bounded by expression complexity,
                // which is limited by MAX_STACK_DEPTH. Realistically < 2^16 constants.
                #[allow(clippy::cast_possible_truncation)]
                let idx = self.constants.len() as u32;
                self.constants.push(val);
                v.insert(idx);
                idx
            }
        }
    }

    /// Track a push operation, validating stack depth.
    ///
    /// # Errors
    ///
    /// Returns `DiffError::StackOverflow` if the stack would exceed `MAX_STACK_DEPTH`.
    #[inline]
    pub fn push(&mut self) -> Result<(), DiffError> {
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
    #[inline]
    pub fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    /// Consume the compiler and return the compiled bytecode and metadata.
    ///
    /// Returns a tuple of:
    /// - `instructions` - The emitted bytecode
    /// - `constants` - The constant pool
    /// - `max_stack` - Maximum stack depth required
    /// - `param_count` - Number of parameters
    /// - `cache_size` - Number of CSE cache slots
    #[inline]
    pub fn into_parts(self) -> (Vec<Instruction>, Vec<f64>, usize, usize, usize) {
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
    /// Caching has overhead (store/load instructions), so we only cache:
    /// - Function calls (transcendentals, special functions)
    /// - Power operations (often involve expensive `powf`)
    /// - Division (can involve NaN checks)
    /// - Large sums/products (3+ terms to amortize overhead)
    pub fn is_expensive(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::FunctionCall { .. } | ExprKind::Div(..) => true,
            ExprKind::Pow(_, exp) => {
                // Integer powers (n in range [-16, 16]) are optimized to cheap Powi/Square/etc.
                // We only cache if it's a non-integer power (likely using powf).
                Self::try_eval_const(exp).is_none_or(|n| (n - n.round()).abs() > 1e-10)
            }

            // Cache larger sums/products (4+ terms to amortize Store/Load overhead)
            ExprKind::Sum(terms) | ExprKind::Product(terms) => terms.len() >= 4,

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
    pub fn try_eval_const(expr: &Expr) -> Option<f64> {
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
    #[allow(clippy::too_many_lines)]
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), DiffError> {
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
                    #[allow(clippy::cast_possible_truncation)]
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
            #[allow(clippy::cast_possible_truncation)]
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
            #[allow(clippy::unreachable)]
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
            #[allow(clippy::cast_possible_truncation)]
            self.emit(Instruction::StoreCached(slot as u32));
            self.cse_cache.insert(expr.hash, slot);
        }

        Ok(())
    }

    /// Compile a sum expression: `a + b + c + ...`
    fn compile_sum(&mut self, terms: &[Arc<Expr>]) -> Result<(), DiffError> {
        if terms.is_empty() {
            let idx = self.add_const(0.0);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
        } else {
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
                    // a * b * c + remainder -> (a * b) * c + remainder
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

        // Check for negation pattern: Product([-1, x]) = -x
        // Exact comparison for -1.0 is mathematically intentional
        #[allow(clippy::float_cmp)]
        if factors.len() == 2 {
            if Self::try_eval_const(&factors[0]) == Some(-1.0) {
                self.compile_expr(&factors[1])?;
                self.emit(Instruction::Neg);
                return Ok(());
            }
            if Self::try_eval_const(&factors[1]) == Some(-1.0) {
                self.compile_expr(&factors[0])?;
                self.emit(Instruction::Neg);
                return Ok(());
            }
        }

        // Group identical factors to use Square/Cube/Pow4
        let mut grouped: Vec<(Arc<Expr>, usize)> = Vec::with_capacity(factors.len());
        for factor in factors {
            if let Some(existing) = grouped.iter_mut().find(|(e, _)| e == factor) {
                existing.1 += 1;
            } else {
                grouped.push((Arc::clone(factor), 1));
            }
        }

        // Compile first group
        let (expr, count) = &grouped[0];
        self.compile_expr_with_count(expr, *count)?;

        // Multiply remaining groups
        for (expr, count) in &grouped[1..] {
            self.compile_expr_with_count(expr, *count)?;
            self.emit(Instruction::Mul);
            self.pop();
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
                #[allow(clippy::cast_possible_truncation)]
                self.emit(Instruction::Powi(count as i8));
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

        // Optimization: E / C -> E * (1/C)
        // Multiplication is generally faster than division on most CPUs
        if let Some(val) = Self::try_eval_const(den).filter(|&v| v != 0.0) {
            self.compile_expr(num)?;
            let idx = self.add_const(1.0 / val);
            self.emit(Instruction::LoadConst(idx));
            self.push()?;
            self.emit(Instruction::Mul);
            self.pop();
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
    fn compile_power(&mut self, base: &Expr, exp: &Expr) -> Result<(), DiffError> {
        // Check for fused instruction patterns with integer/half exponents
        if let ExprKind::Number(n) = &exp.kind {
            let n_val = *n;
            let n_rounded = n_val.round();
            let is_integer = (n_val - n_rounded).abs() < 1e-10;

            if is_integer {
                // Truncation safe: we only use this for small integer exponents
                #[allow(clippy::cast_possible_truncation)]
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
                    -1 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::Recip);
                        return Ok(());
                    }
                    -2 => {
                        self.compile_expr(base)?;
                        self.emit(Instruction::Square);
                        self.emit(Instruction::Recip);
                        return Ok(());
                    }
                    -16..=-3 | 5..=16 => {
                        // Use powi for other small integers
                        // Truncation safe: n_int is in range [-16, 16]
                        #[allow(clippy::cast_possible_truncation)]
                        let n_i8 = n_int as i8;
                        self.compile_expr(base)?;
                        self.emit(Instruction::Powi(n_i8));
                        return Ok(());
                    }
                    _ => {} // Fall through to general case
                }
            } else if (n_val - 0.5).abs() < 1e-10 {
                // x^0.5 → Sqrt
                self.compile_expr(base)?;
                self.emit(Instruction::Sqrt);
                return Ok(());
            } else if (n_val + 0.5).abs() < 1e-10 {
                // x^-0.5 → 1/sqrt(x)
                self.compile_expr(base)?;
                self.emit(Instruction::Sqrt);
                self.emit(Instruction::Recip);
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
    //
    // Allow too_many_lines: This is a dispatch table mapping 50+ function names
    // to instructions. Splitting would make the mapping harder to maintain.
    #[allow(clippy::too_many_lines)]
    fn compile_function_call(
        &mut self,
        func_name: &InternedSymbol,
        args: &[Arc<Expr>],
    ) -> Result<(), DiffError> {
        // Compile arguments first (in order for proper stack layout)
        // Arc<Expr> auto-derefs to &Expr
        for arg in args {
            self.compile_expr(arg)?;
        }

        let id = func_name.id();
        let arity = args.len();
        let ks = &*KS;

        // Emit function instruction
        // We use an if-else chain (or match guards) here because KS fields are not constants,
        // so we cannot match against them directly in patterns.
        let instr = if arity == 1 {
            if id == ks.sin {
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
                Instruction::Cot
            } else if id == ks.sec {
                Instruction::Sec
            } else if id == ks.csc {
                Instruction::Csc
            } else if id == ks.acot {
                Instruction::Acot
            } else if id == ks.asec {
                Instruction::Asec
            } else if id == ks.acsc {
                Instruction::Acsc
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
                Instruction::Coth
            } else if id == ks.sech {
                Instruction::Sech
            } else if id == ks.csch {
                Instruction::Csch
            } else if id == ks.acoth {
                Instruction::Acoth
            } else if id == ks.asech {
                Instruction::Asech
            } else if id == ks.acsch {
                Instruction::Acsch
            }
            // Exponential
            else if id == ks.exp {
                Instruction::Exp
            } else if id == ks.ln {
                Instruction::Ln
            } else if id == ks.log10 {
                Instruction::Log10
            } else if id == ks.log2 {
                Instruction::Log2
            } else if id == ks.sqrt {
                Instruction::Sqrt
            } else if id == ks.cbrt {
                Instruction::Cbrt
            }
            // Special
            else if id == ks.abs {
                Instruction::Abs
            } else if id == ks.signum {
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
                return self.compile_unknown_function(func_name, arity);
            }
        } else if arity == 2 {
            if id == ks.log {
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
                return self.compile_unknown_function(func_name, arity);
            }
        } else if arity == 3 {
            if id == ks.assoc_legendre {
                self.pop();
                self.pop();
                Instruction::AssocLegendre
            } else {
                return self.compile_unknown_function(func_name, arity);
            }
        } else if arity == 4 {
            if id == ks.spherical_harmonic || id == ks.ynm {
                self.pop();
                self.pop();
                self.pop();
                Instruction::SphericalHarmonic
            } else {
                return self.compile_unknown_function(func_name, arity);
            }
        } else {
            return self.compile_unknown_function(func_name, arity);
        };

        self.emit(instr);
        Ok(())
    }

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
        sorted_terms.sort_by(|a, b| b.0.cmp(&a.0));

        let max_pow = sorted_terms[0].0;

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
            let const_idx = self.add_const(sorted_terms[0].1);
            self.emit(Instruction::LoadConst(const_idx));
            self.push()?;

            let mut term_iter = sorted_terms.iter().skip(1).peekable();

            for pow in (0..max_pow).rev() {
                // Optimized multiply-add using Horner's method:
                // current = current * x + coeff

                // Push x
                // Truncation safe: param count bounded by realistic expression size
                #[allow(clippy::cast_possible_truncation)]
                self.emit(Instruction::LoadParam(idx as u32));
                self.push()?;

                if let Some((_, coeff)) = term_iter.peek().filter(|t| t.0 == pow) {
                    // We have a coefficient for this power: use MulAdd
                    let const_val = *coeff;
                    let const_idx = self.add_const(const_val);
                    self.emit(Instruction::LoadConst(const_idx));
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
        let const_idx = self.add_const(first_coeff);
        self.emit(Instruction::LoadConst(const_idx));
        self.push()?;

        // Replaying base: ensure we have enough stack space
        self.max_stack = self.max_stack.max(self.current_stack + base_headroom);

        for instr in &base_instrs {
            self.emit(*instr);
        }
        self.push()?;

        let const_idx = self.add_const(f64::from(first_pow));
        self.emit(Instruction::LoadConst(const_idx));
        self.push()?;
        self.emit(Instruction::Pow);
        self.pop();
        self.emit(Instruction::Mul);
        self.pop();

        // Remaining terms
        for &(pow, coeff) in &sorted_terms[1..] {
            let const_idx = self.add_const(coeff);
            self.emit(Instruction::LoadConst(const_idx));
            self.push()?;

            // Replay base again
            self.max_stack = self.max_stack.max(self.current_stack + base_headroom);
            for instr in &base_instrs {
                self.emit(*instr);
            }
            self.push()?;

            let const_idx = self.add_const(f64::from(pow));
            self.emit(Instruction::LoadConst(const_idx));
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
        assert!((result.expect("constant folding should succeed") - expected).abs() < 1e-10);
    }

    #[test]
    fn test_is_expensive() {
        let simple = parse_expr("x + 1");
        assert!(!Compiler::is_expensive(&simple));

        let expensive = parse_expr("sin(x)");
        assert!(Compiler::is_expensive(&expensive));

        let large_sum = parse_expr("a + b + c + d");
        assert!(Compiler::is_expensive(&large_sum));
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
        let muladd_count = compiler
            .instructions()
            .iter()
            .filter(|i| matches!(i, Instruction::MulAdd))
            .count();
        assert!(muladd_count >= 1);
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
    fn test_compile_div_const() {
        // x / 2.0 should become x * 0.5
        let expr = parse_expr("x / 2.0");
        let x = crate::symb("x");
        let param_ids = vec![x.id()];

        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");

        // Should NOT find Div instruction
        assert!(
            !compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Div))
        );
        // Should find Mul instruction
        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Mul))
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
        let expr = parse_expr("x * x * x");
        let mut compiler = Compiler::new(&param_ids, None);
        compiler.compile_expr(&expr).expect("Should compile");
        assert!(
            compiler
                .instructions()
                .iter()
                .any(|i| matches!(i, Instruction::Cube))
        );
    }
}
