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

/// Which admitted surface may PRODUCE a node (5.5E1 ruling). Orthogonal to
/// algebra and class: this answers "who may author it", not "what it means".
///
/// A public semantic node is invalid only when it has no lawful producer
/// through any admitted surface. The producer path for a node carrying
/// `CanonicalProgramImageOnly` is canonical ProgramImage authoring → ordinary
/// ProgramImage decoding → independent validation → admission. An external
/// compiler or SDK may encode such a node, but authorship grants no trust:
/// admission remains the authority, and compiler provenance bypasses nothing
/// (docs/10, DEC-051; `compiler_emitted_invalid_image_is_rejected`).
///
/// There is deliberately no `ContractLowered` variant: MacBat generates
/// contract vocabulary, not ProgramImages, so no V1 node names such a
/// producer. A variant with no consumer is the GateResolution species; a
/// contract-lowering surface enters this enum together with its first
/// producer, by ruling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeAuthoringSurface {
    /// The frozen BatQL V1 language lowers to this node. The node need not
    /// have its own keyword: `Seek` has no syntax, but `GET` lowers to it
    /// (docs/13 bounded traversal lowering).
    BatQlLowered,
    /// Reachable only as authored canonical ProgramImage bytes. Not part of
    /// the frozen BatQL V1 source grammar; naming it in BatQL's semantic
    /// algebras is a finding unless the language-amendment law admits it.
    CanonicalProgramImageOnly,
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
pub struct AdmittedAsPublicSemanticNode(pub(crate) ());

/// Where one `PakVmNodeSpec` field legitimately comes from. Every admitted field
/// resolves through exactly one of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmRule<T: 'static> {
    /// Invariant for the whole algebra; declared once, never repeated per node.
    AlgebraConstant(T),
    /// Varies within the algebra; the node class declares it.
    ClassDeclared,
}
