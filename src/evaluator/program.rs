//! Shared numerical programs with independently owned execution workspaces.

#![expect(
    clippy::self_named_module_files,
    reason = "the compiled program owns its compiler and execution implementations"
)]

use core::{iter::repeat_n, ops::Range};

use alloc::{sync::Arc, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Scalar, SymbolData},
};

use super::Traversal;

mod compile;
mod disassemble;
mod execute;
mod floating;
mod optimize;
mod registers;

/// An immutable compiled scalar numerical program.
///
/// A program owns instructions, pooled operands, parameter metadata, stored
/// constants, and the result register. It contains no mutable execution state
/// and can be shared through `Arc` or cloned cheaply.
///
/// Concurrent evaluations require one [`Workspace`] per thread. A workspace
/// can be reused across programs and owns no reference to the expression tree.
///
/// `Number` preserves exact arithmetic and complex promotion. `f64` executes
/// native real arithmetic. `Real`, `Complex`, and custom [`super::EvaluationValue`]
/// implementations use the same compiler. Functions follow `NumAnaFis` domains.
/// Native floating programs use algebraic reassociation and efficient FMA;
/// rounding, intermediate overflow, and signed zeros may differ from direct DAG
/// evaluation. Finite nonzero constant divisors use multiplication when their
/// rounded reciprocal is finite and nonzero. Generic numerical programs retain
/// backend operation semantics.
/// No execution path constructs expressions, interns names, or expands bodies.
///
/// ```
/// use symb_anafis::{Context, Program};
/// let context = Context::new()?;
/// let x = context.symb("x")?;
/// let expression = x.pow(2) + 1;
/// let program = Program::<f64>::compile(&expression, &[x])?;
/// let mut workspace = program.workspace();
/// assert_eq!(program.evaluate(&mut workspace, &[3.0])?, 10.0);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Program<Value = Number> {
    data: Arc<ProgramData<Value>>,
}

/// Mutable register storage for one numerical execution.
///
/// A workspace can be rebound to another program; evaluation copies that
/// program's constants and reuses the existing allocation when its capacity is
/// sufficient. Each concurrent execution must own a distinct workspace.
///
/// ```
/// use symb_anafis::{Context, Program, Workspace};
/// let context = Context::new()?;
/// let x = context.symb("x")?;
/// let first = Program::<f64>::compile(&(x + 1), &[x])?;
/// let second = Program::<f64>::compile(&(x * 2), &[x])?;
/// let mut workspace = Workspace::new();
/// assert_eq!(first.evaluate(&mut workspace, &[3.0])?, 4.0);
/// assert_eq!(second.evaluate(&mut workspace, &[3.0])?, 6.0);
/// assert_eq!(first.evaluate(&mut workspace, &[4.0])?, 5.0);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Workspace<Value = Number> {
    registers: Vec<Value>,
    /// The bound program and the number of rows held per register.
    program: Option<(Program<Value>, usize)>,
}

/// The rows that one native floating batch instruction evaluates together.
const BLOCK: usize = 16;

#[derive(Clone, Debug)]
struct ProgramData<Value> {
    instructions: Vec<Instruction<Value>>,
    operands: Vec<usize>,
    parameters: Vec<Arc<SymbolData>>,
    result: usize,
    constants: Vec<Value>,
    register_count: usize,
    initial: Value,
}

#[derive(Clone, Debug)]
struct Instruction<Value> {
    output: usize,
    operation: Operation<Value>,
}

/// Lowering state: SSA sources in creation order, their literals, and the
/// program under construction.
///
/// Source invariants, kept by every pass and relied on by unchecked reads:
/// - every input of a computed source, and the producer of a projection, is
///   an earlier source;
/// - every constant source indexes `constants`;
/// - every pooled range of a computed source lies in `program.operands`;
/// - `program.result` is a source, and `unit`, which native floating plans
///   create before lowering, precedes every computed source.
struct Compilation<Value> {
    program: ProgramData<Value>,
    sources: Vec<Source<Value>>,
    constants: Vec<Value>,
    parameters: FixedMap<usize, usize>,
    powers: FixedMap<(usize, u64), usize>,
    /// The denominator sources of the divisions lowered in native floating
    /// plans, each flagged when several divisions share it, until the
    /// reciprocal pass consumes them.
    denominators: FixedMap<usize, bool>,
    unit: Option<usize>,
}

type PairedKernel<Value> = fn(&Value) -> (Value, Value);

#[derive(Clone, Debug)]
enum Operation<Value> {
    Add([usize; 2]),
    Subtract([usize; 2]),
    Negate([usize; 1]),
    Multiply([usize; 2]),
    NegatedMultiply([usize; 2]),
    /// A left-to-right fold of pooled operands: at least three, or for
    /// [`Fold::Dot`] at least two pairs. The fold kind is a separate field, so
    /// execution branches on it apart from the operation and the operation
    /// dispatch keeps few arms.
    Fold(Fold, Range<usize>),
    Divide([usize; 2]),
    Power([usize; 2]),
    Fused([usize; 3]),
    FusedSubtract([usize; 3]),
    NegatedFusedAdd([usize; 3]),
    NegatedFusedSubtract([usize; 3]),
    Unary(BuiltinFunction, fn(&Value) -> Value, [usize; 1]),
    Pair(PairedFunction, PairedKernel<Value>, [usize; 1], usize),
    Binary(BuiltinFunction, fn(&Value, &Value) -> Value, [usize; 2]),
    Ternary(
        BuiltinFunction,
        fn(&Value, &Value, &Value) -> Value,
        [usize; 3],
    ),
}

/// The combination applied by [`Operation::Fold`]. Execution runs every kind
/// inline; with at most three kinds the branch on the kind stays a chain of
/// compares rather than a jump table.
#[derive(Clone, Copy, Debug)]
enum Fold {
    Sum,
    Product,
    /// The sum of the products of consecutive operand pairs, from left to
    /// right, each product rounded before it is added.
    Dot,
}

enum Source<Value> {
    Parameter(usize),
    Constant(usize),
    Computed(Operation<Value>),
    Projection(usize),
}

#[derive(Clone, Copy, Debug)]
enum PairedFunction {
    SinCos,
    SinhCosh,
}

impl<Value> Workspace<Value> {
    /// Create an empty workspace. The first evaluation initializes it for the
    /// selected program.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registers: Vec::new(),
            program: None,
        }
    }
}

impl<Value> Default for Workspace<Value> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Value: Scalar> Workspace<Value> {
    /// Bind the workspace to `program` with `LANES` rows per register and
    /// return its `register_count` initialized register blocks, the constants
    /// broadcast directly after the parameters.
    ///
    /// The binding is cleared before the registers change, so a panicking
    /// clone leaves an unbound workspace that the next call rebuilds.
    fn prepare<const LANES: usize>(&mut self, program: &Program<Value>) -> &mut [[Value; LANES]] {
        if !self.program.as_ref().is_some_and(|(bound, lanes)| {
            *lanes == LANES && Arc::ptr_eq(&bound.data, &program.data)
        }) {
            self.program = None;
            let data = &program.data;
            // Compilation bounds the bytes of `register_count` blocks of
            // `BLOCK` values, so these counts do not saturate.
            self.registers.clear();
            self.registers.resize(
                data.parameters.len().saturating_mul(LANES),
                data.initial.clone(),
            );
            self.registers.extend(
                data.constants
                    .iter()
                    .flat_map(|constant| repeat_n(constant, LANES))
                    .cloned(),
            );
            self.registers.resize(
                data.register_count.saturating_mul(LANES),
                data.initial.clone(),
            );
            self.program = Some((program.clone(), LANES));
        }
        self.registers.as_chunks_mut().0
    }
}

#[cfg(test)]
mod tests;
