//! Copy symbol handles and public registry operations.

#![expect(
    unsafe_code,
    reason = "a handle is a pointer into a live Arc allocation, read and shared without a registry lookup"
)]

use core::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Add, Div, Mul, Neg, Sub},
    ptr::NonNull,
};

use alloc::{string::String, sync::Arc, vec::Vec};

use num_anafis::Number;

use super::{Context, Expr, Facts, Properties, Property, SYMBOLS, SymbolData, SymbolError};

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

// SAFETY: a handle behaves as `&'owner SymbolData` while carrying its `Arc`'s
// provenance, and `SymbolData` is `Send + Sync` (immutable identity/name fields
// plus atomically synchronized `facts`).
unsafe impl Send for Symbol<'_> {}

// SAFETY: as for `Send`; shared access through `&SymbolData` only reads
// immutable fields or synchronizes atomic `facts` updates.
unsafe impl Sync for Symbol<'_> {}

impl Symbol<'static> {
    /// Allocate a distinct anonymous symbol.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when a new identity cannot be allocated.
    pub fn anon() -> Result<Self, SymbolError> {
        SYMBOLS.anonymous()
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
    pub fn intern(name: &str) -> Result<Self, SymbolError> {
        SYMBOLS.intern(name)
    }

    /// Register a new global name.
    ///
    /// # Errors
    /// Returns [`SymbolError::DuplicateName`] for a bound name and
    /// [`SymbolError::IdExhausted`] when no identity can be allocated.
    pub fn register(name: &str) -> Result<Self, SymbolError> {
        SYMBOLS.register(name)
    }

    /// Look up an existing global name without creating it.
    #[must_use]
    pub fn get(name: &str) -> Option<Self> {
        SYMBOLS.get(name)
    }

    /// Remove a global name binding. Existing handles remain usable; registering
    /// the name again allocates a distinct identity.
    #[must_use]
    pub fn remove(name: &str) -> bool {
        SYMBOLS.remove(name)
    }

    /// Count current global registrations, including anonymous symbols.
    #[must_use]
    pub fn count() -> usize {
        SYMBOLS.len()
    }

    /// List global names in lexicographic order.
    #[must_use]
    pub fn names() -> Vec<String> {
        SYMBOLS.names()
    }
}

impl<'owner> Symbol<'owner> {
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
    #[inline]
    pub fn to_expr(&self) -> Expr {
        SymbolData::to_expr(*self)
    }

    /// Remove this symbol's global or anonymous registration, keeping its
    /// metadata, and return whether one was removed. Context bindings are
    /// removed through the context.
    #[must_use]
    pub fn unregister(self) -> bool {
        // SAFETY: the pointer addresses metadata of a live `Arc` allocation,
        // kept alive for `'owner`; immutable fields never change and `facts`
        // uses interior atomic mutability.
        SYMBOLS.unregister(unsafe { self.0.as_ref() })
    }

    /// Borrow the name for `'owner`, or `None` for an anonymous symbol.
    #[must_use]
    pub fn as_str(&self) -> Option<&'owner str> {
        // SAFETY: as in `unregister`, and the immutable `Arc<str>` lives for
        // at least `'owner`.
        let data: &'owner SymbolData = unsafe { self.0.as_ref() };
        data.name()
    }

    /// The identity.
    #[must_use]
    pub const fn id(&self) -> usize {
        // SAFETY: as in `unregister`.
        unsafe { self.0.as_ref() }.id()
    }

    /// Read the declared mathematical properties of this symbol (always
    /// including [`Property::Finite`]).
    #[must_use]
    pub fn properties(&self) -> Properties {
        // SAFETY: as in `unregister`.
        unsafe { self.0.as_ref() }.facts().into_properties()
    }

    /// Replace the declared mathematical properties of this symbol, closing all
    /// logical implications and invalidating cached expression properties and
    /// settled simplification masks when the symbol's facts change.
    ///
    /// [`Property::Finite`] always holds on symbols. Passing `&[]` resets the
    /// symbol to a general finite complex scalar. Global symbols created via
    /// [`Self::intern`] or [`Self::register`] share one identity process-wide;
    /// use [`crate::Context`] for isolated assumptions.
    ///
    /// # Errors
    /// Returns [`SymbolError::ContradictoryProperties`] when two declared
    /// properties contradict each other (such as [`Property::Positive`] and
    /// [`Property::Negative`], or [`Property::Zero`] and [`Property::Nonzero`]).
    pub fn set_properties(&self, properties: &[Property]) -> Result<(), SymbolError> {
        let Some(facts) = Facts::from_properties(properties, true) else {
            return Err(SymbolError::ContradictoryProperties);
        };
        // SAFETY: as in `unregister`.
        unsafe { self.0.as_ref() }.set_facts(facts);
        Ok(())
    }
}

// Each identity has exactly one metadata allocation, so pointer equality is
// identity equality. Hashing uses the identity, which is deterministic across
// runs.
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

impl Display for Symbol<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.as_str() {
            Some(name) => formatter.write_str(name),
            None => write!(formatter, "${}", self.id()),
        }
    }
}

impl SymbolData {
    /// Borrow the metadata allocation of `symbol` without touching its reference count.
    #[must_use]
    pub const fn borrow(symbol: Symbol<'_>) -> &Self {
        // SAFETY: as in `Symbol::unregister`, `symbol.0` addresses metadata of
        // a live `Arc` allocation kept alive for at least `'owner`.
        unsafe { symbol.0.as_ref() }
    }

    /// Share the metadata allocation that `symbol` borrows.
    #[must_use]
    pub fn of(symbol: Symbol<'_>) -> Arc<Self> {
        let pointer = symbol.0.as_ptr().cast_const();
        // SAFETY: handles are created only from `Arc::as_ptr` (in
        // `SymbolData::symbol` and `SymbolData::scoped_symbol`) or from
        // `Arc::into_raw` (in `SymbolData::permanent_symbol`), so `pointer`
        // carries the provenance of a live `Arc` allocation, and `'owner`
        // keeps one of its strong owners alive. Raising the count first gives
        // the reconstructed `Arc` its own strong reference.
        unsafe {
            Arc::increment_strong_count(pointer);
            Arc::from_raw(pointer)
        }
    }

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

    /// A `'static` handle backed by a leaked `Arc::into_raw` allocation.
    ///
    /// # Safety
    /// `pointer` must originate from `Arc::into_raw` on a `SymbolData`
    /// allocation whose strong count remains nonzero for the process lifetime.
    pub const unsafe fn permanent_symbol(pointer: NonNull<Self>) -> Symbol<'static> {
        Symbol(pointer, PhantomData)
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
