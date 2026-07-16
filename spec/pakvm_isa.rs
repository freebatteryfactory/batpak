//! Typed PakVM semantic ISA (D4c1).
//!
//! This file is the ONE authority for what a PakVM semantic node MEANS. It is
//! the first of the three separated ISA layers and never crosses into the other
//! two:
//!
//! ```text
//! semantic ISA    what an operation means             THIS FILE
//! VPak wire ISA   how an operation is encoded         DEC-036, frozen at G5
//! internal micro-ops / SpecializedPlan  how an operation is executed fast   DEC-073
//! ```
//!
//! No numeric opcode is assigned here and none may be inferred. `PakVmNodeId`
//! carries no `#[repr]` and no explicit discriminant: the declaration order below
//! is the authored docs/07 reading order, NOT an encoding. Exact opcode numbers
//! and the ISA version value are G5 implementation constants (docs/07), exact
//! VPak bytes are DEC-036 at G5, and EventFrameV2 bytes are G2. A node's identity
//! here is symbolic and stays symbolic.
//!
//! The node inventory is not invented: `07_PAKVM_ISA.md` authors exactly these
//! 36 nodes under exactly these three algebras, and `bootstrap/audit.py`
//! independently verifies that the typed inventory and the document agree.
//!
//! ADMISSION LAW (5.5D4b, applied here). Every field of a `PakVmNodeSpec` comes
//! from exactly one of: a value authored per node, a value declared once as
//! algebra policy, a value declared once as node-class policy, or one named
//! lawful derivation. A projector serializes an admitted spec; it may not
//! complete one. A node whose spec does not admit is not in the semantic ISA.
//! This is the same law that repaired the Guarantee Graph, applied before the
//! table exists rather than after it shipped.

/// A stable symbolic PakVM semantic node identity.
///
/// NOT an opcode. Variants carry no discriminant, the enum carries no `#[repr]`,
/// and nothing may cast one to an integer. Declaration order follows docs/07 so
/// a reader can check the inventory against its authoring document; order is not
/// encoding and confers no numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmNodeId {
    // Formula and decision (docs/07: 16)
    Literal,
    Binding,
    Field,
    Let,
    Compare,
    All,
    Any,
    Not,
    Case,
    Allow,
    Deny,
    Defer,
    Require,
    Margin,
    Explain,
    KernelCall,
    // Query and dataflow (docs/07: 14)
    Scan,
    Seek,
    Select,
    Filter,
    Project,
    Fold,
    Group,
    Aggregate,
    Join,
    SequenceMatch,
    Sort,
    Page,
    Limit,
    MaterializeTile,
    // Effects (docs/07: 6)
    Append,
    AppendBatch,
    RequestEffect,
    StageArtifact,
    EmitResult,
    AdvanceCheckpointIntent,
}

/// The three instruction algebras authored by docs/07. There is no fourth, and a
/// node belongs to exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmAlgebra {
    FormulaDecision,
    QueryDataflow,
    Effect,
}

/// A node class is a family of nodes that share a cost law and an execution
/// posture. It exists so that an invariant is declared once for the family
/// instead of repeated across every node that happens to share it, and so that a
/// genuinely node-specific fact has somewhere to live that is not a family
/// default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmNodeClass {
    // Formula and decision
    /// Scalar computation over the closed value algebra.
    ScalarComputation,
    /// A terminal decision value. Allow, Deny, and Defer remain distinct.
    DecisionOutcome,
    /// A decision guard that can refuse.
    DecisionGuard,
    /// A call into an admitted built-in kernel.
    KernelInvocation,
    // Query and dataflow
    /// Produces rows by decoding admitted source material.
    SourceTraversal,
    /// Row in, row out.
    RowTransform,
    /// Rows in, accumulated value out.
    RowReduction,
    /// Rows in, groups out.
    Grouping,
    /// Two row streams in, rows out.
    Joining,
    /// Rows in, sequence matches out.
    SequenceMatching,
    /// Rows in, ordered rows out.
    Ordering,
    /// Bounds discovery work over rows.
    Windowing,
    /// Rows in, tile bytes out.
    TileMaterialization,
    // Effects
    /// Appends durable events.
    DurableAppend,
    /// Yields a typed PortRequest for Bvisor mediation.
    PortEffectRequest,
    /// Stages a content-addressed artifact.
    ArtifactStaging,
    /// Emits the semantic result.
    ResultEmission,
    /// Declares intent to advance a checkpoint.
    CheckpointIntent,
}

/// The closed work-accounting unit vocabulary authored by docs/07: "PakVM counts
/// actual instructions, rows, decoded bytes, tile bytes, groups, matches,
/// outputs, artifacts, effects, and call depth." No unit is added here, and the
/// auditor proves every unit is claimed by at least one node family: a unit
/// nothing produces would mean the inventory or the unit list is wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkUnit {
    Instructions,
    Rows,
    DecodedBytes,
    TileBytes,
    Groups,
    Matches,
    Outputs,
    Artifacts,
    Effects,
    CallDepth,
}

/// The shape of a node's declared cost law, named by the authored work units it
/// consumes and produces.
///
/// A family names UNITS, not a complexity exponent. docs/07 says PakVM *counts*
/// these units; a scaling exponent is a measured benchmark property owned by
/// docs/24, not an ISA declaration. `Sort` and `Join` therefore name `Rows`: what
/// they count is rows, and how that count grows is measured, not declared here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkFormulaFamily {
    /// One interpreted step over scalar values. Consumes no rows and produces no
    /// group, match, output, artifact, effect, or call depth.
    ConstantInstruction,
    /// The kernel's own declared cost contract, bound into `KernelInterfaceHash`
    /// alongside its bounds (docs/07). PakVM contributes the call itself and the
    /// call depth; the kernel contract owns the rest. Kernel call is the only
    /// node that adds call depth: V1 admits no general recursion and no other
    /// call form.
    KernelCostContract,
    /// Linear in the rows traversed and the bytes those rows decode.
    RowsAndDecodedBytes,
    /// Linear in the rows examined.
    Rows,
    /// Linear in rows examined, accumulating into groups.
    RowsIntoGroups,
    /// Linear in rows examined, accumulating into sequence matches.
    RowsIntoMatches,
    /// Linear in rows examined, producing tile bytes.
    RowsIntoTileBytes,
    /// Declared durable or port effects.
    DeclaredEffects,
    /// Staged content-addressed artifacts.
    StagedArtifacts,
    /// Emitted semantic outputs.
    EmittedOutputs,
}

impl WorkFormulaFamily {
    /// The authored work units a family accounts in. Total by construction: a new
    /// family cannot be added without stating its units.
    pub const fn units(self) -> &'static [WorkUnit] {
        match self {
            WorkFormulaFamily::ConstantInstruction => &[WorkUnit::Instructions],
            WorkFormulaFamily::KernelCostContract => {
                &[WorkUnit::Instructions, WorkUnit::CallDepth]
            }
            WorkFormulaFamily::RowsAndDecodedBytes => &[WorkUnit::Rows, WorkUnit::DecodedBytes],
            WorkFormulaFamily::Rows => &[WorkUnit::Rows],
            WorkFormulaFamily::RowsIntoGroups => &[WorkUnit::Rows, WorkUnit::Groups],
            WorkFormulaFamily::RowsIntoMatches => &[WorkUnit::Rows, WorkUnit::Matches],
            WorkFormulaFamily::RowsIntoTileBytes => &[WorkUnit::Rows, WorkUnit::TileBytes],
            WorkFormulaFamily::DeclaredEffects => &[WorkUnit::Effects],
            WorkFormulaFamily::StagedArtifacts => &[WorkUnit::Artifacts],
            WorkFormulaFamily::EmittedOutputs => &[WorkUnit::Outputs],
        }
    }
}

impl PakVmAlgebra {
    /// The work-accounting plane this algebra plays on: the only units a family
    /// of this algebra may account in.
    ///
    /// SCOPE: this law governs the V1 PUBLIC semantic ISA — the 36 nodes admitted
    /// here — and nothing else. It is not a claim about internal lowering
    /// identities, `SpecializedPlan` micro-ops, physical attempts, or
    /// implementation mechanisms: those are a different layer with their own
    /// accounting, and they have no `PakVmNodeId` to admit. A future mechanism
    /// that measures cost differently does not contradict this partition; it is
    /// simply not governed by it.
    ///
    /// The three planes PARTITION the authored unit vocabulary — every unit
    /// belongs to exactly one algebra, and no unit belongs to two. That is the
    /// lineage docs/07 already carries: `Groups`, `Matches`, `TileBytes`, and
    /// `DecodedBytes` exist because query/dataflow nodes produce them; `Effects`
    /// and `Artifacts` because effect nodes do; `CallDepth` because kernel call is
    /// the only call form V1 admits.
    ///
    /// This is what makes a work-formula family CHECKABLE rather than merely
    /// non-empty. Proving every unit is claimed by some family proves coverage,
    /// not correctness: a family in the wrong plane still claims its unit. Page
    /// and Limit accounted in `Outputs` would satisfy coverage perfectly while
    /// being the exact defect LEG-028's
    /// `page_limit_bounds_discovery_work_not_only_output` exists to reject.
    pub const fn work_plane(self) -> &'static [WorkUnit] {
        match self {
            PakVmAlgebra::FormulaDecision => &[WorkUnit::Instructions, WorkUnit::CallDepth],
            PakVmAlgebra::QueryDataflow => &[
                WorkUnit::Rows,
                WorkUnit::DecodedBytes,
                WorkUnit::Groups,
                WorkUnit::Matches,
                WorkUnit::TileBytes,
            ],
            PakVmAlgebra::Effect => &[WorkUnit::Effects, WorkUnit::Artifacts, WorkUnit::Outputs],
        }
    }

    /// The unit every family of this algebra MUST account in.
    ///
    /// A formula/decision node costs interpreted instructions. A query/dataflow
    /// node costs discovery over rows: LEG-028 states the bound constrains
    /// discovery before decode, allocation, or materialization, so a query family
    /// that does not count rows is not bounded traversal whatever else it counts.
    /// An effect node has no single mandatory unit: a durable append, a staged
    /// artifact, and an emitted result are counted on their own planes.
    pub const fn mandatory_work_unit(self) -> Option<WorkUnit> {
        match self {
            PakVmAlgebra::FormulaDecision => Some(WorkUnit::Instructions),
            PakVmAlgebra::QueryDataflow => Some(WorkUnit::Rows),
            PakVmAlgebra::Effect => None,
        }
    }
}

/// Whether a node performs an effect. docs/07: "A pure query image cannot contain
/// Effect instructions. The validator rejects the image before execution."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectPosture {
    /// Computes over values already in hand. Reads no source material and
    /// performs no effect.
    Pure,
    /// Reads admitted source material under the invocation's envelope. Performs
    /// no durable effect and is admissible in a pure query image.
    ObservationalOnly,
    /// The kernel's `KernelInterfaceHash` declares determinism and effects. The
    /// node does not carry a posture of its own, and a kernel cannot smuggle an
    /// ambient host callback (docs/07).
    DeclaredByKernelContract,
    /// Performs a declared effect. Forbidden in a pure query image.
    Effectful,
}

/// What authority a node needs. A capability is never implied by execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityRequirement {
    /// None. The node touches no source material and no host.
    None,
    /// Reads under the invocation's read envelope. A query program's envelope is
    /// server-issued and read-only (DEC-050); the node cannot widen it.
    ReadOnlySourceEnvelope,
    /// Declared by the kernel's interface hash, and satisfied only from the
    /// admitted WorldImage and capability closure (docs/07).
    DeclaredByKernelContract,
    /// The capability the node's declared effect names, drawn from the
    /// invocation's grant. An undeclared effect is refused however the program is
    /// written (DEC-049).
    DeclaredEffectCapability,
}

/// Whether a node's iteration is bounded, and by what. docs/07: V1 excludes
/// arbitrary backward jumps, general recursion, raw memory access, ambient
/// callbacks, host syscalls, and unbounded loops; iteration occurs through
/// bounded operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundednessPosture {
    /// One step. Nothing to bound.
    ConstantWork,
    /// Iterates, under a bound validated before admission (docs/07 work-bound
    /// validation).
    BoundedIteration,
    /// Bounded by the kernel's declared bounds, bound into its interface hash.
    BoundedByKernelContract,
    /// Bounded by the declared effect itself.
    BoundedByDeclaredEffect,
}

/// How a node's behaviour is judged INDEPENDENTLY of the implementation under
/// test. SEED-INDEPENDENT-ORACLE: the expected result may never be generated by
/// the implementation being proven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    /// An independent model executes the node and judges the result
    /// (`MutationLane::SemanticIr`). The reference interpreter is the executable
    /// semantic authority (DEC-073); it is not its own oracle.
    ReferenceInterpreterModel,
    /// The kernel's `KernelQualificationReceiptId` proves the implementation
    /// satisfies its contract (DEC-062).
    KernelQualificationReceipt,
    /// Judged against the durable journal, artifact, or checkpoint plane.
    DurablePlaneObservation,
    /// Judged against Bvisor port mediation, response identity validation, and
    /// resume under the same AttemptId (docs/07 suspension).
    PortMediationObservation,
}

/// What a policy may PROPOSE about an identity's lowering posture. This is
/// CANDIDATE vocabulary: admission may encounter `InternalLoweringIdentity` and
/// must reject it. It is deliberately expressible here so that the refusal has
/// something to refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateLoweringPosture {
    /// Appears in a ProgramImage; the reference interpreter evaluates it.
    PublicSemanticIdentity,
    /// A `SpecializedPlan` micro-op (DEC-073, D2) or a decision-circuit gate.
    /// An execution identity, never a meaning.
    InternalLoweringIdentity,
}

/// Proof that admission classified a node as a public semantic node.
///
/// The unit field is private, so this is constructible ONLY inside this module
/// and only by `admit`. No generator, auditor, projector, runtime, or fixture can
/// mint one. `PakVmNodeSpec` therefore cannot represent an internal lowering
/// identity AT ALL: the forbidden state is unrepresentable after admission rather
/// than merely promised absent by a validator everyone is trusted to have called.
///
/// The weaker shape — carrying the candidate enum into the admitted struct —
/// would mean the valid-state type contains a state the system says can never be
/// valid, and every reader downstream would have to take admission's word for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedAsPublicSemanticNode(());

/// Where one `PakVmNodeSpec` field legitimately comes from. Every admitted field
/// resolves through exactly one of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmRule<T: 'static> {
    /// Invariant for the whole algebra; declared once, never repeated per node.
    AlgebraConstant(T),
    /// Varies within the algebra; the node class declares it.
    ClassDeclared,
}

/// Typed algebra policy: what is invariant for every node of an algebra.
///
/// A field declared `AlgebraConstant` here may NOT also be declared by a class:
/// two owners for one fact is the duplicate-policy defect. A field declared
/// `ClassDeclared` MUST be declared by every class in the algebra: a family
/// default standing in for a missing node-specific fact is the opposite defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PakVmAlgebraPolicy {
    pub algebra: PakVmAlgebra,
    pub effect: PakVmRule<EffectPosture>,
    pub capability: PakVmRule<CapabilityRequirement>,
    pub evidence: PakVmRule<EvidenceClass>,
    pub lowering: PakVmRule<CandidateLoweringPosture>,
}

/// The three authored algebras and what each fixes for every node inside it.
pub const PAKVM_ALGEBRA_POLICIES: &[PakVmAlgebraPolicy] = &[
    PakVmAlgebraPolicy {
        algebra: PakVmAlgebra::FormulaDecision,
        // A kernel call sits in this algebra and defers its posture to the kernel
        // contract, so effect, capability, and evidence genuinely vary here.
        effect: PakVmRule::ClassDeclared,
        capability: PakVmRule::ClassDeclared,
        evidence: PakVmRule::ClassDeclared,
        lowering: PakVmRule::AlgebraConstant(CandidateLoweringPosture::PublicSemanticIdentity),
    },
    PakVmAlgebraPolicy {
        algebra: PakVmAlgebra::QueryDataflow,
        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),
        capability: PakVmRule::AlgebraConstant(CapabilityRequirement::ReadOnlySourceEnvelope),
        evidence: PakVmRule::AlgebraConstant(EvidenceClass::ReferenceInterpreterModel),
        lowering: PakVmRule::AlgebraConstant(CandidateLoweringPosture::PublicSemanticIdentity),
    },
    PakVmAlgebraPolicy {
        algebra: PakVmAlgebra::Effect,
        effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),
        capability: PakVmRule::AlgebraConstant(CapabilityRequirement::DeclaredEffectCapability),
        // A durable append, a mediated port request, and an emitted result are
        // judged on different planes; one evidence route for all three would
        // certify none of them.
        evidence: PakVmRule::ClassDeclared,
        lowering: PakVmRule::AlgebraConstant(CandidateLoweringPosture::PublicSemanticIdentity),
    },
];

/// Typed node-class policy: the cost law and posture a family shares.
///
/// `None` means the algebra already fixed that field. `Some` means the algebra
/// delegated it here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PakVmNodeClassPolicy {
    pub class: PakVmNodeClass,
    pub algebra: PakVmAlgebra,
    pub work_formula: WorkFormulaFamily,
    pub boundedness: BoundednessPosture,
    pub effect: Option<EffectPosture>,
    pub capability: Option<CapabilityRequirement>,
    pub evidence: Option<EvidenceClass>,
}

/// Every node class, its cost law, and the fields its algebra delegated to it.
///
/// Work-formula families are derived from each class's authored semantics and
/// typed signature, never from node names or symmetry. The authored work-unit
/// vocabulary maps onto these families by construction: `Groups` exist because
/// `Group` produces them, `Matches` because `SequenceMatch` does, `TileBytes`
/// because `MaterializeTile` does, `Artifacts` because `StageArtifact` does, and
/// `CallDepth` because `KernelCall` is the only call form V1 admits.
///
/// `Windowing` names `Rows`, not `Outputs`, on authored law rather than
/// judgement: LEG-028's proof row `page_limit_bounds_discovery_work_not_only_output`
/// states that the page limit and work budget constrain DISCOVERY before decode,
/// allocation, or materialization, and that truncating the returned output does
/// not satisfy bounded traversal. A `Windowing` family accounted in outputs would
/// be the exact defect that row exists to reject.
pub const PAKVM_NODE_CLASS_POLICIES: &[PakVmNodeClassPolicy] = &[
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::ScalarComputation,
        algebra: PakVmAlgebra::FormulaDecision,
        work_formula: WorkFormulaFamily::ConstantInstruction,
        boundedness: BoundednessPosture::ConstantWork,
        effect: Some(EffectPosture::Pure),
        capability: Some(CapabilityRequirement::None),
        evidence: Some(EvidenceClass::ReferenceInterpreterModel),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::DecisionOutcome,
        algebra: PakVmAlgebra::FormulaDecision,
        work_formula: WorkFormulaFamily::ConstantInstruction,
        boundedness: BoundednessPosture::ConstantWork,
        effect: Some(EffectPosture::Pure),
        capability: Some(CapabilityRequirement::None),
        evidence: Some(EvidenceClass::ReferenceInterpreterModel),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::DecisionGuard,
        algebra: PakVmAlgebra::FormulaDecision,
        work_formula: WorkFormulaFamily::ConstantInstruction,
        boundedness: BoundednessPosture::ConstantWork,
        effect: Some(EffectPosture::Pure),
        capability: Some(CapabilityRequirement::None),
        evidence: Some(EvidenceClass::ReferenceInterpreterModel),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::KernelInvocation,
        algebra: PakVmAlgebra::FormulaDecision,
        work_formula: WorkFormulaFamily::KernelCostContract,
        boundedness: BoundednessPosture::BoundedByKernelContract,
        effect: Some(EffectPosture::DeclaredByKernelContract),
        capability: Some(CapabilityRequirement::DeclaredByKernelContract),
        evidence: Some(EvidenceClass::KernelQualificationReceipt),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::SourceTraversal,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::RowsAndDecodedBytes,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::RowTransform,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::Rows,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::RowReduction,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::Rows,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::Grouping,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::RowsIntoGroups,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::Joining,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::Rows,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::SequenceMatching,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::RowsIntoMatches,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::Ordering,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::Rows,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::Windowing,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::Rows,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::TileMaterialization,
        algebra: PakVmAlgebra::QueryDataflow,
        work_formula: WorkFormulaFamily::RowsIntoTileBytes,
        boundedness: BoundednessPosture::BoundedIteration,
        effect: None,
        capability: None,
        evidence: None,
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::DurableAppend,
        algebra: PakVmAlgebra::Effect,
        work_formula: WorkFormulaFamily::DeclaredEffects,
        boundedness: BoundednessPosture::BoundedByDeclaredEffect,
        effect: None,
        capability: None,
        evidence: Some(EvidenceClass::DurablePlaneObservation),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::PortEffectRequest,
        algebra: PakVmAlgebra::Effect,
        work_formula: WorkFormulaFamily::DeclaredEffects,
        boundedness: BoundednessPosture::BoundedByDeclaredEffect,
        effect: None,
        capability: None,
        evidence: Some(EvidenceClass::PortMediationObservation),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::ArtifactStaging,
        algebra: PakVmAlgebra::Effect,
        work_formula: WorkFormulaFamily::StagedArtifacts,
        boundedness: BoundednessPosture::BoundedByDeclaredEffect,
        effect: None,
        capability: None,
        evidence: Some(EvidenceClass::DurablePlaneObservation),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::ResultEmission,
        algebra: PakVmAlgebra::Effect,
        work_formula: WorkFormulaFamily::EmittedOutputs,
        boundedness: BoundednessPosture::BoundedByDeclaredEffect,
        effect: None,
        capability: None,
        // Result parity against the reference interpreter is what judges an
        // emitted result (docs/07): value, Availability, Truth, Decision,
        // Completeness, ProofDisposition, Freshness, and TypedMargin all survive
        // the faster path or the path is wrong.
        evidence: Some(EvidenceClass::ReferenceInterpreterModel),
    },
    PakVmNodeClassPolicy {
        class: PakVmNodeClass::CheckpointIntent,
        algebra: PakVmAlgebra::Effect,
        work_formula: WorkFormulaFamily::DeclaredEffects,
        boundedness: BoundednessPosture::BoundedByDeclaredEffect,
        effect: None,
        capability: None,
        evidence: Some(EvidenceClass::DurablePlaneObservation),
    },
];

impl PakVmNodeId {
    /// The authored docs/07 spelling. This is the projection surface and the
    /// audit key; it is not an encoding.
    pub const fn authored_name(self) -> &'static str {
        match self {
            PakVmNodeId::Literal => "literal",
            PakVmNodeId::Binding => "binding",
            PakVmNodeId::Field => "field",
            PakVmNodeId::Let => "let",
            PakVmNodeId::Compare => "compare",
            PakVmNodeId::All => "all",
            PakVmNodeId::Any => "any",
            PakVmNodeId::Not => "not",
            PakVmNodeId::Case => "case",
            PakVmNodeId::Allow => "allow",
            PakVmNodeId::Deny => "deny",
            PakVmNodeId::Defer => "defer",
            PakVmNodeId::Require => "require",
            PakVmNodeId::Margin => "margin",
            PakVmNodeId::Explain => "explain",
            PakVmNodeId::KernelCall => "built-in kernel call",
            PakVmNodeId::Scan => "scan",
            PakVmNodeId::Seek => "seek",
            PakVmNodeId::Select => "select",
            PakVmNodeId::Filter => "filter",
            PakVmNodeId::Project => "project",
            PakVmNodeId::Fold => "fold",
            PakVmNodeId::Group => "group",
            PakVmNodeId::Aggregate => "aggregate",
            PakVmNodeId::Join => "join",
            PakVmNodeId::SequenceMatch => "sequence match",
            PakVmNodeId::Sort => "sort",
            PakVmNodeId::Page => "page",
            PakVmNodeId::Limit => "limit",
            PakVmNodeId::MaterializeTile => "materialize tile",
            PakVmNodeId::Append => "append",
            PakVmNodeId::AppendBatch => "append batch",
            PakVmNodeId::RequestEffect => "request effect",
            PakVmNodeId::StageArtifact => "stage artifact",
            PakVmNodeId::EmitResult => "emit result",
            PakVmNodeId::AdvanceCheckpointIntent => "advance checkpoint intent",
        }
    }

    /// Authored algebra membership: docs/07 lists each node under exactly one of
    /// the three algebra headings. Total by construction — a new node cannot be
    /// added without stating which algebra authored it.
    pub const fn algebra(self) -> PakVmAlgebra {
        match self {
            PakVmNodeId::Literal
            | PakVmNodeId::Binding
            | PakVmNodeId::Field
            | PakVmNodeId::Let
            | PakVmNodeId::Compare
            | PakVmNodeId::All
            | PakVmNodeId::Any
            | PakVmNodeId::Not
            | PakVmNodeId::Case
            | PakVmNodeId::Allow
            | PakVmNodeId::Deny
            | PakVmNodeId::Defer
            | PakVmNodeId::Require
            | PakVmNodeId::Margin
            | PakVmNodeId::Explain
            | PakVmNodeId::KernelCall => PakVmAlgebra::FormulaDecision,
            PakVmNodeId::Scan
            | PakVmNodeId::Seek
            | PakVmNodeId::Select
            | PakVmNodeId::Filter
            | PakVmNodeId::Project
            | PakVmNodeId::Fold
            | PakVmNodeId::Group
            | PakVmNodeId::Aggregate
            | PakVmNodeId::Join
            | PakVmNodeId::SequenceMatch
            | PakVmNodeId::Sort
            | PakVmNodeId::Page
            | PakVmNodeId::Limit
            | PakVmNodeId::MaterializeTile => PakVmAlgebra::QueryDataflow,
            PakVmNodeId::Append
            | PakVmNodeId::AppendBatch
            | PakVmNodeId::RequestEffect
            | PakVmNodeId::StageArtifact
            | PakVmNodeId::EmitResult
            | PakVmNodeId::AdvanceCheckpointIntent => PakVmAlgebra::Effect,
        }
    }

    /// Authored family membership. Total by construction.
    pub const fn class(self) -> PakVmNodeClass {
        match self {
            PakVmNodeId::Literal
            | PakVmNodeId::Binding
            | PakVmNodeId::Field
            | PakVmNodeId::Let
            | PakVmNodeId::Compare
            | PakVmNodeId::All
            | PakVmNodeId::Any
            | PakVmNodeId::Not
            | PakVmNodeId::Case
            | PakVmNodeId::Margin
            | PakVmNodeId::Explain => PakVmNodeClass::ScalarComputation,
            PakVmNodeId::Allow | PakVmNodeId::Deny | PakVmNodeId::Defer => {
                PakVmNodeClass::DecisionOutcome
            }
            PakVmNodeId::Require => PakVmNodeClass::DecisionGuard,
            PakVmNodeId::KernelCall => PakVmNodeClass::KernelInvocation,
            PakVmNodeId::Scan | PakVmNodeId::Seek => PakVmNodeClass::SourceTraversal,
            PakVmNodeId::Select | PakVmNodeId::Filter | PakVmNodeId::Project => {
                PakVmNodeClass::RowTransform
            }
            PakVmNodeId::Fold | PakVmNodeId::Aggregate => PakVmNodeClass::RowReduction,
            PakVmNodeId::Group => PakVmNodeClass::Grouping,
            PakVmNodeId::Join => PakVmNodeClass::Joining,
            PakVmNodeId::SequenceMatch => PakVmNodeClass::SequenceMatching,
            PakVmNodeId::Sort => PakVmNodeClass::Ordering,
            PakVmNodeId::Page | PakVmNodeId::Limit => PakVmNodeClass::Windowing,
            PakVmNodeId::MaterializeTile => PakVmNodeClass::TileMaterialization,
            PakVmNodeId::Append | PakVmNodeId::AppendBatch => PakVmNodeClass::DurableAppend,
            PakVmNodeId::RequestEffect => PakVmNodeClass::PortEffectRequest,
            PakVmNodeId::StageArtifact => PakVmNodeClass::ArtifactStaging,
            PakVmNodeId::EmitResult => PakVmNodeClass::ResultEmission,
            PakVmNodeId::AdvanceCheckpointIntent => PakVmNodeClass::CheckpointIntent,
        }
    }

    /// Operand sorts, in the closed value algebra docs/07 names. Genuinely
    /// node-specific: a class fixes a cost law, not a signature.
    pub const fn operand_sorts(self) -> &'static str {
        match self {
            PakVmNodeId::Literal => "none; a constant-pool reference",
            PakVmNodeId::Binding => "none; a validated frame slot reference",
            PakVmNodeId::Field => "record or enum value, plus a declared field reference",
            PakVmNodeId::Let => "value, plus a frame slot to bind",
            PakVmNodeId::Compare => "two values of one comparable sort",
            PakVmNodeId::All | PakVmNodeId::Any => "bounded list of Truth",
            PakVmNodeId::Not => "Truth",
            PakVmNodeId::Case => "scrutinee value, plus bounded declared arms",
            PakVmNodeId::Allow | PakVmNodeId::Deny => "none; a decision terminal",
            PakVmNodeId::Defer => "declared deferral reason",
            PakVmNodeId::Require => "Truth, plus a declared refusal reason",
            PakVmNodeId::Margin => "two comparable values",
            PakVmNodeId::Explain => "origin reference, plus explanation-dictionary reference",
            PakVmNodeId::KernelCall => "declared by the KernelInterfaceHash",
            PakVmNodeId::Scan => "declared source reference, selector, and bounds",
            PakVmNodeId::Seek => "declared source reference, key, and bounds",
            PakVmNodeId::Select | PakVmNodeId::Project => "row stream, plus declared field selection",
            PakVmNodeId::Filter => "row stream, plus a Truth-valued predicate",
            PakVmNodeId::Fold => "row stream, seed value, and a declared step",
            PakVmNodeId::Group => "row stream, plus a declared grouping key",
            PakVmNodeId::Aggregate => "row or group stream, plus a declared aggregation",
            PakVmNodeId::Join => "two row streams, plus a declared join key",
            PakVmNodeId::SequenceMatch => "row stream, plus a declared bounded pattern",
            PakVmNodeId::Sort => "row stream, plus a declared total sort key",
            PakVmNodeId::Page => "row stream, plus a declared page limit and cursor",
            PakVmNodeId::Limit => "row stream, plus a declared limit",
            PakVmNodeId::MaterializeTile => "row stream, plus a declared typed layout",
            PakVmNodeId::Append => "declared event value",
            PakVmNodeId::AppendBatch => "bounded list of declared event values",
            PakVmNodeId::RequestEffect => "declared PortRequest value",
            PakVmNodeId::StageArtifact => "bounded artifact content",
            PakVmNodeId::EmitResult => "semantic result value, plus its orthogonal axes",
            PakVmNodeId::AdvanceCheckpointIntent => "declared checkpoint position",
        }
    }

    /// Result sorts, in the closed value algebra. No generic host object or `Any`
    /// value crosses the boundary (docs/07).
    pub const fn result_sorts(self) -> &'static str {
        match self {
            PakVmNodeId::Literal | PakVmNodeId::Binding | PakVmNodeId::Field => "one value",
            PakVmNodeId::Let => "unit; the frame slot is bound",
            PakVmNodeId::Compare => "Truth with TypedMargin",
            PakVmNodeId::All | PakVmNodeId::Any | PakVmNodeId::Not => "Truth",
            PakVmNodeId::Case => "the selected arm's value",
            PakVmNodeId::Allow | PakVmNodeId::Deny | PakVmNodeId::Defer => "Decision",
            PakVmNodeId::Require => "Truth, or a typed refusal",
            PakVmNodeId::Margin => "TypedMargin",
            PakVmNodeId::Explain => "explanation reference",
            PakVmNodeId::KernelCall => "declared by the KernelInterfaceHash",
            PakVmNodeId::Scan | PakVmNodeId::Seek => "row stream, plus a source stamp",
            PakVmNodeId::Select
            | PakVmNodeId::Filter
            | PakVmNodeId::Project
            | PakVmNodeId::Join
            | PakVmNodeId::Sort
            | PakVmNodeId::Limit => "row stream",
            PakVmNodeId::Fold => "one accumulated value",
            PakVmNodeId::Group => "group stream",
            PakVmNodeId::Aggregate => "one aggregated value per group",
            PakVmNodeId::SequenceMatch => "match stream",
            PakVmNodeId::Page => "page: ordered entries, completeness, next cursor, source stamp, WorkObservation, proof result",
            PakVmNodeId::MaterializeTile => "tile reference",
            PakVmNodeId::Append | PakVmNodeId::AppendBatch => "durable append outcome",
            PakVmNodeId::RequestEffect => "suspension; resumed under the same AttemptId",
            PakVmNodeId::StageArtifact => "ArtifactRef, only on completion",
            PakVmNodeId::EmitResult => "ExecutedResult",
            PakVmNodeId::AdvanceCheckpointIntent => "checkpoint-advance outcome",
        }
    }

    /// The authoring document clause this node's identity and semantics come
    /// from. Lineage sufficient for an auditor to check the node against its
    /// source rather than against this file.
    pub const fn source_origin(self) -> &'static str {
        match self.algebra() {
            PakVmAlgebra::FormulaDecision => "docs/07 Three instruction algebras: Formula and decision",
            PakVmAlgebra::QueryDataflow => "docs/07 Three instruction algebras: Query and dataflow",
            PakVmAlgebra::Effect => "docs/07 Three instruction algebras: Effects",
        }
    }
}

/// The authored node inventory, in docs/07 reading order. Order is provenance,
/// not encoding.
pub const PAKVM_NODES: &[PakVmNodeId] = &[
    PakVmNodeId::Literal,
    PakVmNodeId::Binding,
    PakVmNodeId::Field,
    PakVmNodeId::Let,
    PakVmNodeId::Compare,
    PakVmNodeId::All,
    PakVmNodeId::Any,
    PakVmNodeId::Not,
    PakVmNodeId::Case,
    PakVmNodeId::Allow,
    PakVmNodeId::Deny,
    PakVmNodeId::Defer,
    PakVmNodeId::Require,
    PakVmNodeId::Margin,
    PakVmNodeId::Explain,
    PakVmNodeId::KernelCall,
    PakVmNodeId::Scan,
    PakVmNodeId::Seek,
    PakVmNodeId::Select,
    PakVmNodeId::Filter,
    PakVmNodeId::Project,
    PakVmNodeId::Fold,
    PakVmNodeId::Group,
    PakVmNodeId::Aggregate,
    PakVmNodeId::Join,
    PakVmNodeId::SequenceMatch,
    PakVmNodeId::Sort,
    PakVmNodeId::Page,
    PakVmNodeId::Limit,
    PakVmNodeId::MaterializeTile,
    PakVmNodeId::Append,
    PakVmNodeId::AppendBatch,
    PakVmNodeId::RequestEffect,
    PakVmNodeId::StageArtifact,
    PakVmNodeId::EmitResult,
    PakVmNodeId::AdvanceCheckpointIntent,
];

/// One admitted PakVM semantic node.
///
/// Every field is supplied by exactly one owner: a value authored per node, a
/// value declared once as algebra policy, a value declared once as class policy,
/// or one named lawful derivation. `bootstrap/project.py` serializes an admitted
/// spec into the docs/07 inventory projection; it may not complete one, and
/// neither may `bootstrap/audit.py`, which independently re-derives every field
/// and compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PakVmNodeSpec {
    pub id: PakVmNodeId,
    pub authored_name: &'static str,
    pub algebra: PakVmAlgebra,
    pub class: PakVmNodeClass,
    pub effect: EffectPosture,
    pub operand_sorts: &'static str,
    pub result_sorts: &'static str,
    pub boundedness: BoundednessPosture,
    pub capability: CapabilityRequirement,
    pub work_formula: WorkFormulaFamily,
    pub evidence: EvidenceClass,
    /// Positive proof of admission. Not a per-node value: every admitted node is
    /// a public semantic node, and nothing else can become one.
    pub lowering: AdmittedAsPublicSemanticNode,
    pub source_origin: &'static str,
}

/// The outcome of admitting one node. `Refused` carries why: an unresolved
/// field, two owners for one field, or a posture the ISA cannot hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmAdmission {
    Admitted(PakVmNodeSpec),
    Refused(&'static str),
}

pub fn algebra_policy(algebra: PakVmAlgebra) -> Option<&'static PakVmAlgebraPolicy> {
    let mut i = 0;
    while i < PAKVM_ALGEBRA_POLICIES.len() {
        if PAKVM_ALGEBRA_POLICIES[i].algebra == algebra {
            return Some(&PAKVM_ALGEBRA_POLICIES[i]);
        }
        i += 1;
    }
    None
}

pub fn class_policy(class: PakVmNodeClass) -> Option<&'static PakVmNodeClassPolicy> {
    let mut i = 0;
    while i < PAKVM_NODE_CLASS_POLICIES.len() {
        if PAKVM_NODE_CLASS_POLICIES[i].class == class {
            return Some(&PAKVM_NODE_CLASS_POLICIES[i]);
        }
        i += 1;
    }
    None
}

/// Resolve one delegated field. Exactly one owner, never zero and never two.
///
/// - the algebra fixed it and the class stayed silent -> the algebra's value;
/// - the algebra delegated and the class declared -> the class's value;
/// - the algebra fixed it and the class ALSO declared -> refused, two owners;
/// - the algebra delegated and the class stayed silent -> refused, no owner.
fn resolve<T: Copy>(rule: PakVmRule<T>, declared: Option<T>, field: &'static str)
    -> Result<T, &'static str> {
    match (rule, declared) {
        (PakVmRule::AlgebraConstant(_), Some(_)) => Err(field),
        (PakVmRule::AlgebraConstant(v), None) => Ok(v),
        (PakVmRule::ClassDeclared, Some(v)) => Ok(v),
        (PakVmRule::ClassDeclared, None) => Err(field),
    }
}

/// Admit one node into the semantic ISA.
///
/// A node that does not admit is not in the ISA. Nothing downstream may supply a
/// field this refuses to resolve: that is precisely how the Guarantee Graph came
/// to carry invented lifetimes and a qualification target sitting in a gate
/// column, and the repair is to fail closed here rather than to let a projector
/// be helpful.
pub fn admit(id: PakVmNodeId) -> PakVmAdmission {
    let ap = match algebra_policy(id.algebra()) {
        Some(p) => p,
        None => return PakVmAdmission::Refused("algebra declares no policy"),
    };
    let cp = match class_policy(id.class()) {
        Some(p) => p,
        None => return PakVmAdmission::Refused("node class declares no policy"),
    };
    admit_resolved(id, ap, cp)
}

/// Hand admission a policy the canonical tables do not contain.
///
/// TEST-ONLY, and absent from every production build: without `cfg(test)` this
/// function does not exist, so no runtime, generator, auditor, or projector can
/// call it even by mistake. It cannot mint canonical provenance either — it
/// returns whatever `admit_resolved` decides, and a node's `source_origin` still
/// comes from its own authored lineage.
///
/// The seam exists because every refusal below would otherwise be unfalsifiable:
/// reachable only by editing the specification, which means a rule nothing can
/// trip. A hostile fixture needs to construct the forbidden proposal; production
/// must never be able to.
#[cfg(test)]
pub(crate) fn admit_candidate_policy(
    id: PakVmNodeId,
    ap: &PakVmAlgebraPolicy,
    cp: &PakVmNodeClassPolicy,
) -> PakVmAdmission {
    admit_resolved(id, ap, cp)
}

/// The admission decision itself. PRIVATE: the only public route in is `admit`,
/// which supplies the canonical authored tables and nothing else.
fn admit_resolved(
    id: PakVmNodeId,
    ap: &PakVmAlgebraPolicy,
    cp: &PakVmNodeClassPolicy,
) -> PakVmAdmission {
    let algebra = id.algebra();
    let class = id.class();
    if ap.algebra != algebra {
        return PakVmAdmission::Refused("algebra policy names a different algebra");
    }
    if cp.class != class {
        return PakVmAdmission::Refused("node class policy names a different class");
    }
    // A class belongs to one algebra. A class policy filed under another algebra
    // would silently import that algebra's constants.
    if cp.algebra != algebra {
        return PakVmAdmission::Refused("node class policy names a different algebra");
    }
    let effect = match resolve(ap.effect, cp.effect, "effect posture has no single owner") {
        Ok(v) => v,
        Err(e) => return PakVmAdmission::Refused(e),
    };
    let capability = match resolve(ap.capability, cp.capability,
                                  "capability requirement has no single owner") {
        Ok(v) => v,
        Err(e) => return PakVmAdmission::Refused(e),
    };
    let evidence = match resolve(ap.evidence, cp.evidence, "evidence class has no single owner") {
        Ok(v) => v,
        Err(e) => return PakVmAdmission::Refused(e),
    };
    let lowering = match resolve(ap.lowering, None, "lowering posture has no single owner") {
        Ok(v) => v,
        Err(e) => return PakVmAdmission::Refused(e),
    };
    // The semantic ISA holds public semantic nodes only. A SpecializedPlan
    // micro-op or a decision-circuit gate is an execution identity, not a
    // meaning, and cannot acquire one by being listed here.
    let lowering = match lowering {
        CandidateLoweringPosture::PublicSemanticIdentity => AdmittedAsPublicSemanticNode(()),
        CandidateLoweringPosture::InternalLoweringIdentity => {
            return PakVmAdmission::Refused("an internal lowering identity is not a semantic node")
        }
    };
    // docs/07: "A pure query image cannot contain Effect instructions." The two
    // directions are both real. An Effect-algebra node that claimed a pure
    // posture would pass the validator that rejects Effect images; a node outside
    // the Effect algebra that claimed Effectful would smuggle an effect past it.
    let effectful = matches!(effect, EffectPosture::Effectful);
    if effectful != matches!(algebra, PakVmAlgebra::Effect) {
        return PakVmAdmission::Refused("effect posture and algebra disagree");
    }
    // An effect needs the capability its declaration names; nothing else may.
    let needs_effect_capability = matches!(capability, CapabilityRequirement::DeclaredEffectCapability);
    if needs_effect_capability != effectful {
        return PakVmAdmission::Refused("effect capability is required by a node that declares no effect");
    }
    // A node that computes over values in hand cannot reach a host or a source.
    if matches!(effect, EffectPosture::Pure)
        && !matches!(capability, CapabilityRequirement::None) {
        return PakVmAdmission::Refused("a pure node requires a capability");
    }
    // Work-formula LINEAGE. A family must play on its algebra's plane, and must
    // account in the unit that algebra's semantics turn on. Coverage — every unit
    // claimed by someone — cannot catch a family in the wrong plane, because the
    // wrong family still claims its unit perfectly well.
    let plane = algebra.work_plane();
    let units = cp.work_formula.units();
    let mut u = 0;
    while u < units.len() {
        let mut on_plane = false;
        let mut p = 0;
        while p < plane.len() {
            if plane[p] == units[u] {
                on_plane = true;
            }
            p += 1;
        }
        if !on_plane {
            return PakVmAdmission::Refused(
                "work formula accounts in a unit outside its algebra's work plane");
        }
        u += 1;
    }
    if let Some(required) = algebra.mandatory_work_unit() {
        let mut found = false;
        let mut u = 0;
        while u < units.len() {
            if units[u] == required {
                found = true;
            }
            u += 1;
        }
        if !found {
            return PakVmAdmission::Refused(
                "work formula omits the unit its algebra's cost law turns on");
        }
    }
    // Work accounting must agree with iteration. A node that iterates cannot cost
    // one instruction, and a node that does not iterate cannot be bounded by an
    // iteration bound.
    let constant_work = matches!(cp.boundedness, BoundednessPosture::ConstantWork);
    let constant_cost = matches!(cp.work_formula, WorkFormulaFamily::ConstantInstruction);
    if constant_work != constant_cost {
        return PakVmAdmission::Refused("boundedness posture and work formula disagree");
    }
    PakVmAdmission::Admitted(PakVmNodeSpec {
        id,
        authored_name: id.authored_name(),
        algebra,
        class,
        effect,
        operand_sorts: id.operand_sorts(),
        result_sorts: id.result_sorts(),
        boundedness: cp.boundedness,
        capability,
        work_formula: cp.work_formula,
        evidence,
        lowering,
        source_origin: id.source_origin(),
    })
}
