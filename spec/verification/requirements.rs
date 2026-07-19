use super::*;

/// One authored verification requirement: the axes a piece of evidence must
/// match EXACTLY to admit. No axis ranks another and no strength converts.
/// The independent route rides inside `basis`; `enforcement` is authored
/// policy and is never observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationRequirement {
    pub method: VerificationMethod,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub lane: VerificationLane,
    pub enforcement: VerificationEnforcementPosture,
}

/// Why a requirement cannot be admitted as authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementAdmissionError {
    /// The route inside the basis does not match the basis: an independent
    /// reference names a hostile boundary, or a direct boundary names a
    /// non-hostile route.
    RouteKindDoesNotMatchBasis,
    /// Runtime observation covers only executions that occurred.
    RuntimeObservationRequiresObservedHistory,
    /// Observed-history coverage exists only for runtime observation or for an
    /// independent replay of a recorded history.
    ObservedHistoryRequiresReplayOrRuntime,
    /// The runtime lane carries runtime-observation evidence only.
    RuntimeLaneRequiresRuntimeObservation,
    /// Runtime observation executes in the runtime lane only.
    RuntimeObservationRequiresRuntimeLane,
}

/// A requirement that passed admission. Sealed: `admit` is the only
/// constructor, so an incoherent requirement is unrepresentable downstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedVerificationRequirement {
    pub(crate) requirement: VerificationRequirement,
    _seal: (),
}

impl AdmittedVerificationRequirement {
    pub const fn requirement(&self) -> &VerificationRequirement {
        &self.requirement
    }
}

/// Admit an authored requirement, refusing the incoherent shapes. Total,
/// panic-free, and const: every check is a structural match, never a numeric
/// comparison of an axis.
pub const fn admit(
    requirement: VerificationRequirement,
) -> Result<AdmittedVerificationRequirement, RequirementAdmissionError> {
    // Route/basis matching: an independent reference is never a hostile
    // boundary, and a direct boundary's only independent route is hostile.
    match requirement.basis {
        VerificationBasis::IndependentReference { route } => {
            if !matches!(
                route,
                IndependentEvidenceRouteKind::DifferentialImplementation
                    | IndependentEvidenceRouteKind::IndependentHistoryReplay
            ) {
                return Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis);
            }
        }
        VerificationBasis::DirectBoundary { route: Some(kind) } => {
            if !matches!(kind, IndependentEvidenceRouteKind::HostileBoundary) {
                return Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis);
            }
        }
        VerificationBasis::ContractProjection
        | VerificationBasis::DirectBoundary { route: None }
        | VerificationBasis::RuntimeObservation => {}
    }
    if matches!(requirement.basis, VerificationBasis::RuntimeObservation)
        && !matches!(requirement.coverage, VerificationCoverage::ObservedHistory)
    {
        return Err(RequirementAdmissionError::RuntimeObservationRequiresObservedHistory);
    }
    if matches!(requirement.coverage, VerificationCoverage::ObservedHistory)
        && !matches!(
            requirement.basis,
            VerificationBasis::RuntimeObservation
                | VerificationBasis::IndependentReference {
                    route: IndependentEvidenceRouteKind::IndependentHistoryReplay
                }
        )
    {
        return Err(RequirementAdmissionError::ObservedHistoryRequiresReplayOrRuntime);
    }
    if matches!(requirement.lane, VerificationLane::Runtime)
        && !matches!(requirement.basis, VerificationBasis::RuntimeObservation)
    {
        return Err(RequirementAdmissionError::RuntimeLaneRequiresRuntimeObservation);
    }
    if matches!(requirement.basis, VerificationBasis::RuntimeObservation)
        && !matches!(requirement.lane, VerificationLane::Runtime)
    {
        return Err(RequirementAdmissionError::RuntimeObservationRequiresRuntimeLane);
    }
    Ok(AdmittedVerificationRequirement { requirement, _seal: () })
}
