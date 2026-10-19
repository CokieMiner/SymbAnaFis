# Simplification rule reference

This file specifies the builtin passes, their domain contracts, and the destination
of every rule registered by `old/src/simplification/logic/rules`. Public entry
points are `SimplificationSequence`, `SimplificationStep`, and `DomainSafety`.

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
| **P** — `Preserving` | Required scalar properties must be proved `Truth::True`. Equal represented partial functions retain their poles and principal branches. |
| **A** — `AssumptionDependent` | Includes P rules and permits their specifically supported unknown preconditions to be supplied by the caller. Additional A templates require the hypotheses below. This mode may remove exclusions or choose a branch. A tested `Truth::False` condition is rejected. |
| Custom `Rewrite` / `Map` | The caller owns mathematical validity and callback purity in either mode. These steps require no builtin domain tag and are not filtered by the mode. |

In the formulas, \(z,w,a,b\) are finite complex scalars unless restricted;
\(x,r\) are finite real scalars, \(p,q>0\), and \(n\) is a positive exact
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
| Powers | \(z^1=z\); distribute positive integer powers over products and quotients, preserving every pole; combine nested positive integer exponents with checked arithmetic. Known negative real exponents use reciprocals. |
| Quotients | \((a/b)/c=a/(bc)\), product denominator collection, exact coefficient extraction, and numeric sign normalization. Symbolic factors are not cancelled without proof. |
| Aliases | Exact \(z^{1/2}\) uses `sqrt(z)`, \(e^z\) uses `exp(z)`, and exact logarithm bases \(e,2,10\) use unary builtins. `cbrt` retains the backend's real cube-root behavior on negative reals. |

Approximate literals retain approximate representation. There is no epsilon-based
integer snapping, nearby-pi recognition, or conversion of `0.5` into an exact
rational. The canonical representation can differ from the legacy float AST.

## Algebraic

The direct identities run in a memoized logical DAG pass, followed by the
absolute-value/sign templates.

| Identity | P precondition | A extension |
| --- | --- | --- |
| \(0z=0\) | Every factor is finite. | Unknown finiteness may be assumed. |
| \(z^0=1\), \(0/z=0\), \(z/z=1\) | \(z\ne0\). | Unknown nonzeroness may be assumed. |
| \(1^z=1\), \(0^p=0\) | Finite exponent; proved positive symbolic exponent in the latter case. Numeric cases use core folding. | Unknown finiteness for the unit base. |
| \(z^a z^b=z^{a+b}\), including a plain factor as exponent one | Nonzero base and finite exponents. All compatible factors are grouped in one pass. | Unknown nonzeroness may be assumed. |
| \(z^a/z^b=z^{a-b}\) | Nonzero base and finite exponents. | Unknown nonzeroness may be assumed. |
| \((p^r)^s=p^{rs}\) | Positive base and real exponents. | These unknown properties may be assumed. |
| \((z^a)^k=z^{ak}\) | Nonzero base, finite inner exponent, integer outer exponent. | Unknown nonzeroness may be assumed. |
| \((x^{2n})^r=\lvert x\rvert^{2nr}\) | Real base and positive exact rational outer exponent. Exponent products must fit. | Unknown realness may be assumed. |
| \(p^a q^a=(pq)^a\), \(p^a/q^a=(p/q)^a\) | Positive bases; finite common exponent for products. `sqrt` aliases also participate. | Unknown positive-base conditions may be assumed. |
| \((p/q)^r=p^r/q^r\) | Positive numerator/denominator and real exponent; integer cases also normalize in the core. | These unknown properties may be assumed. |
| \(z^{-a}=1/z^a\) | A manifest negative exact coefficient in the symbolic exponent and a nonzero base. | Unknown nonzeroness may be assumed. |
| \((z/w)/(a/b)=zb/(wa)\), \(z/(a/b)=zb/a\) | Removed inner denominator \(b\) is nonzero; the other direct operands are finite. The core handles numerator nesting. | Unknown nonzeroness of \(b\) may be assumed. |
| \(a/b+c/d=(ad+cb)/(bd)\) | Finite component numerators and denominators; no denominator factor is cancelled. Equal denominators share one copy. Plain finite terms can join a fraction. | Same rule. |
| Manifest \(fz/(fw)=z/w\) | Removed factor \(f\) is nonzero. Positive integral multiplicities are subtracted exactly. | Unknown nonzeroness may be assumed. |
| Polynomial \(P/Q=(P/G)/(Q/G)\) | Exact polynomial coefficients; finite inputs; verified exact divisibility; removed GCD expression \(G\ne0\). | Unknown finiteness and nonzeroness may be assumed. |
| \(\lvert\lvert z\rvert\rvert=\lvert z\rvert\), \(\lvert-z\rvert=\lvert z\rvert\) | Finite operand; explicit negative prefix for the latter. | Same rule. |
| \(\lvert z^n\rvert=\lvert z\rvert^n\) | Positive exact integer exponent and finite base. | Same rule. |
| \(\lvert p\rvert=p\), \(\lvert x\rvert=-x\) for \(x\le0\) | Proved corresponding sign. | Same direct sign proof. |
| \(\lvert x\rvert^2=x^2\), \(\operatorname{sign}(\operatorname{sign}x)=\operatorname{sign}x\), \(\lvert x\rvert\operatorname{sign}x=x\) | Real input. | Unknown realness may be assumed. |
| \(\operatorname{sign}(\lvert z\rvert)=1\) | Nonzero input. | Unknown nonzeroness may be assumed. |

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
| \(\sqrt{x^{2n}}=\lvert x\rvert^n\) | Real base. | Unknown realness may be assumed. |
| \(\sqrt{x^{2n+1}}=\lvert x\rvert^n\sqrt{x}\) | Real base and positive exact odd exponent. | Unknown realness may be assumed. |
| \(\operatorname{cbrt}(x^{3n})=x^n\), \(\operatorname{cbrt}(x^n)=\operatorname{cbrt}(x)^n\) | Real base and positive exact integer exponent. | Unknown realness may be assumed. |
| \(\sqrt{p^r}=p^{r/2}\), \(\operatorname{cbrt}(p^r)=p^{r/3}\) | Positive base; real symbolic or exact rational exponent. | These unknown scalar properties may be assumed. |
| \(\sqrt{a}\sqrt{b}=\sqrt{ab}\) | Both radicands nonnegative. | Caller must ensure the principal roots multiply without a branch change; nonnegative radicands suffice. |
| \(\sqrt{a}/\sqrt{b}=\sqrt{a/b}\) | Nonnegative numerator radicand, positive denominator radicand. | Denominator radicand assumed nonzero and caller supplies branch compatibility. |
| \(\sqrt{x^2 z}=\lvert x\rvert\sqrt{z}\) | Real \(x\), finite \(z\). | Unknown realness/finiteness may be assumed. |
| \(\operatorname{cbrt}(x^3)=x\), \(\operatorname{cbrt}(z)^3=z\) | Real input for the first, finite input for the second. | Unknown realness for the first through the direct power rule. |

## Exponential and logarithmic

Write \(L\) for `ln`, `log2`, `log10`, or `log(base, value)`.
Generic-base direct rules additionally require a positive real base unequal to
one; A mode permits those unknown base conditions to be assumed.

| Identity | P precondition | A extension |
| --- | --- | --- |
| \(\exp(\ln z)=z\), \(\exp(a\ln z)=z^a\) | Nonzero \(z\), finite \(a\). | Unknown nonzeroness/finiteness may be assumed. |
| \(\ln(\exp x)=x\) | Real input. | Unknown realness may be assumed. |
| \(\exp a\exp b=\exp(a+b)\), \(\exp a/\exp b=\exp(a-b)\) | Finite arguments. | Same rule. |
| \(L(p^r)=rL(p)\) | Positive base and real exponent. | These unknown properties may be assumed. |
| \(L(x^{2n})=2nL(\lvert x\rvert)\) | Real input. The logarithmic exclusion at zero survives. | Unknown realness may be assumed. |
| \(L(1/p)=-L(p)\) | Positive denominator. | Unknown positivity may be assumed. |
| \(L(\sqrt z)=L(z)/2\) | Finite input, with the same principal square-root/logarithm conventions. Both retain zero as excluded. | Same rule. |
| \(L(\operatorname{cbrt}p)=L(p)/3\), \(L(\exp x)=xL(e)\) | Positive cube-root input; real exponential argument. | Unknown sign/realness may be assumed. |
| \(L(1)=0\), \(\log_b b=1\) | Generic base positive and unequal to one. Unary numeric cases normalize in the core. | Unknown generic-base conditions may be assumed. |
| \(\ln p+\ln q=\ln(pq)\), \(\ln p-\ln q=\ln(p/q)\) | Positive arguments. | A templates also permit finite arguments with caller-supplied nonzero and principal-argument compatibility. Positive arguments suffice. |
| \(\log_2 p+\log_2 q=\log_2(pq)\), and base ten | Positive arguments. | Same guarded templates. |

The principal logarithm is not globally additive. For the A logarithmic sum,
\(\operatorname{Arg}a+\operatorname{Arg}b\) must stay in the selected principal
argument range; for the difference, the corresponding difference must stay there.
Selecting A mode does not verify these range obligations.

## Trigonometric

The following P formulas accept finite complex arguments. Quotient rewrites keep
the same denominator zeros unless an extra guard is stated.

| Group | Active formulas |
| --- | --- |
| Pythagorean | \(\sin^2a+\cos^2a=1\); \(1-\sin^2a=\cos^2a\); \(1-\cos^2a=\sin^2a\). |
| Tangent/cotangent | \(1+\tan^2a=\sec^2a\), \(1+\cot^2a=\csc^2a\), and the rearrangements \(\sec^2a-1=\tan^2a\), \(\csc^2a-1=\cot^2a\). |
| Pole removal | \(\sec^2a-\tan^2a=1\) additionally proves \(\cos a\ne0\); A mode may assume this exclusion. |
| Double angles | \(\cos^2a-\sin^2a=\cos2a\), the sign-reversed form, \((\cos a-\sin a)(\cos a+\sin a)=\cos2a\), and \(\sin a\cos a=\sin(2a)/2\), including scalar multiples. |
| Addition/subtraction | \(\sin a\cos b\pm\cos a\sin b=\sin(a\pm b)\); \(\cos a\cos b\mp\sin a\sin b=\cos(a\pm b)\). |
| Triple angles | \(3\sin a-4\sin^3a=\sin3a\); \(4\cos^3a-3\cos a=\cos3a\). |
| Quotients | \(\sin a/\cos a=\tan a\), \(\cos a/\sin a=\cot a\), \(1/\cos a=\sec a\), \(1/\sin a=\csc a\). |
| Reciprocal tangent | \(1/\tan a=\cot a\) proves \(\cos a\ne0\); A mode may assume this exclusion. |
| Forward inverse compositions | \(\sin(\arcsin a)=a\), \(\cos(\arccos a)=a\); \(\tan(\arctan a)=a\) additionally proves \(1+a^2\ne0\). A mode may assume the latter exclusion. |
| Cofunctions | At \(\pi/2-a\): sin/cos, tan/cot, and sec/csc exchange in both directions. |
| Parity | sin, tan, cot, csc are odd; cos, sec are even. An explicit negative prefix is normalized. |
| Exact angles | Exact symbolic multiples \(k\pi/12\), reduced modulo \(2\pi\), yield rational/radical sine and cosine values; tan/cot/sec/csc use these values only when their denominator is nonzero. |
| Quarter turns | Translate all six functions by exact multiples of \(\pi/2\), including full periods, reflection about \(\pi\), and \(3\pi/2\) shifts. |

Additional **A-only** reverse compositions are
\(\arcsin(\sin x)=x\) for \(-\pi/2\le x\le\pi/2\),
\(\arccos(\cos x)=x\) for \(0\le x\le\pi\), and
\(\arctan(\tan x)=x\) for \(-\pi/2<x<\pi/2\).
The caller supplies these real principal-range restrictions.

## Hyperbolic

| Group | P formulas / conditions |
| --- | --- |
| Entire identities | \(\cosh^2a-\sinh^2a=1\), \(1+\sinh^2a=\cosh^2a\), \(\cosh^2a-1=\sinh^2a\). |
| Meromorphic identities | \(1-\tanh^2a=\operatorname{sech}^2a\), \(\coth^2a-1=\operatorname{csch}^2a\), retaining their poles. |
| Pole removal | \(\tanh^2a+\operatorname{sech}^2a=1\) proves \(\cosh a\ne0\); A mode may assume it. |
| Double angles | \(2\sinh a\cosh a=\sinh2a\), \(\cosh^2a+\sinh^2a=\cosh2a\). |
| Addition/subtraction | \(\sinh a\cosh b\pm\cosh a\sinh b=\sinh(a\pm b)\); \(\cosh a\cosh b\pm\sinh a\sinh b=\cosh(a\pm b)\). |
| Triple angles | \(3\sinh a+4\sinh^3a=\sinh3a\); \(4\cosh^3a-3\cosh a=\cosh3a\). |
| Quotients | \(\sinh a/\cosh a=\tanh a\), \(\cosh a/\sinh a=\coth a\), \(1/\cosh a=\operatorname{sech}a\), \(1/\sinh a=\operatorname{csch}a\). |
| Reciprocal tanh | \(1/\tanh a=\coth a\) proves \(\cosh a\ne0\); A mode may assume it. |
| Forward inverse compositions | \(\sinh(\operatorname{asinh}a)=a\), \(\cosh(\operatorname{acosh}a)=a\); \(\tanh(\operatorname{atanh}a)=a\) additionally proves \(1-a^2\ne0\), or assumes it in A mode. |
| Reverse compositions | \(\operatorname{asinh}(\sinh x)=x\), \(\operatorname{atanh}(\tanh x)=x\) for real \(x\); \(\operatorname{acosh}(\cosh x)=x\) for nonnegative real \(x\). A mode admits caller-assumed restrictions. |
| Parity | sinh, tanh, coth, csch are odd; cosh, sech are even. |

Exponential conversions use \(E=e^a\), \(F=e^{-a}\), \(T=e^{2a}\):

| Input form | Output |
| --- | --- |
| \((E-F)/2\), \((T-1)/(2E)\) | \(\sinh a\) |
| \((E+F)/2\), \((T+1)/(2E)\) | \(\cosh a\) |
| \((E-F)/(E+F)\), \((T-1)/(T+1)\) | \(\tanh a\) |
| \((E+F)/(E-F)\), \((T+1)/(T-1)\) | \(\coth a\) |
| \(2/(E+F)\), \(2E/(T+1)\) | \(\operatorname{sech}a\) |
| \(2/(E-F)\), \(2E/(T-1)\) | \(\operatorname{csch}a\) |

All these P conversions have matching pole sets for finite arguments.

## Expand, Factor, and Compilation

| Step | P transformations |
| --- | --- |
| Expand | Distribute finite products over sums and positive exact integer powers of sums, using Cartesian products and exponentiation by squaring. Generated term counts are checked before allocation. |
| Factor | Extract shared manifest positive integer powers from finite addends, including subsets of a sum. Extract an exact integer coefficient GCD when it fits a positive native integer. No symbolic value is divided out. |
| Factor templates | \(a^2\pm2ab+b^2=(a\pm b)^2\); \(a^3\pm3a^2b+3ab^2\pm b^3=(a\pm b)^3\); explicit \(b=1\) forms; \(a^2-b^2=(a-b)(a+b)\). Inputs must be finite. |
| Compilation | \(e^a-1=\operatorname{expm1}a\), \(1-e^a=-\operatorname{expm1}a\), \(\ln(1+a)=\operatorname{log1p}a\), \(1-\operatorname{erf}a=\operatorname{erfc}a\), and \(\operatorname{expm1}a+1=e^a\). |
| Compilation cube root | Exact \(p^{1/3}=\operatorname{cbrt}p\), with a proved positive real base. |
| Compilation sinc | \(\sin a/a=\operatorname{sinc}a\), requiring \(a\ne0\). A mode may assume this exclusion. |
| Compilation inverses | \(\operatorname{log1p}(\operatorname{expm1}x)=x\) for real \(x\); \(\operatorname{expm1}(\operatorname{log1p}a)=a\) for \(1+a\ne0\). A mode may supply these unknown conditions. |
| Compilation factoring | The same manifest-factor kernel as Factor, allowing repeated sweeps to form nested polynomial arithmetic. |

Expand and Factor have no A-only templates. The compilation objective uses portable
operation estimates; it does not establish a numerical speed guarantee. Custom
scores can model a particular backend. Mathematical equivalence does not promise
bitwise identical floating evaluation: `exp_m1` and `ln_1p` intentionally improve
cancellation behavior.

## Legacy rule coverage

There are **128 registered entries**: 46 algebraic, 19 numeric, 6 root,
9 exponential, 27 trigonometric, and 21 hyperbolic. Repeated names in different
families refer to different implementations. The tables account for every entry;
several entries share one normalized kernel. P/A refer to the conditions above.
The migration preserves the mathematical capability under its valid hypotheses,
not the legacy float approximations or unsound branch/pole removal.

### Algebraic — 46 entries

| Legacy entries | Active destination |
| --- | --- |
| `ExpLnRule`, `EPowLnRule` | Core exponential alias + Exponential P/A inverse logarithm. |
| `LnExpRule` | Exponential P/A real logarithm of an exponential. |
| `ExpMulLnRule`, `EPowMulLnRule` | Core alias + Exponential P/A logarithmic exponent. |
| `PowerZeroRule` | Algebraic P/A nonzero-base zero exponent. |
| `PowerOneRule` | Core normalization. |
| `PowerPowerRule` | Core checked integer nesting; Algebraic guarded nesting and real even-power roots; Roots for square-root aliases. |
| `PowerProductRule`, `PowerCollectionRule` | Core manifest integer collection; Algebraic P/A same-base grouping. |
| `PowerDivRule` | Algebraic P/A same-base quotient and cancellation. |
| `CommonExponentDivRule`, `CommonExponentProductRule` | Algebraic P/A positive-base grouping; Roots covers nonnegative square roots. |
| `NegativeExponentToFractionRule` | Core known negative literal exponents; Algebraic P/A negative symbolic coefficient. |
| `PowerOfQuotientRule` | Core positive integer distribution; Algebraic P/A branch-compatible quotient powers. |
| `DivSelfRule` | Algebraic P/A cancellation. |
| `DivDivRule` | Core numerator flattening; Algebraic P/A denominator reciprocal flattening. |
| `CombineNestedFractionRule`, `AddFractionRule` | Algebraic finite fraction addition + core quotient normalization. |
| `FractionToEndRule` | Core denominator collection. |
| `AbsNumericRule`, `SignNumericRule` | Core literal folding. |
| `AbsAbsRule`, `AbsNegRule`, `AbsSquareRule`, `AbsPowEvenRule` | Algebraic absolute-value identities with finite inputs. |
| `SignSignRule`, `SignAbsRule`, `AbsSignMulRule` | Algebraic P/A real-sign/nonzero identities. |
| `ExpandPowerForCancellationRule` | Algebraic exact polynomial GCD; an explicit Expand step exposes expanded candidates when desired. |
| `PowerExpansionRule` | Expand. Compact presets use polynomial cancellation without publishing an expansion. |
| `FractionCancellationRule` | Algebraic manifest and polynomial P/A cancellation. |
| `PerfectSquareRule`, `FactorDifferenceOfSquaresRule`, `PerfectCubeRule` | Factor templates. |
| `NumericGcdFactoringRule`, `CommonTermFactoringRule`, `CommonPowerFactoringRule` | Factor manifest kernels. |
| `PolyGcdSimplifyRule` | Algebraic exact multivariate primitive polynomial GCD and certified quotient reduction. |
| `CanonicalizeProductRule`, `CanonicalizeSumRule`, `SimplifyNegativeProductRule` | Core normalization. |
| `ProductDivCombinationRule`, `CombineTermsRule`, `CombineFactorsRule`, `CombineLikeTermsInSumRule` | Core denominator/term/factor collection. |

### Numeric — 19 entries

| Legacy entries | Active destination |
| --- | --- |
| `SumIdentityRule`, `ProductIdentityRule`, `DivOneRule` | Core identities. |
| `ProductZeroRule`, `ZeroDivRule`, `PowZeroRule`, `OnePowRule` | Core valid literal cases + Algebraic finite/nonzero P/A guards. |
| `PowOneRule`, `ZeroPowRule` | Core powers; Algebraic adds proved positive symbolic exponents. |
| `EvaluateNumericFunctionRule` | Core exact/approximate builtin folding; custom body expansion through caller Map. |
| `NormalizeSignDivRule` | Core exact coefficient and denominator sign normalization. |
| `ConstantFoldSumRule`, `ConstantFoldProductRule`, `ConstantFoldDivRule`, `ConstantFoldPowRule` | Core backend-owned literal arithmetic. |
| `FractionSimplifyRule` | Core exact reduced rational coefficients. |
| `PerfectSquareRule`, `PerfectCubeRule` | Core exact builtin root folding. |
| `ProductHalfRule` | Core exact rational coefficient form; approximate halves retain floating representation. |

### Root — 6 entries; exponential — 9 entries

| Legacy family / entries | Active destination |
| --- | --- |
| Root: `SqrtPowerRule`, `CbrtPowerRule` | Roots P/A real and positive-base power identities. |
| Root: `SqrtExtractSquareRule` | Roots P/A real-square extraction. |
| Root: `SqrtProductRule`, `SqrtDivRule` | Roots P/A branch-compatible products and quotients. |
| Root: `NormalizeRootsRule` | Core square-root alias; real cube-root builtin retained to preserve its negative-real meaning. |
| Exponential: `LnOneRule`, `LnERule`, `ExpZeroRule` | Core exact builtin identities. |
| Exponential: `ExpToEPowRule` | Core canonical `exp` alias, shared with exact Euler powers. |
| Exponential: `ExpLnIdentityRule`, `LnExpIdentityRule` | Exponential P/A inverse compositions. |
| Exponential: `LogPowerRule` | Exponential direct unary/generic-base power, absolute-value, root, and reciprocal identities. |
| Exponential: `LogBaseRules` | Core unary values + Exponential P/A generic-base one/self identities. |
| Exponential: `LogCombinationRule` | Exponential positive-real P or caller branch-compatible A sums/differences. |

### Trigonometric — 27 entries

| Legacy entries | Active destination |
| --- | --- |
| `SinZeroRule`, `CosZeroRule`, `TanZeroRule` | Core exact zero values. |
| `SinPiRule`, `CosPiRule`, `SinPiOverTwoRule`, `CosPiOverTwoRule`, `TrigExactValuesRule` | Trigonometric exact symbolic-pi angle kernel. |
| `PythagoreanIdentityRule`, `PythagoreanComplementsRule`, `PythagoreanTangentRule` | Trigonometric guarded P identities. |
| `InverseTrigIdentityRule` | Trigonometric A reverse principal-range compositions. |
| `InverseTrigCompositionRule` | Trigonometric P forward inverse compositions, guarded at atan singularities; A may assume the exclusion. |
| `CofunctionIdentityRule`, `TrigPeriodicityRule`, `TrigReflectionRule`, `TrigThreePiOverTwoRule` | Trigonometric cofunction templates and exact quarter-turn kernel. |
| `TrigNegArgRule` | Trigonometric parity kernel. |
| `CosDoubleAngleDifferenceRule`, `TrigProductToDoubleAngleRule`, `SinProductToDoubleAngleRule` | Trigonometric double-angle templates, including arbitrary exact scalar multiples of sine/cosine products. |
| `TrigSumDifferenceRule`, `TrigTripleAngleRule` | Trigonometric addition/subtraction and triple-angle templates. |
| `OneCosToSecRule`, `OneSinToCscRule`, `SinCosToTanRule`, `CosSinToCotRule` | Trigonometric quotient templates with matching poles. |

### Hyperbolic — 21 entries

| Legacy entries | Active destination |
| --- | --- |
| `SinhZeroRule`, `CoshZeroRule` | Core exact zero values. |
| `SinhAsinhIdentityRule`, `CoshAcoshIdentityRule`, `TanhAtanhIdentityRule` | Hyperbolic forward inverse compositions; atanh singularities guarded in P and assumable in A. |
| `SinhNegationRule`, `CoshNegationRule`, `TanhNegationRule` | Hyperbolic parity kernel. |
| `HyperbolicIdentityRule` | Hyperbolic P identities with pole-removal guards; A may assume missing exclusions. |
| `SinhCoshToTanhRule`, `CoshSinhToCothRule`, `OneCoshToSechRule`, `OneSinhToCschRule` | Hyperbolic quotient templates with matching poles. |
| `OneTanhToCothRule` | Hyperbolic reciprocal tanh; P proves nonzero cosh, A may assume it. |
| `SinhFromExpRule`, `CoshFromExpRule`, `TanhFromExpRule`, `SechFromExpRule`, `CschFromExpRule`, `CothFromExpRule` | Hyperbolic exponential conversion table. |
| `HyperbolicTripleAngleRule` | Hyperbolic sinh/cosh triple-angle templates. |

## Execution invariants and limits

Rule plans compile when a step is added, and A plans when that mode is enabled.
Sequence clones share immutable compiled storage. Application never recompiles
templates. Necessary function identities reject irrelevant rules before bounded
commutative matching; square-root heads are exempt because counted rational powers
can supply their groups implicitly. Function sets refresh after changed outputs.

Traversal is iterative and memoized. A rule does not revisit its inserted output;
the next rule and the next sweep can visit it. Unmatched root operands survive.
Completed candidates are checked against node/depth limits before selection.
Failure publishes no partially transformed expression. The caller remains
responsible for callback work, purity, and side effects.

Defaults: 16 sweeps, 100,000 stored nodes, depth 512, `Some(100_000)` candidate
attempts per core rewrite, 4,096 generated expansion terms per expansion pass,
and 100,000 polynomial work units per algebraic pass. The polynomial budget also
bounds exact exponent capacity; it does not extend the numeric backend's range.
On 16-bit targets, the 100,000-unit defaults are capped at `usize::MAX`.
Callback and numeric backend capacity panics retain their existing contracts.

## Mathematical conventions

The principal logarithm and power conventions are described in
[DLMF §4.2](https://dlmf.nist.gov/4.2). Trigonometric additions and Pythagorean
identities follow [DLMF §4.21](https://dlmf.nist.gov/4.21); inverse ranges follow
[§4.23](https://dlmf.nist.gov/4.23). Hyperbolic identities follow
[§4.35](https://dlmf.nist.gov/4.35), with inverse conventions in
[§4.37](https://dlmf.nist.gov/4.37). The implementation imposes the additional
guards above to preserve equality of partial functions. Cube roots on real
inputs use the real builtin rather than a general complex fractional power.
