use super::CompiledEvaluator;
use super::Instruction;
use super::compact::compact_constants;
use super::dce::{DceScratch, eliminate_dead_code};
use super::fusion::fuse_instructions;
#[cfg(debug_assertions)]
use super::helper::validate_program;
use super::helper::{ConstantPool, calculate_use_count};
use super::power_chain::optimize_power_chains;
use super::strength_reduction::reduce_strength;
use crate::core::error::DiffError;
use rustc_hash::FxHashMap;

impl CompiledEvaluator {
    /// Post-compilation optimization pass that fuses instruction patterns.
    ///
    /// Currently detects:
    /// - **FMA Fusions**: `[Mul, Add]` → `MulAdd`, `[Mul, Sub]` → `MulSub`, `[Mul, Sub(rev)]` → `NegMulAdd`
    /// - **Inverse Fusions**: `[Sqrt, Recip]` → `InvSqrt`, `[Square, Recip]` → `InvSquare`, `[Cube, Recip]` → `InvCube`
    /// - **Power Fusions**: `[Square, Mul]` → `Cube`, `[Pow4, Recip]` → `Square + InvSquare` (for $x^{-4}$)
    /// - **Exponential Fusions**: `[Neg, Exp]` → `ExpNeg`
    ///
    ///
    /// # Errors
    ///
    /// Returns `DiffError` if the instruction stream is invalid or if optimization
    /// passes fail to maintain program consistency.
    ///
    /// # Panics
    ///
    /// May panic if the number of parameters or registers exceeds `u32::MAX`.
    pub fn optimize_instructions(
        instructions: Vec<Instruction>,
        constants: &mut Vec<f64>,
        const_map: FxHashMap<u64, u32>,
        arg_pool: &mut [u32],
        param_count: usize,
        max_phys: usize,
        result_reg: u32,
    ) -> Result<(Vec<Instruction>, usize, u32), DiffError> {
        if instructions.is_empty() {
            let rc = param_count + constants.len();
            return Ok((instructions, rc, result_reg));
        }

        let max_reg_idx = u32::try_from(max_phys)
            .expect("Register index overflow")
            .max(result_reg)
            .saturating_sub(1);
        let old_const_count = constants.len();

        // Pre-allocate use_count buffer reused across DCE and fusion passes.
        // No initial population needed — the first DCE pass recalculates internally.
        let mut use_count = vec![0_usize; (max_reg_idx + 1) as usize];

        // Single owned pool for all mutation-safe constant interning throughout the pipeline.
        let mut pool = ConstantPool::with_index(
            constants,
            const_map,
            u32::try_from(param_count).expect("Param count overflow"),
        );

        // 1. Initial strength reduction and power chain analysis
        // These passes may convert neutral math to 'Copy' or rewrite power sequences.
        let mut out = instructions;
        reduce_strength(&mut out, &mut pool);
        optimize_power_chains(&mut out);

        // 2. Main DCE / Copy Forwarding pass
        // Cleans up artifacts from VIR lowering and the previous transformation passes.
        // Essential to enable Fusion by skipping over redundant 'Copy' instructions.
        let mut dce_scratch = DceScratch::new();
        out = eliminate_dead_code(
            out,
            arg_pool,
            &mut use_count,
            param_count,
            old_const_count,
            max_reg_idx,
            result_reg,
            &mut dce_scratch,
        );

        // 3. Final fusion pass: Catch FMA/Pow patterns on the cleaned instruction stream
        loop {
            calculate_use_count(&out, &mut use_count, &mut dce_scratch.dirty_uses, arg_pool);
            let (new_out, changed) = fuse_instructions(&out, &mut pool, &use_count, arg_pool);
            out = new_out;
            if !changed {
                break;
            }
        }

        // 4. Second DCE pass: Final polish to remove orphans created by Fusion
        out = eliminate_dead_code(
            out,
            arg_pool,
            &mut use_count,
            param_count,
            old_const_count,
            max_reg_idx,
            result_reg,
            &mut dce_scratch,
        );

        // 5. Constant Compaction & Register Re-indexing
        // This is the final pass. It removes unused constants and shifts all
        // registers down to create a dense, minimal workspace.
        let (const_vec, _) = pool.into_parts();
        let (out, rc, result_reg) = compact_constants(
            out,
            const_vec,
            arg_pool,
            param_count,
            old_const_count,
            result_reg,
        );

        #[cfg(debug_assertions)]
        validate_program(&out, const_vec, arg_pool, rc, param_count)?;

        Ok((out, rc, result_reg))
    }
}
