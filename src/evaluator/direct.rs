//! Memoized numerical evaluation without an instruction-compilation phase.

use alloc::{
    borrow::{Cow, ToOwned},
    format,
};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Expr, ExprView, Kernel, StoredForm, Symbol},
    errors::EvalError,
};

use super::{EvaluationValue, Traversal};

/// The descent depth before explicit stack traversal takes over. A recursive
/// call costs less than a stack frame, so the descent covers the operand
/// structure of ordinary expressions; a deep chain switches to the explicit
/// stack after six levels, which bounds the call stack.
const RECURSION_DEPTH: u8 = 6;

/// Parameter count threshold below which bindings stay inline on the stack
/// without allocating a hash table.
const INLINE_BINDINGS: usize = 8;

enum Bindings<Value> {
    Inline {
        entries: [Option<(usize, Value)>; INLINE_BINDINGS],
        len: usize,
    },
    Map(FixedMap<usize, Value>),
}

impl<Value: Clone> Bindings<Value> {
    fn build<'owner>(
        parameters: impl IntoIterator<Item = (Symbol<'owner>, Value)>,
    ) -> Result<Self, EvalError> {
        let iter = parameters.into_iter();
        let hint = iter.size_hint().0;
        if hint > INLINE_BINDINGS {
            let mut map = FixedMap::default();
            map.reserve(hint);
            for (index, (symbol, value)) in iter.enumerate() {
                if map.insert(symbol.id(), value).is_some() {
                    return Err(EvalError::DuplicateParameter { index });
                }
            }
            return Ok(Self::Map(map));
        }

        let mut entries: [Option<(usize, Value)>; INLINE_BINDINGS] =
            [const { None }; INLINE_BINDINGS];
        let mut len = 0_usize;
        let mut enumerated = iter.enumerate();
        while let Some((index, (symbol, value))) = enumerated.next() {
            let id = symbol.id();
            if entries
                .get(..len)
                .is_some_and(|prefix| prefix.iter().flatten().any(|(seen, _)| *seen == id))
            {
                return Err(EvalError::DuplicateParameter { index });
            }
            if let Some(slot) = entries.get_mut(len) {
                *slot = Some((id, value));
                len = len.saturating_add(1);
                continue;
            }
            let mut map = FixedMap::default();
            map.reserve(
                INLINE_BINDINGS
                    .saturating_add(1)
                    .saturating_add(enumerated.size_hint().0),
            );
            for (seen_id, seen_val) in entries.into_iter().flatten() {
                map.insert(seen_id, seen_val);
            }
            map.insert(id, value);
            for (rem_index, (rem_sym, rem_val)) in enumerated {
                if map.insert(rem_sym.id(), rem_val).is_some() {
                    return Err(EvalError::DuplicateParameter { index: rem_index });
                }
            }
            return Ok(Self::Map(map));
        }
        Ok(Self::Inline { entries, len })
    }

    #[inline]
    fn get(&self, id: usize) -> Option<Value> {
        match self {
            Self::Inline { entries, len } => entries
                .get(..*len)?
                .iter()
                .flatten()
                .find(|(seen, _)| *seen == id)
                .map(|(_, value)| value.clone()),
            Self::Map(map) => map.get(&id).cloned(),
        }
    }
}

impl Expr {
    /// Evaluate a scalar DAG directly with identity-based parameter bindings.
    ///
    /// Computed nodes are memoized without cloning expression handles when later
    /// branches remain to be visited; simple tree spines and small parameter
    /// lists allocate nothing on the heap. Bounded recursive descent handles
    /// shallow expressions; deep subgraphs and custom bodies use an explicit
    /// stack with the same memo. Repeated evaluations can use [`super::Program`]
    /// with a mutable [`super::Workspace`] to reuse compiled instructions and
    /// registers. Functions and arithmetic follow the selected scalar's
    /// numerical domains.
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
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let expression = x.pow(2) + 1;
    /// assert_eq!(expression.evaluate([(x, 3.0)])?, 10.0);
    /// assert_eq!(expression.evaluate([(x, Number::from_int(3))])?, Number::from_int(10));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn evaluate<'owner, Value: EvaluationValue>(
        &self,
        parameters: impl IntoIterator<Item = (Symbol<'owner>, Value)>,
    ) -> Result<Value, EvalError> {
        let bindings = Bindings::build(parameters)?;
        visit(
            self,
            &bindings,
            &mut FixedMap::default(),
            RECURSION_DEPTH,
            false,
        )
    }
}

#[inline]
const fn is_memoized(view: &ExprView<'_>) -> bool {
    matches!(
        view,
        ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(_, _)
            | ExprView::Pow(_, _)
            | ExprView::Derivative { .. }
    )
}

fn visit<'expr, Value: EvaluationValue>(
    node: &'expr Expr,
    bindings: &Bindings<Value>,
    values: &mut FixedMap<Cow<'expr, Expr>, Value>,
    depth: u8,
    live_after: bool,
) -> Result<Value, EvalError> {
    let view = node.view();
    let memoized = is_memoized(&view);
    if memoized {
        if depth < RECURSION_DEPTH
            && !values.is_empty()
            && let Some(value) = values.get(node)
        {
            return Ok(value.clone());
        }
        let custom =
            matches!(view, ExprView::Function { name, .. } if BuiltinFunction::of(name).is_none());
        if depth == 0 || custom {
            return Traversal::new().walk_root(node, values, |original, completed| {
                let original_view = original.view();
                reduce_node(
                    original,
                    &original_view,
                    bindings,
                    false,
                    |child, _after| {
                        completed
                            .get(child)
                            .cloned()
                            .ok_or(EvalError::InvalidProgram)
                    },
                )
            });
        }
    }
    let next_depth = depth.saturating_sub(1);
    let value = reduce_node(node, &view, bindings, live_after, |child, child_live| {
        visit(child, bindings, values, next_depth, child_live)
    })?;
    if memoized && live_after && depth < RECURSION_DEPTH {
        values.insert(Cow::Borrowed(node), value.clone());
    }
    Ok(value)
}

/// Evaluate one node from the values of its children, forwarding whether later
/// compound siblings remain to be visited after each child.
fn reduce_node<'expr, Value: EvaluationValue>(
    node: &'expr Expr,
    view: &ExprView<'expr>,
    bindings: &Bindings<Value>,
    live_after: bool,
    mut child: impl FnMut(&'expr Expr, bool) -> Result<Value, EvalError>,
) -> Result<Value, EvalError> {
    if node.monomials().is_some() {
        return reduce_compact_poly(node, live_after, child);
    }
    Ok(match view {
        ExprView::Number(value) => {
            Value::from_number(value).ok_or_else(|| EvalError::UnsupportedLiteral {
                value: (*value).clone(),
            })?
        }
        ExprView::ImaginaryUnit => Value::from_number(&Number::i())
            .ok_or_else(|| EvalError::UnsupportedLiteral { value: Number::i() })?,
        ExprView::Pi => Value::from_number(&Number::pi())
            .ok_or(EvalError::UnsupportedConstant { name: "pi" })?,
        ExprView::Euler => {
            Value::from_number(&Number::e()).ok_or(EvalError::UnsupportedConstant { name: "e" })?
        }
        ExprView::Symbol(symbol) => {
            bindings
                .get(symbol.id())
                .ok_or_else(|| EvalError::UnboundSymbol {
                    name: symbol
                        .as_str()
                        .map_or_else(|| format!("${}", symbol.id()), ToOwned::to_owned),
                })?
        }
        ExprView::Sum(_) => {
            let terms = node.stored_terms().ok_or(EvalError::InvalidProgram)?;
            let last_compound = terms.iter().rposition(|term| is_memoized(&term.view()));
            let (first, rest) = terms.split_first().ok_or(EvalError::InvalidProgram)?;
            let first_live = live_after || last_compound.is_some_and(|last| last > 0);
            rest.iter()
                .enumerate()
                .try_fold(child(first, first_live)?, |sum, (offset, term)| {
                    let index = offset.saturating_add(1);
                    let term_live = live_after || last_compound.is_some_and(|last| index < last);
                    Ok::<_, EvalError>(sum.add(child(term, term_live)?))
                })?
        }
        ExprView::Product(factors) => {
            let last_compound = factors
                .iter()
                .rposition(|factor| is_memoized(&factor.view()));
            let (first, rest) = factors.split_first().ok_or(EvalError::InvalidProgram)?;
            let first_live = live_after || last_compound.is_some_and(|last| last > 0);
            rest.iter().enumerate().try_fold(
                child(first, first_live)?,
                |product, (offset, factor)| {
                    let index = offset.saturating_add(1);
                    let factor_live = live_after || last_compound.is_some_and(|last| index < last);
                    Ok::<_, EvalError>(product.mul(child(factor, factor_live)?))
                },
            )?
        }
        ExprView::Div(left, right) => {
            let left_val = child(left, live_after || is_memoized(&right.view()))?;
            left_val.div(child(right, live_after)?)
        }
        ExprView::Pow(base, exponent) => {
            let base_val = child(base, live_after || is_memoized(&exponent.view()))?;
            Value::pow(&base_val, &child(exponent, live_after)?)
        }
        ExprView::Function { name, args } => {
            let builtin = BuiltinFunction::of(name).ok_or_else(|| EvalError::MissingBody {
                name: name.as_str().to_owned(),
            })?;
            match (Kernel::of(builtin), args) {
                (Kernel::Unary(kernel), [arg]) => kernel(&child(arg, live_after)?),
                (Kernel::Binary(kernel), [first, second]) => {
                    let v0 = child(first, live_after || is_memoized(&second.view()))?;
                    kernel(&v0, &child(second, live_after)?)
                }
                (Kernel::Ternary(kernel), [first, second, third]) => {
                    let third_memo = is_memoized(&third.view());
                    let v0 = child(
                        first,
                        live_after || is_memoized(&second.view()) || third_memo,
                    )?;
                    let v1 = child(second, live_after || third_memo)?;
                    kernel(&v0, &v1, &child(third, live_after)?)
                }
                (Kernel::Unary(_) | Kernel::Binary(_) | Kernel::Ternary(_), _) => {
                    return Err(EvalError::InvalidProgram);
                }
            }
        }
        ExprView::Derivative { .. } => return Err(EvalError::UnevaluatedDerivative),
    })
}

fn reduce_compact_poly<'expr, Value: EvaluationValue>(
    node: &'expr Expr,
    live_after: bool,
    mut child: impl FnMut(&'expr Expr, bool) -> Result<Value, EvalError>,
) -> Result<Value, EvalError> {
    let monomials = node.monomials().ok_or(EvalError::InvalidProgram)?;
    let total_monomials = monomials.len();
    let mut sum: Option<Value> = None;
    for (mono_idx, (coefficient, powers)) in monomials.enumerate() {
        let has_later_mono = live_after || mono_idx.saturating_add(1) < total_monomials;
        let total_powers = powers.len();
        let mut product = if coefficient.is_exact_one() {
            None
        } else {
            Some(child(coefficient, has_later_mono || total_powers > 0)?)
        };
        for (pow_idx, (base, count)) in powers.enumerate() {
            let has_later_pow = has_later_mono || pow_idx.saturating_add(1) < total_powers;
            let base_value = child(base, has_later_pow)?;
            let factor = if count > 1 {
                if Value::OPTIMIZE_FLOATS {
                    let mut power = base_value.clone();
                    for bit in (0..count.ilog2()).rev() {
                        power = power.clone().mul(&power);
                        if count & (1_u32 << bit) != 0 {
                            power = power.mul(&base_value);
                        }
                    }
                    power
                } else {
                    let number = Number::from_int(i64::from(count));
                    let exponent = Value::from_number(&number)
                        .ok_or(EvalError::UnsupportedLiteral { value: number })?;
                    Value::pow(&base_value, &exponent)
                }
            } else {
                base_value
            };
            product = Some(match product {
                Some(value) => value.mul(factor),
                None => factor,
            });
        }
        let term = match product {
            Some(value) => value,
            None => child(coefficient, has_later_mono)?,
        };
        sum = Some(match sum {
            Some(value) => value.add(term),
            None => term,
        });
    }
    sum.ok_or(EvalError::InvalidProgram)
}
