    use super::*;

    fn refusal(a: PakVmAdmission) -> &'static str {
        match a {
            PakVmAdmission::Refused(why) => why,
            PakVmAdmission::Admitted(_) => "ADMITTED",
        }
    }

    // The premise every probe below depends on: the node exists, its real policy
    // admits, and each mutation therefore changes a live outcome rather than a
    // dead one. A red light from an already-broken node would prove nothing.
    #[test]
    fn the_probe_targets_admit_before_mutation() {
        for &node in PAKVM_NODES {
            assert!(matches!(admit(node), PakVmAdmission::Admitted(_)), "{node:?}");
        }
    }

    #[test]
    fn an_internal_lowering_identity_is_not_a_semantic_node() {
        // A SpecializedPlan micro-op wearing a semantic node's identity.
        let ap = PakVmAlgebraPolicy {
            lowering: PakVmRule::AlgebraConstant(CandidateLoweringPosture::InternalLoweringIdentity),
            ..*algebra_policy(PakVmAlgebra::QueryDataflow).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::RowTransform).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "an internal lowering identity is not a semantic node");
    }

    #[test]
    fn two_owners_for_one_field_is_refused() {
        // The algebra fixes the effect posture AND the class declares one.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            effect: Some(EffectPosture::ObservationalOnly),
            ..*class_policy(PakVmNodeClass::RowTransform).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "effect posture has no single owner");
    }

    #[test]
    fn no_owner_for_one_field_is_refused() {
        // The algebra delegates and the class stays silent. Admission must fail
        // rather than let a downstream projector pick a value.
        let ap = PakVmAlgebraPolicy {
            effect: PakVmRule::ClassDeclared,
            ..*algebra_policy(PakVmAlgebra::QueryDataflow).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::RowTransform).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "effect posture has no single owner");
    }

    #[test]
    fn an_effect_node_claiming_purity_is_refused() {
        // Would slip past the validator that rejects Effect nodes in a pure
        // query image (docs/07, DEC-050).
        let ap = PakVmAlgebraPolicy {
            effect: PakVmRule::AlgebraConstant(EffectPosture::Pure),
            capability: PakVmRule::AlgebraConstant(CapabilityRequirement::None),
            ..*algebra_policy(PakVmAlgebra::Effect).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::DurableAppend).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "effect posture and algebra disagree");
    }

    #[test]
    fn a_query_node_claiming_effectfulness_is_refused() {
        // The other direction: smuggling an effect into the query algebra.
        let ap = PakVmAlgebraPolicy {
            effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),
            ..*algebra_policy(PakVmAlgebra::QueryDataflow).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::RowTransform).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "effect posture and algebra disagree");
    }

    #[test]
    fn an_effect_without_its_declared_capability_is_refused() {
        let ap = PakVmAlgebraPolicy {
            capability: PakVmRule::AlgebraConstant(CapabilityRequirement::ReadOnlySourceEnvelope),
            ..*algebra_policy(PakVmAlgebra::Effect).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::DurableAppend).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "effect capability is required by a node that declares no effect");
    }

    #[test]
    fn a_pure_node_requiring_a_capability_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::FormulaDecision).unwrap();
        let cp = PakVmNodeClassPolicy {
            capability: Some(CapabilityRequirement::ReadOnlySourceEnvelope),
            ..*class_policy(PakVmNodeClass::ScalarComputation).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Compare, &ap, &cp)),
                   "a pure node requires a capability");
    }

    #[test]
    fn a_cost_law_disagreeing_with_iteration_is_refused() {
        // The mutation must stay ON its algebra's work plane, or the plane rule
        // fires first and this probe stops testing the rule it names: a red light
        // from the wrong severed wire is not evidence the alarm works.
        //
        // KernelCostContract is on the formula/decision plane and carries the
        // mandatory Instructions unit, so it clears every earlier check. Giving it
        // to ScalarComputation, whose boundedness is ConstantWork, leaves a node
        // that claims one interpreted step while costing a kernel's contract.
        let ap = *algebra_policy(PakVmAlgebra::FormulaDecision).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::KernelCostContract,
            ..*class_policy(PakVmNodeClass::ScalarComputation).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Compare, &ap, &cp)),
                   "boundedness posture and work formula disagree");
    }

    #[test]
    fn an_iterating_node_costing_one_instruction_is_refused() {
        // A scan declaring constant cost would make bounded traversal
        // unmeasurable: the budget could never bite. Two independent laws reject
        // it, and the work plane is the one that reaches it first â€” Instructions
        // is not a query/dataflow unit at all.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::ConstantInstruction,
            ..*class_policy(PakVmNodeClass::SourceTraversal).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Scan, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn a_class_policy_filed_under_a_foreign_algebra_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            algebra: PakVmAlgebra::Effect,
            ..*class_policy(PakVmNodeClass::RowTransform).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "node class policy names a different algebra");
    }

    #[test]
    fn every_authored_work_unit_is_accounted_by_some_family() {
        for unit in [WorkUnit::Instructions, WorkUnit::Rows, WorkUnit::DecodedBytes,
                     WorkUnit::TileBytes, WorkUnit::Groups, WorkUnit::Matches,
                     WorkUnit::Outputs, WorkUnit::Artifacts, WorkUnit::Effects,
                     WorkUnit::CallDepth] {
            assert!(PAKVM_NODES.iter().any(|&n| match admit(n) {
                PakVmAdmission::Admitted(s) => s.work_formula.units().contains(&unit),
                PakVmAdmission::Refused(_) => false,
            }), "{unit:?} is accounted by no node family");
        }
    }

    #[test]
    fn the_authored_algebra_populations_match_docs_07() {
        let count = |a: PakVmAlgebra| PAKVM_NODES.iter().filter(|n| n.algebra() == a).count();
        assert_eq!(count(PakVmAlgebra::FormulaDecision), 16);
        assert_eq!(count(PakVmAlgebra::QueryDataflow), 14);
        assert_eq!(count(PakVmAlgebra::Effect), 6);
        assert_eq!(PAKVM_NODES.len(), 36);
    }

    // --- work-formula LINEAGE, not mere coverage -------------------------------
    // Every one of these mutations ADMITTED before the work-plane law existed.
    // "All ten units are claimed" was satisfied in each case: the wrong family
    // still claims its unit. Coverage proves no bucket is empty, never that
    // anything is in the right bucket.

    #[test]
    fn windowing_accounted_by_returned_output_is_refused() {
        // The exact defect LEG-028's page_limit_bounds_discovery_work_not_only_output
        // exists to reject: bounding the returned vector instead of discovery.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::EmittedOutputs,
            ..*class_policy(PakVmNodeClass::Windowing).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Page, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn an_effect_node_given_a_query_cost_law_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::Effect).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::Rows,
            ..*class_policy(PakVmNodeClass::DurableAppend).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn an_effect_node_given_a_pure_query_family_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::Effect).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::ConstantInstruction,
            ..*class_policy(PakVmNodeClass::DurableAppend).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn a_query_node_given_an_unrelated_but_real_unit_is_refused() {
        // Artifacts exist in the authored vocabulary â€” StageArtifact produces
        // them â€” so this mutation is plausible and coverage-clean.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::StagedArtifacts,
            ..*class_policy(PakVmNodeClass::RowTransform).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn a_formula_node_given_a_row_cost_law_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::FormulaDecision).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::Rows,
            ..*class_policy(PakVmNodeClass::KernelInvocation).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::KernelCall, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn the_three_work_planes_partition_the_authored_vocabulary() {
        // Each unit belongs to exactly one algebra. If a unit sat on two planes,
        // the plane rule would admit a family in the wrong one; if it sat on
        // none, no family could ever account in it.
        let all = [WorkUnit::Instructions, WorkUnit::Rows, WorkUnit::DecodedBytes,
                   WorkUnit::TileBytes, WorkUnit::Groups, WorkUnit::Matches,
                   WorkUnit::Outputs, WorkUnit::Artifacts, WorkUnit::Effects,
                   WorkUnit::CallDepth];
        let planes = [PakVmAlgebra::FormulaDecision, PakVmAlgebra::QueryDataflow,
                      PakVmAlgebra::Effect];
        for unit in all {
            let owners = planes.iter().filter(|a| a.work_plane().contains(&unit)).count();
            assert_eq!(owners, 1, "{unit:?} is claimed by {owners} work planes");
        }
        let total: usize = planes.iter().map(|a| a.work_plane().len()).sum();
        assert_eq!(total, all.len());
    }

    #[test]
    fn canonical_tables_cannot_produce_the_forbidden_state() {
        // The seam proves refusals fire; this proves the real specification never
        // needs them. Every canonical algebra policy proposes a public identity.
        for algebra in [PakVmAlgebra::FormulaDecision, PakVmAlgebra::QueryDataflow,
                        PakVmAlgebra::Effect] {
            let ap = algebra_policy(algebra).unwrap();
            assert!(matches!(ap.lowering,
                PakVmRule::AlgebraConstant(CandidateLoweringPosture::PublicSemanticIdentity)),
                "{algebra:?} proposes a non-public lowering identity");
        }
    }

    #[test]
    fn removing_the_seam_would_not_change_production_admission() {
        // admit() must agree with the seam fed the canonical tables. If it did
        // not, the fixtures would be proving something production never runs.
        for &node in PAKVM_NODES {
            let ap = algebra_policy(node.algebra()).unwrap();
            let cp = class_policy(node.class()).unwrap();
            assert_eq!(admit(node), admit_candidate_policy(node, ap, cp), "{node:?}");
        }
    }
