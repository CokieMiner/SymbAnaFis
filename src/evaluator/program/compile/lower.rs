//! Canonical DAG lowering into scalar register instructions.

use core::{mem::size_of, slice::from_ref};

use alloc::{
    borrow::{Cow, ToOwned},
    format,
    sync::Arc,
    vec::Vec,
};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Expr, ExprView, Kernel, StoredForm, Symbol, SymbolData},
    errors::EvalError,
};

use super::{
    BLOCK, Combine, Compilation, EvaluationValue, Fold, Fused, GROUP, MultiProgram, NativeSlot,
    OperandSpan, Operation, Program, ProgramData, Source, Traversal,
};

impl<Value: EvaluationValue> Program<Value> {
    /// Compile a scalar DAG with an explicit, identity-based parameter order.
    ///
    /// Shared stored nodes compile once. Builtin signatures dispatch once;
    /// custom bodies expand once per distinct call, after their arguments.
    /// Recursive body definitions are rejected. Generic plans preserve stored
    /// operand order. Native `f64`/`f32` plans optimize powers, polynomials, signs,
    /// constant and repeated denominators, and fused arithmetic.
    /// No separate simplifier is invoked.
    /// Unused parameters are allowed but must still be supplied at execution.
    ///
    /// The returned program is immutable and may be shared between threads.
    /// Each concurrent execution requires its own [`crate::Workspace`].
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
    /// use symb_anafis::{Context, Program, Workspace};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let expression = x.pow(2) + 1;
    /// let program = Program::<f64>::compile(&expression, &[x])?;
    /// let mut workspace = Workspace::new();
    /// assert_eq!(program.eval_with(&mut workspace, &[3.0])?, 10.0);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn compile(expression: &Expr, parameters: &[Symbol<'_>]) -> Result<Self, EvalError> {
        let data = ProgramData::compile_expressions(from_ref(expression), parameters)?;
        Ok(Self {
            data,
            native: NativeSlot::none(),
        })
    }
}

impl<Value: EvaluationValue> MultiProgram<Value> {
    /// Compile multiple scalar expressions over one shared parameter order into
    /// a single joint register program.
    ///
    /// Subexpressions, constants, shared denominators, and paired trigonometric
    /// calls (`sin`/`cos`, `sinh`/`cosh`) occurring across different outputs are
    /// deduplicated and computed once. Unused parameters are allowed and must
    /// still be supplied at execution. An empty `expressions` slice produces a
    /// zero-output program.
    ///
    /// # Errors
    /// Returns [`EvalError`] for duplicate parameters, unbound symbols,
    /// unsupported scalar constants, missing or recursive custom bodies,
    /// unevaluated derivatives, or counts exceeding native capacity.
    ///
    /// # Panics
    /// Propagates user callback and numerical backend panics.
    ///
    /// ```
    /// use symb_anafis::{Context, MultiProgram};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let y = context.symb("y")?;
    /// let system = MultiProgram::<f64>::compile(&[x * y + 1, x * y - 1], &[x, y])?;
    /// assert_eq!(system.output_count(), 2);
    /// assert_eq!(system.eval(&[3.0, 4.0])?, vec![13.0, 11.0]);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn compile(expressions: &[Expr], parameters: &[Symbol<'_>]) -> Result<Self, EvalError> {
        let data = ProgramData::compile_expressions(expressions, parameters)?;
        Ok(Self {
            data,
            native: NativeSlot::none(),
        })
    }
}

impl<Value: EvaluationValue> ProgramData<Value> {
    fn compile_expressions(
        expressions: &[Expr],
        parameters: &[Symbol<'_>],
    ) -> Result<Arc<Self>, EvalError> {
        let zero = Number::zero();
        let initial =
            Value::from_number(&zero).ok_or(EvalError::UnsupportedLiteral { value: zero })?;
        let mut parameter_indices = FixedMap::default();
        parameter_indices.reserve(parameters.len());
        let mut parameter_nodes = Vec::with_capacity(parameters.len());
        for (index, symbol) in parameters.iter().enumerate() {
            let param_id = u32::try_from(index).map_err(|_overflow| EvalError::ProgramTooLarge)?;
            if parameter_indices.insert(symbol.id(), param_id).is_some() {
                return Err(EvalError::DuplicateParameter { index });
            }
            parameter_nodes.push(SymbolData::of(*symbol));
        }
        let mut compilation = Compilation {
            operands: Vec::new(),
            parameters: parameter_nodes.into_boxed_slice(),
            results: Vec::with_capacity(expressions.len()),
            sources: Vec::new(),
            constants: Vec::new(),
            parameter_map: parameter_indices,
            powers: FixedMap::default(),
            constant_reciprocals: FixedMap::default(),
            constant_bits: FixedMap::default(),
            exponent_constants: FixedMap::default(),
            denominators: FixedMap::default(),
            binary: FixedMap::default(),
            fused: FixedMap::default(),
            folds: FixedMap::default(),
            unit: None,
            initial,
            trig_mask: 0,
        };
        if Value::OPTIMIZE_FLOATS {
            // The unit precedes every computed source, as later passes that
            // pair operands with it require.
            compilation.coefficient(None)?;
        }
        let mut lowered = FixedMap::default();
        let mut traversal = Traversal::new();
        for expression in expressions {
            let root = traversal.walk_root(expression, &mut lowered, |node, values| {
                compilation.lower(node, values)
            })?;
            compilation.results.push(root);
        }
        if Value::OPTIMIZE_FLOATS {
            compilation.reuse_reciprocals()?;
            compilation.optimize_arithmetic()?;
        }
        compilation.pair_functions()?;
        parameters
            .len()
            .checked_add(compilation.constants.len())
            .and_then(|prefix| prefix.checked_add(compilation.sources.len()))
            .and_then(|max_regs| u32::try_from(max_regs).ok())
            .ok_or(EvalError::ProgramTooLarge)?;
        let program = compilation.finish()?;
        let prefix = parameters
            .len()
            .checked_add(program.constants.len())
            .ok_or(EvalError::ProgramTooLarge)?;
        program
            .register_count
            .checked_mul(size_of::<[Value; BLOCK]>())
            .filter(|bytes| *bytes <= isize::MAX.unsigned_abs())
            .ok_or(EvalError::ProgramTooLarge)?;
        if !program.validate(program.register_count, prefix) {
            return Err(EvalError::InvalidProgram);
        }
        Ok(Arc::new(program))
    }
}

impl<Value: EvaluationValue> Compilation<Value> {
    fn lower(
        &mut self,
        expression: &Expr,
        values: &FixedMap<Cow<'_, Expr>, u32>,
    ) -> Result<u32, EvalError> {
        if expression.monomials().is_some() {
            if Value::OPTIMIZE_FLOATS
                && let Some(result) = self.polynomial(expression, values)?
            {
                return Ok(result);
            }
            return self.compact_polynomial(expression, values);
        }
        match expression.view() {
            ExprView::Number(number) => {
                if Value::OPTIMIZE_FLOATS && expression.is_exact_one() {
                    return self.coefficient(None);
                }
                let value =
                    Value::from_number(number).ok_or_else(|| EvalError::UnsupportedLiteral {
                        value: number.clone(),
                    })?;
                self.constant(value)
            }
            ExprView::ImaginaryUnit => {
                let unit = Number::i();
                let value = Value::from_number(&unit)
                    .ok_or(EvalError::UnsupportedLiteral { value: unit })?;
                self.constant(value)
            }
            ExprView::Pi => self.constant(
                Value::from_number(&Number::pi())
                    .ok_or(EvalError::UnsupportedConstant { name: "pi" })?,
            ),
            ExprView::Euler => self.constant(
                Value::from_number(&Number::e())
                    .ok_or(EvalError::UnsupportedConstant { name: "e" })?,
            ),
            ExprView::Symbol(symbol) => {
                let parameter = *self.parameter_map.get(&symbol.id()).ok_or_else(|| {
                    EvalError::UnboundSymbol {
                        name: symbol
                            .as_str()
                            .map_or_else(|| format!("${}", symbol.id()), ToOwned::to_owned),
                    }
                })?;
                self.push_source(Source::Parameter(parameter))
            }
            ExprView::Function { name, args } => {
                let builtin = BuiltinFunction::of(name).ok_or_else(|| EvalError::MissingBody {
                    name: name.as_str().to_owned(),
                })?;
                self.lower_builtin(builtin, args, values)
            }
            ExprView::Sum(_) => {
                if Value::OPTIMIZE_FLOATS
                    && let Some(polynomial) = self.polynomial(expression, values)?
                {
                    return Ok(polynomial);
                }
                let terms = expression.stored_terms().ok_or(EvalError::InvalidProgram)?;
                self.group(terms.iter().map(|term| values[term]), false)
            }
            ExprView::Product(factors) => {
                self.group(factors.iter().map(|factor| values[factor]), true)
            }
            ExprView::Div(left, right) => {
                let operation = self.quotient(values[left], values[right])?;
                self.push_source(Source::Computed(operation))
            }
            ExprView::Pow(base, exponent) => {
                if Value::OPTIMIZE_FLOATS
                    && let Some(count) = exponent.positive_integer()
                {
                    return self.integer_power(values[base], count);
                }
                if let Some(count) = exponent
                    .positive_integer()
                    .and_then(|count| u32::try_from(count.get()).ok())
                {
                    return self.stored_power(values[base], count);
                }
                self.push_source(Source::Computed(Operation::Power([
                    values[base],
                    values[exponent],
                ])))
            }
            ExprView::Derivative { .. } => Err(EvalError::UnevaluatedDerivative),
        }
    }

    fn quotient(&mut self, numerator: u32, denominator: u32) -> Result<Operation, EvalError> {
        let src_idx = usize::try_from(denominator).unwrap_or(usize::MAX);
        if Value::OPTIMIZE_FLOATS
            && let Some(&Source::Constant(constant)) = self.sources.get(src_idx)
            && let Some(constant_val) = self
                .constants
                .get(usize::try_from(constant).unwrap_or(usize::MAX))
            && let Some(reciprocal) = Value::constant_reciprocal(constant_val)
        {
            let inverse = if let Some(&existing) = self.constant_reciprocals.get(&denominator) {
                existing
            } else {
                let created = self.constant(reciprocal)?;
                self.constant_reciprocals.insert(denominator, created);
                created
            };
            Ok(Operation::Combine(Combine::Multiply, [numerator, inverse]))
        } else {
            if Value::OPTIMIZE_FLOATS {
                self.denominators
                    .entry(denominator)
                    .and_modify(|shared| *shared = true)
                    .or_insert(false);
            }
            Ok(Operation::Divide([numerator, denominator]))
        }
    }

    fn lower_builtin(
        &mut self,
        builtin: BuiltinFunction,
        args: &[Expr],
        values: &FixedMap<Cow<'_, Expr>, u32>,
    ) -> Result<u32, EvalError> {
        if Value::IS_REAL {
            match args {
                [argument] if matches!(builtin, BuiltinFunction::Re | BuiltinFunction::Conj) => {
                    return Ok(values[argument]);
                }
                [_] if builtin == BuiltinFunction::Im => {
                    return self.constant(self.initial.clone());
                }
                [value, lower, upper]
                    if Value::OPTIMIZE_FLOATS && builtin == BuiltinFunction::Clamp =>
                {
                    let floored = self.push_source(Source::Computed(Operation::Binary(
                        BuiltinFunction::Max,
                        [values[value], values[lower]],
                    )))?;
                    return self.push_source(Source::Computed(Operation::Binary(
                        BuiltinFunction::Min,
                        [floored, values[upper]],
                    )));
                }
                _ => {}
            }
        }
        let operation = self.function(builtin, args, values)?;
        self.push_source(Source::Computed(operation))
    }

    fn function(
        &mut self,
        name: BuiltinFunction,
        arguments: &[Expr],
        values: &FixedMap<Cow<'_, Expr>, u32>,
    ) -> Result<Operation, EvalError> {
        if matches!(name, BuiltinFunction::Sin) {
            self.trig_mask |= 1;
        } else if matches!(name, BuiltinFunction::Cos) {
            self.trig_mask |= 2;
        } else if matches!(name, BuiltinFunction::Sinh) {
            self.trig_mask |= 4;
        } else if matches!(name, BuiltinFunction::Cosh) {
            self.trig_mask |= 8;
        }
        Ok(match (Kernel::<Value>::of(name), arguments) {
            (Kernel::Unary(_), [argument]) => Operation::Unary(name, [values[argument]]),
            (Kernel::Binary(_), [first, second]) => {
                Operation::Binary(name, [values[first], values[second]])
            }
            (Kernel::Ternary(_), [first, second, third]) => {
                Operation::Ternary(name, [values[first], values[second], values[third]])
            }
            (Kernel::Unary(_) | Kernel::Binary(_) | Kernel::Ternary(_), _) => {
                return Err(EvalError::InvalidProgram);
            }
        })
    }

    /// Sum or multiply `operands` from left to right and return the result source.
    pub fn group(
        &mut self,
        operands: impl IntoIterator<Item = u32>,
        multiply: bool,
    ) -> Result<u32, EvalError> {
        let mut start = self.operands.len();
        for operand in operands {
            if self.operands.len() == start.saturating_add(GROUP) {
                let folded = self.emit_fold(start, multiply)?;
                start = self.operands.len();
                self.operands.push(folded);
            }
            self.operands.push(operand);
        }
        let range = start..self.operands.len();
        match self.operands.get(range) {
            Some(&[single]) => {
                self.operands.truncate(start);
                Ok(single)
            }
            Some(&[left, right]) => {
                self.operands.truncate(start);
                self.emit_binary(multiply, left, right)
            }
            Some([_, _, _, ..]) => self.emit_fold(start, multiply),
            Some([]) | None => Err(EvalError::InvalidProgram),
        }
    }

    pub fn emit_binary(&mut self, multiply: bool, left: u32, right: u32) -> Result<u32, EvalError> {
        let kind = if multiply {
            Combine::Multiply
        } else {
            Combine::Add
        };
        let op = Operation::Combine(kind, [left, right]);
        if !Value::OPTIMIZE_FLOATS {
            return self.push_source(Source::Computed(op));
        }
        let key = (multiply, left.min(right), left.max(right));
        if let Some(&existing) = self.binary.get(&key) {
            return Ok(existing);
        }
        let output = self.push_source(Source::Computed(op))?;
        self.binary.insert(key, output);
        Ok(output)
    }

    pub fn emit_fused(
        &mut self,
        fused: Fused,
        [left, right, addend]: [u32; 3],
    ) -> Result<u32, EvalError> {
        let op = Operation::Fused(fused, [left, right, addend]);
        if !Value::OPTIMIZE_FLOATS {
            return self.push_source(Source::Computed(op));
        }
        let sign_key = u8::from(fused.negate_product) | (u8::from(fused.negate_addend) << 1);
        let key = (sign_key, left.min(right), left.max(right), addend);
        if let Some(&existing) = self.fused.get(&key) {
            return Ok(existing);
        }
        let output = self.push_source(Source::Computed(op))?;
        self.fused.insert(key, output);
        Ok(output)
    }

    fn emit_fold(&mut self, start: usize, multiply: bool) -> Result<u32, EvalError> {
        let fold = if multiply { Fold::Product } else { Fold::Sum };
        let start_u32 = u32::try_from(start).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        let end_u32 =
            u32::try_from(self.operands.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        let span = OperandSpan {
            start: start_u32,
            end: end_u32,
        };
        if !Value::OPTIMIZE_FLOATS {
            return self.push_source(Source::Computed(Operation::Fold(fold, span)));
        }
        let slice = self
            .operands
            .get(start..)
            .ok_or(EvalError::InvalidProgram)?;
        let len = slice.len();
        let len_u8 = u8::try_from(len).map_err(|_overflow| EvalError::InvalidProgram)?;
        let mut key = [0_u32; GROUP];
        let Some(prefix) = key.get_mut(..len) else {
            return Err(EvalError::InvalidProgram);
        };
        prefix.copy_from_slice(slice);
        if let Some(&existing) = self.folds.get(&(multiply, key, len_u8)) {
            self.operands.truncate(start);
            return Ok(existing);
        }
        let output = self.push_source(Source::Computed(Operation::Fold(fold, span)))?;
        self.folds.insert((multiply, key, len_u8), output);
        Ok(output)
    }

    pub fn push_source(&mut self, source: Source) -> Result<u32, EvalError> {
        let output =
            u32::try_from(self.sources.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        self.sources.push(source);
        Ok(output)
    }

    pub fn constant(&mut self, value: Value) -> Result<u32, EvalError> {
        let bits = Value::constant_bits(&value);
        if let Some(key) = bits
            && let Some(&existing) = self.constant_bits.get(&key)
        {
            return Ok(existing);
        }
        let index =
            u32::try_from(self.constants.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        self.constants.push(value);
        let output = self.push_source(Source::Constant(index))?;
        if let Some(key) = bits {
            self.constant_bits.insert(key, output);
        }
        Ok(output)
    }
}
