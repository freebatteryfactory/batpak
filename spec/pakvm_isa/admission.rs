use super::*;

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
