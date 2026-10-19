# Simplification sequences

`SimplificationSequence` executes reusable ordered passes on canonical expressions.
Construction already handles finite literal folding, exact coefficient reduction,
like terms, positive integer factor collection, canonical aliases, and ordering.
The explicit passes select additional mathematical identities and polynomial forms.
The [complete rule reference](RULES.md) lists every active family, both domain
modes, and the destination of all 128 registered legacy rule entries.

## Presets and objectives

`new()` creates an empty sequence. `default()` is the compact preset.

| Preset | Ordered steps | Objective |
| --- | --- | --- |
| `compact()` | Algebraic, Roots, Exponential, Trigonometric, Hyperbolic, Factor | Logical expression size. |
| `expanded()` | Algebraic, Roots, Exponential, Trigonometric, Hyperbolic, Expand | Remaining distributable sites, then logical size. |
| `factored()` | Algebraic, Roots, Exponential, Trigonometric, Hyperbolic, Factor | Composite terms under sums, then logical size. |
| `compilation()` | Compact steps, then Compilation | Estimated operations over distinct logical DAG nodes, then logical size. |

Logical size counts repeated edges, independently of shared nodes and private
polynomial storage. Expanded sites are products with sum factors and positive exact
integer powers of sums. Composite terms are positive integer powers or products
with multiple nonnumeric factors, including integer powers behind scalar prefixes.
Costs saturate at `usize::MAX`. A caller score replaces the primary cost; logical
size breaks ties. Equal complete scores select the latest candidate.

Compilation costs count shared nodes once. Addition and multiplication have unit
cost per binary operation; division costs four. Positive integer powers use the
multiplication count of binary exponentiation; other powers cost 24. Simple scalar
functions cost one, square roots four, cube roots eight, other calls 16, and retained
derivatives 32. Leaves cost zero. These portable estimates do not certify backend
timings; `score_with` can supply a backend's own objective.

The input and each completed step are candidates. Subsequent steps always receive
the preceding step's output, including a candidate with a worse score. The runner
does not reject growth before later cancellation. An unchanged complete sweep is
a fixed point; a repeated boundary with changed steps is a cycle. The best visited
candidate survives either termination or the iteration limit. Targets describe a
heuristic over the visited candidates and do not certify a global normal form.

## Scalar domains

Symbols range over finite complex values. `DomainSafety::Preserving` is the
default: `Properties` supplies conservative proofs, and `Truth::Unknown` never
authorizes a required condition. `domain_safety(DomainSafety::AssumptionDependent)`
also enables explicitly tagged identities whose missing domain and branch
hypotheses belong to the caller. Known false tested conditions remain rejected.
The [rule reference](RULES.md) states the obligations for each family. Identities
refer to mathematical scalar values and principal functions. Literal arithmetic
continues to use the numeric backend's overflow, representation, and capacity
contracts. No pass converts approximate literals into exact representations.

| Step | Identities and guards |
| --- | --- |
| Algebraic | Finite exact-zero annihilation; \(a/a=1\) and factor cancellation only with a nonzero proof; certified exact multivariate polynomial GCD cancellation; absolute values and real signs; fraction addition retaining all denominator zeros; guarded principal-power combination. |
| Roots | \(\sqrt{x^{2n}}=|x|^n\) for finite real \(x\); odd powers retain a square root; real cube roots; nonnegative square-root products; positive-denominator square-root quotients. |
| Exponential | \(\exp(\log z)=z\) for finite nonzero \(z\); \(\log(\exp x)=x\) for real \(x\); exponential products and quotients; logarithmic sums and power rules on positive real bases. |
| Trigonometric | Entire Pythagorean and angle identities; explicit parity and quarter turns; exact multiples of \(\pi/12\); inverse compositions with their singularities excluded; quotients with identical pole sets. |
| Hyperbolic | Entire hyperbolic angle identities; real inverse restrictions; guarded inverse singularities; exponential representations and ratios with identical pole sets. |
| Expand | Finite distributivity and positive exact integer powers, using bounded Cartesian products and exponentiation by squaring. |
| Factor | Common manifest integer powers, checked integer coefficient GCDs, squares, cubes, and differences of squares. Every operand of a factored sum has a finite-value proof. |
| Compilation | Native `exp_m1`, `ln_1p`, and `erfc`; positive-real exact cube-root powers; nonzero-guarded `sinc`; guarded inverse compositions; common polynomial factors for nested arithmetic. |

Polynomial factoring extracts powers by subtracting manifest nonnegative integer
multiplicities, rather than dividing by a symbolic value. Therefore a common
factor is allowed to be zero. Fraction addition uses
\(a/b+c/d=(ad+cb)/(bd)\), or \((a+c)/b\) for equal denominators; both sides
exclude the same denominator zeros. Cancellation removes a factor only when it
is finite and nonzero. Numeric coefficients remain under core normalization.

For a nonzero base, \(z^a=\exp(a\log z)\) establishes
\(z^az^b=z^{a+b}\) and \(z^a/z^b=z^{a-b}\). Nested powers combine when the
inner base is positive with real exponents, or when the base is nonzero and the
outer exponent is an integer. These hypotheses remove the logarithmic branch
ambiguity. General nested fractional powers remain explicit.
Native integer exponent addition, subtraction, and multiplication are checked;
overflow retains the explicit powers. Approximate literal exponents and complex
literal exponents retain their representation and nesting.

For real \(x\), \(x^{2n}=|x|^{2n}\), and the principal square root is
\(|x|^n\). For odd powers, positive-real scaling gives
\(\sqrt{x^{2n+1}}=|x|^n\sqrt{x}\). Real cube-root multiplicativity gives
\(\operatorname{cbrt}(x^n)=\operatorname{cbrt}(x)^n\). These identities do
not assign realness to arbitrary complex symbols.

Quotient identities preserve poles rather than removing them by continuity.
In particular, \(1/\tan z=\cot z\) requires a nonzero cosine proof, and
\(1/\tanh z=\coth z\) requires a nonzero hyperbolic cosine proof.
Inverse tangent compositions exclude \(z=\pm i\); inverse hyperbolic tangent
compositions exclude \(z=\pm1\). Real inverse compositions use their stated
real or nonnegative domains.

Exact angle reduction computes rational multiples of symbolic pi using checked
integer remainders. It never identifies a nearby floating value with pi. A compact
target may retain an unevaluated exact angle if its radical form is larger; a
caller score can prefer evaluation of all such angles.

Compilation uses \(\operatorname{expm1}(z)=\exp(z)-1\),
\(\operatorname{log1p}(z)=\log(1+z)\), and
\(\operatorname{erfc}(z)=1-\operatorname{erf}(z)\) with the same principal
domains. The cancellation-resistant native calls can retain tiny nonzero numerical
results. The identity \(\sin z/z=\operatorname{sinc}(z)\) requires a nonzero
proof to retain the input's pole at zero. Exact powers \(p^{1/3}\) become
\(\operatorname{cbrt}(p)\) only for positive real \(p\).
The inverse \(\operatorname{log1p}(\operatorname{expm1}(x))=x\) requires real
\(x\); \(\operatorname{expm1}(\operatorname{log1p}(z))=z\) requires
\(1+z\ne0\). Common-factor extraction iterates through subsequent sweeps to
produce nested polynomial arithmetic without expanding intermediate powers.

## Bounds and traversal

The default limits are 16 complete sweeps, 100,000 distinct stored nodes,
stored depth 512, 100,000 candidate attempts per core rewrite, 4,096 generated
Cartesian terms per expansion pass, and 100,000 sparse polynomial work units per
algebraic pass. `polynomial_limit` bounds conversion, products, and remainder
steps; failed exact-division certificates discard the cancellation candidate.
The 100,000-unit defaults are capped at `usize::MAX` on 16-bit targets.
Expansion reserves each Cartesian product's
term count before allocating its output. Stored-node and depth limits validate
the input and every completed candidate, including internal rule outputs.
Custom callbacks retain responsibility for their own work and allocations.

Core `map`, `fold`, and `RewriteRule` provide iterative, memoized logical DAG
traversal. Shared nodes receive consistent replacements; inserted nodes are not
revisited in the same core pass. Rewrite programs compile when a sequence step
is added; assumption-dependent programs compile when that mode is enabled.
Sequence clones share immutable compiled plans, which are reused across calls
and sweeps. Every fallible pass discards partial output.
Each compiled builtin rule records its necessary explicit function identities.
A missing identity rejects that rule before commutative search. Square-root
identities are omitted from this filter because counted rational powers can
supply square-root groups without storing a square-root call. Function sets are
refreshed after a changed rule output, so newly introduced identities remain
available to later rules.

`Rewrite` and `Map` steps use the existing core transformation contracts. Their
mathematical equivalence and callback purity belong to the caller in either mode.
Caller scoring never changes builtin domain policy.

## Mathematical sources

Trigonometric additions use [DLMF 4.21.2](https://dlmf.nist.gov/4.21.E2) and
[4.21.3](https://dlmf.nist.gov/4.21.E3); Pythagorean identities use
[4.21.12](https://dlmf.nist.gov/4.21.E12). Double and triple angles follow by
substitution into the addition formulas. Hyperbolic identities follow from their
exponential definitions and [DLMF §4.35](https://dlmf.nist.gov/4.35).
Principal logarithms, exponentials, and powers use the branch conventions in
[DLMF §4.2](https://dlmf.nist.gov/4.2). Domain guards are additional obligations
for equality of the represented partial functions. The complementary error function
uses [DLMF 7.2.2](https://dlmf.nist.gov/7.2.E2).
