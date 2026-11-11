//! Builtin function methods for expressions and symbols.

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType};

use super::{BuiltinFunction, ExactIdentity, Expr, Function, FunctionDefinition, Node, Symbol};

/// The builtin methods: each method name, its builtin, and the names of the
/// arguments before and after the receiver. The pattern methods repeat this
/// table and are checked against [`BuiltinFunction::is_method`] at compile
/// time.
macro_rules! builtin_methods {
    ($($name:ident => $builtin:ident([$($before:ident),*], [$($after:ident),*]) $(: $doc:literal)?),* $(,)?) => {
        impl BuiltinFunction {
            /// Whether `name`, with the arguments `before` ahead of the
            /// receiver and `after` behind it, is this builtin's method form.
            /// Every builtin has exactly one, and the argument names must match
            /// in order, so a transposed table fails the build.
            ///
            /// The argument lists arrive as their comma-separated names, which
            /// is the one form a `const` comparison can read without a slice.
            pub const fn is_method(self, name: &str, before: &str, after: &str) -> bool {
                let (expected, expected_before, expected_after) = match self {
                    $(Self::$builtin => (
                        stringify!($name),
                        concat!($(stringify!($before), ","),*),
                        concat!($(stringify!($after), ","),*),
                    ),)*
                };
                same_text(name, expected)
                    && same_text(before, expected_before)
                    && same_text(after, expected_after)
            }

            /// The number of arguments ahead of and behind the receiver in this
            /// builtin's method form.
            #[must_use]
            pub const fn method_arity(self) -> (usize, usize) {
                match self {
                    $(Self::$builtin => (
                        <[&str]>::len(&[$(stringify!($before)),*]),
                        <[&str]>::len(&[$(stringify!($after)),*]),
                    ),)*
                }
            }
        }

        // The receiver and the named arguments fill the builtin's arity.
        const _: () = {
            $(
                let (before, after) = BuiltinFunction::$builtin.method_arity();
                assert!(
                    BuiltinFunction::$builtin.definition().arity() == 1 + before + after,
                    concat!("the method `", stringify!($name), "` does not match its builtin's arity"),
                );
            )*
        };

        impl Expr {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")`. Literal arguments fold through an exact cell, or numerically when an argument is approximate and the result is finite.")]
                $(#[doc = $doc])?
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Self>,)* $($after: impl Into<Self>,)*) -> Self {
                    FunctionDefinition::construct(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.clone(), $($after.into(),)*],
                    )
                }
            )*
        }

        impl Symbol<'_> {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")` for this symbol.")]
                $(#[doc = $doc])?
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Expr>,)* $($after: impl Into<Expr>,)*) -> Expr {
                    FunctionDefinition::construct(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.to_expr(), $($after.into(),)*],
                    )
                }
            )*
        }
    };
}

builtin_methods! {
    sin => Sin([], []), cos => Cos([], []), tan => Tan([], []),
    cot => Cot([], []), sec => Sec([], []), csc => Csc([], []),
    asin => Asin([], []), acos => Acos([], []), atan => Atan([], []),
    acot => Acot([], []), asec => Asec([], []), acsc => Acsc([], []),
    sinh => Sinh([], []), cosh => Cosh([], []), tanh => Tanh([], []),
    coth => Coth([], []), sech => Sech([], []), csch => Csch([], []),
    asinh => Asinh([], []), acosh => Acosh([], []), atanh => Atanh([], []),
    acoth => Acoth([], []), asech => Asech([], []), acsch => Acsch([], []),
    exp => Exp([], []), exp_m1 => ExpM1([], []),
    ln => Ln([], []), ln_1p => Ln1p([], []), log10 => Log10([], []), log2 => Log2([], []),
    sqrt => Sqrt([], []), cbrt => Cbrt([], []),
    floor => Floor([], []), ceil => Ceil([], []), round => Round([], []),
    trunc => Trunc([], []), fract => Fract([], []),
    abs => Abs([], []), signum => Signum([], []),
    re => Re([], []), im => Im([], []), conj => Conj([], []), heaviside => Heaviside([], []),
    sinc => Sinc([], []),
    erf => Erf([], []), erfc => Erfc([], []), gamma => Gamma([], []),
    lgamma => Lgamma([], []), digamma => Digamma([], []), trigamma => Trigamma([], []),
    tetragamma => Tetragamma([], []), zeta => Zeta([], []),
    elliptic_k => EllipticK([], []), elliptic_e => EllipticE([], []),
    polygamma => Polygamma([order], []), besselj => BesselJ([order], []),
    bessely => BesselY([order], []), besseli => BesselI([order], []),
    besselk => BesselK([order], []), zeta_deriv => ZetaDeriv([order], []),
    log => Log([base], []), hermite => Hermite([degree], []),
    lambertw => LambertW([branch], []),
    beta => Beta([], [other]), atan2 => Atan2([], [x]),
    hypot => Hypot([], [other]), max => Max([], [other]), min => Min([], [other]),
    modulo => Modulo([], [other]), rem => Rem([], [other]),
    assoc_legendre => AssocLegendre([degree, order], []),
    clamp => Clamp([], [min, max]),
}

impl Symbol<'_> {
    /// This symbol raised to `exponent`.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Expr>) -> Expr {
        self.to_expr().pow(exponent)
    }
}

impl FunctionDefinition {
    /// Construct a validated call, applying the builtin's canonical aliases and
    /// exact literal cells. Custom functions are stored unchanged.
    pub fn construct(function: Function, args: Vec<Expr>) -> Expr {
        let Some(builtin) = BuiltinFunction::of(&function) else {
            return Expr::intern(Node::Function {
                name: function,
                args,
            });
        };
        match (builtin, args.as_slice()) {
            (BuiltinFunction::Log, [base, argument]) => {
                if let Some(alias) = Self::logarithm_alias(base) {
                    return Self::construct(alias.function(), vec![argument.clone()]);
                }
            }
            (BuiltinFunction::Sqrt, [argument]) if matches!(argument.node(), Node::Euler) => {
                return Self::construct(
                    BuiltinFunction::Exp.function(),
                    vec![Expr::number(Number::from_rational(RationalType::new(1, 2)))],
                );
            }
            (BuiltinFunction::Ln, [argument]) if matches!(argument.node(), Node::Euler) => {
                return Expr::number(1);
            }
            (BuiltinFunction::Exp, [argument]) if argument.exact_ratio() == Some((1, 1)) => {
                return Expr::e();
            }
            _ => {}
        }
        if let Some(value) = builtin.definition().fold_literals(&args) {
            return Expr::number(value);
        }
        Expr::intern(Node::Function {
            name: function,
            args,
        })
    }

    /// Fold a call whose arguments are all literals.
    fn fold_literals(&self, args: &[Expr]) -> Option<Number> {
        match args {
            [argument] => self.fold(&[argument.literal()?]),
            [first, second] => self.fold(&[first.literal()?, second.literal()?]),
            [first, second, third] => {
                self.fold(&[first.literal()?, second.literal()?, third.literal()?])
            }
            _ => None,
        }
    }
}

/// Whether two strings are equal, byte by byte. Slice equality is not `const`,
/// so the method tables compare their names with this loop.
const fn same_text(left: &str, right: &str) -> bool {
    let (mut remaining_left, mut remaining_right) = (left.as_bytes(), right.as_bytes());
    loop {
        match (remaining_left, remaining_right) {
            ([], []) => return true,
            ([first, rest @ ..], [second, others @ ..]) if *first == *second => {
                remaining_left = rest;
                remaining_right = others;
            }
            _ => return false,
        }
    }
}
