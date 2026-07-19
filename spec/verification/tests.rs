use crate::guarantees::ContractId;
use crate::proof::ProofUnitTerminal;

    use super::*;

    const fn base_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::PropertySequence,
            basis: VerificationBasis::DirectBoundary {
                route: Some(IndependentEvidenceRouteKind::HostileBoundary),
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn matching_observation() -> RequirementEvidenceObservation {
        RequirementEvidenceObservation {
            method: VerificationMethod::PropertySequence,
            basis: VerificationBasis::DirectBoundary {
                route: Some(IndependentEvidenceRouteKind::HostileBoundary),
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal: ProofUnitTerminal::Passed,
        }
    }

    #[test]
    fn matching_observation_admits() {
        let admitted = admit(base_requirement()).expect("coherent requirement admits");
        let observed =
            admit_observation(&admitted, &matching_observation()).expect("exact match admits");
        assert_eq!(observed.terminal(), ProofUnitTerminal::Passed);
        assert_eq!(observed.requirement().method, VerificationMethod::PropertySequence);
    }

    #[test]
    fn route_kind_must_match_basis() {
        // An independent reference is never a hostile boundary.
        let mut hostile_reference = base_requirement();
        hostile_reference.basis = VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::HostileBoundary,
        };
        assert_eq!(
            admit(hostile_reference),
            Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis)
        );
        // A direct boundary's only independent route is hostile.
        let mut differential_boundary = base_requirement();
        differential_boundary.basis = VerificationBasis::DirectBoundary {
            route: Some(IndependentEvidenceRouteKind::DifferentialImplementation),
        };
        assert_eq!(
            admit(differential_boundary),
            Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis)
        );
    }

    #[test]
    fn lawful_route_and_basis_pairings_admit() {
        let mut differential_reference = base_requirement();
        differential_reference.method = VerificationMethod::DifferentialExecution;
        differential_reference.basis = VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::DifferentialImplementation,
        };
        assert!(admit(differential_reference).is_ok());

        let mut replay_reference = base_requirement();
        replay_reference.method = VerificationMethod::HistoryReplay;
        replay_reference.basis = VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
        };
        assert!(admit(replay_reference).is_ok());

        let mut own_boundary = base_requirement();
        own_boundary.basis = VerificationBasis::DirectBoundary { route: None };
        assert!(admit(own_boundary).is_ok());

        let mut projection = base_requirement();
        projection.basis = VerificationBasis::ContractProjection;
        assert!(admit(projection).is_ok());
    }

    #[test]
    fn runtime_observation_pairs_only_with_observed_history_and_runtime_lane() {
        let mut requirement = base_requirement();
        requirement.basis = VerificationBasis::RuntimeObservation;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::RuntimeObservationRequiresObservedHistory)
        );
        requirement.coverage = VerificationCoverage::ObservedHistory;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::RuntimeObservationRequiresRuntimeLane)
        );
        requirement.lane = VerificationLane::Runtime;
        assert!(admit(requirement).is_ok());
    }

    #[test]
    fn observed_history_is_lawful_only_for_runtime_or_independent_replay() {
        // A direct boundary cannot carry observed-history coverage.
        let mut boundary = base_requirement();
        boundary.coverage = VerificationCoverage::ObservedHistory;
        assert_eq!(
            admit(boundary),
            Err(RequirementAdmissionError::ObservedHistoryRequiresReplayOrRuntime)
        );
        // An independent history replay is the newly lawful non-runtime pairing.
        let replay = VerificationRequirement {
            method: VerificationMethod::HistoryReplay,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        };
        assert!(admit(replay).is_ok());
    }

    #[test]
    fn runtime_lane_requires_runtime_observation() {
        let mut requirement = base_requirement();
        requirement.lane = VerificationLane::Runtime;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::RuntimeLaneRequiresRuntimeObservation)
        );
    }

    #[test]
    fn exhaustive_observation_does_not_satisfy_a_sampled_requirement() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut observation = matching_observation();
        observation.coverage = VerificationCoverage::ExhaustiveWithinDeclaredModel;
        assert!(matches!(
            admit_observation(&admitted, &observation),
            Err(ObservationAdmissionError::CoverageMismatch { .. })
        ));
    }

    #[test]
    fn faster_lane_does_not_satisfy_the_declared_lane() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut observation = matching_observation();
        observation.lane = VerificationLane::PullRequestFast;
        assert!(matches!(
            admit_observation(&admitted, &observation),
            Err(ObservationAdmissionError::LaneMismatch { .. })
        ));
    }

    #[test]
    fn method_and_basis_mismatches_are_refused() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut wrong_method = matching_observation();
        wrong_method.method = VerificationMethod::StructuralRule;
        assert!(matches!(
            admit_observation(&admitted, &wrong_method),
            Err(ObservationAdmissionError::MethodMismatch { .. })
        ));
        let mut wrong_basis = matching_observation();
        wrong_basis.basis = VerificationBasis::ContractProjection;
        assert_eq!(
            admit_observation(&admitted, &wrong_basis),
            Err(ObservationAdmissionError::BasisMismatch)
        );
    }

    #[test]
    fn conclusive_verdict_alone_is_not_admission() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut stale = matching_observation();
        stale.fresh = false;
        assert_eq!(
            admit_observation(&admitted, &stale),
            Err(ObservationAdmissionError::Stale)
        );
        let mut unexecuted = matching_observation();
        unexecuted.executed = false;
        assert_eq!(
            admit_observation(&admitted, &unexecuted),
            Err(ObservationAdmissionError::NotExecuted)
        );
        let mut unbound = matching_observation();
        unbound.artifacts_bound = false;
        assert_eq!(
            admit_observation(&admitted, &unbound),
            Err(ObservationAdmissionError::ArtifactsUnbound)
        );
        let mut contested = matching_observation();
        contested.counterexample_outstanding = true;
        assert_eq!(
            admit_observation(&admitted, &contested),
            Err(ObservationAdmissionError::CounterexampleOutstanding)
        );
    }

    #[test]
    fn non_verdict_terminal_is_refused() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut expired = matching_observation();
        expired.terminal = ProofUnitTerminal::Expired;
        assert!(matches!(
            admit_observation(&admitted, &expired),
            Err(ObservationAdmissionError::InconclusiveTerminal { .. })
        ));
    }

    #[test]
    fn empty_and_duplicate_plans_are_refused() {
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &[], &[]),
            Err(RowAssessmentError::EmptyPlan)
        );
        let plan = [base_requirement(), base_requirement()];
        assert!(matches!(
            assess_plan(
                VerificationClaimKind::Safety,
                &plan,
                &[matching_observation(), matching_observation()]
            ),
            Err(RowAssessmentError::DuplicateRequirement { index: 1 })
        ));
    }

    #[test]
    fn observation_count_must_match_plan() {
        let plan = [base_requirement()];
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &plan, &[]),
            Err(RowAssessmentError::ObservationCountMismatch { required: 1, presented: 0 })
        );
    }

    #[test]
    fn full_row_assesses_when_every_observation_admits() {
        let plan = [base_requirement()];
        let assessment =
            assess_plan(VerificationClaimKind::Safety, &plan, &[matching_observation()])
                .expect("matching row assesses");
        assert_eq!(assessment.claim(), VerificationClaimKind::Safety);
        assert_eq!(assessment.satisfied(), 1);
        assert_eq!(assessment.advisory_failures(), 0);
        assert_eq!(assessment.quarantine_failures(), 0);
        assert!(assessment.is_positive());
    }

    #[test]
    fn advisory_failure_does_not_block() {
        let mut requirement = base_requirement();
        requirement.enforcement = VerificationEnforcementPosture::Advisory;
        let plan = [requirement];
        let mut observation = matching_observation();
        observation.executed = false;
        let assessment = assess_plan(VerificationClaimKind::ResourceEnvelope, &plan, &[observation])
            .expect("an advisory failure never blocks");
        assert_eq!(assessment.satisfied(), 0);
        assert_eq!(assessment.advisory_failures(), 1);
        assert_eq!(assessment.quarantine_failures(), 0);
        assert!(assessment.is_positive());
    }

    #[test]
    fn quarantine_failure_is_not_advisory_and_is_not_positive() {
        let mut requirement = base_requirement();
        requirement.enforcement = VerificationEnforcementPosture::Quarantine;
        let plan = [requirement];
        let mut observation = matching_observation();
        observation.executed = false;
        let assessment = assess_plan(VerificationClaimKind::Refinement, &plan, &[observation])
            .expect("a quarantine failure is recorded, not returned");
        assert_eq!(assessment.satisfied(), 0);
        assert_eq!(assessment.advisory_failures(), 0);
        assert_eq!(assessment.quarantine_failures(), 1);
        assert!(!assessment.is_positive());
    }

    #[test]
    fn blocking_failure_is_returned() {
        let plan = [base_requirement()];
        let mut observation = matching_observation();
        observation.executed = false;
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &plan, &[observation]),
            Err(RowAssessmentError::BlockingRequirementFailed {
                index: 0,
                error: ObservationAdmissionError::NotExecuted,
            })
        );
    }

    // A projection requirement carrying a distinct method, so two of them are
    // not a duplicate plan entry.
    const fn projection_requirement(method: VerificationMethod) -> VerificationRequirement {
        VerificationRequirement {
            method,
            basis: VerificationBasis::ContractProjection,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn projection_observation(
        method: VerificationMethod,
        terminal: ProofUnitTerminal,
    ) -> RequirementEvidenceObservation {
        RequirementEvidenceObservation {
            method,
            basis: VerificationBasis::ContractProjection,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal,
        }
    }

    const fn replay_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::HistoryReplay,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn differential_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::DifferentialExecution,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::DifferentialImplementation,
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn independent_observation(
        requirement: VerificationRequirement,
        terminal: ProofUnitTerminal,
    ) -> RequirementEvidenceObservation {
        RequirementEvidenceObservation {
            method: requirement.method,
            basis: requirement.basis,
            coverage: requirement.coverage,
            lane: requirement.lane,
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal,
        }
    }

    #[test]
    fn two_projection_witnesses_that_disagree_refuse() {
        let plan = [
            projection_requirement(VerificationMethod::StructuralRule),
            projection_requirement(VerificationMethod::PropertySequence),
        ];
        let observations = [
            projection_observation(VerificationMethod::StructuralRule, ProofUnitTerminal::Passed),
            projection_observation(VerificationMethod::PropertySequence, ProofUnitTerminal::Failed),
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &plan, &observations),
            Err(RowAssessmentError::ProjectionWitnessesDisagree {
                first: ProofUnitTerminal::Passed,
                second: ProofUnitTerminal::Failed,
            })
        );
    }

    #[test]
    fn two_independent_witnesses_that_disagree_refuse() {
        let differential = differential_requirement();
        let replay = replay_requirement();
        let plan = [differential, replay];
        let observations = [
            independent_observation(differential, ProofUnitTerminal::Passed),
            independent_observation(replay, ProofUnitTerminal::Failed),
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Refinement, &plan, &observations),
            Err(RowAssessmentError::IndependentWitnessesDisagree {
                first: ProofUnitTerminal::Passed,
                second: ProofUnitTerminal::Failed,
            })
        );
    }

    #[test]
    fn projection_disagreeing_with_independent_reference_refuses() {
        // The DEC-075 planted-disagreement law: the generated projection says
        // Passed, the independent reference says Failed — the row refuses and
        // never chooses the projection.
        let replay = replay_requirement();
        let plan = [
            projection_requirement(VerificationMethod::StructuralRule),
            replay,
        ];
        let observations = [
            projection_observation(VerificationMethod::StructuralRule, ProofUnitTerminal::Passed),
            independent_observation(replay, ProofUnitTerminal::Failed),
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Determinism, &plan, &observations),
            Err(RowAssessmentError::ProjectionDisagreesWithIndependentReference {
                projection: ProofUnitTerminal::Passed,
                independent: ProofUnitTerminal::Failed,
            })
        );
    }

    #[test]
    fn a_blocking_defect_is_reported_before_any_disagreement() {
        // A genuine cross-family disagreement (projection Passed vs independent
        // Failed) coexists with an invalid blocking observation. Every
        // observation is validated first, and the blocking defect is reported
        // instead of the disagreement: invalid evidence is never a witness.
        let replay = replay_requirement();
        let plan = [
            projection_requirement(VerificationMethod::StructuralRule),
            replay,
            base_requirement(),
        ];
        let mut defective = matching_observation();
        defective.executed = false;
        let observations = [
            projection_observation(VerificationMethod::StructuralRule, ProofUnitTerminal::Passed),
            independent_observation(replay, ProofUnitTerminal::Failed),
            defective,
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Determinism, &plan, &observations),
            Err(RowAssessmentError::BlockingRequirementFailed {
                index: 2,
                error: ObservationAdmissionError::NotExecuted,
            })
        );
    }

    #[test]
    fn runtime_matrix_is_exact() {
        use RuntimeVerificationDisposition as D;
        use RuntimeVerificationMode as M;
        // The three refused pairings are the law.
        assert!(!M::InBandObservation.admits(D::ConformantForObservedHistory));
        assert!(!M::PreventiveGuard.admits(D::ConformantForObservedHistory));
        assert!(!M::OfflineReplay.admits(D::NoDivergenceObserved));
        // A preventive guard needs no replay before refusing.
        assert!(M::PreventiveGuard.admits(D::GuardRefused));
        assert!(M::PreventiveGuard.admits(D::GuardAdmitted));
        // In-band can honestly report a trace-scoped absence of divergence.
        assert!(M::InBandObservation.admits(D::NoDivergenceObserved));
        // Only offline replay concludes conformance, for observed history only.
        assert!(M::OfflineReplay.admits(D::ConformantForObservedHistory));
        // Guard dispositions belong to the guard alone.
        assert!(!M::InBandObservation.admits(D::GuardAdmitted));
        assert!(!M::OfflineReplay.admits(D::GuardRefused));
    }

    #[test]
    fn runtime_admission_green_paths() {
        // A preventive guard against a direct boundary admits.
        let guard = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::PreventiveGuard,
            disposition: RuntimeVerificationDisposition::GuardAdmitted,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert!(admit_runtime_verification(guard).is_ok());
        // An in-band no-divergence observation over observed history admits.
        let in_band = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: false,
        };
        assert!(admit_runtime_verification(in_band).is_ok());
        // Offline replay concludes conformance only with a fresh, complete history.
        let replay = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: true,
            history_complete: true,
        };
        assert!(admit_runtime_verification(replay).is_ok());
    }

    #[test]
    fn runtime_admission_refuses_every_error_arm() {
        // (a) The mode does not admit the disposition.
        let mismatch = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::PreventiveGuard,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(mismatch),
            Err(RuntimeVerificationError::DispositionNotAdmittedByMode)
        );
        // (b) A guard against a non-boundary basis.
        let guard_basis = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::PreventiveGuard,
            disposition: RuntimeVerificationDisposition::GuardAdmitted,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(guard_basis),
            Err(RuntimeVerificationError::GuardRequiresDirectBoundary)
        );
        // (c) In-band against a non-runtime basis.
        let in_band_basis = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(in_band_basis),
            Err(RuntimeVerificationError::InBandRequiresRuntimeObservation)
        );
        // (c) In-band without observed-history coverage.
        let in_band_coverage = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(in_band_coverage),
            Err(RuntimeVerificationError::InBandRequiresObservedHistory)
        );
        // (d) Replay against a non-replay basis.
        let replay_basis = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::Divergent,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(replay_basis),
            Err(RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay)
        );
        // (d) Replay without observed-history coverage.
        let replay_coverage = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::Divergent,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(replay_coverage),
            Err(RuntimeVerificationError::ReplayRequiresObservedHistory)
        );
        // (e) Conformance without a fresh history.
        let stale_conformance = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: true,
        };
        assert_eq!(
            admit_runtime_verification(stale_conformance),
            Err(RuntimeVerificationError::ConformanceRequiresFreshHistory)
        );
        // (e) Conformance without a complete history.
        let incomplete_conformance = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: true,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(incomplete_conformance),
            Err(RuntimeVerificationError::ConformanceRequiresCompleteHistory)
        );
    }

    #[test]
    fn runtime_error_law_owner_and_repair_are_stated_for_every_arm() {
        let arms = [
            RuntimeVerificationError::DispositionNotAdmittedByMode,
            RuntimeVerificationError::GuardRequiresDirectBoundary,
            RuntimeVerificationError::InBandRequiresRuntimeObservation,
            RuntimeVerificationError::InBandRequiresObservedHistory,
            RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay,
            RuntimeVerificationError::ReplayRequiresObservedHistory,
            RuntimeVerificationError::ConformanceRequiresFreshHistory,
            RuntimeVerificationError::ConformanceRequiresCompleteHistory,
        ];
        for arm in arms {
            assert!(!arm.law().is_empty());
            assert!(!arm.repair().is_empty());
            assert_eq!(arm.owner(), ContractId("BP-DYNAMIC-VERIFICATION-1"));
        }
    }
