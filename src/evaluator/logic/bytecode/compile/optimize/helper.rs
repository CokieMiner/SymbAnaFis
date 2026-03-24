use crate::core::error::DiffError;
use crate::evaluator::logic::bytecode::instruction::Instruction;
use rustc_hash::FxHashMap;

/// Constant pool pairing a value vec with a bit-keyed index map.
///
/// Ensures the map and the `Vec<f64>` are always in sync by going
/// through a single `get_or_insert` call site instead of the previous
/// loose `(Vec, FxHashMap)` pair that callers had to keep consistent.
pub(super) struct ConstantPool<'pool> {
    constants: &'pool mut Vec<f64>,
    index: FxHashMap<u64, u32>,
}

impl<'pool> ConstantPool<'pool> {
    /// Build a `ConstantPool` around an existing constant vec.
    /// Reconstructs the index from the current contents.
    pub(super) fn new(constants: &'pool mut Vec<f64>) -> Self {
        let index = constants
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                (
                    v.to_bits(),
                    u32::try_from(i).expect("constant pool overflow"),
                )
            })
            .collect();
        Self { constants, index }
    }

    /// Return the index of `val`; insert it if not already present.
    pub(super) fn get_or_insert(&mut self, val: f64) -> u32 {
        let bits = val.to_bits();
        if let Some(&idx) = self.index.get(&bits) {
            return idx;
        }
        let idx = u32::try_from(self.constants.len()).expect("constant pool overflow");
        self.constants.push(val);
        self.index.insert(bits, idx);
        idx
    }

    /// Direct constant lookup by index.
    pub(super) fn get(&self, idx: u32) -> f64 {
        self.constants[idx as usize]
    }

    /// Decompose the pool back into its parts for the compact pass,
    /// which needs to own and rewrite both structures.
    pub(super) fn into_parts(self) -> (&'pool mut Vec<f64>, FxHashMap<u64, u32>) {
        (self.constants, self.index)
    }
}

pub(super) fn calculate_use_count(
    instrs: &[Instruction],
    use_count: &mut [usize],
    arg_pool: &[u32],
) {
    use_count.fill(0);
    for instr in instrs {
        instr.for_each_read(|r| {
            if let Some(slot) = use_count.get_mut(r as usize) {
                *slot += 1;
            }
        });
        instr.for_each_pooled_reg(arg_pool, |reg_idx| {
            if let Some(slot) = use_count.get_mut(reg_idx as usize) {
                *slot += 1;
            }
        });
    }
}

pub(super) fn validate_program(
    instrs: &[Instruction],
    constants: &[f64],
    arg_pool: &[u32],
    workspace_size: usize,
    param_count: usize,
) -> Result<(), DiffError> {
    let const_base = param_count;
    let const_limit = param_count + constants.len();
    let mut defined = vec![false; workspace_size];
    defined[..const_limit].fill(true);

    let err =
        |msg: String| DiffError::UnsupportedExpression(format!("Internal evaluator error: {msg}"));

    for (instr_idx, instr) in instrs.iter().enumerate() {
        let check_reg = |reg_idx: u32, context: &str| -> Result<(), DiffError> {
            let reg = reg_idx as usize;
            if reg >= workspace_size {
                return Err(err(format!(
                    "{context} register R{reg_idx} out of bounds at instruction {instr_idx}"
                )));
            }
            if !defined[reg] {
                return Err(err(format!(
                    "{context} register R{reg_idx} read before definition at instruction {instr_idx}"
                )));
            }
            Ok(())
        };

        let mut read_error = None;
        instr.for_each_read(|reg_idx| {
            if read_error.is_none()
                && let Err(error) = check_reg(reg_idx, "source")
            {
                read_error = Some(error);
            }
        });
        if let Some(error) = read_error {
            return Err(error);
        }

        if let Some((start_idx, count)) = instr.arg_pool_range() {
            let pool_end = start_idx as usize + count as usize;
            if pool_end > arg_pool.len() {
                return Err(err(format!(
                    "arg pool slice [{start_idx}..{pool_end}) out of bounds at instruction {instr_idx}"
                )));
            }
            instr.for_each_pooled_reg(arg_pool, |reg_idx| {
                if read_error.is_none()
                    && let Err(error) = check_reg(reg_idx, "arg-pool source")
                {
                    read_error = Some(error);
                }
            });
            if let Some(error) = read_error.take() {
                return Err(error);
            }
        }

        match *instr {
            Instruction::LoadConst { const_idx, .. }
            | Instruction::MulAddConst { const_idx, .. }
            | Instruction::MulSubConst { const_idx, .. }
            | Instruction::NegMulAddConst { const_idx, .. }
            | Instruction::AddConst { const_idx, .. }
            | Instruction::MulConst { const_idx, .. }
            | Instruction::SubConst { const_idx, .. }
            | Instruction::ConstSub { const_idx, .. }
            | Instruction::DivConst { const_idx, .. }
            | Instruction::NegMulConst { const_idx, .. }
            | Instruction::ConstDiv { const_idx, .. } => {
                if const_idx as usize >= constants.len() {
                    return Err(err(format!(
                        "constant C{const_idx} out of bounds at instruction {instr_idx}"
                    )));
                }
            }
            _ => {}
        }

        let dest = instr.dest_reg() as usize;
        if dest >= workspace_size {
            return Err(err(format!(
                "destination register R{} out of bounds at instruction {instr_idx}",
                instr.dest_reg()
            )));
        }
        defined[dest] = true;

        if const_limit > workspace_size || const_base > const_limit {
            return Err(err("invalid register layout after optimization".to_owned()));
        }
    }

    Ok(())
}
