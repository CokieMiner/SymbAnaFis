//! The shared handle owner, its private node representation, and its
//! interning and iterative release.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]
#![expect(
    unsafe_code,
    reason = "a released handle's field is moved out exactly once, so no extra reference count is touched"
)]

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    mem::ManuallyDrop,
    num::{NonZeroI128, NonZeroU32, NonZeroU64},
    ptr::from_ref,
    sync::atomic::AtomicUsize,
};

use alloc::{borrow::Cow, boxed::Box, sync::Arc, vec::Vec};

use super::{
    BuiltinFunction, ExprView, Function, FunctionDefinition, Interner, NumericValue, Substitution,
    Substitutions, Symbol, SymbolData, UnaryRule,
};

use polynomial::Projection;

static NODES: Interner<Stored> = Interner::new();

/// An immutable expression: one pointer to a node of the globally interned DAG.
///
/// Equal live expressions share one canonical node (hash-consing), so cloning,
/// equality, and hashing are constant-time pointer operations. Nodes store no
/// IDs; each keeps its structural fingerprint for constant-time unindexing on
/// drop, the scalar facts of its first property query, and settled-stage bits.
///
/// Literals keep their `NumAnaFis` representation. Exact values stay exact,
/// and float components have bitwise identity, including signed zeros and NaN
/// payloads. A literal fold whose exact result exceeds the numeric backend
/// keeps its operation symbolic.
///
/// Dropping the last handle releases its descendants iteratively, so deep
/// expressions do not exhaust the stack. Views and transformations present
/// compact polynomial storage as ordinary sums.
///
/// ```compile_fail
/// use symb_anafis::Polynomial;
/// ```
///
/// ```compile_fail
/// use symb_anafis::Expr;
/// let expression = Expr::pow_static(Expr::number(2), Expr::number(3));
/// ```
///
/// ```compile_fail
/// use symb_anafis::Expr;
/// let expression = Expr::symbol("x").expect("symbol");
/// let terms = expression.additive_operands();
/// ```
///
/// ```compile_fail
/// use symb_anafis::Expr;
/// let _ = Expr::sum_of_products(Vec::new(), Vec::new());
/// ```
#[repr(transparent)]
#[derive(Clone)]
pub struct Expr(ManuallyDrop<Arc<Stored>>);

/// An interned node, its structural fingerprint, and its atomic metadata.
///
/// The fingerprint depends only on the node. Cached facts and settled-stage
/// masks are tagged with the active symbol-property epoch in their upper bits
/// and take no part in identity: equality and hashing read only the node.
#[derive(Debug)]
struct Stored {
    node: Node,
    hash: u64,
    /// Epoch-tagged packed facts; zero until proved for an active epoch.
    facts: AtomicUsize,
    /// Epoch-tagged bitmask of pure simplification stages that keep this node;
    /// see [`Postorder::settled`].
    settled: AtomicUsize,
}

/// One pure simplification stage bit recorded on settled DAG nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SettledStage(NonZeroU32);

/// The distinct nodes reachable from a root, children before parents.
///
/// Each node lists its children as positions of earlier descendants, so values
/// computed in this order are read by position instead of by hashing. The root
/// is never a child, so it is kept apart from the descendants.
pub struct Postorder<'expr> {
    /// The descendants; the root's span also indexes their links.
    numbering: Numbering<'expr>,
    root: Visit<'expr>,
    /// The stage and property epoch that [`Postorder::map`] records on kept
    /// nodes; `None` for numberings that record none.
    settles: Option<(SettledStage, usize)>,
}

/// The distinct nodes reachable from several roots, children before parents.
/// A node shared between roots is numbered once; roots are kept by position.
pub struct Forest<'expr> {
    numbering: Numbering<'expr>,
    roots: Vec<usize>,
}

/// Visits children before parents and their child positions: every span of
/// a visit lies inside `links`, and every link is the position of an earlier
/// visit. Only discovery builds one.
struct Numbering<'expr> {
    visits: Vec<Visit<'expr>>,
    links: Vec<usize>,
}

/// One node of a [`Postorder`] and the span of its child positions.
pub struct Visit<'expr> {
    expression: &'expr Expr,
    links: Span,
    expansion: Expansion,
}

/// Which children a numbering gives a node.
#[derive(Clone, Copy)]
enum Expansion {
    /// None.
    Leaf,
    /// The stored edges, in `stored_children` order.
    Stored,
    /// The bases of compact storage, omitting its literal coefficients.
    Bases,
    /// The logical terms: a compact polynomial's terms, or a sum's operands
    /// with the terms of its compact group in place of the group.
    Projected,
    /// A sum's operands except its compact group in symbols and constants,
    /// which the sum keeps whole.
    Grouped,
    /// None: the node is known to be kept by the numbering's map.
    Settled,
}

/// One node during [`Postorder::evaluate`], with the values of its children.
pub struct Mapped<'run, 'expr, Value> {
    visit: &'run Visit<'expr>,
    links: &'run [usize],
    descendants: &'run [Visit<'expr>],
    values: &'run [Value],
}

/// A half-open index range that is `Copy`.
#[derive(Clone, Copy, Default)]
struct Span {
    start: usize,
    end: usize,
}

/// Logical additive terms for the matching kernels, kept out of the public API.
pub trait AdditiveOperands {
    /// Sums and compact storage borrow their terms; only a sum with a compact
    /// group lists them in a new vector, in structural order.
    fn additive_operands(&self) -> Option<Cow<'_, [Expr]>>;

    /// The canonical structural order in which additive operands are listed.
    fn additive_order(&self, other: &Expr) -> Ordering;
}

/// Exact rational values and manifest exact powers, kept out of the public API.
pub trait ExactIdentity {
    /// An exact real literal as `(numerator, denominator)`, reduced with a
    /// positive denominator; `None` for any other expression.
    fn exact_ratio(&self) -> Option<(i64, u64)>;

    /// A power with an exact positive integer exponent as its base and count;
    /// any other factor except a literal counts once. `None` for a literal or
    /// a power with another exponent.
    fn positive_factor(&self) -> Option<(&Expr, i64)>;
}

/// The stored form read by numerical engines, kept out of the public API:
/// stored edges and compact monomials, without building logical terms.
pub trait StoredForm {
    /// The stored edges: operands, binary operands, or the bases and then the
    /// coefficients of compact storage.
    fn stored_children(&self) -> impl Iterator<Item = &Expr>;

    /// The stored additive operands, with compact storage as one operand;
    /// `None` for a non-sum.
    fn stored_terms(&self) -> Option<&[Expr]>;

    /// The canonically ordered monomials of compact storage, each coefficient
    /// with its bases and positive integer powers; `None` for other storage.
    fn monomials(
        &self,
    ) -> Option<
        impl ExactSizeIterator<Item = (&Expr, impl ExactSizeIterator<Item = (&Expr, u32)> + Clone)>,
    >;

    /// Whether the bases of compact storage are all symbols, $\pi$, or $e$;
    /// `None` for other storage.
    fn simple_bases(&self) -> Option<bool>;

    /// An exact positive integer that fits a bounded power-chain count.
    /// Approximate and larger exponents keep the backend power operation.
    fn positive_integer(&self) -> Option<NonZeroU64>;
}

/// Counted arithmetic operands for the matching kernels.
#[derive(Clone, Debug)]
pub struct AlgebraicOperands {
    sum: bool,
    slots: Vec<WeightedOperand>,
}

/// The original arithmetic contribution selected by a matching attempt.
///
/// The index is a field of its own rather than of each variant, so a scan that
/// compares indices reads one offset for every match.
#[derive(Clone, Debug)]
pub struct OperandMatch {
    /// The operand this match selected.
    index: usize,
    /// What it selected from that operand.
    kind: OperandKind,
}

/// What one matching attempt selected from the operand at its index.
#[derive(Clone, Debug)]
pub enum OperandKind {
    /// Disjoint units of a stored power or collected addend.
    Count { amount: i64 },
    /// One scalar factor whose quotient is reconstructed after selection.
    Coefficient { divisor: NumericValue },
}

/// A requested arithmetic projection without expanding repeated operands.
#[derive(Clone, Copy, Debug)]
pub enum OperandDemand {
    /// One scalar base or additive term.
    Single,
    /// One unit remaining after a whole-operand capture has been tried.
    Split(i64),
    /// The entire available operand.
    Whole,
    /// A proved positive integer multiplicative power.
    Power(i64),
    /// A proved positive integer additive coefficient.
    Copies(i64),
    /// A positive integer block raised to a fixed positive integer exponent.
    Root { exponent: i64, block: i64 },
    /// One exact positive rational power on its unchanged principal base.
    Rational {
        numerator: i64,
        denominator: NonZeroU64,
    },
}

#[derive(Clone, Debug)]
struct WeightedOperand {
    original: Expr,
    base: Expr,
    count: i64,
    denominator: NonZeroU64,
    coefficient: bool,
    negative: bool,
    changed: bool,
    side: FactorSide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorSide {
    Numerator,
    Denominator,
}

#[derive(Debug)]
pub enum RemainingOperand {
    Original(Expr),
    Counted {
        base: Expr,
        count: i64,
    },
    Fixed(Expr),
    Scaled {
        base: Expr,
        coefficient: Expr,
    },
    Divisor(Expr),
    FixedDivisor(Expr),
    DivisorCounted {
        base: Expr,
        count: i64,
    },
    Power {
        base: Expr,
        exponent: NonZeroI128,
        denominator: NonZeroU64,
    },
    /// A common factor's remainder and its inserted destination.
    Factored(PartialSelection),
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Node {
    Number(NumericValue),
    ImaginaryUnit,
    Pi,
    Euler,
    Symbol(Arc<SymbolData>),
    Function {
        name: Function,
        args: Vec<Expr>,
    },
    Sum(Vec<Expr>),
    Poly(Box<Polynomial>),
    Product(Vec<Expr>),
    Div(Expr, Expr),
    Pow(Expr, Expr),
    Derivative {
        inner: Expr,
        var: Arc<SymbolData>,
        order: NonZeroU32,
    },
}

/// A canonical sparse polynomial in shared scalar expression bases.
///
/// Bases may be symbols or opaque expressions, each stored once in strict
/// structural order. Term `t`, ordered lexicographically by its
/// `(base_position, exponent)` power list (monomial power order), has
/// `coefficients[t]`, a finite nonzero literal or the unit, and the next
/// `lengths[t]` entries of `powers`: base positions in ascending order with
/// positive `u32` exponents. Every base occurs in some term. Standalone
/// constants belong to the outer sum; this representation has no degree-zero
/// terms. `projected()` orders the materialized terms in canonical
/// [`Node::compare`] structural order.
///
/// A base position is a `u32`, so a stored power is eight bytes rather than
/// the sixteen a `usize` position would pad it to. Construction rejects a base
/// table that does not fit, which bounds the table far below the address
/// space: a position is only ever a position in `bases`.
#[derive(Debug)]
struct Polynomial {
    bases: Vec<Expr>,
    coefficients: Vec<Expr>,
    lengths: Vec<usize>,
    powers: Vec<(u32, u32)>,
    /// Whether every base is a symbol, $\pi$, or $e$. Such bases are
    /// independent leaves, so the storage is differentiated and evaluated from
    /// its terms and numbered as one leaf by [`Postorder::compact`]. Since
    /// $e^{k}$ is stored as $\exp(k)$, $e$ occurs only to the first
    /// power, and the terms of such storage call no builtin function.
    simple_bases: bool,
    /// The terms as canonical products, one per stored term, so traversals
    /// that list logical children build them once per node, not once per visit.
    projection: Projection,
}

#[derive(Debug)]
pub enum Selection {
    Whole(Expr),
    Partial(PartialSelection),
}

#[derive(Debug)]
pub struct PartialSelection {
    sum: bool,
    remainder: Vec<RemainingOperand>,
    inserted: Vec<Expr>,
}

/// The selector of a concrete substitution pass: exact sources, explicit
/// operand groups of a single source, and the implicit families the sources
/// reach, as the `ConcreteMatcher` family bits.
#[derive(Debug)]
pub struct ConcreteMatcher<'entries> {
    exact: ExactSources<'entries>,
    groups: Vec<OperandSource>,
    families: u8,
}

/// The entries of a concrete pass. One owned entry needs no borrow, so a matcher
/// for it can outlive the code that resolved its destination.
#[derive(Debug)]
pub enum ExactSources<'entries> {
    One(Substitution),
    Many(&'entries Substitutions),
}

impl Expr {
    fn intern(node: Node) -> Self {
        let hash = Interner::<Stored>::fingerprint(&node);
        Self(ManuallyDrop::new(NODES.intern(
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

    #[inline]
    fn literal(&self) -> Option<&NumericValue> {
        match self.node() {
            Node::Number(value) => Some(value),
            Node::ImaginaryUnit => Some(NumericValue::imaginary_unit()),
            Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => None,
        }
    }

    #[inline]
    fn node(&self) -> &Node {
        &self.0.node
    }

    #[inline]
    fn pointer(&self) -> *const Node {
        from_ref(&self.0.node)
    }

    #[inline]
    fn facts(&self) -> &AtomicUsize {
        &self.0.facts
    }
}

impl SettledStage {
    /// The settled-stage bit at zero-based bit `index` (below 20, leaving the
    /// upper bits of `Stored::settled` for the active property epoch).
    ///
    /// # Panics
    /// Panics if `index >= 20`.
    #[must_use]
    pub const fn bit(index: u8) -> Self {
        assert!(index < 20, "settled stage bit index must be below 20");
        Self(NonZeroU32::new(1_u32 << index).expect("a bit shift below 20 is nonzero"))
    }

    /// The single-bit mask recorded on settled DAG nodes.
    #[must_use]
    pub fn mask(self) -> usize {
        usize::try_from(self.0.get()).unwrap_or(0)
    }
}

impl PartialEq for Stored {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl Eq for Stored {}

impl Hash for Stored {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        self.node.hash(state);
    }
}

/// The final owner of a node moves it out of its `Arc` and releases its
/// children from an explicit stack, so dropping a deep expression uses
/// constant call depth.
impl Drop for Expr {
    fn drop(&mut self) {
        // SAFETY: `drop` runs at most once per handle, and `self.0` is never
        // read again after this move.
        let owner = unsafe { ManuallyDrop::take(&mut self.0) };
        let address = Arc::as_ptr(&owner);
        // `Arc::into_inner` gives the value to exactly one of several racing final
        // owners, so no owner falls back to the recursive `Arc` destructor.
        if let Some(stored) = Arc::into_inner(owner) {
            NODES.remove(stored.hash, address);
            // Only nodes that lose their last owner enter the stack, so freeing
            // a node whose children are shared allocates nothing.
            let mut pending = Vec::new();
            release_children(stored.node, &mut pending);
            while let Some(child) = pending.pop() {
                release_children(child, &mut pending);
            }
        }
    }
}

/// Unindex and release a node's orphaned children onto `pending` while each
/// allocation is still hot in cache.
fn release_children(node: Node, pending: &mut Vec<Node>) {
    let mut release = |child: Expr| {
        let mut handle = ManuallyDrop::new(child);
        // SAFETY: `handle` is never dropped or used again, so this is the only
        // move of its field.
        let owner = unsafe { ManuallyDrop::take(&mut handle.0) };
        let child_address = Arc::as_ptr(&owner);
        if let Some(orphaned) = Arc::into_inner(owner) {
            NODES.remove(orphaned.hash, child_address);
            pending.push(orphaned.node);
        }
    };
    match node {
        Node::Number(_) | Node::ImaginaryUnit | Node::Pi | Node::Euler | Node::Symbol(_) => {}
        Node::Function { args, .. } | Node::Sum(args) | Node::Product(args) => {
            args.into_iter().for_each(release);
        }
        Node::Poly(polynomial) => {
            let mut polynomial = *polynomial;
            let projected = polynomial.projection.take().unwrap_or_default();
            polynomial
                .bases
                .into_iter()
                .chain(polynomial.coefficients)
                .chain(projected)
                .for_each(release);
        }
        Node::Div(left, right) | Node::Pow(left, right) => {
            release(left);
            release(right);
        }
        Node::Derivative { inner, .. } => release(inner),
    }
}

mod canonical;
mod inspect;
mod matching;
mod polynomial;
mod properties;
mod traversal;

pub use inspect::ExprChildren;
pub use matching::OperandSource;
pub use polynomial::{CompactStorage, MonomialStatus};
pub use properties::{Facts, Properties, Property, Truth};

#[cfg(test)]
mod tests;
