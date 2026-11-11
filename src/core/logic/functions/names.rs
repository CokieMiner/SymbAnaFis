//! Function identities: static builtins and owned custom definitions.

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
};

use alloc::{sync::Arc, vec::Vec};

use super::{BuiltinFunction, Expr, FunctionDefinition, FunctionError, UserFunction};

/// A function identity: a builtin, or a registered custom definition.
///
/// Builtins need no allocation or registry lock. A custom handle owns its name,
/// arity, body, and partial derivatives, which outlive name removal and context
/// destruction. Registered definitions are immutable, each registration is a
/// distinct identity, and equality and hashing never inspect callbacks.
#[derive(Clone, Debug)]
pub struct Function(FunctionIdentity);

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

impl PartialEq for Function {
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

impl Eq for Function {}

impl PartialOrd for Function {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Function {
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

impl Hash for Function {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        match &self.0 {
            FunctionIdentity::Builtin(function) => function.hash(state),
            FunctionIdentity::Custom(text) => Arc::as_ptr(text).cast::<()>().hash(state),
        }
    }
}

impl AsRef<str> for Function {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Function {
    /// The call `self(args)`, with builtin aliases and literal folding applied.
    /// A custom call stays symbolic; its body and partials are used separately.
    ///
    /// # Errors
    /// Returns [`FunctionError::InvalidArity`] when `args.len()` is outside the
    /// arity.
    pub fn call(&self, args: Vec<Expr>) -> Result<Expr, FunctionError> {
        FunctionDefinition::validate(self, args.len())?;
        Ok(FunctionDefinition::construct(self.clone(), args))
    }

    /// The builtin named `name` or one of its aliases.
    #[must_use]
    pub fn builtin(name: &str) -> Option<Self> {
        BuiltinFunction::from_name(name).map(BuiltinFunction::function)
    }

    /// The accepted argument counts.
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

    /// The definition of a custom function, borrowed without registry access;
    /// `None` for a builtin. It outlives name removal and context destruction.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use symb_anafis::{Context, Expr, FunctionError, UserFunction};
    /// let function = {
    ///     let context = Context::new()?;
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
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    #[must_use]
    pub fn definition(&self) -> Option<&UserFunction> {
        match &self.0 {
            FunctionIdentity::Builtin(_) => None,
            FunctionIdentity::Custom(function) => Some(&function.definition),
        }
    }

    /// The function name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            FunctionIdentity::Builtin(function) => function.as_str(),
            FunctionIdentity::Custom(function) => &function.name,
        }
    }
}

impl BuiltinFunction {
    /// The identity of this builtin, without allocation or name interning.
    pub const fn function(self) -> Function {
        Function(FunctionIdentity::Builtin(self))
    }

    /// The builtin that `function` names. A custom function names none, even
    /// when it shares a builtin's spelling.
    pub const fn of(function: &Function) -> Option<Self> {
        match &function.0 {
            FunctionIdentity::Builtin(builtin) => Some(*builtin),
            FunctionIdentity::Custom(_) => None,
        }
    }
}

impl FunctionDefinition {
    /// Wrap a custom definition in a new identity. The caller validates the
    /// arity before publishing the handle.
    pub fn local(name: Arc<str>, definition: UserFunction) -> Function {
        Function(FunctionIdentity::Custom(Arc::new(CustomFunction {
            name,
            definition,
        })))
    }

    /// Check that `actual` arguments fit the function's arity.
    pub fn validate(function: &Function, actual: usize) -> Result<(), FunctionError> {
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
}
