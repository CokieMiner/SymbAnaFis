//! Public constructors and literal predicates.

#![expect(
    unsafe_code,
    reason = "cached constant and symbol handles hold one permanent leaked reference and clone via raw pointer increment, and fixed small-integer slots are statically bounds-checked"
)]

use core::{
    mem::ManuallyDrop,
    num::NonZeroU32,
    ptr::null_mut,
    sync::atomic::{
        AtomicPtr, AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
    },
};

use alloc::{sync::Arc, vec::Vec};

use num_anafis::{Number, Real};

use crate::errors::SymbolError;

use super::{ExactIdentity, Expr, Interner, NODES, Node, NumericValue, Stored, Symbol, SymbolData};

const SMALL_INT_MIN: i64 = -64;
const SMALL_INT_COUNT: usize = 193;
const ZERO_SLOT: usize = 64;
const ONE_SLOT: usize = 65;

static SMALL_INTS: [AtomicPtr<()>; SMALL_INT_COUNT] =
    [const { AtomicPtr::new(null_mut()) }; SMALL_INT_COUNT];
static IMAGINARY_UNIT: AtomicPtr<()> = AtomicPtr::new(null_mut());
static PI: AtomicPtr<()> = AtomicPtr::new(null_mut());
static EULER: AtomicPtr<()> = AtomicPtr::new(null_mut());

#[inline]
fn cached_expr(slot: &AtomicPtr<()>, build: impl FnOnce() -> Expr) -> Expr {
    let mut pointer = slot.load(Acquire);
    if pointer.is_null() {
        let mut handle = ManuallyDrop::new(build());
        // SAFETY: `handle` suppresses `Expr::drop` and is never read again,
        // so taking `handle.0` moves its sole `Arc<Stored>` ownership once.
        let owner = unsafe { ManuallyDrop::take(&mut handle.0) };
        let fresh = Arc::into_raw(owner).cast_mut().cast::<()>();
        pointer = match slot.compare_exchange(null_mut(), fresh, Release, Acquire) {
            Ok(_) => fresh,
            Err(winner) => {
                // SAFETY: `fresh` came from `Arc::into_raw` above and lost
                // the publication race, so reclaiming it into an `Expr`
                // runs its iterative drop once.
                drop(Expr(ManuallyDrop::new(unsafe {
                    Arc::from_raw(fresh.cast::<Stored>().cast_const())
                })));
                winner
            }
        };
    }
    // SAFETY: `pointer` was published via `Arc::into_raw` and is never
    // reclaimed, so its allocation stays live with strong count >= 1.
    // Raising the count first gives the returned `Expr` its own owner.
    unsafe {
        let typed = pointer.cast::<Stored>().cast_const();
        Arc::increment_strong_count(typed);
        Expr(ManuallyDrop::new(Arc::from_raw(typed)))
    }
}

impl Node {
    /// Return the cached expression for `value` when it lies in `-64..=128`,
    /// bypassing `NumericValue` construction on cache hits.
    #[inline]
    pub fn small_int(value: i64) -> Option<Expr> {
        let offset = value.checked_sub(SMALL_INT_MIN)?;
        let slot = SMALL_INTS.get(usize::try_from(offset).ok()?)?;
        Some(cached_expr(slot, || {
            Expr::intern(Self::Number(NumericValue::new(Number::from_int(value))))
        }))
    }
}

impl Expr {
    /// `self` raised to `exponent`, in canonical form.
    ///
    /// Exact $x^{1/2}$ becomes $\sqrt{x}$ and $e^x$ becomes $\exp(x)$.
    /// A known negative real exponent moves the power to a denominator; an
    /// exponent of unknown sign stays a power.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Self>) -> Self {
        Node::normalize_power(self.clone(), exponent.into())
    }

    /// The canonical sum: flattened, ordered, finite literals folded, and equal
    /// terms collected. A zero coefficient keeps its base, so an undefined term
    /// is not erased.
    #[must_use]
    pub fn sum(terms: Vec<Self>) -> Self {
        Node::normalize_sum(terms)
    }

    /// The canonical product: flattened, ordered, finite literals folded, and
    /// denominators collected. Numeric coefficients stay outside shared symbolic
    /// quotients, and a zero factor does not erase symbolic or nonfinite factors.
    #[must_use]
    pub fn product(factors: Vec<Self>) -> Self {
        Node::normalize_product(factors)
    }

    /// A `NumAnaFis` literal, keeping its representation tier and float bits.
    #[must_use]
    #[inline]
    pub fn number(value: impl Into<Number>) -> Self {
        let literal = NumericValue::new(value.into());
        match literal.gaussian() {
            Some((re, 0)) => {
                Node::small_int(re).unwrap_or_else(|| Self::intern(Node::Number(literal)))
            }
            Some((0, 1)) => Self::i(),
            _ => Self::intern(Node::Number(literal)),
        }
    }

    /// The exact imaginary unit $i$.
    #[must_use]
    #[inline]
    pub fn i() -> Self {
        cached_expr(&IMAGINARY_UNIT, || Self::intern(Node::ImaginaryUnit))
    }

    /// The exact constant $\pi$; it is not a symbol.
    #[must_use]
    #[inline]
    pub fn pi() -> Self {
        cached_expr(&PI, || Self::intern(Node::Pi))
    }

    /// Euler's number $e$, exact; it is not a symbol.
    #[must_use]
    #[inline]
    pub fn e() -> Self {
        cached_expr(&EULER, || Self::intern(Node::Euler))
    }

    /// The global symbol named `name`, created on first use.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when a new identity cannot be allocated.
    pub fn symbol(name: &str) -> Result<Self, SymbolError> {
        Ok(Symbol::intern(name)?.to_expr())
    }

    /// The symbol identity, if this expression is a symbol. The handle borrows
    /// this expression, which owns the metadata, so no registry lookup occurs.
    ///
    /// ```compile_fail
    /// use symb_anafis::Context;
    /// let x = {
    ///     let context = Context::new().expect("context");
    ///     let expression = context.symb("x").expect("symbol allocation").to_expr();
    ///     expression.as_symbol().expect("symbol expression")
    /// };
    /// let expression = x + 1;
    /// ```
    #[must_use]
    pub fn as_symbol(&self) -> Option<Symbol<'_>> {
        if let Node::Symbol(data) = self.node() {
            Some(data.symbol())
        } else {
            None
        }
    }

    /// Whether this is the exact literal zero (`0`, excluding approximate `0.0`
    /// and `-0.0`).
    #[must_use]
    #[inline]
    pub fn is_exact_zero(&self) -> bool {
        const {
            assert!(
                ZERO_SLOT < SMALL_INT_COUNT,
                "zero slot lies inside small-int cache"
            );
        }
        // SAFETY: `ZERO_SLOT` is statically asserted to be within `SMALL_INTS`.
        let slot = unsafe { SMALL_INTS.get_unchecked(ZERO_SLOT) };
        Arc::as_ptr(&self.0) == slot.load(Relaxed).cast::<Stored>().cast_const()
    }

    /// Whether this is the exact literal one (`1`, excluding approximate `1.0`).
    #[must_use]
    #[inline]
    pub fn is_exact_one(&self) -> bool {
        const {
            assert!(
                ONE_SLOT < SMALL_INT_COUNT,
                "unit slot lies inside small-int cache"
            );
        }
        // SAFETY: `ONE_SLOT` is statically asserted to be within `SMALL_INTS`.
        let slot = unsafe { SMALL_INTS.get_unchecked(ONE_SLOT) };
        Arc::as_ptr(&self.0) == slot.load(Relaxed).cast::<Stored>().cast_const()
    }

    /// Whether this is a zero literal, including float zeros of either sign.
    #[must_use]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_zero)
    }

    /// Whether this is a literal equal to one.
    #[must_use]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_one)
    }

    /// Whether this is a literal equal to minus one.
    #[must_use]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number().is_some_and(Number::is_neg_one)
    }

    /// The literal value of a number or of the imaginary unit.
    #[must_use]
    pub fn as_number(&self) -> Option<&Number> {
        self.literal().map(NumericValue::value)
    }
}

impl ExactIdentity for Expr {
    fn exact_ratio(&self) -> Option<(i64, u64)> {
        match self.as_number()?.components() {
            (Real::Int(numerator), Real::Int(0)) => Some((*numerator, 1)),
            (Real::Rational(ratio), Real::Int(0)) => Some((*ratio.numer(), *ratio.denom())),
            _ => None,
        }
    }

    fn positive_factor(&self) -> Option<(&Self, i64)> {
        match self.node() {
            Node::Pow(base, exponent) => match exponent.exact_ratio() {
                Some((count, 1)) if count > 0 => Some((base, count)),
                _ => None,
            },
            Node::Number(_) | Node::ImaginaryUnit => None,
            Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Derivative { .. } => Some((self, 1)),
        }
    }
}

impl SymbolData {
    /// The symbol expression for `symbol`, reusing its permanent handle slot or
    /// cached node fingerprint without cloning metadata on cache hits.
    #[inline]
    pub fn to_expr(symbol: Symbol<'_>) -> Expr {
        let data = Self::borrow(symbol);
        if let Some(slot) = data.permanent_expr_cache() {
            return cached_expr(slot, || Expr::intern(Node::Symbol(Self::of(symbol))));
        }
        #[cfg(target_has_atomic = "64")]
        {
            let cached = data.node_hash().load(Relaxed);
            if cached != 0 {
                return Expr(ManuallyDrop::new(NODES.intern(
                    cached,
                    symbol,
                    |stored, sym| {
                        matches!(&stored.node, Node::Symbol(existing) if existing.id() == sym.id())
                    },
                    |sym| Stored {
                        node: Node::Symbol(Self::of(sym)),
                        hash: cached,
                        facts: AtomicUsize::new(0),
                        settled: AtomicUsize::new(0),
                    },
                )));
            }
            let node = Node::Symbol(Self::of(symbol));
            let hash = Interner::<Stored>::fingerprint(&node);
            data.node_hash().store(hash, Relaxed);
            Expr(ManuallyDrop::new(NODES.intern(
                hash,
                node,
                |stored, pending| stored.node == *pending,
                |pending| Stored {
                    node: pending,
                    hash,
                    facts: AtomicUsize::new(0),
                    settled: AtomicUsize::new(0),
                },
            )))
        }
        #[cfg(not(target_has_atomic = "64"))]
        {
            Expr::intern(Node::Symbol(Self::of(symbol)))
        }
    }

    /// The unevaluated derivative $\partial^{n} f / \partial v^{n}$ of
    /// $f =$ `inner` in this variable $v$, with $n =$ `order`. Nested
    /// derivatives in the same variable combine,
    /// $\partial_{v}^{m} \partial_{v}^{n} f = \partial_{v}^{m+n} f$, while
    /// $m + n$ fits `u32`.
    pub fn derivative_of(self: Arc<Self>, mut inner: Expr, mut order: NonZeroU32) -> Expr {
        while let Node::Derivative {
            inner: child,
            var: previous,
            order: count,
        } = inner.node()
        {
            if *previous != self {
                break;
            }
            let Some(combined) = order.checked_add(count.get()) else {
                break;
            };
            order = combined;
            inner = child.clone();
        }
        Expr::intern(Node::Derivative {
            inner,
            var: self,
            order,
        })
    }
}
