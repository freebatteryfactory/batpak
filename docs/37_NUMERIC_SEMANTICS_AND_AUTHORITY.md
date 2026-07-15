---
status: AUTHORITATIVE
contract_id: BP-NUMERIC-1
authority_scope: exact and approximate numeric semantics, authority crossings, and numeric proof obligations
supersedes: numeric wording previously implied only through DEC-060 and the language companion
last_reconciled: 2026-07-14
---

# Numeric Semantics and Authority

This document is the canonical owner of BatPak's numeric model: the exact
authority categories, first-class Qualified Approximation, the legal crossings
between them, and the numeric proof obligations. It freezes semantic
requirements before durable bytes and runtime arithmetic harden. It does not
implement runtime arithmetic, residual specialization, SIMD, or wide-number
kernels; those are later gates.

Numeric authority (`DEC-069`) locks the exact-versus-approximate boundary.
Concrete typed arithmetic, units, ratios, rounding, and the operator facts
remain owned by `DEC-060` and `spec/operators.rs` (OperatorSpec). The shared
semantic sorts are frozen in `04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md`.

## 1. Exact Authority

Authority-capable numeric categories:

```text
ExactInteger
FixedDecimal
ExactRatio
Money
Percent
PercentagePoints
Duration
Count
TypedMargin
other explicitly unit-bearing exact values admitted by schema
```

The canonical ordinary fixed semantic shape is:

```text
signed i128 coefficient
bounded decimal scale
validated UnitId
checked range
explicit numeric profile (NumericProfileId)
```

The physical EventFrame field layout and the maximum scale are **not** frozen
here; they are G2 format constants. The frozen semantic requirements are:

```text
scale is explicit
unit is explicit
range is checked
scale increase is exact when representable
scale reduction requires an explicit rounding mode
overflow is a typed refusal
no saturation
no silent truncation
no implicit unit conversion
```

Decimal source text parses directly into a coefficient and scale and never
passes through binary floating point:

```text
12.34  ->  coefficient 1234, scale 2
```

`ExactRatio` is canonical:

```text
denominator is nonzero
denominator is positive
numerator and denominator are reduced by gcd
zero is represented canonically as 0/1
overflow is refused
```

When a bounded exact ratio cannot be represented, the qualified wide-exact path
(section 11) is used or a typed range refusal is returned. A ratio is never
silently approximated.

Dimensional arithmetic is preserved from DEC-060:

```text
addition / subtraction        same compatible unit only
multiplication                dimensional value by dimensionless value
division of like dimensions   ExactRatio
Money x Money                 rejected
cross-currency arithmetic     rejected without an explicit conversion function
                              and a receipted rate source
```

## 2. Qualified Approximation

Approximate values are first-class but quarantined from implicit authority. They
use one semantic sort, `ApproximateBinary`, carrying a format identity rather
than creating a separate semantic universe per hardware format.

V1 format identities (`FloatFormatId`):

```text
Binary32
Binary64
```

Adding another format later requires an explicit profile extension and
qualification.

An observed approximate value carries at least:

```text
FloatFormatId
exact raw observed bits
numeric classification
exact finite dyadic decomposition when finite
ApproximationProfileId
producer or input provenance
declared interval or error evidence
approximation-taint state
```

Numeric classification distinguishes:

```text
Finite
PositiveZero
NegativeZero
PositiveInfinity
NegativeInfinity
NaN
```

Observed inputs preserve exact raw bits; for observed NaNs the raw payload bits
remain forensic evidence. BatPak does not assign identities to individual NaNs,
infinities, zero signs, or decimal positions. Stable identity belongs only to:

```text
FloatFormatId
ApproximationProfileId
NumericProfileId
QuantizationPolicyId
RoundingModeId
UnitId
QualifiedKernelId
WideExactProfileId
```

Computed approximate outputs use a profile-defined canonical exceptional-value
policy. Each `ApproximationProfileId` states:

```text
NaN production / canonicalization
infinity behavior
signed-zero behavior
error or interval propagation
cross-target qualification
kernel identity
```

## 3. Canonical evidence versus numeric authority

An approximate observation may have canonical evidence bytes. That does not make
its approximate numeric meaning an authority value.

The exact raw bit pattern MAY be recorded, hashed, committed, receipted,
replayed, and compared for evidence identity. It MAY NOT silently determine:

```text
canonical numeric identity
idempotency identity
durable numeric ordering
commit position
effect admission
proof disposition
fixed exact value
receipt authority meaning
```

The G2 wire encoding must preserve the exact format tag and raw bit pattern and
must not normalize observed raw bits. The precise envelope byte layout is
G2-owned.

## 4. Signed zero, NaN, and ordering

Evidence identity and numeric meaning are kept separate:

```text
Evidence equality            exact format plus raw bits
Numeric equality (finite 0)  PositiveZero equals NegativeZero numerically
Evidence identity (zero)     PositiveZero and NegativeZero remain distinct
NaN numeric equality         not false, not true; typed unordered/non-finite refusal
NaN authority comparison     refused
infinity-to-fixed quantize   refused
NaN-to-fixed quantize        refused
```

There is no ordinary semantic `Ord` over `ApproximateBinary`. A diagnostic
bit-pattern ordering may exist later but must be named evidence ordering and
cannot masquerade as numeric ordering. Authority-sensitive numeric comparison in
V1 requires finite inputs or finite intervals; a non-finite input returns a
typed numeric refusal.

## 5. Approximation taint and provenance

Approximation provenance is sticky. Converting a finite float to its exact
dyadic decomposition is lossless with respect to the observed bits, but it does
not prove the original decimal, physical, or human intent.

The provenance chain is distinct at every stage:

```text
source decimal text
parsed exact decimal
binary observation
finite dyadic representation of that observation
quantized authority value
```

There is no inverse operation that claims to recover source decimal intent from
a binary observation. A quantized value may become exact authority only through
an admitted policy and receipt (section 9); its provenance still records that
its source was approximate.

## 6. Availability and approximation are orthogonal

Numeric classification is never collapsed into availability. These remain
separate axes (see DEC-030 and `04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md`):

```text
Known
Missing
Pending
Invalid
Unavailable
Shredded
NotApplicable
```

Examples:

```text
Known(ApproximateBinary::NaN)   a known value with an exceptional classification
Missing                         no numeric value present
Pending                         resolution or required evidence not complete
Invalid                         supplied or computed material violated its contract
```

Approximate does not imply `Pending`, `Unverified`, `Incomplete`, or `Invalid`.
A result may be `Known`, `Verified`, `Complete`, and `Approximate` at once.

An interval crossing a decision boundary produces:

```text
Availability      Known
Truth             Pending
Decision          Defer
ProofDisposition  unchanged from the evidence
reason            precision insufficient for a conclusive decision
```

## 7. Interval

V1 `Interval` is closed, finite, unit-bearing, and exactly bounded:

```text
lower <= upper
both bounds exact in one compatible comparison domain
```

For binary-float-derived intervals, bounds are exact dyadic or exact-rational
bounds, never raw floating values. V1 adds no open intervals, unbounded
intervals, or infinity endpoints. An interval carries:

```text
lower bound
upper bound
UnitId
ApproximationProfileId
provenance
error-model identity
```

A malformed interval (including `lower > upper`) is a typed refusal.

## 8. IntervalDecision

`IntervalDecision` compares a finite closed interval against an exact compatible
threshold. General law:

```text
whole interval satisfies the predicate      -> True
whole interval contradicts the predicate     -> False
interval overlaps the decision boundary       -> Pending
```

Equality:

```text
singleton interval equal to threshold                 -> True
interval disjoint from threshold                      -> False
non-singleton interval containing threshold           -> Pending
```

Inequality uses the corresponding exact complement. Per-operator truth tables
(interval `[lo, hi]` versus exact threshold `t`, one compatible unit domain):

```text
IS (equal)         lo = hi = t                 -> True
                   t < lo or t > hi            -> False
                   lo <= t <= hi and lo < hi   -> Pending

IS NOT             lo = hi = t                 -> False
                   t < lo or t > hi            -> True
                   lo <= t <= hi and lo < hi   -> Pending

IS LESS THAN       hi < t                      -> True
                   lo >= t                     -> False
                   lo < t <= hi                -> Pending

IS AT MOST         hi <= t                     -> True
                   lo > t                      -> False
                   lo <= t < hi                -> Pending

IS MORE THAN       lo > t                      -> True
                   hi <= t                     -> False
                   lo <= t < hi                -> Pending

IS AT LEAST        lo >= t                     -> True
                   hi < t                      -> False
                   lo < t <= hi                -> Pending
```

Typed margin preserves unit, direction, distance or distance interval, and
whether zero lies inside the margin. If zero lies inside the margin interval,
the decision cannot claim a conclusive side.

## 9. Quantize

`Quantize` is the explicit crossing from Qualified Approximation into Fixed128
authority. It accepts:

```text
finite approximate observation or finite interval
target exact numeric profile
target scale
UnitId
explicit rounding mode
quantization policy (QuantizationPolicyId)
```

It returns:

```text
Fixed128 result
exact or inexact disposition
exact finite source representation
source interval
target profile
rounding mode
discarded remainder or equivalent loss evidence
result error interval
QuantizationReceipt
```

Every approximation-to-authority crossing produces a `QuantizationReceipt`, even
when the conversion is mathematically exact. There is no default rounding mode,
no hidden conversion, no direct cast, and no best-effort conversion.

`Quantize` refuses:

```text
NaN
PositiveInfinity
NegativeInfinity
range overflow
unsupported scale
unit mismatch
missing profile
missing error evidence where the profile requires it
```

## 10. Rounding boundary

The six canonical rounding modes are frozen by DEC-060:

```text
HalfEven
HalfAwayFromZero
TowardZero
AwayFromZero
Floor
Ceiling
```

Authority-changing operations name the mode explicitly. Exact widened
intermediates are preferred when bounded capacity permits; rounding happens only
at a declared scale boundary; every actual rounding event is recorded. General
associativity and distributivity are not claimed for operations that round at
intermediate boundaries. Proof obligations distinguish exact algebra laws,
rounded-profile laws, and valid profile-specific metamorphic laws.

## 11. Wide-exact profile seam

The seam is frozen; the implementation is not. A qualified wide-exact profile
may provide an arbitrary-precision integer, an arbitrary-precision rational, or
another exact representation behind a qualified kernel contract. The profile
binds:

```text
WideExactProfileId
kernel interface identity
kernel implementation identity where pinned
resource budget
canonical artifact format identity
input commitment
output commitment
qualification receipt
```

No bignum dependency is selected and no wide kernels are implemented in this
phase. Wide values are not ordinary inline EventFrame values. Crossing wide
exact into Fixed128 requires a checked range, an explicit scale, explicit
rounding if lossy, and a conversion receipt; out of range returns a typed
refusal. A rare durable wide result may be stored only through a canonical
wide-exact artifact plus `ArtifactRef`, format identity, content commitment, and
a bounds/resource receipt.

## 12. OperatorSpec numeric support

Each arithmetic operator declares one numeric support posture in
`spec/operators.rs`: `ExactSupported`, `QualifiedProfileOnly`, or `Unsupported`.
General approximate arithmetic through `+ - * /` is not automatically admitted;
approximate arithmetic is admitted only when a qualified profile defines sound
error or interval propagation. A typed value wrapper does not make ordinary
float arithmetic safe. Comparison operators may lower interval-versus-threshold
cases through `IntervalDecision` (section 8); logical operators are unchanged.

The operator numeric support matrix is generated from OperatorSpec and
independently re-audited:

<!-- OPERATORS-NUMERIC:BEGIN generated from spec/operators.rs by bootstrap/project.py; do not edit -->
| OperatorId | Class | Numeric support |
| --- | --- | --- |
| OP-ADD | Arithmetic | ExactSupported |
| OP-DIV | Arithmetic | ExactSupported |
| OP-MUL | Arithmetic | ExactSupported |
| OP-SUB | Arithmetic | ExactSupported |
| OP-EQ | Comparison | ExactSupported |
| OP-GE | Comparison | ExactSupported |
| OP-GT | Comparison | ExactSupported |
| OP-LE | Comparison | ExactSupported |
| OP-LT | Comparison | ExactSupported |
| OP-NE | Comparison | ExactSupported |
| OP-AND | Logical | NotApplicable |
| OP-NOT | Logical | NotApplicable |
| OP-OR | Logical | NotApplicable |
<!-- OPERATORS-NUMERIC:END -->

## 13. Numeric proof obligations

The following proof families are declared here as obligations. Their executable
TestPak and Muterprater ownership is realized in `24_GAUNTLET.md` and, in 5.5D,
`12_TESTPAK.md`. Bootstrap does not evaluate, quantize, fuzz, or interval-check
actual numbers; those are future executable gates.

```text
decimal-text-to-coefficient-scale exactness
exact-ratio canonicalization and overflow refusal
dimensional arithmetic legality (DEC-060)
raw-bit preservation on observation
finite dyadic decomposition exactness
numeric classification of raw bits
signed-zero numeric-equality versus evidence-identity separation
NaN unordered/non-finite refusal
interval well-formedness and IntervalDecision truth tables (six operators)
quantization interval containment and exact/inexact disposition
quantization receipt completeness
authority-crossing refusals (NaN, infinity, overflow, unit mismatch, missing profile)
rounding-boundary and mode-specific laws (exact vs rounded vs metamorphic)
wide-exact-to-Fixed128 checked conversion and range refusal
```

## 14. Ownership

```text
DEC-069           exact-versus-approximate authority boundary
DEC-060           typed arithmetic, units, ratios, rounding, OperatorSpec
DEC-030 / DEC-034 orthogonal axes; one evaluation, many projections
docs/04           shared semantic sorts
docs/13           BatQL typing and lowering into shared numeric sorts
companion         language-facing type and operation semantics; type-vocabulary record
docs/14           QuantizationReceipt, approximation evidence, typed margin projection
docs/07           PakVM reference execution of numeric sorts (no specialization here)
docs/24           numeric proof and hostile-test obligations
docs/25           G2 / G5 / TestPak ownership
```
