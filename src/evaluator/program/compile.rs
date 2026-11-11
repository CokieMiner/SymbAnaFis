//! Canonical DAG lowering into scalar register instructions.

use core::mem::size_of;

use alloc::{borrow::ToOwned, format, string::ToString, sync::Arc, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Expr, ExprView, Kernel, Scalar, StoredForm, Symbol},
    errors::EvalError,
};

use super::{
    BLOCK, Compilation, Fold, Operation, Program, ProgramData, Source, Traversal, Workspace,
};

/// The largest number of operands that one pooled instruction reads.
const GROUP: usize = 16;

impl<Value: Scalar> Program<Value> {
    /// Compile a scalar DAG with an explicit, identity-based parameter order.
    ///
    /// Shared stored nodes compile once. Builtin signatures dispatch once;
    /// custom bodies expand once per distinct call, after their arguments.
    /// Recursive body definitions are rejected. Generic plans preserve stored
    /// operand order. Native `f64` plans optimize powers, polynomials, signs,
    /// constant and repeated denominators, and fused arithmetic.
    /// No separate simplifier is invoked.
    /// Unused parameters are allowed but must still be supplied at execution.
    ///
    /// The returned program is immutable and may be shared between threads.
    /// Each concurrent execution requires its own [`super::Workspace`].
    ///
    /// # Errors
    /// Returns [`EvalError`] for duplicate parameters, unbound symbols,
    /// unsupported scalar constants, missing or recursive custom bodies,
    /// unevaluated derivatives, or counts exceeding native capacity.
    /// Custom body execution is not time-bounded.
    ///
    /// # Panics
    /// Propagates user callback and numerical backend panics.
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
    pub fn compile(expression: &Expr, parameters: &[Symbol<'_>]) -> Result<Self, EvalError> {
        let zero = Number::zero();
        let initial = Value::literal(&zero).ok_or(EvalError::UnsupportedLiteral { value: zero })?;
        let mut parameter_indices = FixedMap::default();
        parameter_indices.reserve(parameters.len());
        let mut parameter_nodes = Vec::with_capacity(parameters.len());
        for (index, symbol) in parameters.iter().enumerate() {
            if parameter_indices.insert(symbol.id(), index).is_some() {
                return Err(EvalError::DuplicateParameter { index });
            }
            parameter_nodes.push(symbol.metadata());
        }
        let mut compilation = Compilation {
            program: ProgramData {
                instructions: Vec::new(),
                operands: Vec::new(),
                parameters: parameter_nodes,
                result: 0,
                constants: Vec::new(),
                register_count: 0,
                initial,
            },
            sources: Vec::new(),
            constants: Vec::new(),
            parameters: parameter_indices,
            powers: FixedMap::default(),
            denominators: FixedMap::default(),
            unit: None,
        };
        if Value::OPTIMIZE_FLOATS {
            // The unit precedes every computed source, as later passes that
            // pair operands with it require.
            compilation.coefficient(None)?;
        }
        compilation.program.result =
            Traversal::walk(expression, &mut FixedMap::default(), |node, values| {
                compilation.lower(node, values)
            })?;
        if Value::OPTIMIZE_FLOATS {
            compilation.reuse_reciprocals()?;
            compilation.optimize_arithmetic()?;
        }
        compilation.pair_functions();
        parameters
            .len()
            .checked_add(compilation.constants.len())
            .and_then(|prefix| prefix.checked_add(compilation.sources.len()))
            .ok_or(EvalError::ProgramTooLarge)?;
        let register_count = compilation
            .program
            .allocate_registers(&compilation.sources, &mut compilation.constants);
        let prefix = parameters
            .len()
            .checked_add(compilation.constants.len())
            .ok_or(EvalError::ProgramTooLarge)?;
        register_count
            .checked_mul(size_of::<[Value; BLOCK]>())
            .filter(|bytes| *bytes <= isize::MAX.unsigned_abs())
            .ok_or(EvalError::ProgramTooLarge)?;
        if !compilation.program.validate(register_count, prefix) {
            return Err(EvalError::InvalidProgram);
        }
        compilation.program.constants = compilation.constants;
        compilation.program.register_count = register_count;
        Ok(Self {
            data: Arc::new(compilation.program),
        })
    }

    /// Create an independent workspace for this program.
    ///
    /// The workspace allocates the complete register file, including parameter
    /// and constant slots, exactly once. Reuse it for sequential evaluations or
    /// create one per thread for concurrent execution.
    #[must_use]
    pub fn workspace(&self) -> Workspace<Value> {
        let mut workspace = Workspace::new();
        workspace.prepare::<1>(self);
        workspace
    }
}

impl Expr {
    /// Compile this expression into an immutable scalar numerical program.
    ///
    /// # Errors
    /// Returns [`EvalError`] if parameters contain duplicates or symbols are unbound.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("compile_expr_x")?;
    /// let expr = x.pow(2) + 1;
    /// let program = expr.compile::<f64>(&[x])?;
    /// assert_eq!(program.eval(&[3.0])?, 10.0);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn compile<Value: Scalar>(
        &self,
        parameters: &[Symbol<'_>],
    ) -> Result<Program<Value>, EvalError> {
        Program::compile(self, parameters)
    }
}

impl<Value: Scalar> Compilation<Value> {
    fn lower(
        &mut self,
        expression: &Expr,
        values: &FixedMap<Expr, usize>,
    ) -> Result<usize, EvalError> {
        if expression.monomials().is_some() {
            if Value::OPTIMIZE_FLOATS
                && let Some(result) = self.polynomial(expression, values)?
            {
                return Ok(result);
            }
            return self.compact_polynomial(expression, values);
        }
        let source = match expression.view() {
            ExprView::Number(number) => {
                let value =
                    Value::literal(number).ok_or_else(|| EvalError::UnsupportedLiteral {
                        value: number.clone(),
                    })?;
                self.constant(value)
            }
            ExprView::ImaginaryUnit => {
                let unit = Number::i();
                let value =
                    Value::literal(&unit).ok_or(EvalError::UnsupportedLiteral { value: unit })?;
                self.constant(value)
            }
            ExprView::Pi => {
                self.constant(Value::pi().ok_or(EvalError::UnsupportedConstant { name: "pi" })?)
            }
            ExprView::Euler => {
                self.constant(Value::euler().ok_or(EvalError::UnsupportedConstant { name: "e" })?)
            }
            ExprView::Symbol(symbol) => {
                Source::Parameter(*self.parameters.get(&symbol.id()).ok_or_else(|| {
                    EvalError::UnboundSymbol {
                        name: symbol
                            .name()
                            .map_or_else(|| format!("${}", symbol.id()), |name| name.to_string()),
                    }
                })?)
            }
            ExprView::Function { name, args } => {
                let builtin = BuiltinFunction::of(name).ok_or_else(|| EvalError::MissingBody {
                    name: name.as_str().to_owned(),
                })?;
                Source::Computed(Self::function(builtin, args, values)?)
            }
            ExprView::Sum(_) => {
                if Value::OPTIMIZE_FLOATS
                    && let Some(polynomial) = self.polynomial(expression, values)?
                {
                    return Ok(polynomial);
                }
                let terms = expression.stored_terms().ok_or(EvalError::InvalidProgram)?;
                return self.group(terms.iter().map(|term| values[term]), false);
            }
            ExprView::Product(factors) => {
                return self.group(factors.iter().map(|factor| values[factor]), true);
            }
            ExprView::Div(left, right) => {
                let numerator = values[left];
                let denominator = values[right];
                if Value::OPTIMIZE_FLOATS
                    && let Some(Source::Constant(constant)) = self.sources.get(denominator)
                    && let Some(constant_val) = self.constants.get(*constant)
                    && let Some(reciprocal) = Value::constant_reciprocal(constant_val)
                {
                    let literal = self.constant(reciprocal);
                    let inverse = self.sources.len();
                    self.sources.push(literal);
                    Source::Computed(Operation::Multiply([numerator, inverse]))
                } else {
                    if Value::OPTIMIZE_FLOATS {
                        self.denominators
                            .entry(denominator)
                            .and_modify(|shared| *shared = true)
                            .or_insert(false);
                    }
                    Source::Computed(Operation::Divide([numerator, denominator]))
                }
            }
            ExprView::Pow(base, exponent) => Source::Computed({
                if Value::OPTIMIZE_FLOATS
                    && let Some(count) = exponent.positive_integer()
                {
                    return Ok(self.integer_power(values[base], count));
                }
                if let Some(count) = exponent
                    .positive_integer()
                    .and_then(|count| u32::try_from(count.get()).ok())
                {
                    return self.stored_power(values[base], count);
                }
                Operation::Power([values[base], values[exponent]])
            }),
            ExprView::Derivative { .. } => return Err(EvalError::UnevaluatedDerivative),
        };
        let output = self.sources.len();
        self.sources.push(source);
        Ok(output)
    }

    /// Lower a builtin call to the operation of its kernel signature.
    ///
    /// # Errors
    /// Returns [`EvalError::InvalidProgram`] if the argument count differs
    /// from the kernel signature.
    fn function(
        name: BuiltinFunction,
        arguments: &[Expr],
        values: &FixedMap<Expr, usize>,
    ) -> Result<Operation<Value>, EvalError> {
        Ok(match (Value::kernel(name), arguments) {
            (Kernel::Unary(function), [argument]) => {
                Operation::Unary(name, function, [values[argument]])
            }
            (Kernel::Binary(function), [first, second]) => {
                Operation::Binary(name, function, [values[first], values[second]])
            }
            (Kernel::Ternary(function), [first, second, third]) => Operation::Ternary(
                name,
                function,
                [values[first], values[second], values[third]],
            ),
            (Kernel::Unary(_) | Kernel::Binary(_) | Kernel::Ternary(_), _) => {
                return Err(EvalError::InvalidProgram);
            }
        })
    }

    /// Sum or multiply `operands` from left to right and return the result.
    ///
    /// One operand is its own result. Longer folds are pooled instructions
    /// of at most [`GROUP`] operands, each passing its result to the next as
    /// the first operand, so the order of operations is that of a single fold
    /// while a long sum keeps at most one group of operands live.
    ///
    /// # Errors
    /// Returns [`EvalError::InvalidProgram`] for an empty fold.
    pub fn group(
        &mut self,
        operands: impl IntoIterator<Item = usize>,
        multiply: bool,
    ) -> Result<usize, EvalError> {
        let fold = if multiply { Fold::Product } else { Fold::Sum };
        let mut start = self.program.operands.len();
        for operand in operands {
            if self.program.operands.len() == start.saturating_add(GROUP) {
                let range = start..self.program.operands.len();
                let folded = self.emit(Operation::Fold(fold, range));
                start = self.program.operands.len();
                self.program.operands.push(folded);
            }
            self.program.operands.push(operand);
        }
        let range = start..self.program.operands.len();
        let operation = match self.program.operands.get(range.clone()) {
            Some(&[single]) => {
                self.program.operands.truncate(start);
                return Ok(single);
            }
            Some(&[left, right]) => {
                self.program.operands.truncate(start);
                if multiply {
                    Operation::Multiply([left, right])
                } else {
                    Operation::Add([left, right])
                }
            }
            Some([_, _, _, ..]) => Operation::Fold(fold, range),
            Some([]) | None => return Err(EvalError::InvalidProgram),
        };
        Ok(self.emit(operation))
    }

    pub fn emit(&mut self, operation: Operation<Value>) -> usize {
        let output = self.sources.len();
        self.sources.push(Source::Computed(operation));
        output
    }

    pub fn constant(&mut self, value: Value) -> Source<Value> {
        let index = self.constants.len();
        self.constants.push(value);
        Source::Constant(index)
    }
}
