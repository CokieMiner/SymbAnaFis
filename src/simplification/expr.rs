//! Convenience simplification methods on [`Expr`] backed by lazily initialized
//! static preset sequences.

#![expect(
    unsafe_code,
    reason = "an initialized static preset pointer is leaked once and read with Acquire ordering"
)]

use core::{
    ptr::{from_mut, null_mut},
    sync::atomic::{AtomicPtr, Ordering},
};

use alloc::boxed::Box;

use crate::{core::Expr, errors::SimplifyError, sync::Mutex};

use super::{Simplification, SimplificationSequence};

/// One-time static preset cache with a lock-free `Acquire` fast path.
struct Preset {
    cached: AtomicPtr<SimplificationSequence>,
    init: Mutex<()>,
}

impl Preset {
    const fn new() -> Self {
        Self {
            cached: AtomicPtr::new(null_mut()),
            init: Mutex::new(()),
        }
    }

    fn get(
        &'static self,
        build: fn() -> SimplificationSequence,
    ) -> &'static SimplificationSequence {
        let pointer = self.cached.load(Ordering::Acquire);
        if !pointer.is_null() {
            // SAFETY: `self.cached` is only stored with a leaked `'static`
            // `SimplificationSequence` pointer using `Release` ordering.
            return unsafe { &*pointer };
        }
        let _guard = self.init.lock();
        let existing = self.cached.load(Ordering::Relaxed);
        if !existing.is_null() {
            // SAFETY: initialized under `self.init`, which synchronizes with
            // the publishing thread's release of the lock.
            return unsafe { &*existing };
        }
        let leaked = Box::leak(Box::new(build()));
        self.cached.store(from_mut(leaked), Ordering::Release);
        leaked
    }
}

impl Expr {
    /// Simplify this expression to minimize logical size using scalar identities and factoring.
    ///
    /// The preset's rules compile on the first call; later calls reuse them.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if simplification limits are exceeded.
    ///
    /// ```
    /// use symb_anafis::Expr;
    /// let x = Expr::symbol("compact_x").expect("symbol");
    /// let expr = x.pow(2) - x.pow(2);
    /// assert_eq!(expr.compact()?, Expr::number(0));
    /// # Ok::<(), symb_anafis::SimplifyError>(())
    /// ```
    pub fn compact(&self) -> Result<Self, SimplifyError> {
        static COMPACT: Preset = Preset::new();
        COMPACT
            .get(SimplificationSequence::compact)
            .simplify(self)
            .map(Simplification::into_expression)
    }

    /// Expand products and integer powers in this expression into distributed sums.
    ///
    /// The preset's rules compile on the first call; later calls reuse them.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if simplification limits are exceeded.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let ctx = Context::new()?;
    /// let x = ctx.symb("expand_x")?;
    /// let expr = (x + 1).pow(2);
    /// assert_eq!(expr.expand()?, x.pow(2) + 2 * x + 1);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn expand(&self) -> Result<Self, SimplifyError> {
        static EXPANDED: Preset = Preset::new();
        EXPANDED
            .get(SimplificationSequence::expanded)
            .simplify(self)
            .map(Simplification::into_expression)
    }

    /// Factor common polynomial powers and subexpressions.
    ///
    /// The preset's rules compile on the first call; later calls reuse them.
    ///
    /// # Errors
    /// Returns [`SimplifyError`] if simplification limits are exceeded.
    ///
    /// ```
    /// use symb_anafis::Context;
    /// let ctx = Context::new()?;
    /// let x = ctx.symb("factor_x")?;
    /// let y = ctx.symb("factor_y")?;
    /// let expr = x.pow(2) + x * y;
    /// assert_eq!(expr.factor()?, x * (x + y));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn factor(&self) -> Result<Self, SimplifyError> {
        static FACTORED: Preset = Preset::new();
        FACTORED
            .get(SimplificationSequence::factored)
            .simplify(self)
            .map(Simplification::into_expression)
    }
}
