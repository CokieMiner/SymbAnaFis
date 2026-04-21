use super::Instruction;
#[cfg(debug_assertions)]
use crate::core::error::DiffError;
use rustc_hash::FxHashMap;

/// Constant pool pairing a value vec with a bit-keyed index map.
///
/// In the Unified Memory Layout, constants are mapped to registers starting at `param_count`.
pub struct ConstantPool<'pool> {
    constants: &'pool mut Vec<f64>,
    index: FxHashMap<u64, u32>,
    param_count: u32,
}

impl<'pool> ConstantPool<'pool> {
    /// Build a `ConstantPool` reusing an existing index.
    pub const fn with_index(
        constants: &'pool mut Vec<f64>,
        index: FxHashMap<u64, u32>,
        param_count: u32,
    ) -> Self {
        Self {
            constants,
            index,
            param_count,
        }
    }

    /// Return the REGISTER INDEX of `val`; insert it if not already present.
    pub fn get_or_insert(&mut self, val: f64) -> u32 {
        let bits = val.to_bits();
        if let Some(&rel_idx) = self.index.get(&bits) {
            return self.param_count + rel_idx;
        }
        let rel_idx = u32::try_from(self.constants.len()).expect("constant pool overflow");
        self.constants.push(val);
        self.index.insert(bits, rel_idx);
        self.param_count + rel_idx
    }

    /// Direct constant lookup by REGISTER INDEX.
    pub fn get(&self, register: u32) -> f64 {
        let offset = register - self.param_count;
        self.constants[offset as usize]
    }

    /// Direct constant lookup by RELATIVE index (0..len).
    pub fn get_at(&self, rel_idx: u32) -> f64 {
        self.constants[rel_idx as usize]
    }

    /// Check if a register is in the constant pool.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Constant pool size is architecturally limited to u32::MAX"
    )]
    pub const fn is_constant(&self, reg_idx: u32) -> bool {
        reg_idx >= self.param_count && reg_idx < self.param_count + self.constants.len() as u32
    }

    pub fn into_parts(self) -> (&'pool mut Vec<f64>, FxHashMap<u64, u32>) {
        (self.constants, self.index)
    }
}

pub(super) fn calculate_use_count(
    instrs: &[Instruction],
    use_count: &mut [usize],
    dirty_uses: &mut Vec<u32>,
    arg_pool: &[u32],
) {
    for &idx in dirty_uses.iter() {
        use_count[idx as usize] = 0;
    }
    dirty_uses.clear();

    for instr in instrs {
        instr.for_each_read(|r| {
            if let Some(slot) = use_count.get_mut(r as usize) {
                if *slot == 0 {
                    dirty_uses.push(r);
                }
                *slot += 1;
            }
        });
        instr.for_each_pooled_reg(arg_pool, |reg_idx| {
            if let Some(slot) = use_count.get_mut(reg_idx as usize) {
                if *slot == 0 {
                    dirty_uses.push(reg_idx);
                }
                *slot += 1;
            }
        });
    }
}

#[cfg(debug_assertions)]
pub(super) fn validate_program(
    instrs: &[Instruction],
    constants: &[f64],
    arg_pool: &[u32],
    workspace_size: usize,
    param_count: usize,
) -> Result<(), DiffError> {
    let const_limit = param_count + constants.len();
    if const_limit > workspace_size {
        return Err(DiffError::UnsupportedExpression(
            "Internal evaluator error: invalid register layout during validation".to_owned(),
        ));
    }

    let mut defined = vec![false; workspace_size];
    // Constants are pre-defined
    defined[..const_limit].fill(true);

    for (instr_idx, instr) in instrs.iter().enumerate() {
        validate_instruction(instr_idx, instr, workspace_size, arg_pool, &mut defined)?;
    }

    Ok(())
}

#[cfg(debug_assertions)]
fn validate_instruction(
    instr_idx: usize,
    instr: &Instruction,
    workspace_size: usize,
    arg_pool: &[u32],
    defined: &mut [bool],
) -> Result<(), DiffError> {
    let err =
        |msg: String| DiffError::UnsupportedExpression(format!("Internal evaluator error: {msg}"));

    let check_reg = |reg_idx: u32, context: &str, defs: &[bool]| -> Result<(), DiffError> {
        let reg = reg_idx as usize;
        if reg >= workspace_size {
            return Err(err(format!(
                "{context} register R{reg_idx} out of bounds at instruction {instr_idx}"
            )));
        }
        if !defs[reg] {
            return Err(err(format!(
                "{context} register R{reg_idx} read before definition at instruction {instr_idx}"
            )));
        }
        Ok(())
    };

    // 1. Check source registers
    let mut read_error = None;
    instr.for_each_read(|reg_idx| {
        if read_error.is_none() {
            read_error = check_reg(reg_idx, "source", defined).err();
        }
    });
    if let Some(error) = read_error {
        return Err(error);
    }

    // 2. Check pooled registers
    if let Some((start_idx, count)) = instr.arg_pool_range() {
        let pool_end = start_idx as usize + count as usize;
        if pool_end > arg_pool.len() {
            return Err(err(format!(
                "arg pool slice [{start_idx}..{pool_end}) out of bounds at instruction {instr_idx}"
            )));
        }
        instr.for_each_pooled_reg(arg_pool, |reg_idx| {
            if read_error.is_none() {
                read_error = check_reg(reg_idx, "arg-pool source", defined).err();
            }
        });
        if let Some(error) = read_error {
            return Err(error);
        }
    }

    // 3. Update defined registers
    let mut write_error = None;
    instr.for_each_write(|reg_idx| {
        let dest = reg_idx as usize;
        if dest >= workspace_size {
            write_error = Some(Err(err(format!(
                "destination register R{reg_idx} out of bounds at instruction {instr_idx}"
            ))));
        } else {
            defined[dest] = true;
        }
    });

    if let Some(write_err) = write_error {
        return write_err;
    }

    Ok(())
}
