//! Canonical DAG lowering into scalar register instructions.

use core::mem::size_of;

use alloc::{format, string::ToString, sync::Arc, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::HashMap,
    core::{
        BuiltinFunction, EvaluationView, Expr, ExprView, FunctionDefinition, Kernel, Scalar,
        Symbol, SymbolData,
    },
    errors::EvalError,
};

use super::{Compilation, Evaluator, Operation, Program, Source, Traversal};

impl<Value: Scalar> Evaluator<Value> {
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
    /// use symb_anafis::{Context, Evaluator, Number};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let expression = x.pow(2) + 1;
    /// let mut evaluator = Evaluator::<Number>::compile(&expression, &[x])?;
    /// assert_eq!(evaluator.evaluate(&[Number::from_int(3)])?, Number::from_int(10));
    /// let mut native = Evaluator::<f64>::compile(&expression, &[x])?;
    /// assert_eq!(native.evaluate(&[3.0])?, 10.0);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn compile(expression: &Expr, parameters: &[Symbol<'_>]) -> Result<Self, EvalError> {
        let mut parameter_indices = HashMap::with_capacity(parameters.len());
        let mut parameter_nodes = Vec::with_capacity(parameters.len());
        for (index, symbol) in parameters.iter().enumerate() {
            if parameter_indices.insert(symbol.id(), index).is_some() {
                return Err(EvalError::DuplicateParameter { index });
            }
            parameter_nodes.push(SymbolData::for_symbol(*symbol));
        }
        let mut compilation = Compilation {
            program: Program {
                instructions: Vec::new(),
                operands: Vec::new(),
                parameters: parameter_nodes,
                result: 0,
            },
            sources: Vec::new(),
            constants: Vec::new(),
            parameters: parameter_indices,
            powers: HashMap::new(),
            unit: None,
        };
        compilation.program.result =
            Traversal::walk(expression, &mut HashMap::new(), |node, values| {
                compilation.lower(node, values)
            })?;
        if Value::OPTIMIZE_FLOATS {
            compilation.reuse_reciprocals()?;
            compilation.optimize_arithmetic();
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
            .checked_mul(size_of::<Value>())
            .filter(|bytes| *bytes <= isize::MAX.unsigned_abs())
            .ok_or(EvalError::ProgramTooLarge)?;
        if !compilation.program.validate(register_count, prefix) {
            return Err(EvalError::InvalidProgram);
        }
        let zero = Number::zero();
        let initial = Value::literal(&zero).ok_or(EvalError::UnsupportedLiteral { value: zero })?;
        let mut registers = Vec::with_capacity(register_count);
        registers.resize(parameters.len(), initial.clone());
        registers.extend(compilation.constants);
        registers.resize(register_count, initial);
        Ok(Self {
            program: Arc::new(compilation.program),
            registers,
        })
    }
}

impl<Value: Scalar> Compilation<Value> {
    fn lower(
        &mut self,
        expression: &Expr,
        values: &HashMap<Expr, usize>,
    ) -> Result<usize, EvalError> {
        if EvaluationView::new(expression).monomials().is_some() {
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
                let definition = FunctionDefinition::get(name)
                    .expect("the traversal expands every custom call before reduction");
                Source::Computed(Self::function(definition.builtin(), args, values))
            }
            ExprView::Sum(_) => {
                if Value::OPTIMIZE_FLOATS
                    && let Some(polynomial) = self.polynomial(expression, values)?
                {
                    return Ok(polynomial);
                }
                let terms = EvaluationView::new(expression)
                    .sum()
                    .expect("compact polynomials are lowered before ordinary sums");
                Source::Computed(self.group(terms.iter().map(|term| values[term]), false))
            }
            ExprView::Product(factors) => {
                Source::Computed(self.group(factors.iter().map(|factor| values[factor]), true))
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
                    Source::Computed(Operation::Divide([numerator, denominator]))
                }
            }
            ExprView::Pow(base, exponent) => Source::Computed({
                if Value::OPTIMIZE_FLOATS
                    && let Some(count) = EvaluationView::positive_integer(exponent)
                {
                    return Ok(self.integer_power(values[base], count));
                }
                if let Some(count) = EvaluationView::positive_integer(exponent)
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

    #[expect(
        clippy::indexing_slicing,
        reason = "sealed builtin signatures match constructor-validated arities and every argument has a completed value"
    )]
    fn function(
        name: BuiltinFunction,
        arguments: &[Expr],
        values: &HashMap<Expr, usize>,
    ) -> Operation<Value> {
        match Value::kernel(name) {
            Kernel::Unary(function) => Operation::Unary(name, function, [values[&arguments[0]]]),
            Kernel::Binary(function) => Operation::Binary(
                name,
                function,
                [values[&arguments[0]], values[&arguments[1]]],
            ),
            Kernel::Ternary(function) => Operation::Ternary(
                function,
                [
                    values[&arguments[0]],
                    values[&arguments[1]],
                    values[&arguments[2]],
                ],
            ),
            Kernel::Quaternary(function) => Operation::Quaternary(
                function,
                [
                    values[&arguments[0]],
                    values[&arguments[1]],
                    values[&arguments[2]],
                    values[&arguments[3]],
                ],
            ),
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "start is the operand vector length immediately before appending this normalized group"
    )]
    pub fn group(
        &mut self,
        operands: impl Iterator<Item = usize>,
        multiply: bool,
    ) -> Operation<Value> {
        let start = self.program.operands.len();
        self.program.operands.extend(operands);
        if let [left, right] = self.program.operands[start..] {
            let pair = [left, right];
            self.program.operands.truncate(start);
            if multiply {
                Operation::Multiply(pair)
            } else {
                Operation::Add(pair)
            }
        } else {
            let range = start..self.program.operands.len();
            if multiply {
                Operation::MultiplyMany(range)
            } else {
                Operation::AddMany(range)
            }
        }
    }

    pub fn constant(&mut self, value: Value) -> Source<Value> {
        let index = self.constants.len();
        self.constants.push(value);
        Source::Constant(index)
    }
}
