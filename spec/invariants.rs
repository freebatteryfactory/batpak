//! Frozen bootstrap invariants. Stable IDs become TestPak contract IDs.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvariantSpec {
    pub id: &'static str,
    pub statement: &'static str,
}

pub const INVARIANTS: &[InvariantSpec] = &[
    InvariantSpec { id: "SEED-ONE-OWNER", statement: "Every stable semantic concept has one canonical owner." },
    InvariantSpec { id: "SEED-SYNCBAT-ONE-HEARTBEAT", statement: "SyncBat contains the runtime, PakVM, Bvisor, world, and port planes as modules of one runtime crate." },
    InvariantSpec { id: "SEED-NO-STANDALONE-VM", statement: "No standalone bat-vm, vpak, pakvm, or bvisor package exists in the target graph." },
    InvariantSpec { id: "SEED-FBAT-CORE", statement: ".fbat is BatPak journal authority and does not imply a FileBat package." },
    InvariantSpec { id: "SEED-PAKVM-NAME", statement: "PakVM names the machine; .vpak names the executable package profile; ProgramImage and WorldImage name semantic values." },
    InvariantSpec { id: "SEED-NO-DUAL-PRODUCT", statement: "HostBat and platform-backend product models do not coexist with WorldImage, PakVM, and Bvisor." },
    InvariantSpec { id: "SEED-NO-AMBIENT-AUTHORITY", statement: "PakVM programs have no syscall, host path, raw descriptor, ambient clock, entropy, environment, or process instruction." },
    InvariantSpec { id: "SEED-SEMANTIC-ZERO-LEAKAGE", statement: "Dependency-owned mechanism types do not define ordinary public BatPak semantics." },
    InvariantSpec { id: "SEED-SYNC-FIRST", statement: "Production semantic APIs require no hidden async runtime, hidden thread, or hidden executor." },
    InvariantSpec { id: "SEED-NO-STD-SEMANTIC-PROFILES", statement: "BatPak and SyncBat qualify their semantic profiles under no_std + alloc; std and browser host mechanisms remain explicit adapters." },
    InvariantSpec { id: "SEED-CONCEPT-SPINE", statement: "Primary semantic types live in root concept files paired with same-name implementation directories; no universal _types drawer exists." },
    InvariantSpec { id: "SEED-NO-INLINE-DOMAIN-TYPES", statement: "No domain-significant named type is declared inside a function or hidden in an unrelated algorithm file." },
    InvariantSpec { id: "SEED-EXPLICIT-EFFECTS", statement: "Every effect crosses a named capability terminal and produces a typed outcome." },
    InvariantSpec { id: "SEED-INDEPENDENT-ORACLE", statement: "No optimized or generated subsystem is its own only oracle." },
    InvariantSpec { id: "SEED-AUDITED-DENOMINATOR", statement: "Every planned proof unit terminates with an explicit disposition." },
    InvariantSpec { id: "SEED-MUTERPRATER-SCOPE", statement: "Muterprater owns mutation testing only and lives inside TestPak." },
    InvariantSpec { id: "SEED-BOUNDED-PUSH", statement: "Long-lived push is bounded per message, batch, buffer, and retained window and has a durable pull recovery path." },
    InvariantSpec { id: "SEED-AVAILABILITY-AXES", statement: "Value availability, K3 truth, decision, completeness, freshness, and proof disposition are distinct axes." },
    InvariantSpec { id: "SEED-TIME-AXES", statement: "Observed wall time, monotonic deadline, HLC, commit order, stream position, and causality remain distinct types." },
    InvariantSpec { id: "SEED-DOC-STATUS", statement: "Every normative document declares status, contract ID, authority scope, supersession, and reconciliation date." },
    InvariantSpec { id: "SEED-NO-PLACEHOLDER-LAW", statement: "Normative target documents contain no TBD, implementation-decides, or unnamed future-cleanup clauses." },
    InvariantSpec { id: "SEED-LEGACY-OBLIGATION", statement: "Legacy behavior is deleted only after a named successor obligation and independent witness exist." },
    InvariantSpec { id: "SEED-ECS-NOT-ONTOLOGY", statement: "ECS is a typed table/system implementation algebra; Contract remains the semantic primitive and Tile the physical materialization primitive." },
    InvariantSpec { id: "SEED-BVISOR-HONESTY", statement: "Bvisor may report only established postconditions and never decides semantic restart legality." },
    InvariantSpec { id: "SEED-BATQL-FROZEN", statement: "BatQL 1.0 conceptual grammar is frozen except for defects proven by parser, type, or conformance work." },
];
