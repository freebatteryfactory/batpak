use super::*;

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
