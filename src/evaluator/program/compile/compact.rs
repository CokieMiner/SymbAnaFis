//! Compact multivariate polynomial lowering via bottom-up Horner tries.

use core::{iter::from_fn, ptr::eq};

use alloc::{borrow::Cow, vec, vec::Vec};

use crate::{
    collections::FixedMap,
    core::{Expr, StoredForm},
    errors::EvalError,
};

use super::{Compilation, EvaluationValue};

struct TrieFrame<'expr> {
    base: &'expr Expr,
    exponent: u32,
    power: u32,
    constant: Option<u32>,
    children: Vec<(u32, u32)>,
}

impl<Value: EvaluationValue> Compilation<Value> {
    /// Lower compact storage as the sum of its monomials.
    pub fn compact_polynomial(
        &mut self,
        expression: &Expr,
        values: &FixedMap<Cow<'_, Expr>, u32>,
    ) -> Result<u32, EvalError> {
        let terms = expression.monomials().ok_or(EvalError::InvalidProgram)?;
        if !Value::OPTIMIZE_FLOATS {
            let mut monomials = Vec::with_capacity(terms.len());
            let mut factors = Vec::new();
            for (coefficient, powers) in terms {
                factors.clear();
                if !coefficient.is_exact_one() {
                    factors.push(values[coefficient]);
                }
                for (base, count) in powers {
                    factors.push(self.stored_power(values[base], count)?);
                }
                if factors.is_empty() {
                    factors.push(values[coefficient]);
                }
                monomials.push(self.group(factors.iter().copied(), true)?);
            }
            return self.group(monomials, false);
        }
        let mut stack: Vec<TrieFrame<'_>> = Vec::new();
        let mut spare: Vec<Vec<(u32, u32)>> = Vec::new();
        let mut root_children: Vec<(u32, u32)> = Vec::new();
        let mut root_constant = None;
        let mut terms_buffer = Vec::new();
        for (coefficient, mut powers) in terms {
            let mut lcp = 0_usize;
            let mut first_unmatched = None;
            for (base, count) in &mut powers {
                if stack
                    .get(lcp)
                    .is_some_and(|frame| eq(frame.base, base) && frame.exponent == count)
                {
                    lcp = lcp.saturating_add(1);
                } else {
                    first_unmatched = Some((base, count));
                    break;
                }
            }
            while stack.len() > lcp {
                let frame = stack.pop().ok_or(EvalError::InvalidProgram)?;
                let sub =
                    self.reduce_trie_node(frame.constant, &frame.children, &mut terms_buffer)?;
                let mut children = frame.children;
                children.clear();
                spare.push(children);
                if let Some(parent) = stack.last_mut() {
                    parent.children.push((sub, frame.power));
                } else {
                    root_children.push((sub, frame.power));
                }
            }
            for (base, count) in first_unmatched.into_iter().chain(powers) {
                let power = self.stored_power(values[base], count)?;
                stack.push(TrieFrame {
                    base,
                    exponent: count,
                    power,
                    constant: None,
                    children: spare.pop().unwrap_or_default(),
                });
            }
            let coeff_source = if coefficient.is_exact_one() {
                self.coefficient(None)?
            } else {
                values[coefficient]
            };
            if let Some(top) = stack.last_mut() {
                top.constant = Some(coeff_source);
            } else {
                root_constant = Some(coeff_source);
            }
        }
        while let Some(frame) = stack.pop() {
            let sub = self.reduce_trie_node(frame.constant, &frame.children, &mut terms_buffer)?;
            if let Some(parent) = stack.last_mut() {
                parent.children.push((sub, frame.power));
            } else {
                root_children.push((sub, frame.power));
            }
        }
        self.reduce_trie_node(root_constant, &root_children, &mut terms_buffer)
    }

    fn reduce_trie_node(
        &mut self,
        constant: Option<u32>,
        children: &[(u32, u32)],
        terms: &mut Vec<u32>,
    ) -> Result<u32, EvalError> {
        let scale = |compiler: &mut Self, sub: u32, factor: u32| -> Result<u32, EvalError> {
            if compiler.unit == Some(sub) {
                Ok(factor)
            } else {
                compiler.emit_binary(true, sub, factor)
            }
        };
        match children {
            [] => return constant.ok_or(EvalError::InvalidProgram),
            &[(sub, power)] => {
                let product = scale(self, sub, power)?;
                return constant
                    .map_or(Ok(product), |value| self.emit_binary(false, value, product));
            }
            _ => {}
        }
        terms.clear();
        terms.extend(constant);
        if children.len() <= 16 {
            for (index, &(sub, _)) in children.iter().enumerate() {
                let Some((earlier, rest)) = children.split_at_checked(index) else {
                    continue;
                };
                if earlier.iter().any(|&(candidate, _)| candidate == sub) {
                    continue;
                }
                let powers = rest
                    .iter()
                    .filter(|&&(candidate, _)| candidate == sub)
                    .map(|&(_, power)| power);
                let power_sum = self.group(powers, false)?;
                terms.push(scale(self, sub, power_sum)?);
            }
        } else {
            let mut group_of: FixedMap<u32, usize> = FixedMap::default();
            group_of.reserve(children.len());
            let mut groups: Vec<(u32, usize, usize)> = Vec::with_capacity(children.len());
            let mut next = vec![usize::MAX; children.len()];
            for (index, &(sub, _)) in children.iter().enumerate() {
                if let Some((_, _, tail)) =
                    group_of.get(&sub).and_then(|&slot| groups.get_mut(slot))
                {
                    if let Some(link) = next.get_mut(*tail) {
                        *link = index;
                    }
                    *tail = index;
                } else {
                    group_of.insert(sub, groups.len());
                    groups.push((sub, index, index));
                }
            }
            for (sub, first, _) in groups {
                let mut cursor = Some(first);
                let powers = from_fn(|| {
                    let current = cursor?;
                    cursor = next
                        .get(current)
                        .copied()
                        .filter(|&item| item != usize::MAX);
                    children.get(current).map(|&(_, power)| power)
                });
                let power_sum = self.group(powers, false)?;
                terms.push(scale(self, sub, power_sum)?);
            }
        }
        self.group(terms.iter().copied(), false)
    }
}
