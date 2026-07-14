---
status: AUTHORITATIVE
contract_id: BP-FACTORY-1
authority_scope: factory metaphor and product identity
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Factory Model

## Metaphor boundary

A battery is a bounded unit that stores, transforms, carries, or proves energy in the larger machine. The metaphor is useful only when it reveals ownership. It must never replace established technical terms.

Events are events. Effects are effects. Types are types. Processes are processes. Coordinates remain coordinates. PakVM, Bvisor, BatQL, MacBat, SyncBat, NetBat, TestPak, `.fbat`, and `.vpak` retain their branded identities because they name real product concepts.

## Factory theorem

One declaration may lawfully generate many projections:

```text
one Contract IR
→ Rust implementations
→ schema and codec descriptors
→ ProgramImage facts
→ runtime tables
→ wire profiles
→ docs and narration
→ fuzz grammar
→ mutation sites
→ proof obligations
```

The declaration is the factory. Generated projections are products. An independently written oracle is a separate factory and may intentionally duplicate behavior.

## Current batteries

| Battery | What it owns |
| --- | --- |
| MacBat | compile-time contract meaning |
| BatPak | durable truth and shared semantic law |
| SyncBat runtime | logical progress and effect sequencing |
| PakVM | bounded typed program interpretation |
| Bvisor | attempt admission, capability, budget, supervision |
| BatQL | human query/action language and compiler |
| NetBat | bounded transport |
| TestPak | repository proof and mutation |

PakVM and Bvisor are internal SyncBat batteries. They are modules because they form one execution heartbeat, not independent products.

## Terminals and breakers

A terminal is an explicit typed boundary through which information or an effect crosses. Bvisor is the breaker around program attempts. It establishes capability and budget before execution and records observed terminal facts afterward.

No hidden wire is permitted merely because two modules share a crate.

## Free Battery Factory made of factories

The repository eventually represents its own commands, proof plans, mutation plans, and generated facts as typed programs. That recursion is braided rather than circular:

```text
MacBat generates facts
TestPak inspects and attacks them
PakVM executes bounded proof programs
BatPak stores receipts
seedcheck remains independent
```

No component becomes its own notary.
