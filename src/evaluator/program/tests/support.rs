//! Test-only convenience pairing an immutable program with one workspace.

use core::fmt::{Debug, Display};

use crate::{
    core::{Expr, Symbol},
    errors::EvalError,
};

use super::super::{EvaluationValue, Program, Workspace};

#[derive(Clone, Debug)]
pub struct Compiled<Value> {
    program: Program<Value>,
    workspace: Workspace<Value>,
}

impl<Value: EvaluationValue> Compiled<Value> {
    pub fn compile(expression: &Expr, parameters: &[Symbol<'_>]) -> Result<Self, EvalError> {
        let program = Program::compile(expression, parameters)?;
        Ok(Self {
            program,
            workspace: Workspace::new(),
        })
    }

    pub fn evaluate(&mut self, parameters: &[Value]) -> Result<Value, EvalError> {
        self.program.eval_with(&mut self.workspace, parameters)
    }

    pub fn evaluate_batch(
        &mut self,
        columns: &[&[Value]],
        output: &mut [Value],
    ) -> Result<(), EvalError> {
        self.program
            .eval_batch_with(&mut self.workspace, columns, output)
    }

    pub fn program(&self) -> &Program<Value> {
        &self.program
    }

    pub fn parameters(&self) -> impl ExactSizeIterator<Item = Symbol<'_>> + '_ {
        self.program.parameters()
    }

    pub fn instruction_count(&self) -> usize {
        self.program.instruction_count()
    }

    pub fn workspace_size(&self) -> usize {
        self.program.workspace_size()
    }
}

impl<Value: EvaluationValue + Debug> Compiled<Value> {
    pub fn disassemble(&self) -> impl Display + '_ {
        self.program.disassemble()
    }
}
