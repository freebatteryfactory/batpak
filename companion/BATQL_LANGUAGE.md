---
status: AUTHORITATIVE
contract_id: BP-BATQL-LANGUAGE-1
authority_scope: BatQL 1.0 lexical, syntactic, type, availability, decision, query, transaction, formatting, narration, and diagnostic language
supersedes: all prior BatQL drafts
last_reconciled: 2026-07-13
---

# BatQL Frontend Language Packet

> **Freeze:** BatQL 1.0 is normative, identified by its `BatQlLanguageVersion`. Only defects proven by parser, type, compatibility, or conformance work may amend it through the signed language-change law. Each admitted operator carries a stable `OperatorId` in the typed operator authority (`spec/operators.rs`).


## Definition

**BatQL is a typed, temporal query language for immutable event history.**

It combines:

```text
5W1H question structure
+ spreadsheet formula ergonomics
+ event-native replay and sequence operations
+ structured decisions and explanations
+ explicit proof and completeness
+ loud, receipted transactions
```

BatQL is not SQL with renamed keywords. It is not a policy language wrapped around an event reader. It is not a general-purpose programming language.

Its native questions are:

```text
WHO    Which entity, actor, correlation, or partition is this about?
WHAT   Which values, state, decision, timeline, or action are requested?
WHERE  Which journal, projection, or saved query supplies the evidence?
WHEN   At which historical cut, and across which temporal range?
HOW    How should events be folded, matched, grouped, ordered, and bounded?
WHY    Which explanation, provenance, proof, and cost should accompany the answer?
```

The canonical formatted order is:

```text
WHO
WHAT
WHERE
WHEN
HOW
WHY
```

`WHY` comes last because it explains the complete question.

Authors may dictate or type clauses in any order. The formatter always returns them to canonical order. Each clause may appear at most once in a query.

---

# 1. Core language laws

## 1.1 Two top-level modes

BatQL has two primary top-level forms:

```text
ASK    Pure query. Cannot mutate source truth or request external effects.
DO     Admitted transaction. May perform declared effects and always returns a receipt.
```

That boundary is structural. The typed source-mode inventory is projected
from the command namespaces:

<!-- BATQL-SOURCE-MODES:BEGIN generated from spec/commands.rs by bootstrap/project.py; do not edit -->
```text
ASK        direct     BP-BATQL-LANGUAGE-1
DO         direct     BP-BATQL-LANGUAGE-1
```
<!-- BATQL-SOURCE-MODES:END -->

ASK and DO are language modes, never CLI verbs: `DO` is not a top-level
product command, and `batpak query` executes a canonical query-only
ProgramImage rather than compiling raw source.

An `ASK` containing `APPEND` or `REQUEST EFFECT` does not compile.

A `DO` first evaluates its query, decisions, and requirements without performing effects. Effects execute only after all required conditions reach a conclusive passing state.

## 1.2 One typed form, several readable projections

BatQL has one typed syntax tree and several notational densities:

```text
Readable BatQL    Word-oriented source.
Compact BatQL     Accepted symbolic comparison shorthand.
Spoken BatQL      Deterministic narration.
Plan view         Normalized execution and cost explanation.
Proof view        Provenance and receipt projection.
```

These are projections of one typed form, not separate languages.

The readable source, compact source, narration, execution plan, and proof view must agree because they derive from the same canonical syntax tree.

## 1.3 Words are canonical; symbols are typing sugar

The parser accepts:

```batql
amount <= limit
```

and:

```batql
amount IS AT MOST limit
```

Both produce the same typed comparison node.

The default formatter emits:

```batql
amount IS AT MOST limit
```

Supported comparison spellings:

```text
=     IS
!=    IS NOT
<     IS LESS THAN
<=    IS AT MOST
>     IS MORE THAN
>=    IS AT LEAST
```

`AND`, `OR`, and `NOT` remain words in every density. BatQL does not encourage punctuation soup.

## 1.4 Source order is for humans; plan order is for the machine

The source reads as a question.

The planner may execute it as:

```text
resolve source
→ freeze historical cut
→ constrain subject
→ prune events
→ fold, match, or group
→ calculate outputs
→ produce explanations and proof
```

The author does not need to write the physical plan in execution order.

## 1.5 No ambient historical frame

Saved queries must contain an explicit `WHEN`.

Interactive queries may omit it. The REPL then captures:

```batql
WHEN AS OF HEAD
```

The REPL displays that decision and includes the captured source stamp in the result.

BatQL never silently mixes values from different historical cuts.

## 1.6 One evaluation, many projections

A BatQL expression is evaluated once.

That evaluation may simultaneously produce:

```text
typed value
truth or decision
availability disposition
structured explanation
evidence summary
typed threshold margin
estimated work
actual work
completeness
proof disposition
```

Explanation, cost, proof, and margin must not be produced by independently re-evaluating the expression through parallel implementations.

## 1.7 Meaning is structured; prose is projected

Authority-bearing meaning lives in typed syntax and structured result nodes.

Generated English, terminal output, HTML, diagrams, and spoken narration are projections.

Changing:

```text
is more than
```

to:

```text
exceeds
```

must not change query meaning or identity.

## 1.8 Query identity excludes presentation trivia

Query identity includes semantic structure such as:

```text
source
selector
historical cut
filters
folds
matches
grouping
ordering
bounds
result expressions
proof requirements
effect declarations
```

It excludes:

```text
comments
whitespace
keyword capitalization
formatter line wrapping
spoken wording
display labels that are explicitly non-semantic
```

---

# 2. Top-level forms

## 2.1 Pure query

```batql
ASK customer_revenue(
  cutoff: ReceiptId
) RETURNS RevenueSummary

WHO EACH event.customer_id

WHAT
  LET revenue = SUM(event.amount)
  LET payment_count = COUNT()

  RETURN {
    customer_id: event.customer_id,
    revenue: revenue,
    payment_count: payment_count
  }

WHERE
  JOURNAL "finance/payments"
  AND EVENT IS PaymentSettled

WHEN
  AS OF RECEIPT cutoff

HOW
  REQUIRE COMPLETE
  LIMIT WORK TO 100_000 EVENTS

WHY
  TRACE EVENTS, RECEIPTS
  INCLUDE COST, COMPLETENESS

END ASK
```

## 2.2 Effectful transaction

```batql
DO authorize_payment(
  invoice_id: InvoiceId
) RETURNS Receipt

WHO ENTITY Invoice(invoice_id)

WHAT
  APPEND TO JOURNAL "finance/payments"
    EVENT PaymentAuthorized {
      invoice_id: state.id,
      amount: state.total
    }
    WITH IDEMPOTENCY state.id

  RETURN RECEIPT

WHERE
  JOURNAL "finance/invoices"

WHEN
  AS OF HEAD

HOW
  FOLD USING InvoiceState

  REQUIRE COMPLETE

  REQUIRE state.status IS Approved
    ELSE DENY payment.invoice_not_approved

  REQUIRE state.total IS AT MOST USD(5_000)
    ELSE DENY payment.amount_over_limit
      WITH actual = state.total
      WITH limit = USD(5_000)
      WITH MARGIN state.total - USD(5_000)

  REQUIRE RECEIPT_VALID(state.approval_receipt)
    ELSE DEFER payment.approval_unverified

WHY
  PROVE EVENTS, REQUIREMENTS, APPEND, RECEIPT

END DO
```

## 2.3 Pure reusable function

```batql
DEFINE FUNCTION late_fee(
  amount: Money<USD>,
  days_late: Integer
) RETURNS Money<USD>

CASE
  WHEN days_late IS AT MOST 0
    THEN USD(0)

  WHEN days_late IS AT MOST 30
    THEN ROUND(amount * DECIMAL(0.001) * days_late, 2, HALF_EVEN)

  OTHERWISE
    ROUND(amount * DECIMAL(0.002) * days_late, 2, HALF_EVEN)
END CASE

END FUNCTION
```

## 2.4 Reusable query

```batql
DEFINE ASK open_invoices(
  customer_id: CustomerId,
  cutoff: ReceiptId
) RETURNS EventSet<InvoiceState>

WHO EACH event.entity_id

WHAT
  RETURN state

WHERE
  JOURNAL "finance/invoices"
  AND event.customer_id IS customer_id

WHEN
  AS OF RECEIPT cutoff

HOW
  FOLD USING InvoiceState
  WHERE state.status IS Open
  REQUIRE COMPLETE

END ASK
```

A saved query may be used as a source:

```batql
WHERE ASK open_invoices(customer_id, cutoff)
```

## 2.5 Pinned built-in kernel

```batql
DECLARE FUNCTION risk_score(
  invoice: Invoice
) RETURNS Probability

FROM KERNEL @blake3:1c482f...

PURE
DETERMINISTIC
CAPABILITIES NONE
COST AT MOST 20_000 UNITS

END FUNCTION
```

A declared kernel is never an ambient host callback or a runtime-loaded foreign guest.

Its content identity, interface version, determinism class, capability posture, input type, output type, and cost contract are sealed into the WorldImage. V1 kernels are reviewed Rust implementations already present in the host build.

Effectful external operations are invoked only inside `DO` through `REQUEST EFFECT`.

---

# 3. The six clauses

## 3.1 WHO

`WHO` identifies or partitions the subject of the question.

Canonical forms:

```batql
WHO ENTITY Invoice(invoice_id)
WHO CORRELATION correlation_id
WHO ACTOR actor_id
WHO EACH event.customer_id
WHO EACH EVENT
WHO PATH("customers", customer_id)
WHO ALL
```

### Single subject

```batql
WHO ENTITY Order(order_id)
```

This narrows the query to the canonical identity and selector associated with the entity contract.

### Causal group

```batql
WHO CORRELATION payment_correlation
```

This selects the causal conversation identified by the correlation value.

### Actor

```batql
WHO ACTOR approver_id
```

This selects actions attributed to the named actor.

### One answer per subject

```batql
WHO EACH event.customer_id
```

This establishes the output partition.

Aggregate expressions are evaluated once per distinct subject.

It is the ergonomic equivalent of:

```sql
GROUP BY customer_id
```

without forcing the author to translate “one answer for each customer” into relational jargon.

`HOW GROUP BY` remains available when the grouping key differs from the primary subject or when multiple grouping dimensions are required.

### Each event

```batql
WHO EACH EVENT
```

This produces one result candidate per selected event.

### Projection path

```batql
WHO PATH("customers", customer_id, "accounts")
```

Path segments are typed values. They are not slash-delimited filesystem text.

### Broad subject

```batql
WHO ALL
```

Broad selection must still satisfy source authority and work limits.

`WHO ALL` never silently grants a full-store scan.

---

## 3.2 WHAT

`WHAT` declares immutable local formulas, returned values, decisions, and, inside `DO`, declared effects.

```batql
WHAT
  LET gross = SUM(line.amount)
  LET discounts = SUM(line.discount)
  LET net = ROUND(gross - discounts, 2, HALF_EVEN)

  RETURN {
    gross: gross,
    discounts: discounts,
    net: net
  }
```

`LET` is immutable and lexically scoped.

A name may be bound once in its scope.

There are no mutable local variables.

### Scalar return

```batql
WHAT
  RETURN state.status
```

### Record return

```batql
WHAT
  RETURN {
    id: state.id,
    status: state.status,
    balance: state.balance
  }
```

### Timeline return

```batql
WHAT
  RETURN TIMELINE {
    at: event.hlc,
    kind: event.kind,
    actor: event.actor
  }
```

### Decision return

```batql
WHAT
  LET approval =
    DECIDE
      REQUIRE state.total IS KNOWN
        ELSE DEFER invoice.total_missing

      REQUIRE RECEIPT_VALID(state.approval_receipt)
        ELSE DEFER invoice.approval_unverified

      REQUIRE state.total IS AT MOST state.approval_limit
        ELSE DENY invoice.total_over_limit
          WITH actual = state.total
          WITH limit = state.approval_limit
          WITH MARGIN state.total - state.approval_limit

      ALLOW invoice.total_within_limit
        WITH actual = state.total
        WITH limit = state.approval_limit
        WITH MARGIN state.approval_limit - state.total
    END DECIDE

  RETURN approval
```

### Projection traversal

```batql
WHAT
  RETURN GET PATH("customers", customer_id)
```

```batql
WHAT
  RETURN CHILDREN OF PATH("customers", customer_id)
```

```batql
WHAT
  RETURN SCAN UNDER PATH("customers")
```

### WHAT inside DO

Inside `DO`, `WHAT` may contain declared effects:

```batql
WHAT
  APPEND TO JOURNAL "orders"
    EVENT OrderAccepted {
      order_id: state.id
    }

  REQUEST EFFECT SendConfirmation {
    order_id: state.id,
    recipient: state.customer_email
  }
  WITH IDEMPOTENCY state.id

  RETURN RECEIPT
```

---

## 3.3 WHERE

`WHERE` names the primary source and narrows the candidate evidence.

One primary source is allowed per query in the first stable grammar:

```batql
WHERE JOURNAL "finance/payments"
```

```batql
WHERE PROJECTION CustomerBalances
```

```batql
WHERE ASK open_invoices(customer_id, cutoff)
```

Predicates follow with `AND` or `OR`:

```batql
WHERE
  JOURNAL "finance/payments"
  AND EVENT IS PaymentSettled
  AND event.amount IS AT LEAST USD(1_000)
  AND event.currency IS USD
```

Event contracts are typed names:

```batql
EVENT IS PaymentSettled
```

String event names are confined to explicit dynamic interoperability.

### Boolean grouping

```batql
WHERE
  JOURNAL "support/tickets"
  AND EVENT IS TicketUpdated
  AND (
    event.priority IS Critical
    OR event.customer_tier IS Enterprise
  )
```

### Typed projection source

```batql
WHO PATH("customers", customer_id)

WHERE
  PROJECTION CustomerDirectory
```

Path identity, segment types, comparison, encoding, and collation belong to the projection contract.

### No expression-level FILTER duplicate

BatQL does not provide both:

```batql
WHERE amount IS MORE THAN USD(100)
```

and:

```batql
FILTER(events, amount > USD(100))
```

Set filtering belongs to `WHERE`.

Functions calculate values.

### Post-fold filtering

A condition that depends on reconstructed state belongs in `HOW` after `FOLD`:

```batql
HOW
  FOLD USING InvoiceState
  WHERE state.status IS Open
```

This preserves the distinction between:

```text
filtering source events
filtering reconstructed state
```

---

## 3.4 WHEN

`WHEN` has two separate responsibilities:

1. Freeze the coherent source cut.
2. Optionally constrain events by time.

These must never be conflated.

### Historical cut

```batql
WHEN AS OF HEAD
```

```batql
WHEN AS OF RECEIPT cutoff
```

```batql
WHEN AS OF COMMIT commit_point
```

`HEAD` is captured at query start.

The result records the exact source frontier used.

### Event-time range

```batql
WHEN
  AS OF RECEIPT cutoff
  DURING HLC start THROUGH HLC end
```

### Relative range with explicit anchor

```batql
WHEN
  AS OF RECEIPT cutoff
  DURING LAST DAYS(30) BEFORE RECEIPT cutoff
```

Relative time must be anchored.

A saved query may not contain a floating “thirty days ago” whose meaning changes each time the query is read or executed.

### Since and until

```batql
WHEN
  AS OF HEAD
  SINCE HLC start
```

```batql
WHEN
  AS OF HEAD
  UNTIL HLC end
```

### Time and order remain separate

A `WHEN` range selects events by HLC.

It does not automatically choose replay or output order.

Order belongs in `HOW`:

```batql
HOW ORDER BY COMMIT
```

```batql
HOW ORDER BY HLC
```

```batql
HOW ORDER BY CAUSAL
```

The concepts remain distinct:

```text
COMMIT    Physical journal commit order.
HLC       Causally monotone approximate chronology.
CAUSAL    Explicit predecessor or causation topology.
SEQUENCE  Store progress position.
```

`ORDER BY HLC` is not a bare HLC sort. It lowers to a total, deterministic key (`HLC, GlobalSequence` within one journal; `HLC, StoreId, GlobalSequence` across journals), and `DURING HLC start THROUGH HLC end` is half-open `[start, end)`. The complete ordering law and HLC's forbidden uses are owned by `docs/16_IDENTITY_TIME_AND_NAVIGATION.md` (DEC-061).

### No generic timestamp laundering

BatQL does not use one vague `Timestamp` type for:

```text
wall-clock observation
elapsed duration
journal sequence
historical cut
hybrid logical time
```

Each has its own type and legal operations.

---

## 3.5 HOW

`HOW` describes state reconstruction, sequence recognition, grouping, ordering, pagination, completeness, proof posture, exceptional inputs, and work limits.

### Fold immutable history into state

```batql
HOW
  FOLD USING InvoiceState
```

With an explicit partition key:

```batql
HOW
  FOLD USING AccountState
  BY event.account_id
```

`FOLD` is a first-class event operation.

State is derived from immutable history rather than treated as an unrelated table.

### Filter reconstructed state

```batql
HOW
  FOLD USING InvoiceState
  WHERE state.status IS Open
```

### Sequence matching

```batql
HOW
  MATCH
    request = EVENT PaymentRequested
    THEN approval = EVENT PaymentApproved
    THEN settlement = EVENT PaymentSettled
  BY event.correlation_id
  WITHIN DAYS(2)
  END MATCH
```

Matched bindings are available in `WHAT`:

```batql
WHAT
  RETURN {
    payment_id: request.payment_id,
    approval_delay: DURATION_BETWEEN(
      request.hlc,
      approval.hlc
    ),
    settlement_delay: DURATION_BETWEEN(
      approval.hlc,
      settlement.hlc
    )
  }
```

### Optional sequence steps

```batql
HOW
  MATCH
    request = EVENT PaymentRequested
    THEN OPTIONAL review = EVENT PaymentReviewed
    THEN approval = EVENT PaymentApproved
  BY event.correlation_id
  END MATCH
```

An absent optional step becomes `MISSING`, not `PENDING`.

### Window context

```batql
HOW
  PARTITION BY event.entity_id
  ORDER BY COMMIT
```

Then formulas may use:

```batql
PREVIOUS(event.status)
NEXT(event.status)
RUNNING_SUM(event.amount)
```

The ordering context is explicit.

`PREVIOUS`, `NEXT`, `FIRST`, and `LAST` cannot compile when their order is ambiguous.

### Grouping

```batql
HOW
  GROUP BY event.customer_id, event.currency
```

### Ordering

```batql
HOW
  ORDER BY revenue DESCENDING
```

```batql
HOW
  ORDER BY COMMIT ASCENDING
```

```batql
HOW
  ORDER BY HLC ASCENDING
```

### Take versus page

```batql
HOW
  TAKE 10
```

`TAKE` truncates the result and promises no continuation.

```batql
HOW
  PAGE 100 AFTER cursor
```

`PAGE` produces an opaque continuation cursor bound to:

```text
query identity
source identity
source generation
historical cut
projection contract
selector
path and collation contract
direction
last returned position
```

A cursor cannot be transplanted into another query, source, selector, or generation.

### Sparse projection operations

```batql
WHAT
  RETURN GET PATH("customers", customer_id)
```

```batql
WHAT
  RETURN CHILDREN OF PATH("customers", customer_id)
```

```batql
WHAT
  RETURN SCAN UNDER PATH("customers")
```

Bounds live in `HOW`:

```batql
HOW
  PAGE 100 AFTER cursor
  LIMIT RESULT TO 1_000 ITEMS
  LIMIT BYTES TO MIB(4)
```

`CHILDREN` returns values exactly one segment beneath the parent.

`SCAN UNDER` traverses the full bounded subtree.

### Completeness

```batql
HOW
  REQUIRE COMPLETE
```

```batql
HOW
  ALLOW INCOMPLETE
```

`ASK` may return incomplete data, but the disposition is always visible.

`DO` requires complete query inputs by default.

An effect cannot silently act on a projection known to omit shredded, unavailable, stale, or unresolved source material.

### Proof posture

```batql
HOW
  REQUIRE VERIFIED
```

```batql
HOW
  ALLOW UNVERIFIED
```

Transport security, successful decoding, or a cache hit cannot upgrade proof posture.

### Freshness

```batql
HOW
  REQUIRE CURRENT
```

```batql
HOW
  ALLOW STALE UP TO MINUTES(5)
```

Freshness and completeness are separate.

A result can be current but incomplete, or complete at an older frontier.

### Work limits

```batql
HOW
  LIMIT WORK TO 100_000 EVENTS
  LIMIT BYTES TO MIB(64)
  LIMIT RESULT TO 1_000 ITEMS
```

Additional bounded forms may include:

```batql
HOW
  LIMIT WORK TO 50_000 NODES
```

```batql
HOW
  LIMIT MATCHES TO 10_000
```

Broad selectors require explicit authority and bounded work.

### Exceptional input policy

Defaults:

```text
PENDING        Divert to pending output; result becomes incomplete.
INVALID        Fail the query with a typed error.
UNAVAILABLE    Record unavailable source/capability; result becomes incomplete.
SHREDDED       Record inaccessible historical payload; result becomes incomplete.
MISSING        Preserve as a first-class value.
NOT_APPLICABLE Preserve as an explicit domain disposition.
```

Explicit overrides:

```batql
HOW
  ON PENDING COLLECT AS unresolved
  ON INVALID COLLECT AS invalid_events
  ON UNAVAILABLE COLLECT AS unavailable
  ON SHREDDED COLLECT AS inaccessible
  ON NOT_APPLICABLE COLLECT AS outside_domain
```

Dropping unresolved evidence requires conspicuous syntax:

```batql
HOW
  ON PENDING DROP AND MARK INCOMPLETE
```

There is no silent “pending means false.”

### Determinism requirement

```batql
HOW
  REQUIRE DETERMINISTIC
```

A query containing a function or kernel that cannot satisfy the declared determinism contract fails before execution.

---

## 3.6 WHY

`WHY` requests structured explanation, provenance, proof, completeness details, and cost information.

It does not substitute generated prose for canonical meaning.

### Explain a value or decision

```batql
WHY
  EXPLAIN approval
```

### Trace derivation

```batql
WHY
  TRACE EVENTS, RECEIPTS, CAUSATION, FOLD, FILTERS
```

### Include evaluation projections

```batql
WHY
  INCLUDE MARGIN, EVIDENCE, COST, COMPLETENESS, PLAN
```

### Require independently checkable proof

```batql
WHY
  PROVE RESULT
```

Or:

```batql
WHY
  PROVE EVENTS, RECEIPTS, QUERY, RESULT
```

If the requested proof cannot be produced, BatQL returns an explicit proof-unavailable disposition.

It never presents an ordinary result as verified.

### Explain exclusions

```batql
WHY
  EXPLAIN EXCLUDED EVENTS
```

This may return bounded reason counts such as:

```text
4 events excluded by kind
2 events excluded by time range
1 event excluded because payload was shredded
```

It does not expose unauthorized event content merely because the caller requested an explanation.

### WHY does not rerun the query

Explanation, evidence, margin, cost, completeness, and proof are projections of the same evaluated tree and execution record.

BatQL uses three connected semantic algebras:

```text
Query algebra
Expression algebra
Decision algebra
```

Execution and explanation share those structures.

---

# 4. The value model

BatQL separates concepts that are frequently mashed into `NULL`, `false`, or exceptions.

## 4.1 Availability

```text
KNOWN          A valid typed value is present.
MISSING        No value exists in the selected state.
PENDING        Resolution or required evidence is incomplete.
INVALID        Bytes or values were present but violated their contract.
UNAVAILABLE    A required source or capability cannot currently provide the value.
SHREDDED       The event remains in history, but its payload is irrecoverable.
NOT_APPLICABLE The value is outside the operation's declared domain.
```

Examples:

```batql
state.limit IS KNOWN
state.limit IS MISSING
receipt IS PENDING
event.payload IS INVALID
event.payload IS SHREDDED
projection.proof IS UNAVAILABLE
field IS NOT APPLICABLE
```

`MISSING`, `PENDING`, `INVALID`, `UNAVAILABLE`, `SHREDDED`, and `NOT_APPLICABLE` are not aliases.

## 4.2 Truth

Predicates evaluate under strong three-valued logic:

```text
TRUE
FALSE
PENDING
```

Rules:

```text
FALSE AND anything   = FALSE
TRUE OR anything     = TRUE
TRUE AND PENDING     = PENDING
FALSE OR PENDING     = PENDING
NOT PENDING          = PENDING
```

Evaluation seeks decisive inputs before propagating uncertainty.

This preserves useful short-circuiting without pretending unresolved information is false.

## 4.3 Decision

```text
ALLOW
DENY
DEFER
```

These are ordinary typed values.

```text
ALLOW    Known facts satisfy the decision.
DENY     Known facts conclusively reject the decision.
DEFER    The decision depends on unresolved or unverifiable facts.
```

`DEFER` is not a polite spelling of `DENY`.

## 4.4 Completeness

```text
COMPLETE
INCOMPLETE
```

Completeness describes whether the result reflects every required source fact.

A result may be:

```text
fresh but incomplete
complete but unverified
verified at an older source frontier
current and complete
```

Those states remain distinguishable.

## 4.5 Proof disposition

```text
VERIFIED
LEGACY_WEAK
UNVERIFIED
PROOF_UNAVAILABLE
PROOF_INVALID
```

A query result may be valid data without carrying proof.

`LEGACY_WEAK` is a distinct state, not a synonym for `UNVERIFIED`: it marks a result whose only supporting evidence comes from a weaker legacy proof regime that the current gate would not accept as `VERIFIED`. Collapsing it into `UNVERIFIED` erases a real distinction and is a hard finding.

`REQUIRE VERIFIED` accepts only `VERIFIED`. It never accepts `LEGACY_WEAK`, `UNVERIFIED`, `PROOF_UNAVAILABLE`, or `PROOF_INVALID`.

A proof request must never silently upgrade any weaker disposition to `VERIFIED`, and no projection, transport, formatter, result envelope, or receipt may upgrade or collapse the exact recorded disposition. The five states pass through unchanged; the receipt and explanation projection of this axis is owned by `docs/14_RECEIPTS_AND_EXPLANATION.md`.

## 4.6 Freshness

```text
CURRENT
STALE
UNKNOWN_FRESHNESS
```

Freshness is evaluated relative to the requested source cut and freshness contract.

---

# 5. Expression language

## 5.1 Strict types

BatQL does not perform implicit coercion between:

```text
Money<USD>
Money<EUR>
Integer
UnsignedInteger
Decimal<scale>
Probability
Percent
Duration
WallTime
Hlc
Sequence
EventId
ReceiptId
EntityId
CorrelationId
Path
Text
Bytes
Boolean
Decision
```

This is illegal:

```batql
USD(100) IS MORE THAN EUR(90)
```

The diagnostic requests an explicit conversion operation with an explicit rate source.

This is also illegal:

```batql
Text("1") IS Integer(1)
```

Typed paths likewise preserve distinctions between signed numbers, unsigned numbers, text, and bytes.

## 5.2 Fixed-point numeric discipline

Financial quantities and declared decimals use fixed-point arithmetic.

Ordinary unqualified floating-point values are never authority values. First-class Qualified Approximation is available but quarantined from implicit authority, and it crosses into exact authority only through a receipted `Quantize` or `IntervalDecision`. The full numeric law is owned by `docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md` (DEC-069).

Probabilistic built-in kernels may return a typed `Probability`, but their arithmetic, precision, and reproducibility contract must be declared.

### Type-vocabulary record (BatQL V1)

Qualified approximation and exact authority enter BatQL through the existing `type_name` mechanism (typed parameters, schema fields, event or projection values, qualified kernel results, explicitly typed imported observations). No new type grammar is introduced. The following canonical built-in numeric type names are added additively as a BatQL V1 type-vocabulary extension — this is a recorded public type-vocabulary change, not "no public language change":

```text
new built-in type names:
    FixedDecimal
    ExactRatio
    ApproximateBinary
    Interval

format identities (used as generic arguments):
    Binary32
    Binary64

compatibility:
    additive type vocabulary only
    no existing valid expression changes meaning
    no new structural syntax; type_name already expresses e.g.
    ApproximateBinary<Binary64>, FixedDecimal<USD>, Interval<Money<USD>>

authority:
    DEC-069 plus this type-vocabulary record
```

No unqualified `FLOAT`, `F32`, `F64`, `NaN`, or `Infinity` literal forms exist. A qualified approximate value carries its format identity, classification, provenance, and interval or error evidence; it never enters as a bare numeric literal.

## 5.2a Typed arithmetic legality (DEC-060)

BatQL cannot claim strict typing while leaving operator meaning underdetermined. The legal operator matrix below is a generated projection of each operator's typed legality rules (`spec/operators.rs`, `OperatorTypingRule`); anything not derivable from those rules is a compile-time type error. Rules match in declaration order, so `Percent - Percent` is a `PercentagePoints` difference, never a bare same-unit result. `ObservedWallTime - ObservedWallTime` yields `TimeDelta`, a signed diagnostic observation difference that is never `Duration` or any ordering, budget, or deadline authority (docs/16). `Money * Percent` stays `Money` — exact by scale increase, with scale reduction through an explicit named rounding mode; a percentage of money is money, not a new one-off sort.

<!-- OPERATORS-TYPING:BEGIN generated from spec/operators.rs by bootstrap/project.py; do not edit -->
### Addition (`+`)

```text
Percent + PercentagePoints       -> Percent
PercentagePoints + Percent       -> Percent
Money<USD> + Money<USD>       -> Money<USD>
Money<USD> + Money<EUR>       -> rejected
```

### Division (`/`)

```text
Money<USD> / Money<USD>       -> Ratio
Duration / Duration           -> Ratio
Money<USD> / Money<EUR>       -> rejected
dimensional / dimensionless      -> same dimension
```

### Multiplication (`*`)

```text
Money<USD> * Decimal          -> Money<USD>
Money<USD> * Percent          -> Money<USD>
Duration * Integer            -> Duration
Money * Money                 -> rejected
```

### Subtraction (`-`)

```text
Percent - Percent             -> PercentagePoints
Percent - PercentagePoints    -> Percent
ObservedWallTime - ObservedWallTime -> TimeDelta
Money<USD> - Money<USD>       -> Money<USD>
Money<USD> - Money<EUR>       -> rejected
```

### Comparison and truth

```text
comparisons        exact same-sort pair -> Truth with TypedMargin
NOT                Truth -> Truth (K3 total)
AND / OR           Truth x Truth -> Truth (K3 total)
```
<!-- OPERATORS-TYPING:END -->

### Rounding is explicit

DEC-060 freezes six canonical rounding modes. Every scale-reducing conversion names one of them; two-argument `ROUND` is not valid, and no authority-bearing conversion has an implicit rounding mode.

```text
HalfEven
HalfAwayFromZero
TowardZero
AwayFromZero
Floor
Ceiling
```

The BatQL surface keeps its existing spellings as exact aliases of four modes; `FLOOR` and `CEILING` name the remaining two directly. A surface spelling is legal only because it lowers to exactly one canonical mode:

```text
HALF_EVEN  → HalfEven
HALF_UP    → HalfAwayFromZero
DOWN       → TowardZero
UP         → AwayFromZero
FLOOR      → Floor
CEILING    → Ceiling
```

Alias equivalence is frozen against positive and negative halfway cases, rounded to integer scale:

```text
value   HalfEven   HALF_UP/HalfAwayFromZero   DOWN/TowardZero   UP/AwayFromZero   Floor   Ceiling
+2.5       2                  3                       2                 3            2        3
-2.5      -2                 -3                      -2                -3           -3       -2
+0.5       0                  1                       0                 1            0        1
-0.5       0                 -1                       0                -1           -1        0
```

`Floor` rounds toward negative infinity; `Ceiling` rounds toward positive infinity. The concrete operator and rounding facts are owned by `spec/operators.rs` (OperatorSpec); DEC-060 is the authored decision that points to it.

```batql
ROUND(value, target_scale, HALF_EVEN)
ROUND(value, target_scale, HALF_UP)
ROUND(value, target_scale, DOWN)
ROUND(value, target_scale, UP)
ROUND(value, target_scale, FLOOR)
ROUND(value, target_scale, CEILING)
```

### Language-change record (BatQL V1)

`FLOOR` and `CEILING` did not exist as BatQL spellings before this record; they are added deliberately, not discovered as a grammar defect.

```text
new spellings:
    FLOOR
    CEILING

canonical lowerings:
    FLOOR   -> Floor
    CEILING -> Ceiling

reason:
    all six canonical authority-changing rounding modes must be explicitly nameable

compatibility:
    additive syntax only
    no existing valid expression changes meaning

authority:
    DEC-060 amendment
    plus this BatQL language-change record
```

`HALF_EVEN`, `HALF_UP`, `DOWN`, and `UP` remain compatibility surface aliases (`HALF_EVEN → HalfEven`, `HALF_UP → HalfAwayFromZero`, `DOWN → TowardZero`, `UP → AwayFromZero`). `FLOOR` and `CEILING` directly name their canonical modes `Floor` and `Ceiling` rather than acting as aliases of another spelling.

### Failure is typed, never silent

```text
overflow            → INVALID
division by zero    → INVALID
unsupported scale   → INVALID
illegal units       → compile-time type error
```

Arithmetic never wraps, saturates, truncates, or silently rounds. Currency conversion requires an explicit conversion function and a typed, receipted rate/evidence source; there is no implicit cross-currency arithmetic.

## 5.3 Literals

```batql
USD(5_000)
EUR(125.50)
DECIMAL(0.015)
PERCENT(43)
DAYS(30)
HOURS(2)
MIB(64)
PATH("customers", customer_id)
TRUE
FALSE
MISSING
```

Digit separators are allowed:

```batql
1_000_000
```

## 5.4 Parameters

Named query parameters are immutable:

```batql
ASK invoice_state(
  invoice_id: InvoiceId,
  cutoff: ReceiptId
)
```

Interactive shorthand may prefix parameters with `$`:

```batql
WHO ENTITY Invoice($invoice_id)
```

The formatter resolves this to the named parameter form when the declaration is known.

## 5.5 LET

```batql
LET gross = SUM(event.amount)
LET tax = ROUND(gross * PERCENT(6), 2, HALF_EVEN)
LET total = gross + tax
```

Bindings are immutable.

## 5.6 CASE

```batql
LET fee =
  CASE
    WHEN amount IS AT MOST USD(100)
      THEN USD(2)

    WHEN amount IS AT MOST USD(1_000)
      THEN amount * PERCENT(2)

    OTHERWISE
      amount * PERCENT(1.5)
  END CASE
```

Deeply nested spreadsheet-style:

```text
IF(IF(IF(...)))
```

may be accepted by compatibility importers, but the formatter emits `CASE`.

## 5.7 IF

Simple local branching may use:

```batql
IF(
  account.is_active,
  account.balance,
  USD(0)
)
```

Complex branching belongs in `CASE`.

## 5.8 COALESCE

```batql
COALESCE(state.limit, USD(5_000))
```

`COALESCE` handles `MISSING`.

It does not silently replace:

```text
PENDING
INVALID
SHREDDED
```

Those states require explicit handling.

## 5.9 TRY

A query may explicitly convert `INVALID` into a handled value:

```batql
TRY PARSE_INTEGER(event.legacy_code)
  ON INVALID RETURN MISSING
END TRY
```

This is deliberate and visible.

A broad catch-all exception mechanism is not part of BatQL.

## 5.10 Core scalar functions

```text
ABS
ROUND
CLAMP
MIN
MAX
COALESCE
LENGTH
LOWER
UPPER
STARTS_WITH
ENDS_WITH
CONTAINS
DATE_ADD
DATE_DIFF
DURATION_BETWEEN
PERCENT
```

String operations are exact and locale-independent unless a declared contract says otherwise.

## 5.11 Aggregate functions

```text
COUNT
SUM
AVERAGE
MIN
MAX
FIRST
LAST
UNIQUE
```

Aggregates preserve completeness and evidence disposition.

`SUM` over an incomplete input set produces an incomplete result rather than laundering missing rows into an exact total.

`COUNT()` counts selected records.

`COUNT(value)` counts `KNOWN` values unless an explicit availability variant is requested.

## 5.12 Event-native functions

```text
PREVIOUS
NEXT
LATEST
FIRST_AFTER
LAST_BEFORE
COUNT_SINCE
DURATION_BETWEEN
CHANGED
TRANSITIONED
CORRELATES_WITH
DESCENDS_FROM
ANCESTORS
TIMELINE
```

Examples:

```batql
PREVIOUS(event.status)
```

```batql
CHANGED(event.status)
```

```batql
TRANSITIONED(
  event.status,
  FROM Pending,
  TO Approved
)
```

```batql
DESCENDS_FROM(event.id, root_event_id)
```

## 5.13 Evidence functions

```text
RECEIPT_VALID
EVIDENCE
PROVENANCE
FRESHNESS
COMPLETENESS
PROOF_DISPOSITION
SOURCE_FRONTIER
COST
EXPLAIN
```

Evidence functions return structured typed values, not authority-bearing English strings.

## 5.14 Availability functions

```text
IS_KNOWN
IS_MISSING
IS_PENDING
IS_INVALID
IS_UNAVAILABLE
IS_SHREDDED
IS_NOT_APPLICABLE
```

The readable grammar prefers:

```batql
value IS KNOWN
value IS MISSING
```

The function forms may exist for generated code and interop.

## 5.15 No expression-level side effects

Functions inside `ASK` and formulas are pure.

This is forbidden:

```batql
LET result = SEND_EMAIL(customer.email)
```

Effects belong in `DO`:

```batql
REQUEST EFFECT SendEmail {
  recipient: customer.email
}
```

---

# 6. Decision calculus

BatQL uses `DECIDE` when a query needs a structured `ALLOW`, `DENY`, or `DEFER` value.

```batql
LET eligibility =
  DECIDE
    REQUIRE state.income IS KNOWN
      ELSE DEFER mortgage.income_missing

    REQUIRE state.debts IS KNOWN
      ELSE DEFER mortgage.debts_missing

    LET dti = PERCENT(state.debts / state.income)

    REQUIRE dti IS AT MOST PERCENT(43)
      ELSE DENY mortgage.dti_over_limit
        WITH actual = dti
        WITH limit = PERCENT(43)
        WITH MARGIN dti - PERCENT(43)

    ALLOW mortgage.dti_within_limit
      WITH actual = dti
      WITH limit = PERCENT(43)
      WITH MARGIN PERCENT(43) - dti
  END DECIDE
```

## 6.1 REQUIRE semantics

For:

```batql
REQUIRE predicate
  ELSE DENY reason
```

the outcomes are:

```text
predicate TRUE       Continue.
predicate FALSE      Produce the declared DENY.
predicate PENDING    Produce DEFER, never DENY.
predicate INVALID    Surface a typed invalid-evaluation error.
```

A custom pending reason may be declared:

```batql
REQUIRE RECEIPT_VALID(receipt)
  ELSE DENY approval.receipt_invalid
  ON PENDING DEFER approval.receipt_unresolved
```

## 6.2 Stable explanation codes

```batql
DENY mortgage.dti_over_limit
```

`mortgage.dti_over_limit` is a stable explanation identity.

Its structured arguments are canonical:

```batql
WITH actual = dti
WITH limit = PERCENT(43)
WITH MARGIN dti - PERCENT(43)
```

Rendered English may be:

```text
The debt-to-income ratio is 47 percent.
The allowed limit is 43 percent.
The result is 4 percentage points over the limit.
```

Changing “over the limit” to “above the limit” does not change query identity.

## 6.3 Positive and negative explanations share one tree

Successful and unsuccessful explanations derive from the same comparison and requirement structure.

BatQL uses:

```text
closed core grammar
+ registered typed domain vocabulary
+ stable explanation codes
+ deterministic spoken templates
```

The grammar is closed.

The domain dictionary is extensible and typed.

## 6.4 Structured explanation tree

A decision explanation may contain:

```text
reason code
polarity
actual value
expected value
typed margin
missing evidence
source references
child explanations
```

The result does not store one authority-bearing prose string.

## 6.5 Margin is typed

A margin preserves its unit:

```text
Money<USD>
PercentagePoints
Duration
Bytes
Count
```

This is invalid:

```text
money margin + duration margin
```

Margins support:

```text
boundary testing
ranking near-threshold cases
human escalation
adversarial fixture generation
policy-drift analysis
```

## 6.6 Decision composition

Decisions may be combined explicitly:

```batql
ALL_OF(
  identity_decision,
  income_decision,
  collateral_decision
)
```

```batql
ANY_OF(
  primary_approval,
  exception_approval
)
```

Decision composition follows:

```text
ALL_OF:
  any DENY  → DENY
  no DENY and any DEFER → DEFER
  all ALLOW → ALLOW

ANY_OF:
  any ALLOW → ALLOW
  no ALLOW and any DEFER → DEFER
  all DENY → DENY
```

The resulting explanation identifies the decisive branches.

---

# 7. Event-query patterns

## 7.1 Raw event query

```batql
ASK large_approved_payments(
  cutoff: ReceiptId
) RETURNS EventSet<PaymentApproved>

WHO EACH EVENT

WHAT
  RETURN {
    event_id: event.id,
    at: event.hlc,
    actor: event.actor,
    amount: event.amount
  }

WHERE
  JOURNAL "finance/payments"
  AND EVENT IS PaymentApproved
  AND event.amount IS AT LEAST USD(5_000)

WHEN
  AS OF RECEIPT cutoff

HOW
  ORDER BY COMMIT
  PAGE 100
  REQUIRE COMPLETE

WHY
  INCLUDE COMPLETENESS

END ASK
```

## 7.2 Reconstructed state

```batql
ASK invoice_state(
  invoice_id: InvoiceId,
  cutoff: ReceiptId
) RETURNS InvoiceState

WHO ENTITY Invoice(invoice_id)

WHAT
  RETURN state

WHERE
  JOURNAL "finance/invoices"

WHEN
  AS OF RECEIPT cutoff

HOW
  FOLD USING InvoiceState
  REQUIRE COMPLETE

WHY
  TRACE EVENTS, FOLD, RECEIPTS

END ASK
```

## 7.3 Transition history

```batql
ASK order_status_changes(
  order_id: OrderId,
  cutoff: ReceiptId
) RETURNS EventSet<StatusChange>

WHO ENTITY Order(order_id)

WHAT
  LET prior = PREVIOUS(event.status)

  RETURN {
    at: event.hlc,
    from: prior,
    to: event.status
  }

WHERE
  JOURNAL "sales/orders"
  AND EVENT IS OrderStatusChanged

WHEN
  AS OF RECEIPT cutoff

HOW
  PARTITION BY event.entity_id
  ORDER BY COMMIT

WHY
  TRACE EVENTS, CAUSATION

END ASK
```

## 7.4 Causal sequence

```batql
ASK payment_lifecycle(
  cutoff: ReceiptId
) RETURNS EventSet<PaymentLifecycle>

WHO EACH event.correlation_id

WHAT
  RETURN {
    correlation_id: event.correlation_id,
    requested_at: request.hlc,
    approved_at: approval.hlc,
    settled_at: settlement.hlc,
    total_duration: DURATION_BETWEEN(
      request.hlc,
      settlement.hlc
    )
  }

WHERE
  JOURNAL "finance/payments"

WHEN
  AS OF RECEIPT cutoff

HOW
  MATCH
    request = EVENT PaymentRequested
    THEN approval = EVENT PaymentApproved
    THEN settlement = EVENT PaymentSettled
  BY event.correlation_id
  WITHIN DAYS(2)
  END MATCH

WHY
  TRACE EVENTS, CAUSATION, RECEIPTS

END ASK
```

## 7.5 Sparse projection query

```batql
ASK customer_accounts(
  customer_id: CustomerId,
  cursor: Optional<Cursor>
) RETURNS Page<AccountSummary>

WHO PATH("customers", customer_id, "accounts")

WHAT
  RETURN CHILDREN OF WHO

WHERE
  PROJECTION CustomerDirectory

WHEN
  AS OF HEAD

HOW
  PAGE 100 AFTER cursor
  REQUIRE COMPLETE
  LIMIT BYTES TO MIB(2)

WHY
  INCLUDE SOURCE_FRONTIER, COMPLETENESS

END ASK
```

## 7.6 Explainable eligibility query

```batql
ASK mortgage_eligibility(
  borrower_id: BorrowerId,
  cutoff: ReceiptId
) RETURNS EligibilityResult

WHO ENTITY Borrower(borrower_id)

WHAT
  LET income = state.verified_monthly_income
  LET debts = state.monthly_debt_payments
  LET dti = PERCENT(debts / income)

  LET eligibility =
    DECIDE
      REQUIRE income IS KNOWN
        ELSE DEFER mortgage.income_missing

      REQUIRE debts IS KNOWN
        ELSE DEFER mortgage.debts_missing

      REQUIRE dti IS AT MOST PERCENT(43)
        ELSE DENY mortgage.dti_over_limit
          WITH actual = dti
          WITH limit = PERCENT(43)
          WITH MARGIN dti - PERCENT(43)

      ALLOW mortgage.dti_within_limit
        WITH actual = dti
        WITH limit = PERCENT(43)
        WITH MARGIN PERCENT(43) - dti
    END DECIDE

  RETURN {
    borrower_id: borrower_id,
    income: income,
    debts: debts,
    dti: dti,
    eligibility: eligibility
  }

WHERE
  JOURNAL "mortgage/underwriting"

WHEN
  AS OF RECEIPT cutoff

HOW
  FOLD USING BorrowerUnderwritingState
  REQUIRE COMPLETE

WHY
  EXPLAIN eligibility
  TRACE EVENTS, RECEIPTS, FOLD
  INCLUDE MARGIN, EVIDENCE, COMPLETENESS

END ASK
```

## 7.7 Region materialization

```batql
ASK active_device_fleet(
  tenant_id: TenantId,
  cutoff: ReceiptId,
  cursor: Optional<Cursor>
) RETURNS Page<DeviceSummary>

WHO EACH event.device_id

WHAT
  RETURN {
    device_id: state.id,
    status: state.status,
    last_seen: state.last_seen
  }

WHERE
  JOURNAL "devices/events"
  AND event.tenant_id IS tenant_id

WHEN
  AS OF RECEIPT cutoff

HOW
  FOLD USING DeviceState
  BY event.device_id

  WHERE state.status IS Active

  ORDER BY state.last_seen DESCENDING
  PAGE 250 AFTER cursor

  REQUIRE COMPLETE
  LIMIT WORK TO 500_000 EVENTS
  LIMIT RESULT TO 10_000 ITEMS

WHY
  INCLUDE SOURCE_FRONTIER, COMPLETENESS, COST

END ASK
```

---

# 8. Transactions

## 8.1 Query first, effect second

Every `DO` has two conceptual phases:

```text
read and calculate
→ satisfy requirements
→ reserve effect budget
→ perform declared effects
→ persist receipts
```

No effect occurs while a requirement is unresolved.

## 8.2 Append

```batql
APPEND TO JOURNAL "finance/payments"
  EVENT PaymentAuthorized {
    invoice_id: state.id,
    amount: state.total
  }
  WITH IDEMPOTENCY state.id
```

## 8.3 Atomic batch

```batql
APPEND ATOMIC TO JOURNAL "orders"
  EVENT OrderAccepted {
    order_id: state.id
  }

  EVENT InventoryReserved {
    order_id: state.id,
    items: state.items
  }
END BATCH
```

The batch returns ordered item receipts plus one batch disposition.

Failure before commit produces no partial batch.

## 8.4 External effect

```batql
REQUEST EFFECT CapturePayment {
  payment_id: state.payment_id,
  amount: state.total
}
WITH IDEMPOTENCY state.payment_id
```

Effects must be declared in the containing package contract.

## 8.5 Effect result

```batql
LET capture =
  REQUEST EFFECT CapturePayment {
    payment_id: state.payment_id,
    amount: state.total
  }
  WITH IDEMPOTENCY state.payment_id
```

The effect result is a typed receipt-backed value.

It is not a raw network response.

## 8.6 DO always returns a receipt

Even when the author omits the line, the canonical formatter inserts:

```batql
RETURN RECEIPT
```

A denied or deferred `DO` returns a typed non-execution receipt proving that no effect occurred.

## 8.7 Dropped observation is not cancellation

Once a `DO` has admitted an effect, abandoning the client request does not retroactively erase or cancel it.

Recovery uses the effect’s idempotency and receipt identity.

## 8.8 Cancellation must be explicit

A cancellable pre-admission operation must declare that fact:

```batql
REQUEST CANCELLABLE EFFECT GeneratePreview {
  document_id: state.id
}
```

Cancellation after durable admission is a separate effect with its own receipt.

## 8.9 External effects do not masquerade as atomic journal writes

BatQL may atomically append an effect-request event with domain events.

It cannot claim that a remote service call committed atomically with the local journal unless a stronger declared protocol actually proves that claim.

---

# 9. Result envelopes

## 9.1 ASK result envelope

Every `ASK` result carries:

```text
value
query identity
source identity
source generation
historical cut
reflected frontier
completeness
availability disposition
proof disposition
freshness
estimated work
actual work
next cursor, when paged
```

The ordinary CLI may display only the value and one status line.

The metadata still exists.

Example compact display:

```text
customer_id: C-102
revenue: USD 18,450.00
payments: 14

complete · verified · as of receipt 01J...
```

With `WHY TRACE`:

```text
14 PaymentSettled events
12 source segments
frontier 8,913,021
query hash b3:7fd2...
result hash b3:08aa...
proof verified
actual work 41,280 units
```

## 9.2 DO result envelope

Every `DO` result carries:

```text
transaction identity
query identity
historical cut
requirements and dispositions
admission disposition
effect attempts
effect receipts
appended event receipts
completeness
proof disposition
actual work
terminal disposition
```

Possible terminal dispositions include:

```text
COMPLETED
DENIED
DEFERRED
FAILED
PARTIALLY_OBSERVED
```

`PARTIALLY_OBSERVED` does not imply a partial journal commit. It means the external outcome could not be fully observed and must be reconciled through receipt and idempotency authority.

## 9.3 Pending side outputs

When requested:

```batql
HOW
  ON PENDING COLLECT AS unresolved
```

the result envelope contains a named side output:

```text
result.value
result.unresolved
```

The primary result remains explicitly incomplete.

---

# 10. Spoken BatQL

`batpak batql speak` narrates the typed query.

It does not ask a language model to summarize the source.

For:

```batql
WHO EACH event.customer_id

WHAT
  LET revenue = SUM(event.amount)
  RETURN event.customer_id, revenue

WHERE
  JOURNAL "sales/orders"
  AND EVENT IS OrderPaid

WHEN
  AS OF HEAD

HOW
  TAKE 10
  ORDER BY revenue DESCENDING

WHY
  TRACE RECEIPTS
```

the deterministic narration is:

> For each customer ID, calculate revenue as the sum of event amount. Return the customer ID and revenue. Read Order Paid events from the sales orders journal, as of the captured head. Order the results by revenue from highest to lowest and take ten. Include the receipts that support the answer.

Every function and syntax node has one registered spoken projection.

Example catalog entry:

```text
name: SUM
spoken: “the sum of {value}”
kind: aggregate
purity: pure
determinism: deterministic
```

The same catalog powers:

```text
type checking
autocomplete
documentation
formatting
spoken narration
plan construction
cost estimation
explanation
```

## 10.1 Spoken decision

For:

```batql
REQUIRE dti IS AT MOST PERCENT(43)
  ELSE DENY mortgage.dti_over_limit
```

the spoken projection may be:

> Require the debt-to-income ratio to be at most forty-three percent. Otherwise deny with reason mortgage dot DTI over limit.

The human renderer may replace the stable code with a registered display label, but the stable code remains available.

## 10.2 Spoken historical cut

```batql
WHEN AS OF RECEIPT cutoff
```

narrates as:

> Observe the journal as of receipt cutoff.

## 10.3 Spoken work bounds

```batql
HOW LIMIT WORK TO 100_000 EVENTS
```

narrates as:

> Examine at most one hundred thousand events.

## 10.4 No second spoken grammar

Speech input is parsed into ordinary BatQL.

Spoken output is projected from ordinary BatQL.

There is no separate “voice BatQL” with different semantics.

---

# 11. Formatting and lexical rules

## 11.1 File extension

```text
.batql
```

## 11.2 Keywords

Keywords are case-insensitive.

The formatter emits uppercase keywords.

```batql
who each event.customer_id
```

formats to:

```batql
WHO EACH event.customer_id
```

## 11.3 Identifiers

User identifiers use:

```text
snake_case
```

Type and contract identifiers use:

```text
PascalCase
```

Stable reason codes use:

```text
namespace.dotted_identifier
```

Examples:

```batql
customer_id
InvoiceState
PaymentAuthorized
mortgage.dti_over_limit
```

## 11.4 Strings

Strings use double quotes:

```batql
JOURNAL "finance/payments"
```

String escaping is explicit and deterministic.

## 11.5 Comments

```batql
# This query returns one result per customer.
```

Comments are not part of query identity.

## 11.6 Statement termination

Semicolons are not required.

Newlines separate statements inside clause blocks.

Commas remain required inside:

```text
function arguments
record literals
multi-value return lists
proof item lists
```

when ambiguity would otherwise arise.

## 11.7 Named endings

Nested and top-level blocks use named endings:

```text
END ASK
END DO
END FUNCTION
END CASE
END DECIDE
END MATCH
END BATCH
END TRY
```

The formatter never emits an ambiguous bare `END`.

## 11.8 Clause order

Input:

```batql
WHEN AS OF HEAD
WHERE JOURNAL "orders"
WHAT RETURN COUNT()
WHO ALL
```

Formatted:

```batql
WHO ALL

WHAT
  RETURN COUNT()

WHERE
  JOURNAL "orders"

WHEN
  AS OF HEAD
```

## 11.9 One-line REPL form

The multiline grammar may be written on one line:

```batql
WHO ENTITY Order($id) WHAT RETURN state.status WHERE JOURNAL "orders" WHEN AS OF HEAD HOW FOLD USING OrderState WHY TRACE RECEIPTS
```

There is no separate pipe language in the first stable grammar.

## 11.10 Canonical formatting

Canonical formatting determines:

```text
clause order
keyword capitalization
indentation
line breaks
comparison wording
named endings
record field layout
proof item ordering where order is not semantic
```

Formatting does not change semantic identity.

---

# 12. Diagnostics

BatQL diagnostics use the six-question model.

## 12.1 Missing historical frame

```text
BATQL E-WHEN-001

I know WHO, WHAT, and WHERE.
WHEN is missing.

Saved queries must name a coherent historical cut.

Add one of:
  WHEN AS OF HEAD
  WHEN AS OF RECEIPT cutoff
  WHEN AS OF COMMIT commit_point
```

## 12.2 Missing source

```text
BATQL E-WHERE-001

WHAT is known:
  SUM(event.amount)

WHERE is missing:
  no journal, projection, or saved query supplies event.amount.
```

## 12.3 Unbounded scan

```text
BATQL E-HOW-014

This query can scan the entire projection.

WHO:
  ALL

WHERE:
  PROJECTION CustomerDirectory

Add a bound:
  HOW PAGE 100
  HOW LIMIT RESULT TO 1_000 ITEMS
  HOW LIMIT WORK TO 100_000 NODES
```

## 12.4 Ambiguous previous value

```text
BATQL E-HOW-021

PREVIOUS(event.status) needs a deterministic order.

Choose:
  HOW ORDER BY COMMIT
  HOW ORDER BY HLC
  HOW ORDER BY CAUSAL
```

## 12.5 Unit mismatch

```text
BATQL E-TYPE-008

Money<USD> cannot be compared with Money<EUR>.

Left:
  invoice.total: Money<USD>

Right:
  limit: Money<EUR>

Convert one side through an explicit, receipted exchange-rate source.
```

## 12.6 Effect inside ASK

```text
BATQL E-WHAT-031

ASK is pure and cannot APPEND.

Move the query into DO when source truth should change.
```

## 12.7 Incomplete effect input

```text
BATQL E-HOW-044

DO cannot execute from an incomplete projection.

Missing source:
  2 shredded events
  1 pending receipt verification

The effect was not attempted.
```

## 12.8 Cursor transplant

```text
BATQL E-HOW-052

This cursor belongs to another query or source generation.

Cursor query:
  b3:91ab...

Current query:
  b3:7fd2...

The query was not continued.
```

## 12.9 Pending predicate in filter

```text
BATQL E-WHERE-018

This filter can evaluate to PENDING.

BatQL will not silently treat PENDING as FALSE.

Choose a disposition:
  HOW ON PENDING COLLECT AS unresolved
  HOW ON PENDING DROP AND MARK INCOMPLETE
```

## 12.10 Invalid kernel declaration

```text
BATQL E-FUNCTION-023

Kernel risk_score is not fully pinned.

Missing:
  kernel digest
  interface version
  determinism declaration
```

## 12.11 Query status matrix

`batql check` may display:

```text
WHO     resolved: ENTITY Invoice(invoice_id)
WHAT    resolved: InvoiceStatus
WHERE   resolved: journal finance/invoices
WHEN    resolved: receipt cutoff
HOW     resolved: fold InvoiceState, complete required
WHY     optional: not requested

Query is well-formed.
```

## 12.12 DO admission matrix

```text
WHO       resolved
WHAT      1 append, 1 returned receipt
WHERE     journal finance/invoices
WHEN      captured head
HOW       complete required, verified required
WHY       proof requested

Effects:
  append finance/payments
  idempotency source: state.id

Transaction is admissible.
```

---

# 13. Command-line surface

```text
batpak batql check     Parse, type-check, validate bounds, and inspect authority.
batpak batql format    Emit canonical readable source.
batpak batql compact   Emit compact comparison syntax.
batpak batql speak     Narrate the typed query deterministically.
batpak batql plan      Show selectors, folds, bounds, and estimated work.
batpak batql explain   Show structured decision and formula derivation.
batpak query           Execute an ASK ProgramImage.
batpak run             Invoke a declared WorldImage entrypoint; its program may be ASK or DO.
batpak verify          Verify a result or transaction receipt.
batpak repl            Interactive authoring/query shell.
```

`DO` is a BatQL source-language mode, never a competing top-level product
command (5.5E1 ruling): effectful programs enter through declared WorldImage
entrypoints via `batpak run` (DEC-049). The product command inventory is
owned by `26_COMMAND_PLANE.md`; this section shows the BatQL-facing subset.

The `batql` package supplies the compiler library. `batpak-cli` projects that library through one product binary. All views consume the same typed query.

## 13.1 Check

```text
batpak batql check queries/revenue.batql
```

Checks:

```text
syntax
types
availability handling
historical cut
work bounds
effect purity
kernel pinning
cursor rules
proof requirements
```

## 13.2 Plan

```text
batpak batql plan queries/revenue.batql
```

Displays:

```text
source selectors
historical cut
event kinds
folds and matches
index or tile requirements
estimated events and bytes
proof work
fallback paths
```

## 13.3 Speak

```text
batpak batql speak queries/revenue.batql
```

Produces deterministic narration.

## 13.4 Explain

```text
batpak batql explain queries/eligibility.batql
```

Displays the structured decision tree without executing effects.

## 13.5 Prove

```text
batpak verify result.receipt
```

Verifies:

```text
query identity
source stamps
result hash
event references
receipt references
proof disposition
```

---

# 14. Semantic structure

BatQL compiles into three connected closed algebras.

## 14.1 Query algebra

```text
Source
Subject
Snapshot
TimeRange
Filter
Fold
Match
Partition
Group
Order
Page
Project
ProofRequest
EffectPlan
```

## 14.2 Expression algebra

```text
Literal
Binding
Field
Record
Call
Compare
Logic
Case
Aggregate
Window
AvailabilityTest
```

## 14.3 Decision algebra

```text
Require
Allow
Deny
Defer
Reason
Evidence
Margin
DecisionComposition
```

## 14.4 Node admission rule

Adding a semantic node requires:

```text
parser support
type rule
availability rule
evaluation rule
formatter rule
spoken projection
cost rule
explanation rule
fuzz grammar
canonical identity rule
```

A node without the complete set does not enter the language.

## 14.5 One normalized typed tree

Surface aliases normalize before identity and planning.

For example:

```batql
amount <= limit
```

and:

```batql
amount IS AT MOST limit
```

normalize into one comparison node.

## 14.6 No arbitrary evaluator plugins

A plugin cannot introduce a secret syntax-tree node or bypass type checking.

Extensions enter through declared functions, query contracts, event contracts, projection contracts, and effect contracts.

---

# 15. Illustrative grammar

```text
program
  := declaration*

declaration
  := ask
   | do
   | ask_definition
   | function_definition
   | function_declaration

ask
  := ASK identifier? parameters? returns?
     clause+
     END ASK

do
  := DO identifier parameters? returns?
     clause+
     END DO

ask_definition
  := DEFINE ASK identifier parameters? returns?
     clause+
     END ASK

function_definition
  := DEFINE FUNCTION identifier parameters returns
     expression
     END FUNCTION

function_declaration
  := DECLARE FUNCTION identifier parameters returns
     FROM KERNEL kernel_ref
     purity_clause
     determinism_clause
     capability_clause
     cost_clause
     END FUNCTION

clause
  := who_clause
   | what_clause
   | where_clause
   | when_clause
   | how_clause
   | why_clause

who_clause
  := WHO subject

subject
  := ENTITY type_name "(" expression ")"
   | CORRELATION expression
   | ACTOR expression
   | EACH expression
   | EACH EVENT
   | PATH "(" arguments ")"
   | ALL

what_clause
  := WHAT what_statement+

what_statement
  := let_statement
   | return_statement
   | append_statement
   | effect_statement

let_statement
  := LET identifier "=" expression

return_statement
  := RETURN return_list
   | RETURN RECEIPT
   | RETURN record
   | RETURN TIMELINE record
   | RETURN GET where_value
   | RETURN CHILDREN OF where_value
   | RETURN SCAN UNDER where_value

return_list
  := expression ("," expression)*

where_value
  := expression
   | WHO

where_clause
  := WHERE source predicate_tail*

source
  := JOURNAL string
   | PROJECTION identifier
   | ASK identifier "(" arguments? ")"

predicate_tail
  := AND expression
   | OR expression

when_clause
  := WHEN time_statement+

time_statement
  := AS OF HEAD
   | AS OF RECEIPT expression
   | AS OF COMMIT expression
   | DURING HLC expression THROUGH HLC expression
   | DURING LAST duration BEFORE temporal_anchor
   | SINCE HLC expression
   | UNTIL HLC expression

how_clause
  := HOW how_statement+

how_statement
  := FOLD USING type_name fold_partition?
   | WHERE expression
   | MATCH match_body END MATCH
   | PARTITION BY expression_list
   | GROUP BY expression_list
   | ORDER BY order_key direction?
   | PAGE integer cursor_after?
   | TAKE integer
   | REQUIRE COMPLETE
   | ALLOW INCOMPLETE
   | REQUIRE VERIFIED
   | ALLOW UNVERIFIED
   | REQUIRE CURRENT
   | ALLOW STALE UP TO duration
   | REQUIRE DETERMINISTIC
   | LIMIT WORK TO integer work_unit
   | LIMIT BYTES TO expression
   | LIMIT RESULT TO integer ITEMS
   | LIMIT MATCHES TO integer
   | ON disposition disposition_action

match_body
  := match_step (THEN match_step)+ BY expression within_clause?

match_step
  := OPTIONAL? identifier "=" EVENT type_name

why_clause
  := WHY why_statement+

why_statement
  := EXPLAIN expression
   | EXPLAIN EXCLUDED EVENTS
   | TRACE proof_item_list
   | INCLUDE projection_item_list
   | PROVE proof_item_list

expression
  := literal
   | identifier
   | parameter_ref
   | field_access
   | function_call
   | arithmetic_expression
   | comparison
   | logical_expression
   | case_expression
   | decide_expression
   | try_expression
   | decision_composition
   | rounding_mode
   | record
   | availability_test

comparison
  := expression comparison_operator expression

arithmetic_expression
  := expression arithmetic_operator expression

logical_expression
  := NOT expression
   | expression logical_operator expression

availability_test
  := expression IS KNOWN
   | expression IS MISSING
   | expression IS PENDING
   | expression IS INVALID
   | expression IS UNAVAILABLE
   | expression IS SHREDDED
   | expression IS NOT APPLICABLE

case_expression
  := CASE (WHEN expression THEN expression)+ OTHERWISE expression END CASE

decide_expression
  := DECIDE decide_step+ decision_terminal END DECIDE

decide_step
  := REQUIRE expression (ELSE decision_terminal)? on_pending?
   | LET identifier "=" expression

decision_terminal
  := ALLOW reason with_arg*
   | DENY reason with_arg*
   | DEFER reason with_arg*

on_pending
  := ON PENDING DEFER reason

with_arg
  := WITH identifier "=" expression
   | WITH MARGIN expression

try_expression
  := TRY expression ON INVALID RETURN expression END TRY

decision_composition
  := ALL_OF "(" arguments ")"
   | ANY_OF "(" arguments ")"

append_statement
  := APPEND TO JOURNAL string event_literal with_idempotency?
   | APPEND ATOMIC TO JOURNAL string event_literal+ END BATCH

event_literal
  := EVENT type_name record

effect_statement
  := REQUEST EFFECT type_name record with_idempotency?
   | REQUEST CANCELLABLE EFFECT type_name record with_idempotency?

with_idempotency
  := WITH IDEMPOTENCY expression

rounding_mode
  := HALF_EVEN | HALF_UP | DOWN | UP | FLOOR | CEILING
```

The four function/declaration/MATCH structural forms and every lexical and metavariable leaf are defined below in 15.2. The operator productions (`comparison_operator`, `arithmetic_operator`, `logical_operator`) are generated from `spec/operators.rs` in 15.3; they are the one operator authority. `rounding_mode` lists the surface spellings that lower to the six canonical rounding modes frozen by DEC-060.

The complete grammar must remain parseable without semantic backtracking.

## 15.1 Conformance obligations

One grammar-conformance check parses every normative BatQL code fence (```batql) in this document and in `docs/13_BATQL_CONTRACT.md`; each must parse under the frozen grammar. Two distinct changes are recorded here, and they are not the same kind. The closure of `function_definition`, `function_declaration`, MATCH, and the lexical and metavariable leaves is **grammar repair**: those forms are already used by normative examples and gained no new meaning. The addition of the `FLOOR` and `CEILING` rounding-mode spellings is an **explicitly authorized additive BatQL V1 language extension** (see the language-change record in 5.2a) — two new keywords, not repair, and not a claim that no syntax changed.

Named hostile fixtures (proof owner: batql/TestPak; gate G4):

```text
money_with_different_currencies_is_rejected
money_times_money_is_rejected
division_by_zero_is_invalid
implicit_rounding_is_rejected
scale_reduction_requires_rounding_mode
every_normative_batql_example_matches_the_grammar
every_referenced_grammar_nonterminal_is_defined
match_requires_initial_step_plus_one_then
kernel_declaration_clauses_are_canonically_ordered
require_verified_accepts_only_verified
```

The grammar is closed: every nonterminal referenced in 15, 15.2, and 15.3 has exactly one owning production, so a normative fence can be derived down to lexical leaves. Informal terminals are written between angle brackets; literal terminals are quoted; keywords are uppercase and case-insensitive.

## 15.2 Lexical and metavariable leaves

```text
identifier
  := letter (letter | digit | "_")*

letter
  := <one ASCII letter A through Z or a through z>

digit
  := <one ASCII digit 0 through 9>

integer
  := digit ("_"? digit)*

decimal
  := integer "." integer

boolean_literal
  := TRUE | FALSE

string
  := "\"" string_char* "\""

string_char
  := <any Unicode scalar value except an unescaped double quote or backslash, or a defined backslash escape>

literal
  := integer | decimal | string | boolean_literal | MISSING

type_name
  := identifier ("<" type_name ("," type_name)* ">")?

parameters
  := "(" (parameter ("," parameter)*)? ")"

parameter
  := identifier ":" type_name

returns
  := RETURNS type_name

arguments
  := expression ("," expression)*

record
  := "{" (field_init ("," field_init)*)? "}"

field_init
  := identifier ":" expression

parameter_ref
  := "$" identifier

field_access
  := identifier ("." identifier)+

function_call
  := identifier "(" arguments? ")"

within_clause
  := WITHIN duration

duration
  := duration_unit "(" integer ")"

duration_unit
  := DAYS | HOURS | MINUTES | SECONDS | WEEKS | MILLISECONDS

temporal_anchor
  := HEAD
   | RECEIPT expression
   | COMMIT expression

expression_list
  := expression ("," expression)*

order_key
  := COMMIT | HLC | CAUSAL | SEQUENCE | expression

direction
  := ASCENDING | DESCENDING

cursor_after
  := AFTER expression

fold_partition
  := BY expression

work_unit
  := EVENTS | NODES | UNITS

disposition
  := PENDING | INVALID | UNAVAILABLE | SHREDDED | MISSING | NOT_APPLICABLE

disposition_action
  := COLLECT AS identifier
   | DROP AND MARK INCOMPLETE

proof_item_list
  := proof_item ("," proof_item)*

proof_item
  := EVENTS | RECEIPTS | CAUSATION | FOLD | FILTERS | QUERY | RESULT

projection_item_list
  := projection_item ("," projection_item)*

projection_item
  := MARGIN | EVIDENCE | COST | COMPLETENESS | PLAN

reason
  := identifier ("." identifier)*

kernel_ref
  := "@" identifier ":" hex_digest

hex_digest
  := <lowercase hexadecimal content-hash digits>

purity_clause
  := PURE

determinism_clause
  := DETERMINISTIC

capability_clause
  := CAPABILITIES capability_spec

capability_spec
  := NONE | identifier ("," identifier)*

cost_clause
  := COST AT MOST integer UNITS
```

## 15.3 Generated operator projections

These blocks are generated from `spec/operators.rs` by `bootstrap/project.py` and independently re-checked by `bootstrap/audit.py`. OperatorSpec is the one operator authority; it owns operator relationships and these projected tables and fragments only, never the function, MATCH, declaration, or general lexical grammar. Do not hand-edit the generated content.

<!-- OPERATORS-CATALOG:BEGIN generated from spec/operators.rs by bootstrap/project.py; do not edit -->
| OperatorId | Class | Word | Symbol | Arity | Fixity | Precedence | Associativity | Result sort | Exactness |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| OP-ADD | Arithmetic | `+` |  | Binary | Infix | 60 | Left | exact same-unit | Exact |
| OP-DIV | Arithmetic | `/` |  | Binary | Infix | 70 | Left | Ratio or scaled exact | Exact |
| OP-MUL | Arithmetic | `*` |  | Binary | Infix | 70 | Left | exact dimensional | Exact |
| OP-SUB | Arithmetic | `-` |  | Binary | Infix | 60 | Left | exact same-unit | Exact |
| OP-EQ | Comparison | `IS` | `=` | Binary | Infix | 50 | NonAssociative | Truth with TypedMargin | Exact |
| OP-GE | Comparison | `IS AT LEAST` | `>=` | Binary | Infix | 50 | NonAssociative | Truth with TypedMargin | Exact |
| OP-GT | Comparison | `IS MORE THAN` | `>` | Binary | Infix | 50 | NonAssociative | Truth with TypedMargin | Exact |
| OP-LE | Comparison | `IS AT MOST` | `<=` | Binary | Infix | 50 | NonAssociative | Truth with TypedMargin | Exact |
| OP-LT | Comparison | `IS LESS THAN` | `<` | Binary | Infix | 50 | NonAssociative | Truth with TypedMargin | Exact |
| OP-NE | Comparison | `IS NOT` | `!=` | Binary | Infix | 50 | NonAssociative | Truth with TypedMargin | Exact |
| OP-AND | Logical | `AND` |  | Binary | Infix | 30 | Left | Truth | NotApplicable |
| OP-NOT | Logical | `NOT` |  | Unary | Prefix | 40 | NonAssociative | Truth | NotApplicable |
| OP-OR | Logical | `OR` |  | Binary | Infix | 20 | Left | Truth | NotApplicable |
<!-- OPERATORS-CATALOG:END -->

The operator grammar fragment defines the operator productions referenced by `comparison`, `arithmetic_expression`, and `logical_expression`:

<!-- OPERATORS-GRAMMAR:BEGIN generated from spec/operators.rs by bootstrap/project.py; do not edit -->
```text
comparison_operator
  := IS | IS AT LEAST | IS MORE THAN | IS AT MOST | IS LESS THAN | IS NOT | "=" | ">=" | ">" | "<=" | "<" | "!="

arithmetic_operator
  := "+" | "/" | "*" | "-"

logical_operator
  := AND | OR
```
<!-- OPERATORS-GRAMMAR:END -->

Formatting and spoken projections and legal mutation classes:

<!-- OPERATORS-PROJECTION:BEGIN generated from spec/operators.rs by bootstrap/project.py; do not edit -->
| OperatorId | Formatting | Spoken | Legal mutation classes |
| --- | --- | --- | --- |
| OP-ADD | `+` | plus | precedence, associativity, constant-folding, parenthesis, unit-checking |
| OP-DIV | `/` | divided by | precedence, associativity, constant-folding, parenthesis, division-by-zero, scale-adjustment |
| OP-MUL | `*` | times | precedence, associativity, constant-folding, parenthesis, scale-adjustment |
| OP-SUB | `-` | minus | precedence, associativity, constant-folding, parenthesis, unit-checking |
| OP-EQ | `IS` | is | precedence, comparison-direction, margin-sign, parenthesis |
| OP-GE | `IS AT LEAST` | is at least | precedence, comparison-direction, margin-sign, parenthesis |
| OP-GT | `IS MORE THAN` | is more than | precedence, comparison-direction, margin-sign, parenthesis |
| OP-LE | `IS AT MOST` | is at most | precedence, comparison-direction, margin-sign, parenthesis |
| OP-LT | `IS LESS THAN` | is less than | precedence, comparison-direction, margin-sign, parenthesis |
| OP-NE | `IS NOT` | is not | precedence, comparison-direction, margin-sign, parenthesis |
| OP-AND | `AND` | and | precedence, associativity, parenthesis |
| OP-NOT | `NOT` | not | precedence, parenthesis |
| OP-OR | `OR` | or | precedence, associativity, parenthesis |
<!-- OPERATORS-PROJECTION:END -->

---

# 16. Foundational design invariants

## 16.1 Strong three-valued logic

Incomplete information is represented explicitly.

Boolean optimization remains lawful because:

```text
FALSE decides AND
TRUE decides OR
PENDING propagates only when unresolved input matters
```

## 16.2 One evaluation produces all semantic projections

The evaluator produces value, explanation, margin, evidence, completeness, and cost together.

No view becomes a second semantic authority.

## 16.3 Structural explanation

Positive and negative explanations derive from expression structure.

Negation is pushed toward decisive comparison nodes so the system explains the actual failed condition rather than printing an opaque:

```text
not everything passed
```

## 16.4 Typed margins

Threshold distance preserves its unit and direction.

A passing and failing result may both carry the exact distance from the boundary.

## 16.5 Cost accumulation

Every query node and function carries a cost model.

Planning estimates cost.

Execution records actual work.

A query cannot claim bounded execution without naming its bounds.

## 16.6 Canonical typed structure

Readable source is compiled into a normalized typed tree.

Formatting, narration, plans, proofs, and execution derive from that tree.

## 16.7 No arbitrary host callbacks

Built-in kernel computation must be content-identified, typed, capability-declared, costed, and classified for determinism. Runtime-loaded foreign computation is outside V1.

## 16.8 No generic number semantics

Money, percentages, durations, sequence values, timestamps, probabilities, byte sizes, and identifiers remain distinct types.

## 16.9 Evidence is not scalar confidence

Cryptographic verification, authority, freshness, provenance, independence, and completeness remain structural properties.

A numeric probability is used only when the underlying claim is genuinely probabilistic.

## 16.10 No mutable budget deduction during validation

Validation may calculate required cost.

Reservation and consumption occur only at the explicit execution boundary.

A failed validation cannot consume execution budget.

## 16.11 No repeated requirement evaluation

A requirement is evaluated once for a frozen input state.

Its result is reused for:

```text
admission
explanation
proof
receipt construction
```

## 16.12 No display log as audit authority

Terminal text, browser state, and generated HTML may display receipts.

They are not themselves receipts.

## 16.13 No implicit current time in saved queries

Relative time is always anchored.

Interactive convenience is captured into an explicit source cut before execution.

## 16.14 Pending is not false

`PENDING` remains visible unless another known input already determines the answer.

---

# 17. First stable language boundary

The first stable BatQL grammar includes:

```text
ASK and DO
WHO, WHAT, WHERE, WHEN, HOW, and WHY
typed journal, projection, and saved-query sources
explicit historical cuts
event filtering
post-fold state filtering
FOLD
MATCH
GROUP and PARTITION
ordered window functions
GET, CHILDREN, and SCAN projection reads
LET, IF, CASE, COALESCE, and bounded TRY
typed scalar and aggregate functions
ALLOW, DENY, and DEFER decisions
structured reasons and typed margins
explicit availability states
explicit completeness, freshness, and proof posture
bounded pages, matches, bytes, and work
atomic event append
declared external effects
pinned built-in pure kernels
deterministic formatting
deterministic spoken narration
structured plans and proof views
```

It excludes:

```text
unbounded loops
general recursion
runtime code generation
eval
ambient file access
ambient network access
arbitrary host callbacks
implicit cross-currency arithmetic
locale-dependent comparison
floating-point financial arithmetic
silent cursor continuation across generations
unbounded cross products
hidden effects inside functions
mutable local variables
unknown treated as false
a second spoken-language grammar
a separate pipe syntax
```

---

# 18. The final language identity

BatQL should feel like this when read:

> Who is this about? What answer or action is requested? Where does the evidence live? When is that evidence being observed? How should immutable history become the answer? Why should anyone believe it?

And like this when implemented:

```text
one typed question
→ one frozen historical frame
→ one bounded event plan
→ one evaluation
→ one value or effect disposition
→ many faithful projections
→ one independently verifiable result or receipt
```

The public personality is simple:

```text
SQL you can hear.
Excel without the grid.
Event history without temporal guesswork.
Decisions that explain themselves.
Queries that bring receipts.
```


