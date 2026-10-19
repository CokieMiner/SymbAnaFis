//! Owning Python objects over the public `SymbAnaFis` facade.

mod callbacks;
mod context;
mod diagnostics;
mod evaluator;
mod expression;
mod function;
mod inputs;
mod number;
mod pattern;
mod rule;
mod sequence;
mod substitutions;
mod uncertainty;

use callbacks::Callbacks;
use context::NativeContext;
use diagnostics::{
    Diagnostics, EvaluationError, ParseError, RewriteError, SimplifyError, UncertaintyError,
};
use evaluator::NativeEvaluator;
use expression::NativeExpr;
use function::{FunctionTemplate, NativeFunction};
use inputs::ExprInputs;
use number::NativeNumber;
use pattern::NativePattern;
use rule::NativeRule;
use sequence::NativeSequence;
use substitutions::NativeSubstitutions;
use uncertainty::{NativeCovariance, NativeUncertainty};

/// Private extension registry; the public Python surface consists of classes.
#[pyo3::pymodule]
mod _native {
    #[pymodule_export]
    use super::{
        EvaluationError, NativeContext, NativeCovariance, NativeEvaluator, NativeExpr,
        NativeFunction, NativeNumber, NativePattern, NativeRule, NativeSequence,
        NativeSubstitutions, NativeUncertainty, ParseError, RewriteError, SimplifyError,
        UncertaintyError,
    };
}
