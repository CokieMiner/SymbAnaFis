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

use super::{BuiltinFunction, Expr, FunctionDefinition, FunctionError};

/// An opaque function identity with a static builtin or an owning custom name.
///
/// Builtins require no allocation or registry lock. Custom handles retain their
/// registered name and immutable arity, even after removal from their context.
/// Distinct registrations own distinct identities. The last handle or expression
/// releases unused metadata.
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
    minimum: usize,
    maximum: usize,
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
            FunctionIdentity::Custom(function) => function.minimum..=function.maximum,
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
    /// Create an isolated identity with immutable name and arity metadata.
    pub fn local(name: Arc<str>, arity: RangeInclusive<usize>) -> FunctionName {
        FunctionName(FunctionIdentity::Custom(Arc::new(CustomFunction {
            name,
            minimum: *arity.start(),
            maximum: *arity.end(),
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
