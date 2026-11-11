# Simplification rule reference

This file specifies the builtin passes, their domain contracts, and the simplification
rules. Public entry points are `SimplificationSequence`, `SimplificationStep`,
and `DomainSafety`.

## Presets and domain modes

| Preset | Ordered steps | Candidate selection |
| --- | --- | --- |
| `new()` | None | Compact. |
| `compact()` / `default()` | Algebraic, Roots, Exponential, Trigonometric, Hyperbolic, Factor | Logical expression size. |
| `expanded()` | Algebraic, Roots, Exponential, Trigonometric, Hyperbolic, Expand | Distributable sites, then size. |
| `factored()` | Algebraic, Roots, Exponential, Trigonometric, Hyperbolic, Factor | Composite terms under sums, then size. |
| `compilation()` | Compact steps, then Compilation | Estimated operations over distinct DAG nodes, then size. |

All presets start with `DomainSafety::Preserving`. Domain mode and target are
independent: a target chooses among completed candidates; it never supplies a
mathematical hypothesis. A custom scorer replaces the primary selection cost.
The input is also a candidate, so an available rewrite need not win selection.

| Rule tag / mode | Contract |
| --- | --- |
| **P** - `Preserving` | Required scalar properties must be proved `Truth::True`. Equal represented partial functions retain their poles and principal branches. |
| **A** - `AssumptionDependent` | Includes P rules and permits their specifically supported unknown preconditions to be supplied by the caller. Additional A templates require the hypotheses below. This mode may remove exclusions or choose a branch. A tested `Truth::False` condition is rejected. |
| Custom `Rewrite` / `Map` | The caller owns mathematical validity and callback purity in either mode. These steps require no builtin domain tag and are not filtered by the mode. |

In the formulas, $z, w, a, b$ are finite complex scalars unless restricted;
$x, r$ are finite real scalars, $p, q > 0$, and $n$ is a positive exact
integer. `Nonzero` includes finite-value membership. Symbols alone prove only
finiteness. A bound or branch hypothesis in this document is a mathematical
obligation, not an assertion that the library has inferred it.

## Core normalization, used in every mode

Canonical constructors supply these operations before sequences run:

| Operation | Behavior |
| --- | --- |
| Sums and products | Flatten compatible groups, order operands, collect exact like terms and manifest positive integer powers, combine numeric coefficients, and omit applicable identities. |
| Numeric arithmetic | Backend-owned integer, rational, floating, and complex folding; exact quotients reduce rational coefficients. Nonfinite and capacity-limited expressions remain explicit where the core contract requires it. |
| Numeric functions | Exact recognized builtin values and perfect roots; finite approximate evaluation when arguments contain approximate components. Custom body expansion is an explicit caller `Map` pass. |
| Powers | $z^1 = z$; distribute positive integer powers over products and quotients, preserving every pole; combine nested positive integer exponents with checked arithmetic. Known negative real exponents use reciprocals. |
| Quotients | $(a/b)/c = a/(bc)$, product denominator collection, exact coefficient extraction, and numeric sign normalization. Symbolic factors are not cancelled without proof. |
| Aliases | Exact $z^{1/2}$ uses `sqrt(z)`, $e^z$ uses `exp(z)`, and exact logarithm bases $e, 2, 10$ use unary builtins. `cbrt` retains the backend's real cube-root behavior on negative reals. |

Approximate literals retain approximate representation. There is no epsilon-based
integer snapping, nearby-pi recognition, or conversion of `0.5` into an exact
rational.

## Algebraic

The direct identities run in a memoized logical DAG pass, followed by the
absolute-value/sign templates.

| Identity | P precondition | A extension |
| --- | --- | --- |
| $0z = 0$ | Every factor is finite. | Unknown finiteness may be assumed. |
| $z^0 = 1$, $0/z = 0$, $z/z = 1$ | $z \ne 0$. | Unknown nonzeroness may be assumed. |
| $1^z = 1$, $0^p = 0$ | Finite exponent; proved positive symbolic exponent in the latter case. Numeric cases use core folding. | Unknown finiteness for the unit base. |
| $z^a z^b = z^{a+b}$, including a plain factor as exponent one | Nonzero base and finite exponents. All compatible factors are grouped in one pass. | Unknown nonzeroness may be assumed. |
| $z^a/z^b = z^{a-b}$ | Nonzero base and finite exponents. | Unknown nonzeroness may be assumed. |
| $(p^r)^s = p^{rs}$ | Positive base and real exponents. | These unknown properties may be assumed. |
| $(z^a)^k = z^{ak}$ | Nonzero base, finite inner exponent, integer outer exponent. | Unknown nonzeroness may be assumed. |
| $(x^{2n})^r = \lvert x\rvert^{2nr}$ | Real base and positive non-integer exact rational outer exponent. Exponent products must fit. | Unknown realness may be assumed. |
| $p^a q^a = (pq)^a$, $p^a/q^a = (p/q)^a$ | Positive bases; finite common exponent for products. `sqrt` aliases also participate. | Unknown positive-base conditions (and unknown exponent finiteness for products) may be assumed. |
| $(p/q)^r = p^r/q^r$ | Positive numerator/denominator and real exponent; integer cases also normalize in the core. | These unknown properties may be assumed. |
| $z^{-a} = 1/z^a$ | A manifest negative exact coefficient in the symbolic exponent and a nonzero base. | Unknown nonzeroness may be assumed. |
| $(z/w)/(a/b) = zb/(wa)$, $z/(a/b) = zb/a$ | Removed inner denominator $b$ is nonzero; the other direct operands are finite. The core handles numerator nesting. | Unknown nonzeroness of $b$ may be assumed. |
| $a/b + c/d = (ad + cb)/(bd)$ | Finite component numerators and denominators; no denominator factor is cancelled. Equal denominators share one copy. Plain finite terms can join a fraction. | Same rule. |
| Manifest $fz/(fw) = z/w$ | Removed factor $f$ is nonzero. Positive integral multiplicities are subtracted exactly. | Unknown nonzeroness may be assumed. |
| Polynomial $P/Q = (P/G)/(Q/G)$ | Exact polynomial coefficients; finite inputs; verified exact divisibility; removed GCD expression $G \ne 0$. | Unknown finiteness and nonzeroness may be assumed. |
| $\lvert\lvert z\rvert\rvert = \lvert z\rvert$, $\lvert -z\rvert = \lvert z\rvert$ | Finite operand; explicit negative prefix for the latter. | Same rule. |
| $\lvert z^n\rvert = \lvert z\rvert^n$ | Positive exact integer exponent and finite base. | Same rule. |
| $\lvert x\rvert = x$ for $x \ge 0$, $\lvert x\rvert = -x$ for $x \le 0$ | Proved corresponding sign. | Same direct sign proof. |
| $\lvert x\rvert^2 = x^2$, $\operatorname{sign}(\operatorname{sign} x) = \operatorname{sign} x$, $\lvert x\rvert \operatorname{sign} x = x$ | Real input. | Unknown realness may be assumed. |
| $\operatorname{sign}(\lvert z\rvert) = 1$ | Nonzero input. | Unknown nonzeroness may be assumed. |

Sparse polynomial cancellation uses exact rational coefficients and opaque shared
expression indeterminates. Positive exact `u32` powers, sums, products, and
division by nonzero exact scalar coefficients form the polynomial grammar.
General functions and nonpolynomial expressions remain indivisible atoms.
Gauss's lemma, recursive coefficient contents, and primitive pseudo-remainder
sequences compute a multivariate GCD using an explicit continuation stack.
Every content removal and both final quotients are certified by exact division.
An uncertified candidate is discarded. A shared work budget bounds conversion,
term products, and remainder steps across the algebraic pass.

Native exact integer exponent arithmetic is checked. Overflow retains explicit
powers. Approximate and complex literal exponents are not reassociated by power
collection. Polynomial exponent overflow returns the polynomial work error.

## Roots

| Identity | P precondition | A extension |
| --- | --- | --- |
| $\sqrt{x^{2n}} = \lvert x\rvert^n$ | Real base; a nonnegative base drops the absolute value. | Unknown realness may be assumed. |
| $\sqrt{x^{2n+1}} = \lvert x\rvert^n \sqrt{x}$ | Real base and positive exact odd exponent. | Unknown realness may be assumed. |
| $\sqrt{x^{2n+r} z} = \lvert x\rvert^n \sqrt{x^r z}$, $\sqrt{\pm (p/q)^2 z} = (p/q)\sqrt{\pm z}$ | Every stored integer power of a real base, and an exact coefficient whose numerator and denominator are perfect squares; finite remainder. | Unknown realness/finiteness may be assumed. |
| $\operatorname{cbrt}(x^{3n}) = x^n$, $\operatorname{cbrt}(x^n) = \operatorname{cbrt}(x)^n$ | Real base and positive exact integer exponent. | Unknown realness may be assumed. |
| $\sqrt{p^r} = p^{r/2}$, $\operatorname{cbrt}(p^r) = p^{r/3}$ | Positive base; real symbolic or exact rational exponent. | These unknown scalar properties may be assumed. |
| $\prod \sqrt{a} / \prod \sqrt{b} = (-1)^w \sqrt{\prod a / \prod b}$ | Stored `sqrt` factors of a product or quotient, finite numerator and nonzero denominator radicands, and phase bounds that fix the winding $w$ (see below). | Unknown finiteness/nonzeroness may be assumed, and $w = 0$ when the bounds allow it and an uncancelled non-literal phase remains; literal phases are fixed numbers and are never assumed. |
| $\operatorname{cbrt}(x^3) = x$, $\operatorname{cbrt}(z)^3 = z$ | Real input for the first, finite input for the second. | Unknown realness for the first through the direct power rule. |

Roots of products and sums of logarithms track principal phases
$\alpha \in (-\pi, \pi]$. For signs $s = \pm 1$,
$\prod \sqrt{a}^{\,s} = (-1)^w \sqrt{\prod a^s}$ and
$\sum s \ln a = \ln \prod a^s + 2\pi i w$, where $w$ is the integer with
$\sum s\alpha \in ((2w - 1)\pi, (2w + 1)\pi]$. In units of $\pi$, a phase lies in
$[0, 0]$ for a nonnegative argument, $[1, 1]$ for a negative one, $[0, 1]$ for a
real one, $(0, 1)$ or $(-1, 0)$ for a literal in the upper or lower half-plane,
and $(-1, 1]$ otherwise; the exact product of literal arguments fixes their sum
to a point or an open unit interval. Positive factors do not change a phase, so
arguments that differ only by positive factors share one phase, and a core added
and subtracted equally often contributes exactly zero. A nonnegative argument therefore joins any
one other with $w = 0$, and two negative arguments join with $w = 1$. Arguments
join in order while the summed bounds fix $w$. Only explicit `sqrt` factors take
part: a bare or integral power is never read as a product of roots.

## Exponential and logarithmic

Write $L$ for `ln`, `log2`, `log10`, or `log(base, value)`.
Generic-base direct rules additionally require a positive real base unequal to
one; A mode permits those unknown base conditions to be assumed.

| Identity | P precondition | A extension |
| --- | --- | --- |
| $\exp(\ln z) = z$, $\exp(a \ln z) = z^a$ | Nonzero $z$, finite $a$. | Unknown nonzeroness/finiteness may be assumed. |
| $\ln(\exp x) = x$ | Real input. | Unknown realness may be assumed. |
| $\exp a \exp b = \exp(a + b)$, $\exp a / \exp b = \exp(a - b)$ | Finite arguments. | Same rule. |
| $L(p^r) = r L(p)$ | Positive base and real exponent. | These unknown properties may be assumed. |
| $L(x^{2k}) = 2k L(\lvert x\rvert)$ | Real base and nonzero even exact integer exponent ($k \ne 0$). The logarithmic exclusion at zero survives. | Unknown realness may be assumed. |
| $L(1/p) = -L(p)$ | Positive denominator. | Unknown positivity may be assumed. |
| $L(\sqrt{z}) = L(z)/2$ | Finite input, with the same principal square-root/logarithm conventions. Both retain zero as excluded. | Same rule. |
| $L(\operatorname{cbrt}(p)) = L(p)/3$, $L(\exp x) = x L(e)$ | Positive cube-root input; real exponential argument. | Unknown positivity/realness may be assumed. |
| $L(1) = 0$, $\log_b b = 1$ | Generic base positive and unequal to one. Unary numeric cases normalize in the core. | Unknown generic-base conditions may be assumed. |
| $c \sum s L(a) = c\,L(\prod a^s) + 2\pi i\,c\,w / \ln B$ for `ln`, `log2`, `log10` | Terms of one function with equal exact real coefficient magnitude $c$; finite added and nonzero subtracted arguments; phase bounds that fix the winding $w$ as for roots. $B$ is the base. | Unknown finiteness/nonzeroness may be assumed, and $w = 0$ under the same condition as for roots. |

The principal logarithm is not globally additive: $\ln(-2) + \ln(-3) = \ln 6 + 2\pi i$.
The winding term keeps every combination exact.

## Trigonometric

The following P formulas accept finite complex arguments. Quotient rewrites keep
the same denominator zeros unless an extra guard is stated.

| Group | Active formulas |
| --- | --- |
| Pythagorean | $\sin^2 a + \cos^2 a = 1$; $1 - \sin^2 a = \cos^2 a$; $1 - \cos^2 a = \sin^2 a$. |
| Tangent/cotangent | $1 + \tan^2 a = \sec^2 a$, $1 + \cot^2 a = \csc^2 a$, and the rearrangements $\sec^2 a - 1 = \tan^2 a$, $\csc^2 a - 1 = \cot^2 a$. |
| Pole removal | $\sec^2 a - \tan^2 a = 1$ additionally proves $\cos a \ne 0$; A mode may assume this exclusion. |
| Double angles | $\cos^2 a - \sin^2 a = \cos(2a)$, the sign-reversed form, $(\cos a - \sin a)(\cos a + \sin a) = \cos(2a)$, and $\sin a \cos a = \sin(2a)/2$, including scalar multiples. |
| Addition/subtraction | $\sin a \cos b \pm \cos a \sin b = \sin(a \pm b)$; $\cos a \cos b \mp \sin a \sin b = \cos(a \pm b)$. |
| Triple angles | $3\sin a - 4\sin^3 a = \sin(3a)$; $4\cos^3 a - 3\cos a = \cos(3a)$. |
| Quotients | $\sin a / \cos a = \tan a$, $\cos a / \sin a = \cot a$, $1/\cos a = \sec a$, $1/\sin a = \csc a$. |
| Reciprocal tangent | $1/\tan a = \cot a$ proves $\cos a \ne 0$; A mode may assume this exclusion. |
| Forward inverse compositions | $\sin(\arcsin a) = a$, $\cos(\arccos a) = a$; $\tan(\arctan a) = a$ additionally proves $1 + a^2 \ne 0$. A mode may assume the latter exclusion. |
| Cofunctions | At $\pi/2 - a$: sin/cos, tan/cot, and sec/csc exchange in both directions. |
| Parity | sin, tan, cot, csc are odd; cos, sec are even. An explicit negative prefix is normalized. |
| Exact angles | Exact symbolic multiples $k\pi/12$, reduced modulo $2\pi$, yield rational/radical sine and cosine values; tan/cot/sec/csc use these values only when their denominator is nonzero. |
| Quarter turns | Translate all six functions by exact multiples of $\pi/2$, including full periods, reflection about $\pi$, and $3\pi/2$ shifts. |

Additional **A-only** reverse compositions are
$\arcsin(\sin x) = x$ for $-\pi/2 \le x \le \pi/2$,
$\arccos(\cos x) = x$ for $0 \le x \le \pi$, and
$\arctan(\tan x) = x$ for $-\pi/2 < x < \pi/2$.
The caller supplies these real principal-range restrictions.

## Hyperbolic

| Group | P formulas / conditions |
| --- | --- |
| Entire identities | $\cosh^2 a - \sinh^2 a = 1$, $1 + \sinh^2 a = \cosh^2 a$, $\cosh^2 a - 1 = \sinh^2 a$. |
| Meromorphic identities | $1 - \tanh^2 a = \operatorname{sech}^2 a$, $\coth^2 a - 1 = \operatorname{csch}^2 a$, retaining their poles. |
| Pole removal | $\tanh^2 a + \operatorname{sech}^2 a = 1$ proves $\cosh a \ne 0$; A mode may assume it. |
| Double angles | $2\sinh a \cosh a = \sinh(2a)$, $\cosh^2 a + \sinh^2 a = \cosh(2a)$. |
| Addition/subtraction | $\sinh a \cosh b \pm \cosh a \sinh b = \sinh(a \pm b)$; $\cosh a \cosh b \pm \sinh a \sinh b = \cosh(a \pm b)$. |
| Triple angles | $3\sinh a + 4\sinh^3 a = \sinh(3a)$; $4\cosh^3 a - 3\cosh a = \cosh(3a)$. |
| Quotients | $\sinh a / \cosh a = \tanh a$, $\cosh a / \sinh a = \coth a$, $1/\cosh a = \operatorname{sech} a$, $1/\sinh a = \operatorname{csch} a$. |
| Reciprocal tanh | $1/\tanh a = \coth a$ proves $\cosh a \ne 0$; A mode may assume it. |
| Forward inverse compositions | $\sinh(\operatorname{asinh}a) = a$, $\cosh(\operatorname{acosh}a) = a$; $\tanh(\operatorname{atanh}a) = a$ additionally proves $1 - a^2 \ne 0$, or assumes it in A mode. |
| Reverse compositions | $\operatorname{asinh}(\sinh x) = x$, $\operatorname{atanh}(\tanh x) = x$ for proved real $x$; $\operatorname{acosh}(\cosh x) = x$ for proved nonnegative real $x$. A mode admits unknown realness and caller-assumed nonnegativity (testing only that $x$ is not proved non-real). |
| Parity | sinh, tanh, coth, csch are odd; cosh, sech are even. |

Exponential conversions use $E = e^a$, $F = e^{-a}$, $T = e^{2a}$:

| Input form | Output |
| --- | --- |
| $(E - F)/2$, $(T - 1)/(2E)$ | $\sinh a$ |
| $(E + F)/2$, $(T + 1)/(2E)$ | $\cosh a$ |
| $(E - F)/(E + F)$, $(T - 1)/(T + 1)$ | $\tanh a$ |
| $(E + F)/(E - F)$, $(T + 1)/(T - 1)$ | $\coth a$ |
| $2/(E + F)$, $2E/(T + 1)$ | $\operatorname{sech} a$ |
| $2/(E - F)$, $2E/(T - 1)$ | $\operatorname{csch} a$ |

All these P conversions have matching pole sets for finite arguments.

## Expand, Factor, and Compilation

| Step | P transformations |
| --- | --- |
| Expand | Distribute products over sums and positive exact integer powers of sums, using Cartesian products and exponentiation by squaring (without checking `Property::Finite`). Generated term counts are checked before allocation. Factors whose terms are exact rational multiples of powers of symbols and $\pi$ multiply as sparse polynomials; terms whose coefficients cancel are dropped, which is exact for these finite bases. |
| Factor | On a finite sum: divide out an exact integer coefficient GCD that fits a positive native integer, then the manifest positive integer powers shared by every addend, with their least multiplicities. Greedy multivariate Horner grouping (Ceberio and Kreinovich 2004) then takes the base shared by the most addends out of those addends and factors the group and the remainder in turn. No symbolic value is divided out. |
| Factor identities | On a whole set of two to four addends with exact rational roots: $a^2 - b^2 = (a - b)(a + b)$; $a^3 + b^3 = (a + b)(a^2 - ab + b^2)$; $s(a^2 \pm 2ab + b^2) = s(a \pm b)^2$ for $s = \pm 1$; $a^3 + 3a^2b + 3ab^2 + b^3 = (a + b)^3$. Cube roots carry the sign of their addend. New sums are factored in turn. |
| Compilation | $e^a - 1 = \operatorname{expm1}(a)$, $1 - e^a = -\operatorname{expm1}(a)$, $\ln(1 + a) = \operatorname{log1p}(a)$, $1 - \operatorname{erf}(a) = \operatorname{erfc}(a)$, and $\operatorname{expm1}(a) + 1 = e^a$. |
| Compilation cube root | Exact $p^{1/3} = \operatorname{cbrt}(p)$, with a proved positive real base. |
| Compilation sinc | $\sin a / a = \operatorname{sinc}(a)$, requiring $a \ne 0$. A mode may assume this exclusion. |
| Compilation inverses | $\operatorname{log1p}(\operatorname{expm1}(x)) = x$ for real $x$; $\operatorname{expm1}(\operatorname{log1p}(a)) = a$ for $1 + a \ne 0$. A mode may supply these unknown conditions. |
| Compilation factoring | The Factor kernel without the identities, forming nested polynomial arithmetic in one pass. |

Expand and Factor have no A-only transformations. The compilation objective uses portable
operation estimates; it does not establish a numerical speed guarantee. Custom
scores can model a particular backend. Mathematical equivalence does not promise
bitwise identical floating evaluation: `exp_m1` and `ln_1p` intentionally improve
cancellation behavior.

## Execution invariants and limits

Rule plans compile when a step is added, and A plans when that mode is enabled.
Sequence clones share immutable compiled storage. Application never recompiles
templates. A rule is tried only at nodes whose subtree calls every builtin its
pattern calls; square-root heads are exempt because counted rational powers can
supply their groups implicitly. It is also tried only at node heads where one of
its matching stages can start. A replacement adds the builtins its destination
calls.

Traversal is iterative and memoized. A step runs its direct identities, then its
rules, then, for Factor and Compilation, factoring; the Compilation cube-root
identity shares the factoring traversal. The rules of a step run in one pass: at
each node they are tried in order and the first replacement is kept. Inserted output
is not revisited in that pass; the next sweep visits it. Unmatched root operands survive.
Completed candidates are checked against node/depth limits before selection.
Failure publishes no partially transformed expression. The caller remains
responsible for callback work, purity, and side effects.

Defaults: 16 sweeps and `10_000` candidate attempts per rule in each core
rewrite pass; stored nodes, depth, expansion terms, and polynomial work are
unbounded (`usize::MAX`). A builtin rule that exhausts its attempts is dropped
and the pass restarts without it. The polynomial budget also bounds exact exponent capacity; it does
not extend the numeric backend's range, and a coefficient beyond that range
retains the original expression. Callback panics propagate.

## Mathematical conventions

The principal logarithm and power conventions are described in
[DLMF §4.2](https://dlmf.nist.gov/4.2). Trigonometric additions and Pythagorean
identities follow [DLMF §4.21](https://dlmf.nist.gov/4.21); inverse ranges follow
[§4.23](https://dlmf.nist.gov/4.23). Hyperbolic identities follow
[§4.35](https://dlmf.nist.gov/4.35), with inverse conventions in
[§4.37](https://dlmf.nist.gov/4.37). The implementation imposes the additional
guards above to preserve equality of partial functions. Cube roots on real
inputs use the real builtin rather than a general complex fractional power.
