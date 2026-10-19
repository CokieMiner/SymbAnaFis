//! Static builtins and owning identities for registered custom functions.

#![cfg_attr(
    test,
    expect(
        clippy::self_named_module_files,
        reason = "descendant tests access the private identity representation without widening its fields"
    )
)]

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
};

use alloc::{sync::Arc, vec::Vec};

use super::{BuiltinFunction, Expr, FunctionDefinition, FunctionError, UserFunction};

/// An opaque function identity with a static builtin or an owning custom definition.
///
/// Builtins require no allocation or registry lock. Custom handles retain their
/// name, arity, body, and partial derivatives after removal or context destruction.
/// Registered definitions are immutable. Distinct registrations own distinct
/// identities; equality and hashing do not inspect callbacks. The final handle
/// or expression releases the definition and its captured owners.
#[derive(Clone, Debug)]
pub struct FunctionName(FunctionIdentity);

#[derive(Clone, Debug)]
enum FunctionIdentity {
    Builtin(BuiltinFunction),
    Custom(Arc<CustomFunction>),
}

#[derive(Debug)]
struct CustomFunction {
    name: Arc<str>,
    definition: UserFunction,
}

impl PartialEq for FunctionName {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (FunctionIdentity::Builtin(left), FunctionIdentity::Builtin(right)) => left == right,
            (FunctionIdentity::Custom(left), FunctionIdentity::Custom(right)) => {
                Arc::ptr_eq(left, right)
            }
            (FunctionIdentity::Builtin(_), FunctionIdentity::Custom(_))
            | (FunctionIdentity::Custom(_), FunctionIdentity::Builtin(_)) => false,
        }
    }
}

impl Eq for FunctionName {}

impl PartialOrd for FunctionName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FunctionName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str()
            .cmp(other.as_str())
            .then_with(|| match (&self.0, &other.0) {
                (FunctionIdentity::Builtin(left), FunctionIdentity::Builtin(right)) => {
                    left.cmp(right)
                }
                (FunctionIdentity::Builtin(_), FunctionIdentity::Custom(_)) => Ordering::Less,
                (FunctionIdentity::Custom(_), FunctionIdentity::Builtin(_)) => Ordering::Greater,
                (FunctionIdentity::Custom(left), FunctionIdentity::Custom(right)) => {
                    Arc::as_ptr(left)
                        .cast::<()>()
                        .cmp(&Arc::as_ptr(right).cast::<()>())
                }
            })
    }
}

impl Hash for FunctionName {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        match &self.0 {
            FunctionIdentity::Builtin(function) => function.hash(state),
            FunctionIdentity::Custom(text) => Arc::as_ptr(text).cast::<()>().hash(state),
        }
    }
}

impl AsRef<str> for FunctionName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FunctionName {
    /// Apply this known function to a validated argument list.
    /// Custom functions remain symbolic; their bodies and partials are separate.
    ///
    /// # Errors
    /// Returns [`FunctionError::InvalidArity`] for an unaccepted argument count.
    ///
    /// # Panics
    /// Numeric folding can propagate numeric backend capacity panics.
    pub fn call(&self, args: Vec<Expr>) -> Result<Expr, FunctionError> {
        FunctionDefinition::validate(self, args.len())?;
        Ok(FunctionDefinition::construct(self.clone(), args))
    }

    /// Resolve a builtin or its documented alias. Unknown names return `None`.
    #[must_use]
    pub fn builtin(name: &str) -> Option<Self> {
        BuiltinFunction::from_name(name).map(BuiltinFunction::function)
    }

    /// Return the registered range of accepted argument counts.
    #[must_use]
    pub fn arity(&self) -> RangeInclusive<usize> {
        match &self.0 {
            FunctionIdentity::Builtin(function) => {
                let arity = function.definition().arity();
                arity..=arity
            }
            FunctionIdentity::Custom(function) => function.definition.arity(),
        }
    }

    /// Borrow a custom function's immutable definition without registry access.
    /// Builtins return `None`; their behavior belongs to the static builtin registry.
    /// The definition remains available after removal or context destruction.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use symb_anafis::{Context, Expr, FunctionError, UserFunction};
    /// let function = {
    ///     let context = Context::new();
    ///     context.register_function(
    ///         "add",
    ///         UserFunction::new(2..=2)
    ///             .body(Arc::new(|args| Expr::sum(args.to_vec())))
    ///             .partial(0, Arc::new(|_| Expr::number(1)))?,
    ///     )?
    /// };
    /// let definition = function.definition().expect("registered custom function");
    /// let body = definition.body_fn().expect("defined body");
    /// assert_eq!(body(&[Expr::number(2), Expr::number(3)]), Expr::number(5));
    /// assert!(definition.partial_fn(0).is_some());
    /// # Ok::<(), FunctionError>(())
    /// ```
    #[must_use]
    pub fn definition(&self) -> Option<&UserFunction> {
        match &self.0 {
            FunctionIdentity::Builtin(_) => None,
            FunctionIdentity::Custom(function) => Some(&function.definition),
        }
    }

    /// Borrow the name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            FunctionIdentity::Builtin(function) => function.as_str(),
            FunctionIdentity::Custom(function) => &function.name,
        }
    }
}

impl BuiltinFunction {
    /// Construct the opaque identity without allocating or interning a name.
    pub const fn function(self) -> FunctionName {
        FunctionName(FunctionIdentity::Builtin(self))
    }
}

impl FunctionDefinition {
    /// Move a custom definition into its isolated, shared function identity.
    /// Sealed callers validate the arity before publishing the handle.
    pub fn local(name: Arc<str>, definition: UserFunction) -> FunctionName {
        FunctionName(FunctionIdentity::Custom(Arc::new(CustomFunction {
            name,
            definition,
        })))
    }

    /// Validate a public call before constructing its expression or pattern.
    pub fn validate(function: &FunctionName, actual: usize) -> Result<(), FunctionError> {
        let arity = function.arity();
        if !arity.contains(&actual) {
            return Err(FunctionError::InvalidArity {
                minimum: *arity.start(),
                maximum: *arity.end(),
                actual,
            });
        }
        Ok(())
    }

    /// Dispatch by the private identity; custom spellings acquire no builtin rules.
    pub const fn get(function: &FunctionName) -> Option<&'static Self> {
        match &function.0 {
            FunctionIdentity::Builtin(builtin) => Some(builtin.definition()),
            FunctionIdentity::Custom(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
