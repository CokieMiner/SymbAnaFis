//! Memoized numerical evaluation without an instruction-compilation phase.

use alloc::{format, string::ToString};

use num_anafis::Number;

use crate::{
    collections::HashMap,
    core::{EvaluationView, Expr, ExprView, FunctionDefinition, Kernel, Scalar, Symbol},
    errors::EvalError,
};

use super::Traversal;

const RECURSION_DEPTH: u8 = 4;

impl Expr {
    /// Evaluate a scalar DAG directly with identity-based parameter bindings.
    ///
    /// Computed nodes are memoized. Bounded recursive descent handles shallow
    /// expressions; deep subgraphs and custom bodies use an explicit stack with
    /// the same memo. Literals and inputs require no expression interning.
    /// Repeated evaluations can use [`super::Evaluator`] to reuse a program.
    /// Functions and arithmetic follow the selected scalar's numerical domains.
    ///
    /// # Errors
    /// Returns [`EvalError`] for duplicate bindings, unbound symbols, unsupported
    /// constants, missing or recursive custom bodies, or unevaluated derivatives.
    ///
    /// # Panics
    /// Propagates numerical backend and custom callback panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Number};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let expression = x.pow(2) + 1;
    /// assert_eq!(expression.evaluate([(x, 3.0)])?, 10.0);
    /// assert_eq!(expression.evaluate([(x, Number::from_int(3))])?, Number::from_int(10));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn evaluate<'owner, Value: Scalar>(
        &self,
        parameters: impl IntoIterator<Item = (Symbol<'owner>, Value)>,
    ) -> Result<Value, EvalError> {
        let mut bindings = HashMap::new();
        for (index, (symbol, value)) in parameters.into_iter().enumerate() {
            if bindings.insert(symbol.id(), value).is_some() {
                return Err(EvalError::DuplicateParameter { index });
            }
        }
        visit(self, &bindings, &mut HashMap::new(), RECURSION_DEPTH)
    }
}

fn visit<Value: Scalar>(
    node: &Expr,
    bindings: &HashMap<usize, Value>,
    values: &mut HashMap<Expr, Value>,
    depth: u8,
) -> Result<Value, EvalError> {
    let computed = matches!(
        node.view(),
        ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(_, _)
            | ExprView::Pow(_, _)
            | ExprView::Derivative { .. }
    );
    if computed {
        if let Some(value) = values.get(node) {
            return Ok(value.clone());
        }
        if depth == 0
            || matches!(node.view(), ExprView::Function { name, .. } if FunctionDefinition::get(name).is_none())
        {
            return Traversal::walk(node, values, |original, completed| {
                reduce_node(original, bindings, |child| Ok(completed[child].clone()))
            });
        }
    }
    let value = reduce_node(node, bindings, |child| {
        visit(child, bindings, values, depth.saturating_sub(1))
    })?;
    if computed {
        values.insert(node.clone(), value.clone());
    }
    Ok(value)
}

#[expect(
    clippy::indexing_slicing,
    reason = "sealed builtin signatures agree with constructor-validated argument counts"
)]
fn reduce_node<Value: Scalar>(
    node: &Expr,
    bindings: &HashMap<usize, Value>,
    mut child: impl FnMut(&Expr) -> Result<Value, EvalError>,
) -> Result<Value, EvalError> {
    if let Some(monomials) = EvaluationView::new(node).monomials() {
        let mut sum: Option<Value> = None;
        for (coefficient, powers) in monomials {
            let mut product = if EvaluationView::exact_one(coefficient) {
                None
            } else {
                Some(child(coefficient)?)
            };
            for (base, count) in powers {
                let mut factor = child(base)?;
                if *count > 1 {
                    let number = Number::from_int(i64::from(*count));
                    let exponent = Value::literal(&number)
                        .ok_or(EvalError::UnsupportedLiteral { value: number })?;
                    factor = Value::power(&factor, &exponent);
                }
                product = Some(match product {
                    Some(value) => value.mul(factor),
                    None => factor,
                });
            }
            let term = product.expect("stored monomials contain at least one positive power");
            sum = Some(match sum {
                Some(value) => value.add(term),
                None => term,
            });
        }
        return Ok(sum.expect("compact polynomials contain at least two monomials"));
    }
    let result = match node.view() {
        ExprView::Number(value) => {
            Value::literal(value).ok_or_else(|| EvalError::UnsupportedLiteral {
                value: value.clone(),
            })?
        }
        ExprView::ImaginaryUnit => Value::literal(&Number::i())
            .ok_or_else(|| EvalError::UnsupportedLiteral { value: Number::i() })?,
        ExprView::Pi => Value::pi().ok_or(EvalError::UnsupportedConstant { name: "pi" })?,
        ExprView::Euler => Value::euler().ok_or(EvalError::UnsupportedConstant { name: "e" })?,
        ExprView::Symbol(symbol) => {
            bindings
                .get(&symbol.id())
                .cloned()
                .ok_or_else(|| EvalError::UnboundSymbol {
                    name: symbol
                        .name()
                        .map_or_else(|| format!("${}", symbol.id()), |name| name.to_string()),
                })?
        }
        ExprView::Sum(_) => {
            let terms = EvaluationView::new(node)
                .sum()
                .expect("compact polynomials are reduced before ordinary sums");
            let (first, rest) = terms
                .split_first()
                .expect("normalized sums have at least two terms");
            rest.iter().try_fold(child(first)?, |sum, term| {
                Ok::<_, EvalError>(sum.add(child(term)?))
            })?
        }
        ExprView::Product(factors) => {
            let (first, rest) = factors
                .split_first()
                .expect("normalized products have at least two factors");
            rest.iter().try_fold(child(first)?, |product, factor| {
                Ok::<_, EvalError>(product.mul(child(factor)?))
            })?
        }
        ExprView::Div(left, right) => child(left)?.div(child(right)?),
        ExprView::Pow(base, exponent) => Value::power(&child(base)?, &child(exponent)?),
        ExprView::Function { name, args } => {
            let definition =
                FunctionDefinition::get(name).expect("custom calls expand before scalar reduction");
            match Value::kernel(definition.builtin()) {
                Kernel::Unary(kernel) => kernel(&child(&args[0])?),
                Kernel::Binary(kernel) => kernel(&child(&args[0])?, &child(&args[1])?),
                Kernel::Ternary(kernel) => {
                    kernel(&child(&args[0])?, &child(&args[1])?, &child(&args[2])?)
                }
                Kernel::Quaternary(kernel) => kernel(
                    &child(&args[0])?,
                    &child(&args[1])?,
                    &child(&args[2])?,
                    &child(&args[3])?,
                ),
            }
        }
        ExprView::Derivative { .. } => return Err(EvalError::UnevaluatedDerivative),
    };
    Ok(result)
}
