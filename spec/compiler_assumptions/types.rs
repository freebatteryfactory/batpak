use crate::gates::GateId;
use crate::guarantees::{ContractId, GuaranteeRef};

/// The currently admitted ledgerable compiler-assumption kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilerAssumptionKind {
    /// What an unsafe block, function, or impl assumes about memory and the
    /// compiler. Detected lexically (DEC-067, defense in depth) and by the
    /// AST gate (DEC-068, authoritative); witnessed by
    /// `unledgered_unsafe_fn_is_rejected`; adopted by the admitted unsafe
    /// kernels, adapters, and FFI concept files of docs/19 and the G2
    /// storage/codec mechanisms of docs/25.
    UnsafeMemoryContract,
    /// What a pointer or integer-pointer crossing assumes about provenance.
    /// DEC-067 assigns pointer casts to the ledger boundary; the AST gate
    /// detects the crossings; witnessed by
    /// `unledgered_pointer_cast_is_rejected`.
    PointerProvenance,
}

impl CompilerAssumptionKind {
    pub const ALL: &'static [CompilerAssumptionKind] = &[
        CompilerAssumptionKind::UnsafeMemoryContract,
        CompilerAssumptionKind::PointerProvenance,
    ];

    pub const fn spelling(self) -> &'static str {
        match self {
            CompilerAssumptionKind::UnsafeMemoryContract => "UnsafeMemoryContract",
            CompilerAssumptionKind::PointerProvenance => "PointerProvenance",
        }
    }

    /// Explicit per-variant, deliberately NOT a family default: a future
    /// kind must name its actual semantic owner. Both current kinds are
    /// owned by the security model, which owns dangerous-mechanism
    /// admission and the memory and honesty boundary.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            CompilerAssumptionKind::UnsafeMemoryContract => ContractId("BP-SECURITY-1"),
            CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),
        }
    }

    /// One basis per kind — the decision that actually admits the
    /// mechanism, never one universal passport office.
    pub const fn admission_basis(self) -> GuaranteeRef {
        match self {
            CompilerAssumptionKind::UnsafeMemoryContract => GuaranteeRef::dec("DEC-068"),
            CompilerAssumptionKind::PointerProvenance => GuaranteeRef::dec("DEC-067"),
        }
    }

    /// Whether the kind INTRINSICALLY requires the docs/19
    /// `SAFETY-CONTRACT:` marker. Only the unsafe memory contract does; a
    /// pointer crossing that also uses unsafe Rust picks the marker up
    /// through UnsafeMemoryContract, not through this kind.
    pub const fn requires_safety_contract_marker(self) -> bool {
        match self {
            CompilerAssumptionKind::UnsafeMemoryContract => true,
            CompilerAssumptionKind::PointerProvenance => false,
        }
    }

    /// Where TestPak classifies the ledger boundary. Occurrence-specific
    /// implementation gates stay with the occurrences' own owners.
    pub const fn classification_gate(self) -> GateId {
        match self {
            CompilerAssumptionKind::UnsafeMemoryContract => GateId::G3,
            CompilerAssumptionKind::PointerProvenance => GateId::G3,
        }
    }

    /// Where the release seal consumes the ledger.
    pub const fn release_qualification_gate(self) -> GateId {
        match self {
            CompilerAssumptionKind::UnsafeMemoryContract => GateId::G9,
            CompilerAssumptionKind::PointerProvenance => GateId::G9,
        }
    }
}
