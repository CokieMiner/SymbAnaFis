//! Shared numerical programs with independently owned execution workspaces.

#![expect(
    clippy::self_named_module_files,
    reason = "the compiled program owns its compiler and execution implementations"
)]

use core::ops::Range;

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Symbol, SymbolData},
};

use super::{EvaluationValue, Traversal};

use batch::BatchTarget;
use intrinsics::{InlineBinary, InlineUnary};
use native::NativeSlot;

mod batch;
mod compile;
mod disassemble;
mod intrinsics;
mod native;
mod vm;
mod wasm;

pub use native::NativeScalar;
pub use wasm::WasmScalar;

/// An immutable compiled scalar numerical program.
///
/// A program owns instructions, pooled operands, parameter metadata, stored
/// constants, and the result register. It contains no mutable execution state
/// and can be shared through `Arc` or cloned cheaply. Call [`Self::with_native`]
/// to attach a host machine-code JIT backend (with the `native_jit` feature), or
/// [`Self::to_wasm_bytes`] to export a standalone WebAssembly module.
///
/// Concurrent evaluations require one [`Workspace`] per thread. A workspace
/// can be reused across programs and owns no reference to the expression tree.
///
/// `Number` preserves exact arithmetic and complex promotion. `f64` and `f32`
/// execute native real arithmetic; `Complex<f64>`, `Complex<f32>`, `Real`,
/// `Complex`, and custom [`super::EvaluationValue`] implementations use the
/// same compiler. Functions follow `NumAnaFis` domains.
/// Native floating programs use algebraic reassociation and efficient FMA;
/// rounding, intermediate overflow, and signed zeros may differ from direct DAG
/// evaluation. Finite nonzero constant divisors use multiplication when their
/// rounded reciprocal is finite and nonzero. Generic numerical programs retain
/// backend operation semantics.
/// No execution path constructs expressions, interns names, or expands bodies.
///
/// ```
/// use symb_anafis::{Context, Program, Workspace};
/// let context = Context::new()?;
/// let x = context.symb("x")?;
/// let expression = x.pow(2) + 1;
/// let program = Program::<f64>::compile(&expression, &[x])?;
/// let mut workspace = Workspace::new();
/// assert_eq!(program.eval_with(&mut workspace, &[3.0])?, 10.0);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Program<Value = Number> {
    data: Arc<ProgramData<Value>>,
    native: NativeSlot<Value>,
}

/// An immutable compiled multi-output numerical program.
///
/// Compiles a slice of expressions over one shared parameter order into a single
/// register program so common subexpressions, constants, shared denominators,
/// and paired trigonometric calls (`sin`/`cos`, `sinh`/`cosh`) are computed once
/// across all outputs. Shares [`Workspace`] storage, host native JIT compilation
/// ([`Self::with_native`]), standalone WebAssembly export ([`Self::to_wasm_bytes`]),
/// and Rayon batch parallelism with [`Program`].
///
/// ```
/// use symb_anafis::{Context, MultiProgram};
/// let context = Context::new()?;
/// let x = context.symb("x")?;
/// let y = context.symb("y")?;
/// let system = MultiProgram::<f64>::compile(&[x.sin() + y, x.cos() - y], &[x, y])?;
/// let out = system.eval(&[0.0, 2.0])?;
/// assert_eq!(out, vec![2.0, -1.0]);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct MultiProgram<Value = Number> {
    data: Arc<ProgramData<Value>>,
    native: NativeSlot<Value>,
}

/// Mutable register storage for one numerical execution.
///
/// A workspace can be rebound across programs (both VM and native-backed);
/// evaluation copies that program's constants and reuses the existing
/// allocation when its capacity is sufficient. Switching between one row and a
/// batch keeps the widest register file it has served, so the switch rewrites
/// only the parameter and constant slots. Each concurrent execution must own a
/// distinct workspace.
///
/// ```
/// use symb_anafis::{Context, Program, Workspace};
/// let context = Context::new()?;
/// let x = context.symb("x")?;
/// let first = Program::<f64>::compile(&(x + 1), &[x])?;
/// let second = Program::<f64>::compile(&(x * 2), &[x])?;
/// let mut workspace = Workspace::new();
/// assert_eq!(first.eval_with(&mut workspace, &[3.0])?, 4.0);
/// assert_eq!(second.eval_with(&mut workspace, &[3.0])?, 6.0);
/// assert_eq!(first.eval_with(&mut workspace, &[4.0])?, 5.0);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Workspace<Value = Number> {
    registers: Vec<Value>,
    /// The bound program metadata.
    program: Option<Arc<ProgramData<Value>>>,
    /// The rows per register the fixed prefix currently holds, or zero when it
    /// holds none.
    lanes: usize,
}

/// The rows that one blocked batch instruction evaluates together.
const BLOCK: usize = 16;

const _: () = assert!(
    BLOCK.is_power_of_two(),
    "block width must be a power of two for mask alignment"
);

/// Maximum register count evaluated directly on the call stack in transient
/// one-shot `eval` calls without heap allocation.
const STACK_REGS: usize = 32;

/// The largest number of operands that one pooled instruction reads.
const GROUP: usize = 16;

#[derive(Clone, Debug)]
struct ProgramData<Value> {
    instructions: Box<[Instruction]>,
    operands: Box<[u32]>,
    parameters: Box<[Arc<SymbolData>]>,
    active_parameters: Box<[u32]>,
    results: Box<[u32]>,
    constants: Box<[Value]>,
    register_count: usize,
    initial: Value,
}

#[derive(Clone, Copy, Debug)]
struct Instruction {
    output: u32,
    operation: Operation,
}

/// Contiguous half-open span `start..end` inside `ProgramData::operands`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OperandSpan {
    start: u32,
    end: u32,
}

impl OperandSpan {
    fn range(self) -> Option<Range<usize>> {
        (self.start <= self.end).then(|| {
            usize::try_from(self.start).unwrap_or(0)..usize::try_from(self.end).unwrap_or(0)
        })
    }
}

/// Lowering state: SSA sources in creation order, their literals, and the
/// program tables under construction.
///
/// Source invariants, kept by every pass and relied on by unchecked reads:
/// - every input of a computed source, and the producer of a projection, is
///   an earlier source (`< sources.len() <= u32::MAX`);
/// - every constant source indexes `constants`;
/// - every pooled span of a computed source lies in `operands`;
/// - every entry in `results` is a source, and `unit`, which native floating
///   plans create before lowering, precedes every computed source.
struct Compilation<Value> {
    operands: Vec<u32>,
    parameters: Box<[Arc<SymbolData>]>,
    results: Vec<u32>,
    sources: Vec<Source>,
    constants: Vec<Value>,
    parameter_map: FixedMap<usize, u32>,
    powers: FixedMap<(u32, u64), u32>,
    constant_reciprocals: FixedMap<u32, u32>,
    constant_bits: FixedMap<u64, u32>,
    exponent_constants: FixedMap<u32, u32>,
    denominators: FixedMap<u32, bool>,
    binary: FixedMap<(bool, u32, u32), u32>,
    fused: FixedMap<(u8, u32, u32, u32), u32>,
    folds: FixedMap<(bool, [u32; GROUP], u8), u32>,
    unit: Option<u32>,
    initial: Value,
    trig_mask: u8,
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    /// A binary product, sum, or difference.
    Combine(Combine, [u32; 2]),
    Negate([u32; 1]),
    NegatedMultiply([u32; 2]),
    /// A left-to-right fold of pooled operands: at least three, or for
    /// [`Fold::Dot`] at least two pairs.
    Fold(Fold, OperandSpan),
    Divide([u32; 2]),
    Power([u32; 2]),
    /// A fused multiply-add $(\pm x) y \pm z$.
    Fused(Fused, [u32; 3]),
    Unary(BuiltinFunction, [u32; 1]),
    Pair(PairedFunction, [u32; 1], u32),
    Binary(BuiltinFunction, [u32; 2]),
    Ternary(BuiltinFunction, [u32; 3]),
}

/// The binary arithmetic operation applied by [`Operation::Combine`].
#[derive(Clone, Copy, Debug)]
enum Combine {
    Multiply,
    Add,
    Subtract,
}

/// The combination applied by [`Operation::Fold`].
#[derive(Clone, Copy, Debug)]
enum Fold {
    Sum,
    Product,
    /// Left-to-right sum of consecutive operand products, fused via `mul_add`
    /// when [`ProgramData::fuse`] holds and rounded per product otherwise.
    Dot,
}

/// The signs applied by [`Operation::Fused`], computing $(\pm x) y \pm z$.
#[derive(Clone, Copy, Debug)]
struct Fused {
    negate_product: bool,
    negate_addend: bool,
}

#[derive(Clone, Copy, Debug)]
enum Source {
    Parameter(u32),
    Constant(u32),
    Computed(Operation),
    Projection(u32),
}

#[derive(Clone, Copy, Debug)]
enum PairedFunction {
    SinCos,
    SinhCosh,
}

impl<Value> Program<Value> {
    /// Inspect retained symbol identities in their numerical input order.
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = Symbol<'_>> + '_ {
        self.data.parameters.iter().map(|data| data.symbol())
    }

    /// The number of initialized scalar slots required by this program.
    #[must_use]
    pub fn workspace_size(&self) -> usize {
        self.data.register_count
    }

    /// The number of reachable instructions after shared-node reuse.
    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.data.instructions.len()
    }

    /// Borrow the position-independent machine-code bytes when this program is
    /// backed by a native JIT kernel created via [`Self::with_native`].
    #[must_use]
    pub fn native_bytes(&self) -> Option<&[u8]> {
        self.native.as_bytes()
    }
}

impl<Value> MultiProgram<Value> {
    /// Inspect retained symbol identities in their numerical input order.
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = Symbol<'_>> + '_ {
        self.data.parameters.iter().map(|data| data.symbol())
    }

    /// The number of compiled output expressions produced per evaluation row.
    #[must_use]
    pub fn output_count(&self) -> usize {
        self.data.results.len()
    }

    /// The number of initialized scalar slots required by this program.
    #[must_use]
    pub fn workspace_size(&self) -> usize {
        self.data.register_count
    }

    /// The number of reachable instructions after shared-node reuse across all outputs.
    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.data.instructions.len()
    }

    /// Borrow the position-independent machine-code bytes when this program is
    /// backed by a native JIT kernel created via [`Self::with_native`].
    #[must_use]
    pub fn native_bytes(&self) -> Option<&[u8]> {
        self.native.as_bytes()
    }
}

impl<Value> Workspace<Value> {
    /// Create an empty workspace. The first evaluation initializes it for the
    /// selected program.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registers: Vec::new(),
            program: None,
            lanes: 0,
        }
    }
}

impl<Value> Default for Workspace<Value> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Value: EvaluationValue> Workspace<Value> {
    /// Bind the workspace to `data` with `LANES` rows per register and return
    /// its register blocks, with constants broadcast directly after parameters.
    fn prepare<const LANES: usize>(
        &mut self,
        data: &Arc<ProgramData<Value>>,
    ) -> &mut [[Value; LANES]] {
        let bound = self
            .program
            .as_ref()
            .is_some_and(|bound| Arc::ptr_eq(bound, data));
        if !bound {
            self.program = None;
            self.registers.clear();
            self.lanes = 0;
        }
        let total = data.register_count.saturating_mul(LANES);
        if self.registers.len() < total {
            self.registers.resize(total, data.initial.clone());
        }
        if self.lanes != LANES {
            self.lanes = 0;
            Self::fill_constants::<LANES>(&mut self.registers, data);
            self.lanes = LANES;
        }
        if !bound {
            self.program = Some(Arc::clone(data));
        }
        self.registers.as_chunks_mut().0
    }

    /// Initialize a one-shot workspace for `data` without cloning `Arc<ProgramData>`
    /// so concurrent transient calls never contend on atomic reference counts.
    fn prepare_transient<const LANES: usize>(
        &mut self,
        data: &ProgramData<Value>,
    ) -> &mut [[Value; LANES]] {
        let total = data.register_count.saturating_mul(LANES);
        if self.registers.len() < total {
            self.registers.resize(total, data.initial.clone());
        }
        Self::fill_constants::<LANES>(&mut self.registers, data);
        self.registers.as_chunks_mut().0
    }

    fn fill_constants<const LANES: usize>(registers: &mut [Value], data: &ProgramData<Value>) {
        let parameter_slots = data.parameters.len().saturating_mul(LANES);
        if let Some(constants) = registers.get_mut(parameter_slots..) {
            let mut slots = constants.iter_mut();
            for constant in &data.constants {
                for slot in slots.by_ref().take(LANES) {
                    slot.clone_from(constant);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
