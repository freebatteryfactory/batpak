use spec::{proof, verification};

/// 5.5F3 (DEC-077/DEC-078/DEC-075, docs/38): the typed verification plane is
/// executed law, not declared vocabulary. This check runs the runtime
/// mode/result matrix over EVERY combination, exercises every
/// requirement-admission and observation-admission refusal arm through the
/// sealed constructors (including the newly lawful
/// IndependentReference{IndependentHistoryReplay}+ObservedHistory pairing),
/// executes the assess_plan laws — an advisory failure never blocks and
/// stays positive, a quarantine failure makes the row non-positive, a
/// blocking failure is reported before any witness disagreement, and the
/// intra-family and cross-family DEC-075 juries refuse a row rather than
/// letting a generated projection overrule an independent reference — and
/// admits every runtime-verification error arm plus the green paths, with a
/// non-empty law/owner/repair for each error.
pub(crate) fn check_verification(findings: &mut Vec<String>) {
    use verification::{
        admit, admit_observation, admit_runtime_verification, assess_plan,
        IndependentEvidenceRouteKind, ObservationAdmissionError, RequirementAdmissionError,
        RequirementEvidenceObservation, RowAssessmentError, RuntimeVerificationDisposition,
        RuntimeVerificationError, RuntimeVerificationMode, RuntimeVerificationObservation,
        VerificationBasis, VerificationClaimKind, VerificationCoverage,
        VerificationEnforcementPosture, VerificationLane, VerificationMethod,
        VerificationRequirement, INDEPENDENT_EVIDENCE_ROUTE_KINDS,
        RUNTIME_VERIFICATION_DISPOSITIONS, RUNTIME_VERIFICATION_MODES, VERIFICATION_CLAIM_KINDS,
        VERIFICATION_COVERAGES, VERIFICATION_ENFORCEMENT_POSTURES, VERIFICATION_LANES,
        VERIFICATION_METHODS,
    };
    // Closed inventories are complete and duplicate-free. VERIFICATION_BASES is
    // gone (the variants carry route payloads now), so basis completeness is
    // proven by exhaustive classification, not a const length.
    let counts = [
        ("VERIFICATION_METHODS", VERIFICATION_METHODS.len(), 15),
        ("VERIFICATION_CLAIM_KINDS", VERIFICATION_CLAIM_KINDS.len(), 10),
        ("VERIFICATION_COVERAGES", VERIFICATION_COVERAGES.len(), 4),
        ("VERIFICATION_ENFORCEMENT_POSTURES", VERIFICATION_ENFORCEMENT_POSTURES.len(), 3),
        ("VERIFICATION_LANES", VERIFICATION_LANES.len(), 5),
        ("INDEPENDENT_EVIDENCE_ROUTE_KINDS", INDEPENDENT_EVIDENCE_ROUTE_KINDS.len(), 3),
        ("RUNTIME_VERIFICATION_MODES", RUNTIME_VERIFICATION_MODES.len(), 3),
        ("RUNTIME_VERIFICATION_DISPOSITIONS", RUNTIME_VERIFICATION_DISPOSITIONS.len(), 8),
    ];
    for (name, got, want) in counts {
        if got != want {
            findings.push(format!("{name} carries {got} entries, expected {want}"));
        }
    }
    // The basis vocabulary is closed; a new variant must be classified here.
    for basis in [
        VerificationBasis::ContractProjection,
        VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::DifferentialImplementation,
        },
        VerificationBasis::DirectBoundary { route: None },
        VerificationBasis::RuntimeObservation,
    ] {
        match basis {
            VerificationBasis::ContractProjection
            | VerificationBasis::IndependentReference { .. }
            | VerificationBasis::DirectBoundary { .. }
            | VerificationBasis::RuntimeObservation => {}
        }
    }
    // The exact runtime mode/result matrix, every combination classified.
    use RuntimeVerificationDisposition as D;
    use RuntimeVerificationMode as M;
    for mode in RUNTIME_VERIFICATION_MODES {
        for disposition in RUNTIME_VERIFICATION_DISPOSITIONS {
            let admitted = mode.admits(*disposition);
            let lawful = match (mode, disposition) {
                (M::PreventiveGuard, D::GuardAdmitted | D::GuardRefused | D::Unsupported) => true,
                (M::InBandObservation,
                 D::NoDivergenceObserved | D::Divergent | D::Incomplete | D::Stale
                 | D::Unsupported) => true,
                (M::OfflineReplay,
                 D::ConformantForObservedHistory | D::Divergent | D::Incomplete | D::Stale
                 | D::Unsupported) => true,
                _ => false,
            };
            if admitted != lawful {
                findings.push(format!(
                    "runtime matrix drift: {mode:?} + {disposition:?} admits()={admitted}, law says {lawful}"));
            }
        }
    }
    // An admissible authored requirement: a hostile direct-boundary probe.
    let base = VerificationRequirement {
        method: VerificationMethod::PropertySequence,
        basis: VerificationBasis::DirectBoundary {
            route: Some(IndependentEvidenceRouteKind::HostileBoundary),
        },
        coverage: VerificationCoverage::Sampled,
        lane: VerificationLane::Merge,
        enforcement: VerificationEnforcementPosture::Blocking,
    };
    // Admission refuses every incoherent route/basis and coverage/lane shape.
    // An IndependentReference may not carry the hostile-boundary route.
    if !matches!(
        admit(VerificationRequirement {
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::HostileBoundary,
            },
            ..base
        }),
        Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis)
    ) {
        findings.push("an IndependentReference carrying the hostile-boundary route was admitted".into());
    }
    // A DirectBoundary route, when present, is the hostile-boundary route only.
    if !matches!(
        admit(VerificationRequirement {
            basis: VerificationBasis::DirectBoundary {
                route: Some(IndependentEvidenceRouteKind::DifferentialImplementation),
            },
            ..base
        }),
        Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis)
    ) {
        findings.push("a DirectBoundary carrying a non-hostile route was admitted".into());
    }
    // Runtime observation requires observed-history coverage.
    if !matches!(
        admit(VerificationRequirement {
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Runtime,
            ..base
        }),
        Err(RequirementAdmissionError::RuntimeObservationRequiresObservedHistory)
    ) {
        findings.push("runtime observation admitted without observed-history coverage".into());
    }
    // Observed-history coverage is lawful only with runtime observation or an
    // independent history replay; a direct boundary may not carry it.
    if !matches!(
        admit(VerificationRequirement {
            coverage: VerificationCoverage::ObservedHistory,
            ..base
        }),
        Err(RequirementAdmissionError::ObservedHistoryRequiresReplayOrRuntime)
    ) {
        findings.push("observed-history coverage admitted on a direct-boundary basis".into());
    }
    // The runtime lane carries runtime-observation evidence only.
    if !matches!(
        admit(VerificationRequirement { lane: VerificationLane::Runtime, ..base }),
        Err(RequirementAdmissionError::RuntimeLaneRequiresRuntimeObservation)
    ) {
        findings.push("the runtime lane admitted a non-runtime-observation requirement".into());
    }
    // Runtime observation executes in the runtime lane only.
    if !matches!(
        admit(VerificationRequirement {
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::ObservedHistory,
            lane: VerificationLane::Merge,
            ..base
        }),
        Err(RequirementAdmissionError::RuntimeObservationRequiresRuntimeLane)
    ) {
        findings.push("runtime observation admitted outside the runtime lane".into());
    }
    // The NEW lawful pairing: an independent history replay may cover observed
    // history outside the runtime lane.
    if admit(VerificationRequirement {
        method: VerificationMethod::HistoryReplay,
        basis: VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
        },
        coverage: VerificationCoverage::ObservedHistory,
        lane: VerificationLane::Merge,
        enforcement: VerificationEnforcementPosture::Blocking,
    })
    .is_err()
    {
        findings.push(
            "the lawful IndependentReference{IndependentHistoryReplay}+ObservedHistory pairing was refused".into(),
        );
    }
    let Ok(admitted) = admit(base) else {
        findings.push("a coherent requirement failed admission".into());
        return;
    };
    // Observation admission: exact match on every axis, refusal on any defect.
    let matching = RequirementEvidenceObservation {
        method: base.method,
        basis: base.basis,
        coverage: base.coverage,
        lane: base.lane,
        executed: true,
        fresh: true,
        artifacts_bound: true,
        counterexample_outstanding: false,
        terminal: proof::ProofUnitTerminal::Passed,
    };
    if admit_observation(&admitted, &matching).is_err() {
        findings.push("exactly-matching evidence failed observation admission".into());
    }
    use ObservationAdmissionError as OAE;
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation {
            method: VerificationMethod::Fuzzing, ..matching }),
        Err(OAE::MethodMismatch { .. })
    ) {
        findings.push("a method mismatch was admitted".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation {
            basis: VerificationBasis::ContractProjection, ..matching }),
        Err(OAE::BasisMismatch)
    ) {
        findings.push("a basis mismatch was admitted".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation {
            coverage: VerificationCoverage::ExhaustiveWithinDeclaredModel, ..matching }),
        Err(OAE::CoverageMismatch { .. })
    ) {
        findings.push("exhaustive evidence satisfied a sampled requirement".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation {
            lane: VerificationLane::PullRequestFast, ..matching }),
        Err(OAE::LaneMismatch { .. })
    ) {
        findings.push("a faster lane satisfied the declared lane".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation { executed: false, ..matching }),
        Err(OAE::NotExecuted)
    ) {
        findings.push("an unexecuted plan was admitted".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation { fresh: false, ..matching }),
        Err(OAE::Stale)
    ) {
        findings.push("stale evidence was admitted".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation { artifacts_bound: false, ..matching }),
        Err(OAE::ArtifactsUnbound)
    ) {
        findings.push("unbound artifacts were admitted".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation {
            counterexample_outstanding: true, ..matching }),
        Err(OAE::CounterexampleOutstanding)
    ) {
        findings.push("an outstanding counterexample was admitted".into());
    }
    if !matches!(
        admit_observation(&admitted, &RequirementEvidenceObservation {
            terminal: proof::ProofUnitTerminal::Expired, ..matching }),
        Err(OAE::InconclusiveTerminal { .. })
    ) {
        findings.push("an inconclusive terminal was admitted".into());
    }
    // assess_plan laws. A fully matching single-requirement plan assesses
    // positive with one satisfied requirement and no failures.
    match assess_plan(VerificationClaimKind::Safety, &[base], &[matching]) {
        Ok(assessment)
            if assessment.satisfied() == 1
                && assessment.advisory_failures() == 0
                && assessment.quarantine_failures() == 0
                && assessment.is_positive() => {}
        other => findings.push(format!("a fully satisfied plan did not assess positive: {other:?}")),
    }
    // An advisory requirement whose observation fails never blocks and leaves
    // the row positive; the failure stays visible as an advisory count.
    let advisory_req = VerificationRequirement {
        enforcement: VerificationEnforcementPosture::Advisory,
        ..base
    };
    let axis_failing = RequirementEvidenceObservation { method: VerificationMethod::Fuzzing, ..matching };
    match assess_plan(VerificationClaimKind::Safety, &[advisory_req], &[axis_failing]) {
        Ok(assessment)
            if assessment.advisory_failures() == 1
                && assessment.quarantine_failures() == 0
                && assessment.is_positive() => {}
        other => findings.push(format!(
            "an advisory failure did not stay non-blocking and positive: {other:?}")),
    }
    // A quarantine requirement whose observation fails is not advisory: the
    // row is no longer positive.
    let quarantine_req = VerificationRequirement {
        enforcement: VerificationEnforcementPosture::Quarantine,
        ..base
    };
    match assess_plan(VerificationClaimKind::Safety, &[quarantine_req], &[axis_failing]) {
        Ok(assessment)
            if assessment.quarantine_failures() == 1
                && assessment.advisory_failures() == 0
                && !assessment.is_positive() => {}
        other => findings.push(format!(
            "a quarantine failure was mis-recorded or wrongly stayed positive: {other:?}")),
    }
    // Witness plans for the DEC-075 juries.
    let projection_req = VerificationRequirement {
        basis: VerificationBasis::ContractProjection,
        ..base
    };
    let independent_req = VerificationRequirement {
        method: VerificationMethod::HistoryReplay,
        basis: VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
        },
        ..base
    };
    let projection_pass = RequirementEvidenceObservation {
        basis: VerificationBasis::ContractProjection,
        terminal: proof::ProofUnitTerminal::Passed,
        ..matching
    };
    let independent_fail = RequirementEvidenceObservation {
        method: VerificationMethod::HistoryReplay,
        basis: VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
        },
        terminal: proof::ProofUnitTerminal::Failed,
        ..matching
    };
    // Cross-family: a generated projection says Passed, the independent
    // reference says Failed. The row refuses; it never trusts the projection.
    match assess_plan(
        VerificationClaimKind::Determinism,
        &[independent_req, projection_req],
        &[independent_fail, projection_pass],
    ) {
        Err(RowAssessmentError::ProjectionDisagreesWithIndependentReference { .. }) => {}
        other => findings.push(format!(
            "planted projection/independent disagreement did not refuse the row: {other:?}")),
    }
    // Intra-family: two projection witnesses that disagree refuse the row.
    let projection_req_b = VerificationRequirement {
        basis: VerificationBasis::ContractProjection,
        method: VerificationMethod::StructuralRule,
        ..base
    };
    let projection_fail_b = RequirementEvidenceObservation {
        basis: VerificationBasis::ContractProjection,
        method: VerificationMethod::StructuralRule,
        terminal: proof::ProofUnitTerminal::Failed,
        ..matching
    };
    match assess_plan(
        VerificationClaimKind::Safety,
        &[projection_req, projection_req_b],
        &[projection_pass, projection_fail_b],
    ) {
        Err(RowAssessmentError::ProjectionWitnessesDisagree { .. }) => {}
        other => findings.push(format!(
            "two disagreeing projection witnesses did not refuse the row: {other:?}")),
    }
    // Intra-family: two independent witnesses that disagree refuse the row.
    let independent_req_b = VerificationRequirement {
        method: VerificationMethod::DifferentialExecution,
        basis: VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::DifferentialImplementation,
        },
        ..base
    };
    let independent_pass_b = RequirementEvidenceObservation {
        method: VerificationMethod::DifferentialExecution,
        basis: VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::DifferentialImplementation,
        },
        terminal: proof::ProofUnitTerminal::Passed,
        ..matching
    };
    match assess_plan(
        VerificationClaimKind::Refinement,
        &[independent_req, independent_req_b],
        &[independent_fail, independent_pass_b],
    ) {
        Err(RowAssessmentError::IndependentWitnessesDisagree { .. }) => {}
        other => findings.push(format!(
            "two disagreeing independent witnesses did not refuse the row: {other:?}")),
    }
    // A blocking failure is reported before any witness disagreement: an
    // invalid blocking observation at index 0 preempts the jury.
    let bad_blocking = RequirementEvidenceObservation { method: VerificationMethod::Fuzzing, ..matching };
    match assess_plan(
        VerificationClaimKind::Determinism,
        &[base, independent_req, projection_req],
        &[bad_blocking, independent_fail, projection_pass],
    ) {
        Err(RowAssessmentError::BlockingRequirementFailed { index: 0, .. }) => {}
        other => findings.push(format!(
            "a blocking failure was not reported before witness disagreement: {other:?}")),
    }
    // The route-kind vocabulary is closed by adoption: every kind is consumed
    // by at least one active proof-row plan today, read out of the basis.
    for kind in INDEPENDENT_EVIDENCE_ROUTE_KINDS {
        let adopted = proof::PROOF_ROWS.iter().any(|record| match record.state {
            proof::ProofRowState::Active { verification, .. } => verification.iter().any(|requirement| {
                let route = match requirement.basis {
                    VerificationBasis::IndependentReference { route } => Some(route),
                    VerificationBasis::DirectBoundary { route } => route,
                    VerificationBasis::ContractProjection
                    | VerificationBasis::RuntimeObservation => None,
                };
                route == Some(*kind)
            }),
            proof::ProofRowState::Retired { .. } => false,
        });
        if !adopted {
            findings.push(format!(
                "IndependentEvidenceRouteKind::{kind:?} has no active proof-row adopter; \
                 a route kind arrives with its adopter, never before"));
        }
    }
    // Every active row's claim stays within the closed vocabulary.
    for record in proof::PROOF_ROWS {
        if let proof::ProofRowState::Active { claim, .. } = record.state {
            let _named: VerificationClaimKind = claim;
        }
    }
    // The runtime admission law, executed across the green paths, every error
    // arm, and the non-empty law/owner/repair projections.
    let guard_ok = RuntimeVerificationObservation {
        mode: RuntimeVerificationMode::PreventiveGuard,
        disposition: RuntimeVerificationDisposition::GuardAdmitted,
        basis: VerificationBasis::DirectBoundary {
            route: Some(IndependentEvidenceRouteKind::HostileBoundary),
        },
        coverage: VerificationCoverage::Sampled,
        fresh: true,
        history_complete: true,
    };
    for (observation, law) in [
        (guard_ok, "preventive guard admit"),
        (RuntimeVerificationObservation {
            disposition: RuntimeVerificationDisposition::GuardRefused, ..guard_ok },
         "preventive guard refuse"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::ObservedHistory,
            ..guard_ok },
         "in-band no-divergence"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            ..guard_ok },
         "offline replay conformant for a fresh complete history"),
    ] {
        if admit_runtime_verification(observation).is_err() {
            findings.push(format!("a lawful runtime verification was refused: {law}"));
        }
    }
    use RuntimeVerificationError as RVE;
    let runtime_refusals: [(RuntimeVerificationObservation, RVE, &str); 8] = [
        (RuntimeVerificationObservation {
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved, ..guard_ok },
         RVE::DispositionNotAdmittedByMode, "a guard emitting an in-band disposition"),
        (RuntimeVerificationObservation {
            basis: VerificationBasis::RuntimeObservation, ..guard_ok },
         RVE::GuardRequiresDirectBoundary, "a guard off a non-boundary basis"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::ObservedHistory,
            ..guard_ok },
         RVE::InBandRequiresRuntimeObservation, "in-band off a non-runtime basis"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::Sampled,
            ..guard_ok },
         RVE::InBandRequiresObservedHistory, "in-band without observed history"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::Divergent,
            coverage: VerificationCoverage::ObservedHistory,
            ..guard_ok },
         RVE::ReplayRequiresIndependentHistoryReplay, "replay off a non-replay basis"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::Divergent,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::Sampled,
            ..guard_ok },
         RVE::ReplayRequiresObservedHistory, "replay without observed history"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            ..guard_ok },
         RVE::ConformanceRequiresFreshHistory, "conformance on a stale history"),
        (RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            history_complete: false,
            ..guard_ok },
         RVE::ConformanceRequiresCompleteHistory, "conformance on an incomplete history"),
    ];
    for (observation, expected, law) in runtime_refusals {
        match admit_runtime_verification(observation) {
            Err(error) if error == expected => {}
            other => findings.push(format!(
                "runtime verification law failed ({law}): expected {expected:?}, got {other:?}")),
        }
    }
    for error in [
        RVE::DispositionNotAdmittedByMode,
        RVE::GuardRequiresDirectBoundary,
        RVE::InBandRequiresRuntimeObservation,
        RVE::InBandRequiresObservedHistory,
        RVE::ReplayRequiresIndependentHistoryReplay,
        RVE::ReplayRequiresObservedHistory,
        RVE::ConformanceRequiresFreshHistory,
        RVE::ConformanceRequiresCompleteHistory,
    ] {
        if error.law().trim().is_empty() {
            findings.push(format!("runtime verification error {error:?} states no law"));
        }
        if error.owner().raw().trim().is_empty() {
            findings.push(format!("runtime verification error {error:?} names no owner"));
        }
        if error.repair().trim().is_empty() {
            findings.push(format!("runtime verification error {error:?} states no repair"));
        }
    }
}

