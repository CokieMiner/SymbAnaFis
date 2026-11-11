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
    sync::atomic::AtomicU32,
};

use alloc::{borrow::Cow, boxed::Box, sync::Arc, vec::Vec};

use super::{
    BuiltinFunction, ExprView, Function, FunctionDefinition, Interner, NumericValue, SYMBOLS,
    Substitution, Substitutions, Symbol, SymbolData,
};

use polynomial::Projection;

static NODES: Interner<Stored> = Interner::new();

/// An immutable expression: one pointer to a node of the globally interned DAG.
///
/// Equal live expressions share one canonical node (hash-consing), so cloning,
/// equality, and hashing are constant-time pointer operations. Nodes store
/// neither IDs nor cached hashes; each keeps the scalar facts of its first
/// property query.
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
/// let expression = Expr::symbol("x");
/// let terms = expression.additive_operands();
/// ```
///
/// ```compile_fail
/// use symb_anafis::Expr;
/// let _ = Expr::sum_of_products(Vec::new());
/// ```
#[derive(Clone)]
pub struct Expr(ManuallyDrop<Arc<Stored>>);

/// An interned node and its packed scalar facts.
///
/// Facts are a function of the node alone, so they take no part in identity:
/// equality and hashing read only the node. Racing first queries store the
/// same value.
#[derive(Debug)]
struct Stored {
    node: Node,
    /// Zero until the facts are proved; a stored value is nonzero.
    facts: AtomicU32,
    /// One bit per pure node map, set once that map is found to keep this
    /// node; see [`Postorder::settled`].
    settled: AtomicU32,
}

/// The distinct nodes reachable from a root, children before parents.
///
/// Each node lists its children as positions of earlier descendants, so values
/// computed in this order are read by position instead of by hashing. The root
/// is never a child, so it is kept apart from the descendants.
pub struct Postorder<'expr> {
    /// The descendants; the root's span also indexes their links.
    numbering: Numbering<'expr>,
    root: Visit<'expr>,
    /// The bit that [`Postorder::map`] sets on the nodes it keeps; zero for
    /// numberings that record none.
    settles: u32,
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

/// Exact literal values and manifest exact powers, kept out of the public API.
///
/// An approximate `0.0` or `1.0` keeps its tier, so normalization removes only
/// exact identities and an approximate result never becomes exact.
pub trait ExactIdentity {
    /// Whether this is the exact literal zero.
    fn is_exact_zero(&self) -> bool;

    /// Whether this is the exact literal one.
    fn is_exact_one(&self) -> bool;

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

    /// Whether the bases of compact storage are all symbols, \(\pi\), or \(e\);
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
#[derive(Clone, Debug)]
pub enum OperandMatch {
    /// Disjoint units of a stored power or collected addend.
    Count { index: usize, amount: i64 },
    /// One scalar factor whose quotient is reconstructed after selection.
    Coefficient { index: usize, divisor: NumericValue },
}

/// A requested arithmetic projection without expanding repeated operands.
#[derive(Clone, Copy, Debug)]
pub enum OperandDemand {
    /// One scalar base or additive term.
    Atom,
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
/// structural order. Term `t`, in structural order, has `coefficients[t]`, a
/// finite nonzero literal or the unit, and the next `lengths[t]` entries of
/// `powers`: base positions in ascending order with positive `u32` exponents.
/// Every base occurs in some term. Standalone constants belong to the outer
/// sum; this representation has no degree-zero terms.
#[derive(Debug)]
struct Polynomial {
    bases: Vec<Expr>,
    coefficients: Vec<Expr>,
    lengths: Vec<usize>,
    powers: Vec<(usize, u32)>,
    /// Whether every base is a symbol, \(\pi\), or \(e\). Such bases are
    /// independent leaves, so the storage is differentiated and evaluated from
    /// its terms and numbered as one leaf by [`Postorder::compact`]. Since
    /// \(e^{k}\) is stored as \(\exp(k)\), \(e\) occurs only to the first
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
        Self(ManuallyDrop::new(NODES.intern(Stored {
            node,
            facts: AtomicU32::new(0),
            settled: AtomicU32::new(0),
        })))
    }

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
    fn facts(&self) -> &AtomicU32 {
        &self.0.facts
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
            // Only nodes that lose their last owner enter the stack, so freeing
            // a node whose children are shared allocates nothing.
            let mut pending = Vec::new();
            release(stored.node, address, &mut pending);
            while let Some((child, child_address)) = pending.pop() {
                release(child, child_address, &mut pending);
            }
        }
    }
}

/// Unindex a node that lost its last owner, then orphan its children. The
/// address identifies the node's index entry and is never dereferenced.
fn release(node: Node, address: *const Stored, pending: &mut Vec<(Node, *const Stored)>) {
    // Remove the expired index entry while the children are still alive,
    // because their pointers form the fingerprint. A stored node hashes as
    // its node alone.
    NODES.remove(Interner::<Stored>::fingerprint(&node), address);
    let mut orphan = |child: Expr| {
        let mut handle = ManuallyDrop::new(child);
        // SAFETY: `handle` is never dropped or used again, so this is the only
        // move of its field.
        let owner = unsafe { ManuallyDrop::take(&mut handle.0) };
        let child_address = Arc::as_ptr(&owner);
        if let Some(orphaned) = Arc::into_inner(owner) {
            pending.push((orphaned.node, child_address));
        }
    };
    match node {
        Node::Number(_) | Node::ImaginaryUnit | Node::Pi | Node::Euler | Node::Symbol(_) => {}
        Node::Function { args, .. } | Node::Sum(args) | Node::Product(args) => {
            args.into_iter().for_each(orphan);
        }
        Node::Poly(polynomial) => {
            let mut polynomial = *polynomial;
            let projected = polynomial.projection.take().unwrap_or_default();
            polynomial
                .bases
                .into_iter()
                .chain(polynomial.coefficients)
                .chain(projected)
                .for_each(orphan);
        }
        Node::Div(left, right) | Node::Pow(left, right) => {
            orphan(left);
            orphan(right);
        }
        Node::Derivative { inner, .. } => orphan(inner),
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
pub use polynomial::CompactStorage;
pub use properties::{Properties, Property, Truth};

#[cfg(test)]
mod tests;
