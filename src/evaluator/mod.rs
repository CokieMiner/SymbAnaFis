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
#![allow(
    unsafe_code,
    reason = "Safety is guaranteed by compile-time stack validation"
)]

// Internal submodules - visibility is controlled by parent module (not exported from crate root)
mod compiler;
mod execution;
mod instruction;
mod simd;
mod stack;

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
#[repr(align(64))]
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
    pub stack_size: usize,
    /// Number of parameters expected
    pub param_count: usize,
    /// Number of CSE cache slots required
    pub cache_size: usize,
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

        let mut compiler = Compiler::new(&param_ids, context);

        // Single-pass compilation with CSE
        compiler.compile_expr(&expanded_expr)?;

        // Extract compilation results
        let (instructions, mut constants, max_stack, param_count, mut cache_size) =
            compiler.into_parts();

        // Post-compilation optimization pass: fuse instructions
        let optimized_instructions =
            Self::optimize_instructions(instructions, &mut constants, &mut cache_size);

        Ok(Self {
            instructions: optimized_instructions.into_boxed_slice(),
            constants: constants.into_boxed_slice(),
            param_names: param_names.into_boxed_slice(),
            stack_size: max_stack,
            param_count,
            cache_size,
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
        let vars = expr.variables();
        let mut param_order: Vec<String> = vars
            .into_iter()
            .filter(|v| {
                let id = crate::core::symbol::symb_interned(v.as_str()).id();
                !crate::core::known_symbols::is_known_constant_by_id(id)
            })
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
    #[allow(
        clippy::needless_pass_by_value,
        reason = "Takes ownership to match call site pattern where caller builds Vec and passes it directly without needing it afterwards."
    )]
    fn optimize_instructions(
        mut instructions: Vec<Instruction>,
        constants: &mut Vec<f64>,
        cache_size: &mut usize,
    ) -> Vec<Instruction> {
        // Pass 1: Peephole optimizations (local fusion)
        Self::peephole_optimize(&mut instructions, constants);

        // Pass 2: Fuse MulAdd (requires 3 instructions)
        instructions = Self::fuse_muladd(&instructions);

        // Pass 3: Dead store elimination (global analysis)
        Self::eliminate_dead_stores(&mut instructions, cache_size);

        // Pass 4: Constant pool deduplication
        Self::deduplicate_constants(&mut instructions, constants);

        // Pass 5: Cache slot reuse optimization
        Self::optimize_cache_slots(&mut instructions, cache_size);

        instructions
    }

    /// Deduplicate constant pool to reduce memory usage and improve cache hits.
    fn deduplicate_constants(instructions: &mut [Instruction], constants: &mut Vec<f64>) {
        use std::collections::HashMap;

        let mut const_map: HashMap<u64, u32> = HashMap::with_capacity(constants.len());
        let mut new_constants = Vec::with_capacity(constants.len());

        for instr in instructions.iter_mut() {
            match instr {
                Instruction::LoadConst(idx)
                | Instruction::MulConst(idx)
                | Instruction::AddConst(idx)
                | Instruction::SubConst(idx)
                | Instruction::ConstSub(idx) => {
                    let val = constants[*idx as usize];
                    let bits = val.to_bits();
                    *idx = *const_map.entry(bits).or_insert_with(|| {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Constant pool index will not exceed u32::MAX"
                        )]
                        let new_idx = new_constants.len() as u32;
                        new_constants.push(val);
                        new_idx
                    });
                }
                Instruction::PolyEval(idx) => {
                    let old_idx = *idx as usize;
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "Degree is stored as f64 in constant pool"
                    )]
                    let degree = constants[old_idx] as usize;
                    let block_len = degree + 2;

                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "Constant pool index will not exceed u32::MAX"
                    )]
                    let new_idx = new_constants.len() as u32;
                    for i in 0..block_len {
                        new_constants.push(constants[old_idx + i]);
                    }
                    *idx = new_idx;
                }
                _ => {}
            }
        }

        *constants = new_constants;
    }

    /// Implement live-range analysis and cache slot reuse.
    fn optimize_cache_slots(instructions: &mut [Instruction], cache_size: &mut usize) {
        use std::collections::HashMap;

        // Track live ranges more precisely
        let mut slot_live_ranges: HashMap<u32, (usize, usize, bool)> = HashMap::new();
        let mut last_use_map: HashMap<u32, usize> = HashMap::new();

        for (i, instr) in instructions.iter().enumerate() {
            match instr {
                Instruction::StoreCached(slot) => {
                    slot_live_ranges.entry(*slot).or_insert((i, i, false)).1 = i;
                    last_use_map.insert(*slot, i);
                }
                Instruction::LoadCached(slot) => {
                    if let Some((start, _, _)) = slot_live_ranges.get_mut(slot) {
                        if *start > i {
                            *start = i; // Earlier use than we thought
                        }
                    } else {
                        // Load without prior store - mark as read-only
                        slot_live_ranges.insert(*slot, (i, i, true));
                    }
                    last_use_map.insert(*slot, i);
                }
                _ => {}
            }
        }

        if slot_live_ranges.is_empty() {
            *cache_size = 0;
            return;
        }

        // Sort old slots by first use
        let mut old_slots: Vec<u32> = slot_live_ranges.keys().copied().collect();
        old_slots.sort_by_key(|&s| slot_live_ranges[&s].0);

        let mut remap: HashMap<u32, u32> = HashMap::new();
        let mut active_slots: Vec<(u32, usize)> = Vec::new(); // (new_slot, last_use)
        let mut next_new_slot = 0;

        for &old_slot in &old_slots {
            let (first, _last, _readonly) = slot_live_ranges[&old_slot];
            // Use precise last use from last_use_map
            let last = *last_use_map.get(&old_slot).unwrap_or(&first);

            let mut reused = false;
            for (new_slot, last_use) in &mut active_slots {
                if *last_use < first {
                    remap.insert(old_slot, *new_slot);
                    *last_use = last;
                    reused = true;
                    break;
                }
            }

            if !reused {
                let new_slot = next_new_slot;
                next_new_slot += 1;
                remap.insert(old_slot, new_slot);
                active_slots.push((new_slot, last));
            }
        }

        // Apply remapping
        for instr in instructions.iter_mut() {
            match instr {
                Instruction::StoreCached(slot) | Instruction::LoadCached(slot) => {
                    if let Some(&new_slot) = remap.get(slot) {
                        *slot = new_slot;
                    }
                }
                _ => {}
            }
        }

        *cache_size = next_new_slot as usize;
    }

    /// Perform local peephole optimizations.
    ///
    /// Uses a single-pass approach (building a new Vec) instead of in-place
    /// `Vec::remove()` calls, giving O(n) instead of O(n²) per pass.
    /// Loops until convergence for cascading optimizations.
    #[allow(
        clippy::too_many_lines,
        clippy::collapsible_if,
        clippy::match_same_arms,
        clippy::branches_sharing_code,
        reason = "function is complex and splitting it would reduce readability; match arms kept separate for self-documenting per-pattern comments"
    )]
    fn peephole_optimize(instructions: &mut Vec<Instruction>, constants: &mut Vec<f64>) {
        loop {
            let mut result = Vec::with_capacity(instructions.len());
            let mut changed = false;
            let mut i = 0;

            while i < instructions.len() {
                // 3-instruction patterns (check before 2-instruction for longer match priority)
                if i + 2 < instructions.len() {
                    // 4-instruction pattern: (-a) + b -> b - a
                    // Catches bytecode like [LoadA, Neg, LoadB, Add].
                    if i + 3 < instructions.len() {
                        match (
                            instructions[i],
                            instructions[i + 1],
                            instructions[i + 2],
                            instructions[i + 3],
                        ) {
                            (
                                load_a,
                                Instruction::Neg,
                                load_b,
                                Instruction::Add,
                            ) if matches!(
                                load_a,
                                Instruction::LoadParam(_)
                                    | Instruction::LoadCached(_)
                                    | Instruction::LoadConst(_)
                            ) && matches!(
                                load_b,
                                Instruction::LoadParam(_)
                                    | Instruction::LoadCached(_)
                                    | Instruction::LoadConst(_)
                            ) =>
                            {
                                result.push(load_b);
                                result.push(load_a);
                                result.push(Instruction::Sub);
                                i += 4;
                                changed = true;
                                continue;
                            }
                            _ => {}
                        }
                    }

                    match (instructions[i], instructions[i + 1], instructions[i + 2]) {
                        // x * x * x → Cube
                        (Instruction::Dup, Instruction::Mul, Instruction::Mul) => {
                            result.push(Instruction::Cube);
                            i += 3;
                            changed = true;
                            continue;
                        }
                        // LoadConst + Swap + Sub → ConstSub or Neg
                        (Instruction::LoadConst(idx), Instruction::Swap, Instruction::Sub) => {
                            if constants[idx as usize] == 0.0 {
                                result.push(Instruction::Neg);
                            } else {
                                result.push(Instruction::ConstSub(idx));
                            }
                            i += 3;
                            changed = true;
                            continue;
                        }
                        // LoadConst(1.0) + Swap + Div → Recip
                        (Instruction::LoadConst(idx), Instruction::Swap, Instruction::Div)
                            if (constants[idx as usize] - 1.0).abs() < f64::EPSILON =>
                        {
                            result.push(Instruction::Recip);
                            i += 3;
                            changed = true;
                            continue;
                        }
                        // LoadConst + LoadX + Add → LoadX + AddConst (commute to fuse)
                        (Instruction::LoadConst(idx), load_instr, Instruction::Add)
                            if matches!(
                                load_instr,
                                Instruction::LoadParam(_)
                                    | Instruction::LoadCached(_)
                                    | Instruction::LoadConst(_)
                            ) && constants[idx as usize] != 0.0 =>
                        {
                            result.push(load_instr);
                            result.push(Instruction::AddConst(idx));
                            i += 3;
                            changed = true;
                            continue;
                        }
                        // LoadConst + LoadX + Mul → LoadX + MulConst (commute to fuse)
                        (Instruction::LoadConst(idx), load_instr, Instruction::Mul)
                            if matches!(
                                load_instr,
                                Instruction::LoadParam(_)
                                    | Instruction::LoadCached(_)
                                    | Instruction::LoadConst(_)
                            ) =>
                        {
                            let val = constants[idx as usize];
                            if (val - 1.0).abs() < f64::EPSILON {
                                // C * x where C=1 → just x
                                result.push(load_instr);
                            } else if (val + 1.0).abs() < f64::EPSILON {
                                // C * x where C=-1 → -x
                                result.push(load_instr);
                                result.push(Instruction::Neg);
                            } else {
                                result.push(load_instr);
                                result.push(Instruction::MulConst(idx));
                            }
                            i += 3;
                            changed = true;
                            continue;
                        }
                        // (x / y) then Neg then Exp  ->  (x / y) then ExpNeg
                        // This catches normalized forms from compile_division where negation
                        // is intentionally moved after division.
                        (Instruction::Div, Instruction::Neg, Instruction::Exp) => {
                            result.push(Instruction::Div);
                            result.push(Instruction::ExpNeg);
                            i += 3;
                            changed = true;
                            continue;
                        }
                        _ => {}
                    }
                }

                // 2-instruction patterns
                if i + 1 < instructions.len() {
                    let mut matched = true;
                    match (instructions[i], instructions[i + 1]) {
                        // Consecutive identical loads → Load + Dup
                        (Instruction::LoadParam(a), Instruction::LoadParam(b)) if a == b => {
                            result.push(Instruction::LoadParam(a));
                            result.push(Instruction::Dup);
                        }
                        (Instruction::LoadConst(a), Instruction::LoadConst(b)) if a == b => {
                            result.push(Instruction::LoadConst(a));
                            result.push(Instruction::Dup);
                        }
                        (Instruction::LoadCached(a), Instruction::LoadCached(b)) if a == b => {
                            result.push(Instruction::LoadCached(a));
                            result.push(Instruction::Dup);
                        }
                        // (-x)² = x² → remove Neg before Square
                        (Instruction::Neg, Instruction::Square) => {
                            result.push(Instruction::Square);
                        }
                        // Neg + Exp → ExpNeg
                        (Instruction::Neg, Instruction::Exp) => {
                            result.push(Instruction::ExpNeg);
                        }
                        // Inverse function cancellation
                        (Instruction::Exp, Instruction::Ln)
                        | (Instruction::Ln, Instruction::Exp) => {
                            // remove both
                        }
                        (Instruction::Sqrt, Instruction::Square) => {
                            // sqrt(x)² = x for x >= 0 (which is sqrt's domain)
                            // remove both
                        }
                        // x * x → Square
                        (Instruction::Dup, Instruction::Mul) => {
                            result.push(Instruction::Square);
                        }
                        // Bubble Negation forward (outward) through Mul/Div to enable ExpNeg/cancellation
                        // Case 1: (-a) * b → -(a * b) OR (-a) / b → -(a / b)
                        (Instruction::Neg, load_instr)
                            if matches!(
                                load_instr,
                                Instruction::LoadParam(_)
                                    | Instruction::LoadCached(_)
                                    | Instruction::LoadConst(_)
                            ) =>
                        {
                            // Peek ahead for Mul/Div
                            if i + 2 < instructions.len() {
                                match instructions[i + 2] {
                                    Instruction::Mul | Instruction::Div => {
                                        result.push(load_instr);
                                        result.push(instructions[i + 2]);
                                        result.push(Instruction::Neg);
                                        i += 3; // consumed Neg, Load, Op
                                        changed = true;
                                        continue;
                                    }
                                    _ => {} // Not a bubble-able op
                                }
                            }
                            // Fallback if no match
                            if matched {
                                // If we fell through from a previous match that set matched=true,
                                // we shouldn't be here. But in this structure, we are in a match arm.
                                // We need to be careful not to consume instructions if we didn't transform.
                                // Actually, this arm matched (Neg, Load). We must emit them if we don't fuse.
                                result.push(Instruction::Neg);
                                result.push(load_instr);
                                i += 2;
                                continue;
                            }
                        }
                        // Case 2: a * (-b) → -(a * b) OR a / (-b) → -(a / b)
                        (load_instr, Instruction::Neg)
                            if matches!(
                                load_instr,
                                Instruction::LoadParam(_)
                                    | Instruction::LoadCached(_)
                                    | Instruction::LoadConst(_)
                            ) =>
                        {
                            // Peek ahead for Mul/Div
                            if i + 2 < instructions.len() {
                                match instructions[i + 2] {
                                    Instruction::Mul | Instruction::Div => {
                                        result.push(load_instr);
                                        result.push(instructions[i + 2]);
                                        result.push(Instruction::Neg);
                                        i += 3; // consumed Load, Neg, Op
                                        changed = true;
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                            // Not a bubble-able pattern — emit both as-is
                            matched = false;
                        }
                        // LoadConst + Mul optimizations
                        (Instruction::LoadConst(idx), Instruction::Mul) => {
                            let val = constants[idx as usize];
                            if (val - 1.0).abs() < f64::EPSILON {
                                // x * 1.0 = x → remove both
                            } else if (val + 1.0).abs() < f64::EPSILON {
                                // x * -1.0 = -x
                                result.push(Instruction::Neg);
                            } else if val == 0.0 {
                                // x * 0.0 → Pop, LoadConst(0)
                                result.push(Instruction::Pop);
                                result.push(Instruction::LoadConst(idx));
                            } else {
                                result.push(Instruction::MulConst(idx));
                            }
                        }
                        // LoadConst + Add optimizations
                        (Instruction::LoadConst(idx), Instruction::Add) => {
                            if constants[idx as usize] == 0.0 {
                                // x + 0.0 = x → remove both
                            } else {
                                result.push(Instruction::AddConst(idx));
                            }
                        }
                        // LoadConst + Sub optimizations
                        (Instruction::LoadConst(idx), Instruction::Sub) => {
                            if constants[idx as usize] == 0.0 {
                                // x - 0.0 = x → remove both
                            } else {
                                result.push(Instruction::SubConst(idx));
                            }
                        }
                        // LoadConst + Div optimizations
                        (Instruction::LoadConst(idx), Instruction::Div) => {
                            let val = constants[idx as usize];
                            if (val - 1.0).abs() < f64::EPSILON {
                                // x / 1.0 = x → remove both
                            } else if (val + 1.0).abs() < f64::EPSILON {
                                // x / -1.0 = -x
                                result.push(Instruction::Neg);
                            } else if val != 0.0 {
                                // x / C = x * (1/C)
                                let inv_val = 1.0 / val;
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    reason = "Constant pool index safe"
                                )]
                                let new_idx = constants.len() as u32;
                                constants.push(inv_val);
                                result.push(Instruction::MulConst(new_idx));
                            } else {
                                // div by zero constant, keep as-is
                                matched = false;
                            }
                        }
                        // LoadCached(s) + StoreCached(s) → Dup
                        (Instruction::LoadCached(s1), Instruction::StoreCached(s2)) if s1 == s2 => {
                            result.push(Instruction::Dup);
                        }
                        // Square + Sqrt → Abs
                        (Instruction::Square, Instruction::Sqrt) => {
                            result.push(Instruction::Abs);
                        }
                        // Square + Square → Pow4
                        (Instruction::Square, Instruction::Square) => {
                            result.push(Instruction::Pow4);
                        }
                        // Square + Recip → InvSquare
                        (Instruction::Square, Instruction::Recip) => {
                            result.push(Instruction::InvSquare);
                        }
                        // Cube + Recip → InvCube
                        (Instruction::Cube, Instruction::Recip) => {
                            result.push(Instruction::InvCube);
                        }
                        // Sqrt + Recip → InvSqrt
                        (Instruction::Sqrt, Instruction::Recip) => {
                            result.push(Instruction::InvSqrt);
                        }
                        // (-x)^4 = x^4 → remove Neg before Pow4
                        (Instruction::Neg, Instruction::Pow4) => {
                            result.push(Instruction::Pow4);
                        }
                        // Double inverse/negation → identity (remove both)
                        (Instruction::Recip, Instruction::Recip)
                        | (Instruction::Neg, Instruction::Neg) => {
                            // remove both → push nothing
                        }
                        // Neg + Add → Sub
                        (Instruction::Neg, Instruction::Add) => {
                            result.push(Instruction::Sub);
                        }
                        // Neg + Sub → Add
                        (Instruction::Neg, Instruction::Sub) => {
                            result.push(Instruction::Add);
                        }
                        // Neg + AddConst → ConstSub: -x + C = C - x
                        (Instruction::Neg, Instruction::AddConst(idx)) => {
                            result.push(Instruction::ConstSub(idx));
                        }
                        _ => {
                            matched = false;
                        }
                    }

                    if matched {
                        i += 2;
                        changed = true;
                        continue;
                    }
                }

                // No pattern matched — emit instruction as-is
                result.push(instructions[i]);
                i += 1;
            }

            *instructions = result;
            if !changed {
                break;
            }
        }
    }

    /// Eliminate `StoreCached` instructions for slots that are never loaded.
    fn eliminate_dead_stores(instructions: &mut Vec<Instruction>, _cache_size: &mut usize) {
        use std::collections::HashSet;

        let mut loaded_slots = HashSet::new();
        for instr in instructions.iter() {
            if let Instruction::LoadCached(slot) = instr {
                loaded_slots.insert(*slot);
            }
        }

        // Remove stores to dead slots
        instructions.retain(|instr| {
            if let Instruction::StoreCached(slot) = instr {
                loaded_slots.contains(slot)
            } else {
                true
            }
        });

        // Optional: Renumber slots to reduce cache_size?
        // For simplicity in Phase 1, just reducing instruction count is enough.
        // Reducing `cache_size` requires remapping all Load/Store.
        // Let's stick to instruction reduction.
    }

    /// Fuse `a * b + c` patterns into `MulAdd` instruction.
    ///
    /// The `MulAdd` instruction uses hardware FMA (fused multiply-add) when available,
    /// which is both faster and more accurate than separate multiply and add.
    fn fuse_muladd(instructions: &[Instruction]) -> Vec<Instruction> {
        let mut result = Vec::with_capacity(instructions.len());
        let mut i = 0;

        while i < instructions.len() {
            if i + 2 < instructions.len() {
                let match_result = match (instructions[i], instructions[i + 1], instructions[i + 2])
                {
                    (Instruction::Mul, load_instr, Instruction::Add)
                        if matches!(
                            load_instr,
                            Instruction::LoadParam(_)
                                | Instruction::LoadConst(_)
                                | Instruction::LoadCached(_)
                        ) =>
                    {
                        Some((load_instr, Instruction::MulAdd))
                    }
                    (Instruction::Mul, load_instr, Instruction::Sub)
                        if matches!(
                            load_instr,
                            Instruction::LoadParam(_)
                                | Instruction::LoadConst(_)
                                | Instruction::LoadCached(_)
                        ) =>
                    {
                        Some((load_instr, Instruction::MulSub))
                    }
                    _ => None,
                };

                if let Some((load, fused)) = match_result {
                    result.push(load);
                    result.push(fused);
                    i += 3;
                    continue;
                }
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

    /// Get constants as SIMD vectors, computed once and cached.
    #[inline]
    pub(crate) fn simd_constants(&self) -> &[f64x4] {
        self.simd_constants
            .get_or_init(|| {
                self.constants
                    .iter()
                    .map(|&c| f64x4::splat(c))
                    .collect::<Vec<_>>()
                    .into()
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
            .field("stack_size", &self.stack_size)
            .field("cache_size", &self.cache_size)
            .field("constant_count", &self.constants.len())
            .field("simd_constants_cached", &self.simd_constants.get().is_some())
            .finish()
    }
}
