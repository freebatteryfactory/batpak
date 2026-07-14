---
status: AUTHORITATIVE
contract_id: BP-SECURITY-1
authority_scope: threats, trusted computing base, guest isolation, capabilities, storage and proof honesty
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Security Model

## Scope

BatPak protects semantic authority, durable truth, capability use, bounded execution, and evidence integrity within a host trust domain.

It does not claim hardware virtualization, resistance to a malicious kernel, or secrecy against a principal explicitly granted plaintext output.

## Threats

Hostile inputs may attempt malformed `.fbat`/`.vpak` lengths, type confusion, unknown instructions, resource amplification, capability escalation, cursor/cache transplant, stale response reuse, effect smuggling, pending-as-false laundering, proof forgery, authority deletion, event reorder, or cross-store substitution.

Faulty mechanisms may report false success, partial writes, stale time, repeated entropy, incorrect bytes, or output beyond admitted bounds.

## Trusted computing base

Initial TCB includes Rust toolchain/runtime, BatPak codecs/store kernels, `.vpak` structural decoder, PakVM validator/interpreter, Bvisor admission/capability/budget logic, SyncBat transition core, cryptographic libraries, and concrete host ports.

TestPak verifies the TCB but is not runtime authority.

## Compile-time ambient-authority floor

The semantic profiles of `batpak` and `syncbat` qualify under `no_std + alloc`. This removes ordinary `std::fs`, `std::net`, `std::process`, `std::thread`, `std::time`, and environment access from their minimal compile universe. Host mechanisms enter only through explicit adapter modules and feature profiles.

This is one security floor, not a complete proof: `no_std` does not prove determinism, bounded work, absence of unsafe code, correct cryptography, or honest receipts. Those remain TestPak and review obligations.

## Unsafe kernel law

Safe Rust is the default implementation strategy. Unsafe Rust is admitted only for a mechanism that cannot be expressed with qualified safe primitives and only inside an explicitly named kernel, adapter, or FFI concept file. Each occurrence carries a `SAFETY-CONTRACT:` record naming the preconditions, invariants, postconditions, failure policy, owner, independent witness, and mutation/fault seam.

A workspace-wide unsafe ban is not used to hide the real requirement. TestPak and seed checks reject unledgered or misplaced unsafe code, and release receipts bind the complete unsafe denominator.

## Guest isolation claim

A validated PakVM guest cannot directly address arbitrary memory, invoke syscalls, open host paths, create threads/processes, read environment, observe ambient time/entropy, use undeclared capabilities, or exceed validated structural bounds without a typed VM fault.

The interpreter shares the host address space. A bug in trusted Rust or admitted unsafe code can violate memory isolation. The claim is language/capability isolation, not hardware process isolation.

## Capability law

Capabilities are explicit, typed, instance/generation scoped, rights/resource bounded, and non-serializable as live authority. Guessing a world path or slot grants nothing.

Observed effects must be a subset of program declaration, WorldImage imports, instance grants, and attempt budget.

## Information release

Authority cannot survive export. Information leaves only through declared, bounded, receipted outputs. Plaintext that is legitimately exported remains plaintext.

## Image authenticity

A content digest identifies bytes; it does not authorize execution. Admission may require expected WorldImageId, signature, principal/delegation, allowed contract/version set, interface hash, provenance, and proof status.

Well-formed untrusted images may be inspected without being admitted.

## Resource denial

Unbounded structures are rejected before execution. PakVM and Bvisor count logical units; host ports enforce physical limits. Allocation, output, suspended-state, group, match, effect, and artifact bounds are explicit.

## Durable trust

Derived tiles/caches never authenticate themselves. Mutable authority damage is not absence. Verification distinguishes content equality, commitment validity, signature, inclusion, external anchor, and generation continuity.

## Postcondition honesty

A report claims only checked results. Planned is not granted; attempted is not completed; written is not durable; checksum matched is not authenticated; transport secured is not proof verified.

## Foreign execution

V1 loads no arbitrary native or Wasm guest. A future external tool adapter must be separately qualified, artifact-in/artifact-out, capability-isolated, and evidence-bearing. It cannot reshape PakVM or Bvisor into an OS normalization product.

## Proof requirements

Critical boundaries require explicit attacker model, hostile fixtures, semantic mutation, independent reference model, bounded fuzzing, exact refusal taxonomy, and freshness-bound proof receipt.
