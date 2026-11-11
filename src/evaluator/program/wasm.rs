//! Standalone WebAssembly (`.wasm`) binary module export for real scalar programs.
//!
//! Emits a self-contained WebAssembly module exporting `"eval"` with one floating-point
//! parameter per compiled program parameter (`(param f64|f32 ...) (result f64|f32)`),
//! inline `f64.const`/`f32.const` literals, stack-scheduled single-use temporaries,
//! `local.tee` forwarding, and compacted WASM locals. No linear memory import is
//! required; only non-inline transcendental functions are imported from `"math"`.

#![expect(
    unsafe_code,
    reason = "validated ProgramData bounds pooled operand spans during backward liveness analysis"
)]

use alloc::{vec, vec::Vec};

use crate::{core::BuiltinFunction, errors::EvalError};

use super::{
    Combine, Fold, InlineBinary, InlineUnary, Instruction, MultiProgram, NativeScalar, OperandSpan,
    Operation, PairedFunction, Program, ProgramData,
};

const OP_END: u8 = 0x0B;
const OP_CALL: u8 = 0x10;
const OP_DROP: u8 = 0x1A;
const OP_LOCAL_GET: u8 = 0x20;
const OP_LOCAL_SET: u8 = 0x21;
const OP_LOCAL_TEE: u8 = 0x22;

/// Sealed WebAssembly opcode and literal-encoding contract (`mod wasm` is private).
pub trait WasmSealed {
    const VAL_TYPE: u8;
    const ABS: u8;
    const NEG: u8;
    const CEIL: u8;
    const FLOOR: u8;
    const TRUNC: u8;
    const SQRT: u8;
    const ADD: u8;
    const SUB: u8;
    const MUL: u8;
    const DIV: u8;
    const MIN: u8;
    const MAX: u8;

    fn push_wasm_const(self, buf: &mut Vec<u8>);
}

/// Real IEEE-754 floating-point scalars supported by standalone WebAssembly (`.wasm`) binary export.
pub trait WasmScalar: NativeScalar + WasmSealed {}

impl WasmSealed for f64 {
    const VAL_TYPE: u8 = 0x7C;
    const ABS: u8 = 0x99;
    const NEG: u8 = 0x9A;
    const CEIL: u8 = 0x9B;
    const FLOOR: u8 = 0x9C;
    const TRUNC: u8 = 0x9D;
    const SQRT: u8 = 0x9F;
    const ADD: u8 = 0xA0;
    const SUB: u8 = 0xA1;
    const MUL: u8 = 0xA2;
    const DIV: u8 = 0xA3;
    const MIN: u8 = 0xA4;
    const MAX: u8 = 0xA5;

    fn push_wasm_const(self, buf: &mut Vec<u8>) {
        buf.push(0x44);
        buf.extend_from_slice(&self.to_bits().to_le_bytes());
    }
}
impl WasmScalar for f64 {}

impl WasmSealed for f32 {
    const VAL_TYPE: u8 = 0x7D;
    const ABS: u8 = 0x8B;
    const NEG: u8 = 0x8C;
    const CEIL: u8 = 0x8D;
    const FLOOR: u8 = 0x8E;
    const TRUNC: u8 = 0x8F;
    const SQRT: u8 = 0x91;
    const ADD: u8 = 0x92;
    const SUB: u8 = 0x93;
    const MUL: u8 = 0x94;
    const DIV: u8 = 0x95;
    const MIN: u8 = 0x96;
    const MAX: u8 = 0x97;

    fn push_wasm_const(self, buf: &mut Vec<u8>) {
        buf.push(0x43);
        buf.extend_from_slice(&self.to_bits().to_le_bytes());
    }
}
impl WasmScalar for f32 {}

impl<Value: WasmScalar> Program<Value> {
    /// Emit a standalone WebAssembly (`.wasm`) binary module for this program.
    ///
    /// The returned bytes begin with the standard `\0asm\x01\0\0\0` header and
    /// export `"eval"` taking one `f64` or `f32` argument per parameter and
    /// returning the scalar result. Constants are embedded inline via
    /// `f64.const`/`f32.const` and temporaries are stack-scheduled or compacted
    /// into WebAssembly locals, so no linear memory import is required.
    ///
    /// # Host `"math"` Import ABI
    ///
    /// Pure arithmetic (`+`, `-`, `*`, `/`, negation) and `abs`, `ceil`,
    /// `floor`, `trunc`, `sqrt`, `min`, `max`, `clamp`, `re`, `im`, `conj`
    /// emit inline WebAssembly instructions and require zero imports. `round`
    /// is imported (rather than lowered to `f64.nearest` / `f32.nearest`) to
    /// preserve round-half-away-from-zero semantics. Every other builtin and
    /// non-integer `pow` is imported from module `"math"` under its canonical
    /// display name in symbolic expression argument order.
    ///
    /// # Errors
    /// Returns [`EvalError::ProgramTooLarge`] if local, parameter, or section
    /// counts exceed 32-bit WebAssembly capacity, or [`EvalError::InvalidProgram`]
    /// if the register layout is inconsistent.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let program = Program::<f64>::compile(&(x.pow(2) + 3), &[x])?;
    /// let wasm = program.to_wasm_bytes()?;
    /// assert_eq!(&wasm[..8], b"\0asm\x01\0\0\0");
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn to_wasm_bytes(&self) -> Result<Vec<u8>, EvalError> {
        WasmEmitter::emit(&self.data)
    }
}

impl<Value: WasmScalar> MultiProgram<Value> {
    /// Emit a standalone WebAssembly (`.wasm`) binary module for this multi-output program.
    ///
    /// Exports `"eval"` with one floating-point argument per parameter and one
    /// multi-value return entry per compiled output (`(param f64|f32 ...) (result f64|f32 ...)`),
    /// using the same `"math"` host import ABI as [`Program::to_wasm_bytes`].
    ///
    /// # Errors
    /// Returns [`EvalError::ProgramTooLarge`] if local, parameter, result, or section
    /// counts exceed 32-bit WebAssembly capacity, or [`EvalError::InvalidProgram`]
    /// if the register layout is inconsistent.
    ///
    /// ```
    /// use symb_anafis::{Context, MultiProgram};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let system = MultiProgram::<f64>::compile(&[x + 1, x * 2], &[x])?;
    /// let wasm = system.to_wasm_bytes()?;
    /// assert_eq!(&wasm[..8], b"\0asm\x01\0\0\0");
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn to_wasm_bytes(&self) -> Result<Vec<u8>, EvalError> {
        WasmEmitter::emit(&self.data)
    }
}

struct WasmEmitter<'prog, Value: WasmScalar> {
    program: &'prog ProgramData<Value>,
    body: Vec<u8>,
    imports: Vec<(&'static str, u8)>,
    temp_local: Vec<u32>,
    n_params: usize,
    prefix: usize,
    on_stack: Option<u32>,
}

impl<'prog, Value: WasmScalar> WasmEmitter<'prog, Value> {
    fn emit(program: &'prog ProgramData<Value>) -> Result<Vec<u8>, EvalError> {
        let n_params = program.parameters.len();
        let prefix = n_params
            .checked_add(program.constants.len())
            .ok_or(EvalError::ProgramTooLarge)?;
        let raw_temps = program
            .register_count
            .checked_sub(prefix)
            .ok_or(EvalError::InvalidProgram)?;

        let def_uses = Self::compute_def_uses(program);
        let mut needs_local = vec![false; raw_temps];
        for (idx, instruction) in program.instructions.iter().enumerate() {
            if let Operation::Pair(_, _, second) = instruction.operation {
                let sec_temp = usize::try_from(second)
                    .ok()
                    .and_then(|s| s.checked_sub(prefix))
                    .ok_or(EvalError::InvalidProgram)?;
                if let Some(flag) = needs_local.get_mut(sec_temp) {
                    *flag = true;
                }
            }
            let uses = def_uses.get(idx).copied().unwrap_or(0);
            if uses != 0
                && !(Some(instruction.output) == Self::next_first_read(program, idx) && uses == 1)
            {
                let out_temp = usize::try_from(instruction.output)
                    .ok()
                    .and_then(|o| o.checked_sub(prefix))
                    .ok_or(EvalError::InvalidProgram)?;
                if let Some(flag) = needs_local.get_mut(out_temp) {
                    *flag = true;
                }
            }
        }

        let mut temp_local = vec![u32::MAX; raw_temps];
        let mut n_temps = 0_usize;
        for (slot, &needed) in temp_local.iter_mut().zip(&needs_local) {
            if needed {
                *slot = n_params
                    .checked_add(n_temps)
                    .and_then(|id| u32::try_from(id).ok())
                    .ok_or(EvalError::ProgramTooLarge)?;
                n_temps = n_temps.saturating_add(1);
            }
        }
        let total_locals = n_params
            .checked_add(n_temps)
            .ok_or(EvalError::ProgramTooLarge)?;
        let _ = u32::try_from(total_locals).map_err(|_overflow| EvalError::ProgramTooLarge)?;

        let mut body = Vec::with_capacity(
            program
                .instructions
                .len()
                .saturating_mul(10)
                .saturating_add(16),
        );
        if n_temps == 0 {
            Self::push_leb_u32(&mut body, 0);
        } else {
            Self::push_leb_u32(&mut body, 1);
            Self::push_leb_u32(
                &mut body,
                u32::try_from(n_temps).map_err(|_overflow| EvalError::ProgramTooLarge)?,
            );
            body.push(Value::VAL_TYPE);
        }

        let mut emitter = Self {
            program,
            body,
            imports: Vec::new(),
            temp_local,
            n_params,
            prefix,
            on_stack: None,
        };
        for (idx, instruction) in program.instructions.iter().enumerate() {
            let uses = def_uses.get(idx).copied().unwrap_or(0);
            emitter.emit_instruction(instruction, uses, Self::next_first_read(program, idx))?;
        }
        for &result in &program.results {
            emitter.emit_read(result)?;
        }
        if emitter.on_stack.is_some() {
            return Err(EvalError::InvalidProgram);
        }
        emitter.body.push(OP_END);
        emitter.finish_module()
    }

    fn emit_instruction(
        &mut self,
        instruction: &Instruction,
        out_uses: u32,
        next_first: Option<u32>,
    ) -> Result<(), EvalError> {
        match &instruction.operation {
            Operation::Combine(kind, [a, b]) => {
                self.emit_read(*a)?;
                self.emit_read(*b)?;
                self.body.push(match kind {
                    Combine::Add => Value::ADD,
                    Combine::Subtract => Value::SUB,
                    Combine::Multiply => Value::MUL,
                });
            }
            Operation::Divide([a, b]) => {
                self.emit_read(*a)?;
                self.emit_read(*b)?;
                self.body.push(Value::DIV);
            }
            Operation::Negate([a]) => {
                self.emit_read(*a)?;
                self.body.push(Value::NEG);
            }
            Operation::NegatedMultiply([a, b]) => {
                self.emit_read(*a)?;
                self.emit_read(*b)?;
                self.body.extend_from_slice(&[Value::MUL, Value::NEG]);
            }
            Operation::Fused(fused, [a, b, c]) => {
                self.emit_read(*a)?;
                self.emit_read(*b)?;
                self.body.push(Value::MUL);
                if fused.negate_product {
                    self.body.push(Value::NEG);
                }
                self.emit_read(*c)?;
                self.body.push(if fused.negate_addend {
                    Value::SUB
                } else {
                    Value::ADD
                });
            }
            Operation::Fold(fold, span) => self.emit_fold_value(*fold, *span)?,
            Operation::Power(args) => {
                let fn_idx = self.resolve_import("pow", 2)?;
                self.emit_call_push(fn_idx, args)?;
            }
            Operation::Unary(builtin, [a]) => self.emit_unary(*builtin, *a)?,
            Operation::Binary(builtin, args) => self.emit_binary(*builtin, *args)?,
            Operation::Ternary(builtin, args) => {
                let fn_idx = self.resolve_import(builtin.as_str(), 3)?;
                self.emit_call_push(fn_idx, args)?;
            }
            Operation::Pair(paired, args, second) => {
                let (n1, n2) = match paired {
                    PairedFunction::SinCos => ("sin", "cos"),
                    PairedFunction::SinhCosh => ("sinh", "cosh"),
                };
                let fn1 = self.resolve_import(n1, 1)?;
                let fn2 = self.resolve_import(n2, 1)?;
                self.emit_call_push(fn2, args)?;
                self.emit_local_op(OP_LOCAL_SET, *second)?;
                self.emit_call_push(fn1, args)?;
            }
        }
        let out = instruction.output;
        if out_uses == 0 {
            self.body.push(OP_DROP);
            return Ok(());
        }
        if Some(out) == next_first {
            if out_uses > 1 {
                self.emit_local_op(OP_LOCAL_TEE, out)?;
            }
            self.on_stack = Some(out);
            return Ok(());
        }
        self.emit_local_op(OP_LOCAL_SET, out)
    }

    fn emit_unary(&mut self, builtin: BuiltinFunction, arg: u32) -> Result<(), EvalError> {
        let inline_op = match InlineUnary::of(builtin) {
            Some(InlineUnary::Abs) => Some(Value::ABS),
            Some(InlineUnary::Ceil) => Some(Value::CEIL),
            Some(InlineUnary::Floor) => Some(Value::FLOOR),
            Some(InlineUnary::Trunc) => Some(Value::TRUNC),
            Some(InlineUnary::Sqrt) => Some(Value::SQRT),
            // `f64.nearest` rounds half-to-even rather than half-away-from-zero.
            Some(InlineUnary::Round) | None => None,
        };
        if let Some(op_byte) = inline_op {
            self.emit_read(arg)?;
            self.body.push(op_byte);
            Ok(())
        } else {
            let fn_idx = self.resolve_import(builtin.as_str(), 1)?;
            self.emit_call_push(fn_idx, &[arg])
        }
    }

    fn emit_binary(&mut self, builtin: BuiltinFunction, args: [u32; 2]) -> Result<(), EvalError> {
        if let Some(inline_op) = InlineBinary::of(builtin) {
            self.emit_read(args[0])?;
            self.emit_read(args[1])?;
            self.body.push(match inline_op {
                InlineBinary::Max => Value::MAX,
                InlineBinary::Min => Value::MIN,
            });
            Ok(())
        } else {
            let fn_idx = self.resolve_import(builtin.as_str(), 2)?;
            self.emit_call_push(fn_idx, &args)
        }
    }

    fn emit_fold_value(&mut self, fold: Fold, span: OperandSpan) -> Result<(), EvalError> {
        let ops = span
            .range()
            .and_then(|range| self.program.operands.get(range))
            .ok_or(EvalError::InvalidProgram)?;
        match fold {
            Fold::Sum | Fold::Product => {
                let &[first, ref rest @ ..] = ops else {
                    return Err(EvalError::InvalidProgram);
                };
                let op_byte = match fold {
                    Fold::Product => Value::MUL,
                    Fold::Sum | Fold::Dot => Value::ADD,
                };
                self.emit_read(first)?;
                for &op in rest {
                    self.emit_read(op)?;
                    self.body.push(op_byte);
                }
            }
            Fold::Dot => {
                let &[l0, r0, ref rest @ ..] = ops else {
                    return Err(EvalError::InvalidProgram);
                };
                self.emit_read(l0)?;
                self.emit_read(r0)?;
                self.body.push(Value::MUL);
                for &[lhs, rhs] in rest.as_chunks::<2>().0 {
                    self.emit_read(lhs)?;
                    self.emit_read(rhs)?;
                    self.body.extend_from_slice(&[Value::MUL, Value::ADD]);
                }
            }
        }
        Ok(())
    }

    fn emit_call_push(&mut self, fn_idx: u32, args: &[u32]) -> Result<(), EvalError> {
        for &arg in args {
            self.emit_read(arg)?;
        }
        self.body.push(OP_CALL);
        Self::push_leb_u32(&mut self.body, fn_idx);
        Ok(())
    }

    fn emit_read(&mut self, reg: u32) -> Result<(), EvalError> {
        if self.on_stack == Some(reg) {
            self.on_stack = None;
            return Ok(());
        }
        if self.on_stack.is_some() {
            return Err(EvalError::InvalidProgram);
        }
        let reg_usize = usize::try_from(reg).map_err(|_overflow| EvalError::InvalidProgram)?;
        if reg_usize < self.n_params {
            self.body.push(OP_LOCAL_GET);
            Self::push_leb_u32(&mut self.body, reg);
        } else if reg_usize < self.prefix {
            let const_idx = reg_usize
                .checked_sub(self.n_params)
                .ok_or(EvalError::InvalidProgram)?;
            let value = self
                .program
                .constants
                .get(const_idx)
                .ok_or(EvalError::InvalidProgram)?;
            value.push_wasm_const(&mut self.body);
        } else {
            self.emit_local_op(OP_LOCAL_GET, reg)?;
        }
        Ok(())
    }

    fn emit_local_op(&mut self, op: u8, reg: u32) -> Result<(), EvalError> {
        let temp_idx = usize::try_from(reg)
            .ok()
            .and_then(|r| r.checked_sub(self.prefix))
            .ok_or(EvalError::InvalidProgram)?;
        let local_id = self
            .temp_local
            .get(temp_idx)
            .copied()
            .filter(|&id| id != u32::MAX)
            .ok_or(EvalError::InvalidProgram)?;
        self.body.push(op);
        Self::push_leb_u32(&mut self.body, local_id);
        Ok(())
    }

    fn resolve_import(&mut self, name: &'static str, arity: u8) -> Result<u32, EvalError> {
        if let Some((pos, &(_, existing_arity))) = self
            .imports
            .iter()
            .enumerate()
            .find(|&(_, &(existing, _))| existing == name)
        {
            if existing_arity != arity {
                return Err(EvalError::InvalidProgram);
            }
            return u32::try_from(pos).map_err(|_overflow| EvalError::ProgramTooLarge);
        }
        let idx =
            u32::try_from(self.imports.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        self.imports.push((name, arity));
        Ok(idx)
    }

    /// Assemble the Type, Import, Function, Export, and Code sections around `self.body`.
    fn finish_module(&self) -> Result<Vec<u8>, EvalError> {
        let mut wasm = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let mut signatures = Vec::new();
        for &(_, arity) in &self.imports {
            let sig = (usize::from(arity), 1_usize);
            if !signatures.contains(&sig) {
                signatures.push(sig);
            }
        }
        let eval_sig = (self.n_params, self.program.results.len());
        let eval_type_idx = if let Some(pos) = signatures.iter().position(|&sig| sig == eval_sig) {
            u32::try_from(pos).map_err(|_overflow| EvalError::ProgramTooLarge)?
        } else {
            let idx =
                u32::try_from(signatures.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?;
            signatures.push(eval_sig);
            idx
        };

        let mut sec = Vec::new();
        Self::push_leb_u32(
            &mut sec,
            u32::try_from(signatures.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?,
        );
        for &(param_count, result_count) in &signatures {
            sec.push(0x60);
            Self::push_leb_u32(
                &mut sec,
                u32::try_from(param_count).map_err(|_overflow| EvalError::ProgramTooLarge)?,
            );
            sec.resize(sec.len().saturating_add(param_count), Value::VAL_TYPE);
            Self::push_leb_u32(
                &mut sec,
                u32::try_from(result_count).map_err(|_overflow| EvalError::ProgramTooLarge)?,
            );
            sec.resize(sec.len().saturating_add(result_count), Value::VAL_TYPE);
        }
        Self::push_section(&mut wasm, 1, &sec)?;

        if !self.imports.is_empty() {
            sec.clear();
            Self::push_leb_u32(
                &mut sec,
                u32::try_from(self.imports.len())
                    .map_err(|_overflow| EvalError::ProgramTooLarge)?,
            );
            for &(name, arity) in &self.imports {
                let sig = (usize::from(arity), 1_usize);
                let type_idx = signatures
                    .iter()
                    .position(|&candidate| candidate == sig)
                    .ok_or(EvalError::InvalidProgram)?;
                Self::push_leb_u32(&mut sec, 4);
                sec.extend_from_slice(b"math");
                Self::push_leb_u32(
                    &mut sec,
                    u32::try_from(name.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?,
                );
                sec.extend_from_slice(name.as_bytes());
                sec.push(0x00);
                Self::push_leb_u32(
                    &mut sec,
                    u32::try_from(type_idx).map_err(|_overflow| EvalError::ProgramTooLarge)?,
                );
            }
            Self::push_section(&mut wasm, 2, &sec)?;
        }

        sec.clear();
        Self::push_leb_u32(&mut sec, 1);
        Self::push_leb_u32(&mut sec, eval_type_idx);
        Self::push_section(&mut wasm, 3, &sec)?;

        sec.clear();
        Self::push_leb_u32(&mut sec, 1);
        Self::push_leb_u32(&mut sec, 4);
        sec.extend_from_slice(b"eval");
        sec.push(0x00);
        Self::push_leb_u32(
            &mut sec,
            u32::try_from(self.imports.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?,
        );
        Self::push_section(&mut wasm, 7, &sec)?;

        sec.clear();
        Self::push_leb_u32(&mut sec, 1);
        Self::push_leb_u32(
            &mut sec,
            u32::try_from(self.body.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?,
        );
        sec.extend_from_slice(&self.body);
        Self::push_section(&mut wasm, 10, &sec)?;

        Ok(wasm)
    }

    fn push_section(wasm: &mut Vec<u8>, id: u8, payload: &[u8]) -> Result<(), EvalError> {
        wasm.push(id);
        Self::push_leb_u32(
            wasm,
            u32::try_from(payload.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?,
        );
        wasm.extend_from_slice(payload);
        Ok(())
    }

    fn push_leb_u32(buf: &mut Vec<u8>, mut value: u32) {
        loop {
            let mut byte = u8::try_from(value & 0x7F).unwrap_or(0);
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    /// Exact remaining WASM operand reads of each instruction's primary output
    /// before that register is redefined or returned.
    fn compute_def_uses(program: &ProgramData<Value>) -> Vec<u32> {
        let mut live_reads = vec![0_u32; program.register_count];
        for &res in &program.results {
            if let Some(slot) = live_reads.get_mut(usize::try_from(res).unwrap_or(usize::MAX)) {
                *slot = slot.saturating_add(1);
            }
        }
        let mut def_uses = vec![0_u32; program.instructions.len()];
        for (idx, inst) in program.instructions.iter().enumerate().rev() {
            let out_idx = usize::try_from(inst.output).unwrap_or(usize::MAX);
            if let Some(slot) = live_reads.get_mut(out_idx) {
                if let Some(du) = def_uses.get_mut(idx) {
                    *du = *slot;
                }
                *slot = 0;
            }
            if let Operation::Pair(_, [inp], second) = inst.operation {
                if let Some(sec_slot) =
                    live_reads.get_mut(usize::try_from(second).unwrap_or(usize::MAX))
                {
                    *sec_slot = 0;
                }
                // WASM lowers `Pair` to two separate unary calls (`cos(inp)` and `sin(inp)`).
                if let Some(in_slot) =
                    live_reads.get_mut(usize::try_from(inp).unwrap_or(usize::MAX))
                {
                    *in_slot = in_slot.saturating_add(2);
                }
                continue;
            }
            // SAFETY: `program` passed `validate`, so every `Fold` span lies in `program.operands`.
            for &inp in unsafe { inst.operation.inputs(&program.operands) } {
                if let Some(slot) = live_reads.get_mut(usize::try_from(inp).unwrap_or(usize::MAX)) {
                    *slot = slot.saturating_add(1);
                }
            }
        }
        def_uses
    }

    fn next_first_read(program: &ProgramData<Value>, idx: usize) -> Option<u32> {
        program.instructions.get(idx.saturating_add(1)).map_or_else(
            || program.results.first().copied(),
            |next_inst| {
                // SAFETY: `program` passed `validate`.
                let inputs = unsafe { next_inst.operation.inputs(&program.operands) };
                inputs.first().copied()
            },
        )
    }
}
