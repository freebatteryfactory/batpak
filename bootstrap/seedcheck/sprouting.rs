use spec::{architecture, guarantees, reconciliation, sprouting, verification};
use std::collections::BTreeSet;

/// 5.5F3 (DEC-079..082/DEC-073, docs/39): the typed sprouting plane is
/// executed law. Every candidate-lineage axis is a closed, unordered
/// vocabulary; the promotion-plan admitter enforces the conjunctive
/// PromotionRequirement denominator and the change-class/repair-authority
/// coupling; the candidate search budget is positive by TYPE (NonZeroU64,
/// constructed panic-free); and the specialized-plan candidate policy is
/// reconstructed field by field. Candidate origin records how a candidate was
/// produced, never whether it is admitted — the two are separate axes.
pub(crate) fn check_sprouting(findings: &mut Vec<String>) {
    use core::num::NonZeroU64;
    use spec::promotion::PromotionRequirement;
    use sprouting::{
        admit_promotion_plan, CandidateChangeClass, CandidateOriginKind, CandidatePromotionPlan,
        CandidateSearchBudget, EvaluationSetRole, PromotionPlanError, RealizationPosture,
        RepairAuthority, CANDIDATE_CHANGE_CLASSES, CANDIDATE_ORIGIN_KINDS, EVALUATION_SET_ROLES,
        REALIZATION_POSTURES, REPAIR_AUTHORITIES, SPECIALIZED_PLAN_CANDIDATE_POLICY,
    };
    use verification::IndependentEvidenceRouteKind;
    // Every sprouting vocabulary is closed and exhaustively classified: a new
    // variant must be classified here, never defaulted into legitimacy.
    for kind in CANDIDATE_ORIGIN_KINDS {
        match kind {
            CandidateOriginKind::DeterministicGeneration
            | CandidateOriginKind::BoundedSearch
            | CandidateOriginKind::TransferReuse
            | CandidateOriginKind::HumanAuthored
            | CandidateOriginKind::MachineAssistedSynthesis
            | CandidateOriginKind::RepairOfCandidate => {}
        }
    }
    if CANDIDATE_ORIGIN_KINDS.len() != 6 {
        findings.push(format!(
            "CANDIDATE_ORIGIN_KINDS carries {} entries, expected 6", CANDIDATE_ORIGIN_KINDS.len()));
    }
    for class in CANDIDATE_CHANGE_CLASSES {
        match class {
            CandidateChangeClass::RealizationPreserving | CandidateChangeClass::LawChanging => {}
        }
    }
    if CANDIDATE_CHANGE_CLASSES.len() != 2 {
        findings.push(format!(
            "CANDIDATE_CHANGE_CLASSES carries {} entries, expected 2",
            CANDIDATE_CHANGE_CLASSES.len()));
    }
    for role in EVALUATION_SET_ROLES {
        match role {
            EvaluationSetRole::Search
            | EvaluationSetRole::Qualification
            | EvaluationSetRole::Holdout
            | EvaluationSetRole::Regression => {}
        }
    }
    if EVALUATION_SET_ROLES.len() != 4 {
        findings.push(format!(
            "EVALUATION_SET_ROLES carries {} entries, expected 4", EVALUATION_SET_ROLES.len()));
    }
    for posture in REALIZATION_POSTURES {
        match posture {
            RealizationPosture::Missing
            | RealizationPosture::Scaffold
            | RealizationPosture::Candidate => {}
        }
    }
    if REALIZATION_POSTURES.len() != 3 {
        findings.push(format!(
            "REALIZATION_POSTURES carries {} entries, expected 3", REALIZATION_POSTURES.len()));
    }
    for authority in REPAIR_AUTHORITIES {
        match authority {
            RepairAuthority::Mechanical
            | RepairAuthority::BoundedSearch
            | RepairAuthority::ArchitectRequired => {}
        }
    }
    if REPAIR_AUTHORITIES.len() != 3 {
        findings.push(format!(
            "REPAIR_AUTHORITIES carries {} entries, expected 3", REPAIR_AUTHORITIES.len()));
    }
    // The promotion-plan admitter: a complete realization-preserving plan
    // admits; every refusal arm fires for its own law.
    let green_plan = CandidatePromotionPlan {
        change_class: CandidateChangeClass::RealizationPreserving,
        repair_authority: RepairAuthority::Mechanical,
        independent_route: IndependentEvidenceRouteKind::DifferentialImplementation,
        requirements: PromotionRequirement::ALL,
    };
    if admit_promotion_plan(green_plan).is_err() {
        findings.push("a complete realization-preserving promotion plan failed admission".into());
    }
    // A law-changing candidate with architect authority also admits.
    if admit_promotion_plan(CandidatePromotionPlan {
        change_class: CandidateChangeClass::LawChanging,
        repair_authority: RepairAuthority::ArchitectRequired,
        ..green_plan
    })
    .is_err()
    {
        findings.push("a law-changing candidate with architect authority was refused".into());
    }
    // A law-changing candidate without architect authority is refused.
    if !matches!(
        admit_promotion_plan(CandidatePromotionPlan {
            change_class: CandidateChangeClass::LawChanging,
            repair_authority: RepairAuthority::Mechanical,
            ..green_plan
        }),
        Err(PromotionPlanError::LawChangingRequiresArchitect)
    ) {
        findings.push("a law-changing candidate without architect authority was admitted".into());
    }
    // A realization-preserving candidate may not require an architect.
    if !matches!(
        admit_promotion_plan(CandidatePromotionPlan {
            change_class: CandidateChangeClass::RealizationPreserving,
            repair_authority: RepairAuthority::ArchitectRequired,
            ..green_plan
        }),
        Err(PromotionPlanError::RealizationPreservingCannotRequireArchitect)
    ) {
        findings.push("a realization-preserving candidate requiring an architect was admitted".into());
    }
    // A repeated requirement is refused.
    const DUPLICATE_REQUIREMENTS: &[PromotionRequirement] = &[
        PromotionRequirement::IndependentEvidenceRoute,
        PromotionRequirement::IndependentEvidenceRoute,
        PromotionRequirement::NamedProofTarget,
        PromotionRequirement::QualifiedHostileEvidence,
        PromotionRequirement::AuditablePromotionReceipt,
    ];
    if !matches!(
        admit_promotion_plan(CandidatePromotionPlan {
            requirements: DUPLICATE_REQUIREMENTS, ..green_plan }),
        Err(PromotionPlanError::DuplicateRequirement { .. })
    ) {
        findings.push("a promotion plan repeating a requirement was admitted".into());
    }
    // A plan omitting a required member is refused.
    const MISSING_REQUIREMENTS: &[PromotionRequirement] = &[
        PromotionRequirement::NamedProofTarget,
        PromotionRequirement::QualifiedHostileEvidence,
        PromotionRequirement::AuditablePromotionReceipt,
    ];
    if !matches!(
        admit_promotion_plan(CandidatePromotionPlan {
            requirements: MISSING_REQUIREMENTS, ..green_plan }),
        Err(PromotionPlanError::MissingRequirement { .. })
    ) {
        findings.push("a promotion plan missing a required member was admitted".into());
    }
    // No residual-length arm exists: over the closed requirement enum, a
    // duplicate-free slice containing every ALL member is exactly ALL by
    // pigeonhole — Duplicate and Missing exhaust the set-shape refusals.
    const EMPTY_REQUIREMENTS: &[PromotionRequirement] = &[];
    if !matches!(
        admit_promotion_plan(CandidatePromotionPlan {
            requirements: EMPTY_REQUIREMENTS, ..green_plan }),
        Err(PromotionPlanError::MissingRequirement { .. })
    ) {
        findings.push("an empty promotion-requirement set was admitted".into());
    }
    // The specialized-plan candidate policy reconstructs field by field.
    let policy = SPECIALIZED_PLAN_CANDIDATE_POLICY;
    if policy.semantic_owner.raw() != "BP-PAKVM-ISA-1" {
        findings.push(format!(
            "the specialized-plan candidate policy names owner {}, not BP-PAKVM-ISA-1",
            policy.semantic_owner.raw()));
    }
    match policy.admission_basis {
        guarantees::GuaranteeRef::Decision(id) if id.raw() == "DEC-073" => {}
        other => findings.push(format!(
            "the specialized-plan candidate policy admission basis is {other:?}, not DEC-073")),
    }
    if policy.change_class != CandidateChangeClass::RealizationPreserving {
        findings.push("the specialized-plan candidate policy is not RealizationPreserving".into());
    }
    if policy.independent_route != IndependentEvidenceRouteKind::DifferentialImplementation {
        findings.push(
            "the specialized-plan candidate policy route is not DifferentialImplementation".into());
    }
    if policy.allowed_origins.len() != CANDIDATE_ORIGIN_KINDS.len() {
        findings.push(format!(
            "the specialized-plan candidate policy allows {} origins, not all {}",
            policy.allowed_origins.len(), CANDIDATE_ORIGIN_KINDS.len()));
    }
    for kind in CANDIDATE_ORIGIN_KINDS {
        if !policy.allowed_origins.contains(kind) {
            findings.push(format!(
                "the specialized-plan candidate policy omits candidate origin {kind:?}"));
        }
    }
    if policy.requirements.len() != PromotionRequirement::ALL.len() {
        findings.push(
            "the specialized-plan candidate policy requirement set disagrees with the promotion denominator".into());
    }
    for requirement in PromotionRequirement::ALL {
        if !policy.requirements.contains(requirement) {
            findings.push(format!(
                "the specialized-plan candidate policy omits promotion requirement {requirement:?}"));
        }
    }
    // The candidate search budget is positive by TYPE: NonZeroU64 forbids a
    // zero ceiling, and construction is panic-free (new + let-else, never
    // unwrap/expect).
    let (
        Some(max_candidates),
        Some(max_logical_work),
        Some(max_memory_bytes),
        Some(max_monotonic_ticks),
    ) = (
        NonZeroU64::new(1024),
        NonZeroU64::new(1_000_000),
        NonZeroU64::new(64 * 1024 * 1024),
        NonZeroU64::new(10_000),
    ) else {
        findings.push("a positive candidate-search-budget bound was rejected by NonZeroU64::new".into());
        return;
    };
    let budget = CandidateSearchBudget {
        max_candidates,
        max_logical_work,
        max_memory_bytes,
        max_monotonic_ticks,
    };
    for (label, bound) in [
        ("max_candidates", budget.max_candidates),
        ("max_logical_work", budget.max_logical_work),
        ("max_memory_bytes", budget.max_memory_bytes),
        ("max_monotonic_ticks", budget.max_monotonic_ticks),
    ] {
        if bound.get() == 0 {
            findings.push(format!(
                "candidate search budget {label} is zero; NonZeroU64 must forbid it"));
        }
    }
}

/// DEC-058 (5.5E1): the release seal binds one typed inventory. Three
/// hand-authored lists disagreed about kernel receipts and SBOM evidence
/// until this enum existed; the projections now derive from it.
pub(crate) fn check_release_seal(findings: &mut Vec<String>) {
    use architecture::ReleaseSealField as F;
    let mut seen = BTreeSet::new();
    for field in architecture::RELEASE_SEAL_FIELDS {
        // Exhaustive: a new field must be classified here, not defaulted.
        match field {
            F::SourceTree | F::Toolchain | F::DependencyGraph | F::GeneratedFacts
            | F::CompatibilityCorpus | F::TestDispositions | F::MutationDispositions
            | F::FuzzDispositions | F::BenchmarkDispositions | F::CompilerAssumptionLedger
            | F::DependencyLedger | F::KernelQualificationSet | F::PackageContents
            | F::PublicApi | F::Sbom | F::LicenseEvidence | F::ProofFreshness => {}
        }
        if !seen.insert(format!("{field:?}")) {
            findings.push(format!("release-seal field {field:?} is listed twice"));
        }
    }
    if seen.len() != 17 {
        findings.push(format!(
            "a release-seal field is missing from RELEASE_SEAL_FIELDS ({} of 17)", seen.len()));
    }
    if !seen.contains("KernelQualificationSet") {
        findings.push("the kernel qualification set left the seal; an empty set \
                       states 'no kernels admitted', it never disappears".into());
    }
}

/// DEC-075: the reconciliation composition stays closed, total, and honest.
/// The fence assertions at the end are the law's teeth: they exercise the
/// real `admissible()` the way production would, so weakening the
/// classification reddens an executed check, not a comment.
pub(crate) fn check_reconciliation(findings: &mut Vec<String>) {
    use reconciliation::{ReconciliationRole, RetrySignal};
    let mut roles = BTreeSet::new();
    for c in reconciliation::RECONCILIATION_COORDINATES {
        // Exhaustive: a new role must be classified here, not defaulted.
        match c.role {
            ReconciliationRole::DurableOrderWitness
            | ReconciliationRole::ChronologyWitness
            | ReconciliationRole::LogicalIdentity
            | ReconciliationRole::PhysicalIdentity
            | ReconciliationRole::BalancingEvidence => {}
        }
        if !roles.insert(format!("{:?}", c.role)) {
            findings.push(format!("reconciliation role {:?} is bound twice", c.role));
        }
        if c.carriers.is_empty() {
            findings.push(format!("reconciliation role {:?} names no carrier", c.role));
        }
        if c.law.trim().is_empty() {
            findings.push(format!("reconciliation role {:?} states no law", c.role));
        }
    }
    if roles.len() != 5 {
        findings.push(format!("a reconciliation role is unbound ({} of 5)", roles.len()));
    }
    let mut axes = BTreeSet::new();
    for a in reconciliation::DOUBLE_ENTRY_AXES {
        if !axes.insert(format!("{a:?}")) {
            findings.push(format!("double-entry axis {a:?} is listed twice"));
        }
    }
    if axes.len() != 10 {
        findings.push(format!("a double-entry axis is missing from DOUBLE_ENTRY_AXES ({} of 10)", axes.len()));
    }
    let mut sigs = BTreeSet::new();
    for s in reconciliation::RETRY_SIGNALS {
        if !sigs.insert(format!("{s:?}")) {
            findings.push(format!("retry signal {s:?} is listed twice"));
        }
    }
    if sigs.len() != 11 {
        findings.push(format!("a retry signal is missing from RETRY_SIGNALS ({} of 11)", sigs.len()));
    }
    if !reconciliation::RETRY_SIGNALS.iter().any(|s| s.admissible()) {
        findings.push("no retry signal is admissible; retry would be impossible".into());
    }
    if RetrySignal::ElapsedWallTime.admissible() {
        findings.push("elapsed wall time may not authorize retry (DEC-075)".into());
    }
    if RetrySignal::ProcessDeath.admissible() {
        findings.push("process death may not authorize retry (DEC-075)".into());
    }
    if RetrySignal::MissingAcknowledgement.admissible() {
        findings.push("a missing acknowledgement requires reconciliation, not retry (DEC-075)".into());
    }
    if RetrySignal::MissingInMemoryWaiter.admissible() {
        findings.push("a missing in-memory waiter may not authorize retry (DEC-075)".into());
    }
}

/// Candidate material stays disposable (spec/architecture.rs): the declared
/// output root must live in the untracked target/ tree and may never sit under
/// a surface candidates are forbidden to write. The two constants state one
/// law, and until this check existed no executing code consumed the output
/// root at all — the containment relation between them was never enforced.
pub(crate) fn check_candidate_containment(findings: &mut Vec<String>) {
    let out = architecture::CANDIDATE_OUTPUT_ROOT;
    if !out.starts_with("target/") {
        findings.push(format!("candidate output root {out} escapes the untracked target/ tree"));
    }
    for forbidden in architecture::CANDIDATE_FORBIDDEN_WRITE_ROOTS {
        if out.starts_with(forbidden) {
            findings.push(format!("candidate output root {out} sits under forbidden write root {forbidden}"));
        }
    }
}

/// The authenticated-history claim contract (DEC-071).
///
/// Structural only: this performs no signature, accumulator, witness, freshness,
/// or rollback verification of any kind, and proves nothing about cryptography.
/// It proves the CONTRACT cannot be weakened.
pub(crate) fn check_authenticated_history(findings: &mut Vec<String>) {
    use architecture::{
        AuthenticatedHistoryProfile, AuthenticityClaim, FreshnessClaim, IntegrityClaim,
        RollbackResistanceClaim, WitnessDisposition, WitnessPolicy,
    };
    // 5.5E2: the profile table dissolved into const fns on the profile enum,
    // and most of the ladder policing that stood here dissolved with it —
    // duplicate rows, row counts, an always-true bool, and per-row gate
    // presence have no spelling once every fact is a total function of the
    // variant. What remains is the AUTHORED semantics, executed through the
    // real fns so a mutated arm reddens the running binary.
    for profile in AuthenticatedHistoryProfile::ALL {
        let profile = *profile;
        // Exhaustive: a new profile must be classified here, not defaulted.
        match profile {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => {}
        }
        for policy in profile.permitted_witness_policies() {
            match policy {
                WitnessPolicy::None | WitnessPolicy::Optional | WitnessPolicy::Required => {}
            }
            if *policy == WitnessPolicy::Required
                && profile != AuthenticatedHistoryProfile::ExternallyAnchoredHistory
            {
                findings.push(format!(
                    "{profile:?} permits WitnessPolicy::Required outside ExternallyAnchoredHistory"
                ));
            }
        }
        // A success bundle states all four axes. Every success verifies
        // integrity, and freshness never drifts from rollback resistance.
        for bundle in [
            profile.unanchored_success_claims(),
            profile.verified_witness_success_claims(),
        ]
        .into_iter()
        .flatten()
        {
            match bundle.integrity {
                IntegrityClaim::InternalConsistencyVerified => {}
            }
            match bundle.authenticity {
                AuthenticityClaim::NotClaimed | AuthenticityClaim::SignedHistoryVerified => {}
            }
            match bundle.freshness {
                FreshnessClaim::NotClaimed | FreshnessClaim::WitnessedGenerationVerified => {}
            }
            match bundle.rollback_resistance {
                RollbackResistanceClaim::Unavailable
                | RollbackResistanceClaim::ScopedToVerifiedWitness => {}
            }
            let fresh = bundle.freshness == FreshnessClaim::WitnessedGenerationVerified;
            let scoped =
                bundle.rollback_resistance == RollbackResistanceClaim::ScopedToVerifiedWitness;
            if fresh != scoped {
                findings.push(format!(
                    "{profile:?} lets freshness and rollback resistance drift apart"
                ));
            }
        }
        // An unanchored success never claims freshness or scoped rollback
        // resistance: a restored older validly signed history satisfies it.
        if let Some(bundle) = profile.unanchored_success_claims() {
            if bundle.freshness != FreshnessClaim::NotClaimed
                || bundle.rollback_resistance != RollbackResistanceClaim::Unavailable
            {
                findings.push(format!(
                    "{profile:?} unanchored success claims freshness or rollback resistance"
                ));
            }
            if profile == AuthenticatedHistoryProfile::InternalConsistency
                && bundle.authenticity != AuthenticityClaim::NotClaimed
            {
                findings.push("InternalConsistency unanchored success claims signed authenticity".into());
            }
        }
        // `None` here means NO SUCCESSFUL UNANCHORED RESULT IS ADMITTED.
        if profile == AuthenticatedHistoryProfile::ExternallyAnchoredHistory
            && profile.unanchored_success_claims().is_some()
        {
            findings.push(
                "ExternallyAnchoredHistory admits an unanchored success bundle; an absent or invalid \
                 required witness must refuse, not fall back to a weaker success"
                    .into(),
            );
        }
        if profile != AuthenticatedHistoryProfile::InternalConsistency
            && profile.verified_witness_success_claims().is_none()
        {
            findings.push(format!("{profile:?} admits no witnessed success bundle"));
        }
        // The requirement fns and the policy matrix may not disagree: a
        // profile requiring an independent witness admits only Required, and
        // a profile requiring signed history is never InternalConsistency.
        if profile.requires_independent_witness_verification()
            != matches!(
                profile.permitted_witness_policies(),
                [WitnessPolicy::Required]
            )
        {
            findings.push(format!(
                "{profile:?} witness requirement disagrees with its permitted policies"
            ));
        }
        if !profile.requires_local_commitment_verification() {
            findings.push(format!(
                "{profile:?} does not require local commitment verification"
            ));
        }
        if profile.requires_signed_history_verification()
            != (profile != AuthenticatedHistoryProfile::InternalConsistency)
        {
            findings.push(format!(
                "{profile:?} signed-history requirement disagrees with the frozen family"
            ));
        }
        if profile.implementation_gates().is_empty()
            || profile.release_qualification_gates().is_empty()
        {
            findings.push(format!("{profile:?} names no implementation or release gate"));
        }
    }
    // A required witness fails closed on every frozen failure class, including
    // one that was never supplied.
    for disposition in architecture::REQUIRED_WITNESS_FAILURE_SET {
        match disposition {
            WitnessDisposition::NotApplicable | WitnessDisposition::Verified => {
                findings.push(format!(
                    "REQUIRED_WITNESS_FAILURE_SET contains the non-failure {disposition:?}"
                ));
            }
            _ => {}
        }
    }
    if !architecture::REQUIRED_WITNESS_FAILURE_SET.contains(&WitnessDisposition::NotProvided) {
        findings.push("a required witness that was never supplied is not a refusal".into());
    }
    // Optional: optional to supply, mandatory to validate once supplied.
    if architecture::OPTIONAL_WITNESS_REFUSAL_SET.contains(&WitnessDisposition::NotProvided) {
        findings.push("OPTIONAL_WITNESS_REFUSAL_SET refuses an absent optional witness".into());
    }
    for disposition in architecture::REQUIRED_WITNESS_FAILURE_SET {
        if *disposition != WitnessDisposition::NotProvided
            && !architecture::OPTIONAL_WITNESS_REFUSAL_SET.contains(disposition)
        {
            findings.push(format!(
                "a supplied {disposition:?} optional witness may degrade to absence or success"
            ));
        }
    }
    if architecture::REFUSAL_PARTIAL_CLAIM_LAW.trim().is_empty() {
        findings.push("no refusal/partial-evidence law is stated".into());
    }
}

