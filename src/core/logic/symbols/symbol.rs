//! Copy symbol handles and public registry operations.

#![expect(
    unsafe_code,
    reason = "a handle is a pointer into a live Arc allocation, read and shared without a registry lookup"
)]

use core::{
    cmp::Ordering,
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Add, Div, Mul, Neg, Sub},
    ptr::NonNull,
};

use alloc::{string::String, sync::Arc, vec::Vec};

use num_anafis::Number;

use super::{Context, Expr, SYMBOLS, SymbolData, SymbolError};

/// A symbol identity: a `Copy` handle borrowed from the owner of its metadata.
///
/// A local handle borrows a [`Context`] or an [`Expr`] and cannot outlive it;
/// expressions built from it own the metadata and may outlive the context. A
/// global handle is `'static`, with permanent metadata. Name removal leaves
/// handles valid. Identities are one native word, never reused, and unique
/// within the running process; they are not portable serialized identities.
///
/// The handle is one pointer to the metadata, so reading the identity, the
/// name, or the shared metadata takes no registry lookup.
#[derive(Clone, Copy)]
pub struct Symbol<'owner>(NonNull<SymbolData>, PhantomData<&'owner SymbolData>);

// SAFETY: a handle is a shared borrow of immutable `SymbolData`, which is
// `Send` and `Sync`; the raw pointer only keeps the provenance of its `Arc`.
unsafe impl Send for Symbol<'_> {}

// SAFETY: as for `Send`; shared access to `SymbolData` is read-only.
unsafe impl Sync for Symbol<'_> {}

impl Symbol<'static> {
    /// Allocate a distinct anonymous symbol.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when a new identity cannot be allocated.
    pub fn anon() -> Result<Self, SymbolError> {
        SYMBOLS.anonymous().map(SymbolData::symbol)
    }

    /// The global symbol with identity `id`. [`Context::symbol_from_id`]
    /// resolves a local identity.
    ///
    /// # Errors
    /// Returns [`SymbolError::InvalidId`] for an unknown or local identity,
    /// including one still kept by a context or expression.
    ///
    /// ```
    /// use symb_anafis::{Symbol, SymbolError};
    /// let x = Symbol::intern("checked_symbol_example")?;
    /// assert_eq!(Symbol::from_id(x.id())?, x);
    /// assert_eq!(Symbol::from_id(0), Err(SymbolError::InvalidId(0)));
    /// # Ok::<(), SymbolError>(())
    /// ```
    pub fn from_id(id: usize) -> Result<Self, SymbolError> {
        SymbolData::lookup_permanent(id)
    }

    /// Get or create a globally named symbol.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when a new identity cannot be allocated.
    pub fn intern(name: impl AsRef<str>) -> Result<Self, SymbolError> {
        SYMBOLS.intern(name.as_ref()).map(SymbolData::symbol)
    }

    /// Register a new global name.
    ///
    /// # Errors
    /// Returns [`SymbolError::DuplicateName`] for a bound name and
    /// [`SymbolError::IdExhausted`] when no identity can be allocated.
    pub fn register(name: &str) -> Result<Self, SymbolError> {
        SYMBOLS.register(name).map(SymbolData::symbol)
    }

    /// Look up an existing global name without creating it.
    ///
    /// # Errors
    /// Returns [`SymbolError::NotFound`] for an unbound name.
    pub fn get(name: &str) -> Result<Self, SymbolError> {
        SYMBOLS.get(name)
    }

    /// Remove a global name binding. Existing handles remain usable; registering
    /// the name again allocates a distinct identity.
    pub fn remove(name: &str) -> bool {
        SYMBOLS.remove(name)
    }

    /// Count current global registrations, including anonymous symbols.
    #[must_use]
    pub fn count() -> usize {
        SYMBOLS.len()
    }

    /// List global names in unspecified order.
    #[must_use]
    pub fn names() -> Vec<String> {
        SYMBOLS.names()
    }
}

impl Symbol<'_> {
    /// This symbol as an expression. It works after name removal, and the
    /// expression may outlive the borrowed owner.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let expression = {
    ///     let context = Context::new()?;
    ///     let x = context.symb("retained_context_symbol")?;
    ///     x + 1
    /// };
    /// assert!(expression.contains_var("retained_context_symbol"));
    /// # Ok::<(), symb_anafis::SymbolError>(())
    /// ```
    ///
    /// A local handle cannot escape its context:
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let x = {
    ///     let context = Context::new().expect("context");
    ///     context.symb("x").expect("symbol allocation")
    /// };
    /// let expression = x + 1;
    /// ```
    #[must_use]
    pub fn to_expr(&self) -> Expr {
        self.metadata().to_expr()
    }

    /// Remove this symbol's global or anonymous registration, keeping its
    /// metadata, and return whether one was removed. Context bindings are
    /// removed through the context.
    pub fn unregister(self) -> bool {
        // SAFETY: the pointer addresses metadata of a live `Arc` allocation,
        // kept alive for `'owner`; the metadata is never mutated.
        SYMBOLS.unregister(unsafe { self.0.as_ref() })
    }

    /// The shared name, or `None` for an anonymous symbol.
    #[must_use]
    pub fn name(&self) -> Option<Arc<str>> {
        // SAFETY: as in `unregister`.
        unsafe { self.0.as_ref() }.name_arc()
    }

    /// The identity.
    #[must_use]
    pub const fn id(&self) -> usize {
        // SAFETY: as in `unregister`.
        unsafe { self.0.as_ref() }.id()
    }

    /// Share the metadata allocation this handle borrows.
    #[must_use]
    pub fn metadata(self) -> Arc<SymbolData> {
        let pointer = self.0.as_ptr().cast_const();
        // SAFETY: handles are created only from `Arc::as_ptr` (in
        // `SymbolData::symbol` and `SymbolData::scoped_symbol`), so `pointer`
        // carries the provenance of a live `Arc` allocation, and `'owner`
        // keeps one of its strong owners alive. Raising the count first gives
        // the reconstructed `Arc` its own strong reference.
        unsafe {
            Arc::increment_strong_count(pointer);
            Arc::from_raw(pointer)
        }
    }

    /// The derivative of this symbol; see [`Expr::derivative`].
    #[inline]
    #[must_use]
    pub fn derivative(&self, var: Self, order: u32) -> Expr {
        self.to_expr().derivative(var, order)
    }

    /// The gradient of this symbol; see [`Expr::gradient`].
    #[inline]
    #[must_use]
    pub fn gradient(&self, variables: &[Self]) -> Vec<Expr> {
        self.to_expr().gradient(variables)
    }

    /// The Hessian of this symbol; see [`Expr::hessian`].
    #[inline]
    #[must_use]
    pub fn hessian(&self, variables: &[Self]) -> Vec<Vec<Expr>> {
        self.to_expr().hessian(variables)
    }
}

// Each identity has exactly one metadata allocation, so pointer equality is
// identity equality. Hashing and ordering use the identity, which is
// deterministic across runs.
impl PartialEq for Symbol<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Symbol<'_> {}

impl Hash for Symbol<'_> {
    #[inline]
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        self.id().hash(state);
    }
}

impl PartialOrd for Symbol<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id().cmp(&other.id())
    }
}

impl Debug for Symbol<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("Symbol")
            .field("id", &self.id())
            // SAFETY: as in `Symbol::unregister`.
            .field("name", &unsafe { self.0.as_ref() }.name())
            .finish()
    }
}

impl SymbolData {
    /// A handle borrowing this metadata through one of its `Arc` owners.
    pub fn symbol(self: &Arc<Self>) -> Symbol<'_> {
        // SAFETY: `Arc::as_ptr` points into a live allocation and is nonnull.
        Symbol(
            unsafe { NonNull::new_unchecked(Arc::as_ptr(self).cast_mut()) },
            PhantomData,
        )
    }

    /// A handle bound to `owner`.
    ///
    /// # Safety
    /// The context map of issued identities shared by `owner` holds this
    /// allocation. Those entries are removed only when the last clone sharing
    /// the map is dropped, and `'owner` borrows one such clone.
    pub unsafe fn scoped_symbol<'owner>(
        self: &Arc<Self>,
        _owner: &'owner Context,
    ) -> Symbol<'owner> {
        // SAFETY: `Arc::as_ptr` is nonnull. By the caller's contract, the owner
        // map keeps a strong reference to this allocation for at least `'owner`.
        Symbol(
            unsafe { NonNull::new_unchecked(Arc::as_ptr(self).cast_mut()) },
            PhantomData,
        )
    }
}

impl From<Symbol<'_>> for Expr {
    fn from(symbol: Symbol<'_>) -> Self {
        symbol.to_expr()
    }
}

impl From<&Symbol<'_>> for Expr {
    fn from(symbol: &Symbol<'_>) -> Self {
        symbol.to_expr()
    }
}

macro_rules! symbol_operators {
    ($left:ty) => {
        impl<Right: Into<Expr>> Add<Right> for $left {
            type Output = Expr;
            fn add(self, right: Right) -> Expr {
                Add::<Expr>::add(self.to_expr(), right.into())
            }
        }
        impl<Right: Into<Expr>> Sub<Right> for $left {
            type Output = Expr;
            fn sub(self, right: Right) -> Expr {
                Sub::<Expr>::sub(self.to_expr(), right.into())
            }
        }
        impl<Right: Into<Expr>> Mul<Right> for $left {
            type Output = Expr;
            fn mul(self, right: Right) -> Expr {
                Mul::<Expr>::mul(self.to_expr(), right.into())
            }
        }
        impl<Right: Into<Expr>> Div<Right> for $left {
            type Output = Expr;
            fn div(self, right: Right) -> Expr {
                Div::<Expr>::div(self.to_expr(), right.into())
            }
        }
    };
}

symbol_operators!(Symbol<'_>);
symbol_operators!(&Symbol<'_>);

macro_rules! primitive_operators {
    ($left:ty, $right:ty) => {
        impl Add<$right> for $left {
            type Output = Expr;
            fn add(self, right: $right) -> Expr {
                Add::<Expr>::add(Expr::from(self), right.to_expr())
            }
        }
        impl Sub<$right> for $left {
            type Output = Expr;
            fn sub(self, right: $right) -> Expr {
                Sub::<Expr>::sub(Expr::from(self), right.to_expr())
            }
        }
        impl Mul<$right> for $left {
            type Output = Expr;
            fn mul(self, right: $right) -> Expr {
                Mul::<Expr>::mul(Expr::from(self), right.to_expr())
            }
        }
        impl Div<$right> for $left {
            type Output = Expr;
            fn div(self, right: $right) -> Expr {
                Div::<Expr>::div(Expr::from(self), right.to_expr())
            }
        }
    };
}

primitive_operators!(f64, Symbol<'_>);
primitive_operators!(f64, &Symbol<'_>);
primitive_operators!(i32, Symbol<'_>);
primitive_operators!(i32, &Symbol<'_>);
primitive_operators!(Number, Symbol<'_>);
primitive_operators!(Number, &Symbol<'_>);
primitive_operators!(&Number, Symbol<'_>);
primitive_operators!(&Number, &Symbol<'_>);

impl Neg for Symbol<'_> {
    type Output = Expr;
    fn neg(self) -> Expr {
        Neg::neg(self.to_expr())
    }
}

impl Neg for &Symbol<'_> {
    type Output = Expr;
    fn neg(self) -> Expr {
        Neg::neg(self.to_expr())
    }
}
