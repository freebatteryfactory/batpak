use crate::guarantees::ContractId;
use super::*;

/// A claimed runtime verification result, before the runtime admission law
/// checks it against its mode. The route rides inside `basis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeVerificationObservation {
    pub mode: RuntimeVerificationMode,
    pub disposition: RuntimeVerificationDisposition,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub fresh: bool,
    pub history_complete: bool,
}

/// A runtime verification observation that passed the runtime admission law.
/// Sealed: `admit_runtime_verification` is the only constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedRuntimeVerification {
    observation: RuntimeVerificationObservation,
    _seal: (),
}

impl AdmittedRuntimeVerification {
    pub const fn observation(&self) -> &RuntimeVerificationObservation {
        &self.observation
    }
}

/// Why a claimed runtime verification is refused. Every arm names the runtime
/// law it violated; the owner is the dynamic-verification contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeVerificationError {
    /// The mode does not admit the claimed disposition (the `admits` matrix).
    DispositionNotAdmittedByMode,
    /// A preventive guard probes the real boundary directly.
    GuardRequiresDirectBoundary,
    /// An in-band observation records from the executing system.
    InBandRequiresRuntimeObservation,
    /// An in-band observation covers only the history it observed.
    InBandRequiresObservedHistory,
    /// Offline replay is an independent re-execution of a recorded history.
    ReplayRequiresIndependentHistoryReplay,
    /// Offline replay covers only the observed history it re-executes.
    ReplayRequiresObservedHistory,
    /// Conformance for an observed history requires the history to be current.
    ConformanceRequiresFreshHistory,
    /// Conformance for an observed history requires the history to be complete.
    ConformanceRequiresCompleteHistory,
}

impl RuntimeVerificationError {
    /// The violated law, stated as the requirement.
    pub const fn law(self) -> &'static str {
        match self {
            Self::DispositionNotAdmittedByMode => {
                "a runtime verification carries only a disposition its mode admits"
            }
            Self::GuardRequiresDirectBoundary => {
                "a preventive guard verifies against a direct boundary"
            }
            Self::InBandRequiresRuntimeObservation => {
                "an in-band observation records from the executing runtime"
            }
            Self::InBandRequiresObservedHistory => {
                "an in-band observation covers only its observed history"
            }
            Self::ReplayRequiresIndependentHistoryReplay => {
                "offline replay verifies through an independent history replay"
            }
            Self::ReplayRequiresObservedHistory => {
                "offline replay covers only the observed history it re-executes"
            }
            Self::ConformanceRequiresFreshHistory => {
                "conformance for an observed history requires a fresh history"
            }
            Self::ConformanceRequiresCompleteHistory => {
                "conformance for an observed history requires a complete history"
            }
        }
    }

    /// The typed owner of the violated law.
    pub const fn owner(self) -> ContractId {
        ContractId("BP-DYNAMIC-VERIFICATION-1")
    }

    /// The repair direction, stated on the offending observation's terms.
    pub const fn repair(self) -> &'static str {
        match self {
            Self::DispositionNotAdmittedByMode => {
                "state a disposition the mode admits, or the mode that admits this disposition"
            }
            Self::GuardRequiresDirectBoundary => {
                "record the guard against a direct-boundary basis"
            }
            Self::InBandRequiresRuntimeObservation => {
                "record the in-band observation against a runtime-observation basis"
            }
            Self::InBandRequiresObservedHistory => {
                "record the in-band observation with observed-history coverage"
            }
            Self::ReplayRequiresIndependentHistoryReplay => {
                "route the replay through an independent-history-replay reference"
            }
            Self::ReplayRequiresObservedHistory => {
                "record the replay with observed-history coverage"
            }
            Self::ConformanceRequiresFreshHistory => {
                "refresh the observed history before claiming conformance"
            }
            Self::ConformanceRequiresCompleteHistory => {
                "complete the observed history before claiming conformance"
            }
        }
    }
}

/// Admit a claimed runtime verification against the runtime law. Total,
/// panic-free, and const: every check is a structural match.
pub const fn admit_runtime_verification(
    observation: RuntimeVerificationObservation,
) -> Result<AdmittedRuntimeVerification, RuntimeVerificationError> {
    // (a) The mode must admit the disposition.
    if !observation.mode.admits(observation.disposition) {
        return Err(RuntimeVerificationError::DispositionNotAdmittedByMode);
    }
    match observation.mode {
        // (b) A preventive guard probes a direct boundary (any route posture).
        RuntimeVerificationMode::PreventiveGuard => {
            if !matches!(observation.basis, VerificationBasis::DirectBoundary { .. }) {
                return Err(RuntimeVerificationError::GuardRequiresDirectBoundary);
            }
        }
        // (c) An in-band observation is a runtime observation over observed history.
        RuntimeVerificationMode::InBandObservation => {
            if !matches!(observation.basis, VerificationBasis::RuntimeObservation) {
                return Err(RuntimeVerificationError::InBandRequiresRuntimeObservation);
            }
            if !matches!(observation.coverage, VerificationCoverage::ObservedHistory) {
                return Err(RuntimeVerificationError::InBandRequiresObservedHistory);
            }
        }
        // (d) Offline replay is an independent history replay over observed history.
        RuntimeVerificationMode::OfflineReplay => {
            if !matches!(
                observation.basis,
                VerificationBasis::IndependentReference {
                    route: IndependentEvidenceRouteKind::IndependentHistoryReplay
                }
            ) {
                return Err(RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay);
            }
            if !matches!(observation.coverage, VerificationCoverage::ObservedHistory) {
                return Err(RuntimeVerificationError::ReplayRequiresObservedHistory);
            }
            // (e) Conformance additionally requires a fresh, complete history.
            if matches!(
                observation.disposition,
                RuntimeVerificationDisposition::ConformantForObservedHistory
            ) {
                if !observation.fresh {
                    return Err(RuntimeVerificationError::ConformanceRequiresFreshHistory);
                }
                if !observation.history_complete {
                    return Err(RuntimeVerificationError::ConformanceRequiresCompleteHistory);
                }
            }
        }
    }
    Ok(AdmittedRuntimeVerification { observation, _seal: () })
}
