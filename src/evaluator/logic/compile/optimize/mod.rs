use self::helper::ConstantPool;
use crate::core::error::DiffError;
use crate::evaluator::CompiledEvaluator;
use crate::evaluator::logic::instruction::Instruction;

pub mod compact;
pub mod dce;
pub mod fusion;
pub mod helper;
pub mod poly_share;
pub mod power_chain;
pub mod strength_reduction;

#[cfg(test)]
mod tests;

impl CompiledEvaluator {
    /// Post-compilation optimization pass that fuses instruction patterns.
    ///
    /// Currently detects:
    /// - **FMA Fusions**: `[Mul, Add]` → `MulAdd`, `[Mul, Sub]` → `MulSub`, `[Mul, Sub(rev)]` → `NegMulAdd`
    /// - **Inverse Fusions**: `[Sqrt, Recip]` → `InvSqrt`, `[Square, Recip]` → `InvSquare`, `[Cube, Recip]` → `InvCube`
    /// - **Power Fusions**: `[Square, Mul]` → `Cube`, `[Pow4, Recip]` → `Square + InvSquare` (for $x^{-4}$)
    /// - **Exponential Fusions**: `[Neg, Exp]` → `ExpNeg`
    ///
    /// The pass also performs copy forwarding and constant pool compaction.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "Takes ownership to match call site pattern where caller builds Vec and passes it directly"
    )]
    pub(crate) fn optimize_instructions(
        instructions: Vec<Instruction>,
        constants: &mut Vec<f64>,
        arg_pool: &mut [u32],
        param_count: usize,
    ) -> Result<(Vec<Instruction>, usize), DiffError> {
        if instructions.is_empty() {
            let rc = param_count + constants.len();
            return Ok((instructions, rc));
        }

        let mut max_reg_idx = 0;
        for instr in &instructions {
            instr.for_each_reg(|r| max_reg_idx = max_reg_idx.max(r));
            instr.for_each_pooled_reg(arg_pool, |reg_idx| max_reg_idx = max_reg_idx.max(reg_idx));
        }
        let old_const_count = constants.len();

        let mut use_count = vec![0_usize; (max_reg_idx + 1) as usize];
        helper::calculate_use_count(&instructions, &mut use_count, arg_pool);

        // Single owned pool for all mutation-safe constant interning throughout the pipeline.
        let mut pool = ConstantPool::new(constants);

        // Initial fusion pass
        let out = fusion::fuse_instructions(&instructions, &mut pool, &use_count);

        // DCE pass
        let out = dce::eliminate_dead_code(
            out,
            arg_pool,
            &mut use_count,
            param_count,
            old_const_count,
            max_reg_idx,
        );

        // Second fusion pass (catches patterns exposed by DCE)
        helper::calculate_use_count(&out, &mut use_count, arg_pool);
        let mut out = fusion::fuse_instructions(&out, &mut pool, &use_count);

        // Strength reduction (may add constants)
        strength_reduction::reduce_strength(&mut out, &mut pool);

        // Third fusion pass (catches patterns exposed by strength reduction)
        helper::calculate_use_count(&out, &mut use_count, arg_pool);
        out = fusion::fuse_instructions(&out, &mut pool, &use_count);

        // Power chain pass
        power_chain::optimize_power_chains(&mut out);

        // Shared polynomial base optimization
        let mut next_reg = max_reg_idx + 1;
        let out = poly_share::optimize_shared_poly_bases(out, pool.as_slice(), &mut next_reg);

        // Re-calculate use_count after potential expansions
        if next_reg as usize > use_count.len() {
            use_count.resize(next_reg as usize, 0);
        }
        helper::calculate_use_count(&out, &mut use_count, arg_pool);

        // Final fusion pass (catches patterns from expansion)
        let out = fusion::fuse_instructions(&out, &mut pool, &use_count);

        // FINAL STEP: Compact constants — must be last.
        // After this, NO MORE constants can be added.
        let (const_vec, mut const_map) = pool.into_parts();
        let (out, rc) = compact::compact_constants(
            out,
            const_vec,
            &mut const_map,
            arg_pool,
            param_count,
            old_const_count,
        );
        if rc > use_count.len() {
            use_count.resize(rc, 0);
        }
        helper::validate_program(&out, const_vec, arg_pool, rc, param_count)?;
        helper::calculate_use_count(&out, &mut use_count, arg_pool);
        Ok((out, rc))
    }
}
