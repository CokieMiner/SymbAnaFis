//! Concrete substitutions of expressions and implicit operand groups.

use alloc::vec::Vec;

use crate::collections::{Entry, FixedMap};

use super::{Expr, SubstitutionError};

/// Replace one concrete expression by another throughout an expression.
///
/// Matching uses canonical node identity, so symbols and functions from
/// different namespaces are distinct. The destination may be any expression,
/// including one that contains the source; destinations are not substituted
/// again in the same pass.
///
/// Inside sums and products the source also matches implicit disjoint groups.
/// Unmatched operands stay in place, and generated coefficients and exponents
/// are not matched again.
///
/// - Integer powers and collected addends: `x^2 -> u` maps `x^5` to `u^2*x`,
///   and `x+y -> u` maps `2*x+2*y` to `2*u`.
/// - Numeric coefficients, divided exactly: `2*x -> u` maps `3*x` to
///   `(3/2)*u` and `x` to `(1/2)*u`. Sum sources accept a common signed or
///   fractional scale, and quotient sources keep unmatched denominators.
/// - Common symbolic factors, never divided out: `x+y -> u` maps
///   `f(z)*x + f(z)*y` to `f(z)*u`. Unmatched factors and arguments are still
///   traversed: `2*x -> u` maps `f(x)*x` to `f(u/2)*u/2`.
/// - Exact rational and nested integer powers, when zero exclusions survive:
///   `x^2 -> u` maps `1/x^4` to `1/u^2`, and `sqrt(x) -> u` maps `x^(3/2)` to
///   `u^3`. A fractional outer power keeps its principal base, so `sqrt(x^2)`
///   does not expose `x`, and a source with a pole never introduces that pole
///   into a defined input.
///
/// A change of variable supplies the chosen inverse \(x \to g(u)\). Every
/// scalar occurrence of \(x\) is replaced, in function arguments, exponents,
/// denominators, and compact polynomial bases, so none remains when \(g(u)\)
/// does not contain \(x\). The caller chooses the domain and branch of the
/// inverse. The differentiation variable of an unevaluated derivative is
/// unchanged; changing that operator is a calculus transformation.
///
/// ```
/// use symb_anafis::{Context, Number, Substitution};
/// let context = Context::new()?;
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
/// let context = Context::new()?;
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
/// let context = Context::new()?;
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

/// A conflict-checked set of simultaneous substitutions.
///
/// Every source is matched against an original node before its children are
/// rebuilt, and a matched node is replaced whole. Destinations and rebuilt
/// nodes are not matched again, so swaps are simultaneous and a destination
/// containing its own source is applied once. Whole-node matches take
/// precedence; overlapping operand groups follow entry order. Implicit groups
/// follow [`Substitution`].
///
/// ```
/// use symb_anafis::{Context, Substitutions, UserFunction};
/// let context = Context::new()?;
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
    replacements: FixedMap<Expr, Expr>,
    entries: Vec<(Expr, Expr)>,
}

impl Substitution {
    /// The substitution `from -> to`; no algebraic validation is performed.
    #[must_use]
    pub const fn new(from: Expr, to: Expr) -> Self {
        Self { from, to }
    }

    /// The source expression.
    #[must_use]
    pub const fn from(&self) -> &Expr {
        &self.from
    }

    /// The destination expression.
    #[must_use]
    pub const fn to(&self) -> &Expr {
        &self.to
    }
}

impl Substitutions {
    /// Collect simultaneous entries, accepting identical duplicates.
    ///
    /// Equal sources must have equal destinations. Literal identity
    /// distinguishes exact from approximate values, signed zeros, and NaN
    /// payloads.
    ///
    /// # Errors
    /// Returns [`SubstitutionError::Conflict`] when one source has distinct
    /// destinations. No partially collected substitution set is returned.
    pub fn new(entries: impl IntoIterator<Item = (Expr, Expr)>) -> Result<Self, SubstitutionError> {
        let mut replacements = FixedMap::default();
        let mut ordered = Vec::new();
        for (from, to) in entries {
            match replacements.entry(from) {
                Entry::Vacant(entry) => {
                    ordered.push((entry.key().clone(), to.clone()));
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
            entries: ordered,
        })
    }

    /// The destination of `from`, if it is a source in this set.
    #[must_use]
    pub fn get(&self, from: &Expr) -> Option<&Expr> {
        self.replacements.get(from)
    }

    /// The distinct `(source, destination)` pairs in entry order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&Expr, &Expr)> {
        self.entries.iter().map(|(from, to)| (from, to))
    }

    /// The number of distinct sources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.replacements.len()
    }

    /// Whether the set has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty()
    }
}
