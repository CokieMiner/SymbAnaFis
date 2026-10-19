//! Shared numerical programs with independently owned execution workspaces.

#![expect(
    clippy::self_named_module_files,
    reason = "the opaque evaluator owns its compiler and execution implementations"
)]

use core::ops::Range;

use alloc::{sync::Arc, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::HashMap,
    core::{BuiltinFunction, SymbolData},
};

use super::Traversal;

/// A scalar numerical program and its reusable register workspace.
///
/// `Number` preserves exact arithmetic and complex promotion. `f64` executes
/// native real arithmetic. `Real`, `Complex`, and custom [`super::EvaluationValue`]
/// implementations use the same compiler. Functions follow `NumAnaFis` domains.
/// Native floating programs use algebraic reassociation and efficient FMA;
/// rounding, intermediate overflow, and signed zeros may differ from direct DAG
/// evaluation. Finite nonzero constant divisors use multiplication when their
/// rounded reciprocal is finite and nonzero. Generic numerical programs retain
/// backend operation semantics.
/// Cloning shares immutable instructions and copies an independent workspace.
/// No execution path constructs expressions, interns names, or expands bodies.
#[derive(Clone, Debug)]
pub struct Evaluator<Value = Number> {
    program: Arc<Program<Value>>,
    registers: Vec<Value>,
}

#[derive(Clone, Debug)]
struct Program<Value> {
    instructions: Vec<Instruction<Value>>,
    operands: Vec<usize>,
    parameters: Vec<Arc<SymbolData>>,
    result: usize,
}

#[derive(Clone, Debug)]
struct Instruction<Value> {
    output: usize,
    operation: Operation<Value>,
}

struct Compilation<Value> {
    program: Program<Value>,
    sources: Vec<Source<Value>>,
    constants: Vec<Value>,
    parameters: HashMap<usize, usize>,
    powers: HashMap<(usize, u64), usize>,
    unit: Option<usize>,
}

type PairedKernel<Value> = fn(&Value) -> (Value, Value);

#[derive(Clone, Debug)]
enum Operation<Value> {
    Add([usize; 2]),
    AddMany(Range<usize>),
    Subtract([usize; 2]),
    Negate([usize; 1]),
    Multiply([usize; 2]),
    MultiplyMany(Range<usize>),
    NegatedMultiply([usize; 2]),
    NegatedMultiplyMany(Range<usize>),
    Divide([usize; 2]),
    Power([usize; 2]),
    Fused([usize; 3]),
    FusedSubtract([usize; 3]),
    NegatedFusedAdd([usize; 3]),
    NegatedFusedSubtract([usize; 3]),
    Unary(BuiltinFunction, fn(&Value) -> Value, [usize; 1]),
    Pair(PairedFunction, PairedKernel<Value>, [usize; 1], usize),
    Binary(BuiltinFunction, fn(&Value, &Value) -> Value, [usize; 2]),
    Ternary(fn(&Value, &Value, &Value) -> Value, [usize; 3]),
    Quaternary(fn(&Value, &Value, &Value, &Value) -> Value, [usize; 4]),
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

mod compile;
mod disassemble;
mod execute;
mod floating;
mod optimize;
mod pairs;
mod registers;
mod schedule;

#[cfg(test)]
mod tests;
