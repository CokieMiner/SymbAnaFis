//! Concrete substitutions with original-node and associative subset matching.

use alloc::vec::Vec;

use hashbrown::hash_map::Entry;

use crate::collections::HashMap;

use super::{Expr, SubstitutionError};

/// Replace one concrete expression with another throughout an input DAG.
///
/// Matching uses canonical node identity, including symbol and function
/// namespaces. Sum and product sources also match groups of original operands;
/// unmatched operands remain in place. The destination may introduce symbols,
/// contain the source, or
/// represent an unrelated mathematical expression. Inserted destinations are
/// not substituted again during the same pass.
/// Positive integer powers and collected integer addends also supply implicit
/// disjoint groups. For example, `x^2 -> u` maps `x^5` to `u^2*x`, and
/// `x+y -> u` maps `2*x+2*y` to `2*u`. Finite nonzero scalar factors permit
/// coefficient quotients through the numeric backend: `2*x -> u` maps `3*x`
/// to `(3/2)*u`, and maps `x` to `(1/2)*u`. Sum sources accept common signed
/// and fractional scales, and quotient sources preserve unmatched denominators.
/// Manifest common symbolic factors also participate: `x+y -> u` maps
/// `f(z)*x + f(z)*y` to `f(z)*u`, without dividing by `f(z)`.
/// Unmatched symbolic factors and function arguments remain traversal inputs:
/// `2*x -> u` maps `f(x)*x` to `f(u/2)*u/2`.
/// Exact rational and nested integer powers expose integral groups when zero
/// exclusions survive: `x^2 -> u` maps `1/x^4` to `1/u^2`, and
/// `sqrt(x) -> u` maps `x^(3/2)` to `u^3`.
/// A source with a pole does not introduce that pole into a defined input.
/// Fractional outer powers retain their principal base: `sqrt(x^2)` does not
/// expose `x` as its square root. Original surviving bases and children are
/// traversed; generated power counts are not new sources in the same pass.
/// Scalar quotient evaluation
/// reduces before promotion, avoiding oversized integer power intermediates.
///
/// A change of variable supplies the chosen inverse, \(x\to g(u)\). Every
/// scalar occurrence of \(x\) is replaced, including function arguments,
/// exponents, denominators, and compact polynomial bases. If \(g(u)\) does not
/// contain \(x\), these scalar occurrences disappear. The caller chooses the
/// domain and inverse branch; no domain proof or equation solving is required.
/// An unevaluated derivative retains its differentiation-variable metadata.
/// Changing that operator requires a calculus transformation.
///
/// ```
/// use symb_anafis::{Context, Number, Substitution};
/// let context = Context::new();
/// let x = context.symb("x")?;
/// let u = context.symb("u")?;
/// let inverse = u.sqrt();
/// let change = Substitution::new(x.into(), inverse.clone());
/// let result = change.apply_all(&(x.pow(5) + x.sin()));
/// let exponent = Number::from_int(5) / Number::from_int(2);
/// assert_eq!(result, u.pow(exponent) + inverse.sin());
/// assert!(!result.contains_symbol(x));
/// # Ok::<(), symb_anafis::SymbolError>(())
/// ```
///
/// ```
/// use symb_anafis::{Context, Substitution, UserFunction};
/// let context = Context::new();
/// let x = context.symb("x")?;
/// let y = context.symb("y")?;
/// let c = context.symb("c")?;
/// let f = context.register_function("f", UserFunction::new(2..=2))?;
/// let pair = context.register_function("pair", UserFunction::new(2..=2))?;
/// let call = f.call(vec![x.into(), y.into()])?;
/// let substitution = Substitution::new(call.clone(), c.into());
/// let input = pair.call(vec![call.clone(), call])?;
/// assert_eq!(substitution.apply_all(&input), pair.call(vec![c.into(), c.into()])?);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// ```
/// use symb_anafis::{Context, Expr, Substitution};
/// let context = Context::new();
/// let x = context.symb("x")?;
/// let u = context.symb("u")?;
/// let substitution = Substitution::new(2*x, u.into());
/// assert_eq!(substitution.apply_all(&(3*x)), (Expr::number(3)/2)*u);
/// assert_eq!(substitution.apply_all(&x.to_expr()), (Expr::number(1)/2)*u);
/// # Ok::<(), symb_anafis::SymbolError>(())
/// ```
#[derive(Clone, Debug)]
pub struct Substitution {
    from: Expr,
    to: Expr,
}

/// A conflict-checked set of simultaneous concrete substitutions.
///
/// Every source is matched against an original node before its children are
/// rebuilt. A matched parent replaces its entire subtree. Destinations and
/// newly rebuilt nodes are not matched again, so swaps are simultaneous and
/// replacements containing their own sources terminate after one pass.
/// Polynomial monomials participate through the logical expression projection.
/// Overlapping operand groups follow entry order; whole-node matches take
/// precedence. Disjoint groups are replaced without matching inserted values.
/// Surviving common factors remain original traversal inputs. Numeric quotients
/// and exponent counts created by decomposition are not new matching targets.
///
/// ```
/// use symb_anafis::{Context, Substitutions, UserFunction};
/// let context = Context::new();
/// let x = context.symb("x")?;
/// let y = context.symb("y")?;
/// let pair = context.register_function("pair", UserFunction::new(2..=2))?;
/// let substitutions = Substitutions::new([
///     (x.into(), y.into()), (y.into(), x.into()),
/// ])?;
/// let input = pair.call(vec![x.into(), y.into()])?;
/// assert_eq!(substitutions.apply_all(&input), pair.call(vec![y.into(), x.into()])?);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct Substitutions {
    replacements: HashMap<Expr, Expr>,
    order: Vec<Expr>,
}

impl Substitution {
    /// Define a concrete source and destination without algebraic validation.
    #[must_use]
    pub const fn new(from: Expr, to: Expr) -> Self {
        Self { from, to }
    }

    /// Borrow the concrete source expression.
    #[must_use]
    pub const fn from(&self) -> &Expr {
        &self.from
    }

    /// Borrow the concrete destination expression.
    #[must_use]
    pub const fn to(&self) -> &Expr {
        &self.to
    }
}

impl Substitutions {
    /// Collect simultaneous entries, accepting identical duplicates.
    ///
    /// Canonically equal sources must have canonically equal destinations.
    /// Structural literal identity distinguishes exact and approximate numeric
    /// representations, signed zeros, and NaN payloads.
    ///
    /// # Errors
    /// Returns [`SubstitutionError::Conflict`] when one source has distinct
    /// destinations. No partially collected substitution set is returned.
    pub fn new(entries: impl IntoIterator<Item = (Expr, Expr)>) -> Result<Self, SubstitutionError> {
        let mut replacements = HashMap::new();
        let mut order = Vec::new();
        for (from, to) in entries {
            match replacements.entry(from) {
                Entry::Vacant(entry) => {
                    order.push(entry.key().clone());
                    entry.insert(to);
                }
                Entry::Occupied(entry) => {
                    if entry.get() != &to {
                        return Err(SubstitutionError::Conflict {
                            from: entry.key().clone(),
                            first: entry.get().clone(),
                            second: to,
                        });
                    }
                }
            }
        }
        Ok(Self {
            replacements,
            order,
        })
    }

    /// Borrow the destination associated with this exact source, if present.
    #[must_use]
    pub fn get(&self, from: &Expr) -> Option<&Expr> {
        self.replacements.get(from)
    }

    /// Borrow distinct source/destination pairs in their original entry order.
    pub fn iter(&self) -> impl Iterator<Item = (&Expr, &Expr)> {
        self.order
            .iter()
            .filter_map(|from| self.replacements.get(from).map(|to| (from, to)))
    }

    /// Return the number of distinct source expressions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.replacements.len()
    }

    /// Return whether the set contains no source expressions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty()
    }
}
