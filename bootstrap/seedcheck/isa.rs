use spec::{architecture, pakvm_isa, syncbat_firewall};

/// The semantic ISA admits every authored node, and admits nothing else.
///
/// This runs the SAME `pakvm_isa::admit` the specification declares. It does not
/// re-derive the answer here: a checker that recomputed the policy would be a
/// second owner of it, which is the defect this whole pass exists to remove.
pub(crate) fn check_pakvm_isa(findings: &mut Vec<String>) {
    use pakvm_isa::*;
    let mut admitted = 0usize;
    for &node in PAKVM_NODES {
        match admit(node) {
            PakVmAdmission::Admitted(spec) => {
                admitted += 1;
                // A projector may serialize an admitted spec; it may never
                // complete one. Every string field must therefore already say
                // something.
                for (field, value) in [
                    ("authored name", spec.authored_name),
                    ("operand sorts", spec.operand_sorts),
                    ("result sorts", spec.result_sorts),
                    ("source origin", spec.source_origin),
                ] {
                    if value.is_empty() {
                        findings.push(format!("PakVM node {:?} admits with an empty {field}", node));
                    }
                }
                if spec.work_formula.units().is_empty() {
                    findings.push(format!(
                        "PakVM node {:?} admits with a work formula accounting in no unit", node));
                }
            }
            PakVmAdmission::Refused(why) => {
                findings.push(format!("PakVM node {:?} does not admit: {why}", node));
            }
        }
    }
    if admitted != PAKVM_NODES.len() {
        findings.push(format!(
            "PakVM semantic ISA admitted {admitted} of {} authored nodes", PAKVM_NODES.len()));
    }
    // Identity is symbolic and unique. A duplicate id or a duplicate authored
    // name would let two meanings share one node.
    for i in 0..PAKVM_NODES.len() {
        for j in (i + 1)..PAKVM_NODES.len() {
            if PAKVM_NODES[i] == PAKVM_NODES[j] {
                findings.push(format!("PakVM node {:?} is listed twice", PAKVM_NODES[i]));
            }
            if PAKVM_NODES[i].authored_name() == PAKVM_NODES[j].authored_name() {
                findings.push(format!(
                    "PakVM nodes {:?} and {:?} share the authored name {:?}",
                    PAKVM_NODES[i], PAKVM_NODES[j], PAKVM_NODES[i].authored_name()));
            }
        }
    }
    // Every authored work unit is claimed by some node family. A unit nothing
    // accounts in means either the unit list or the node inventory is wrong, and
    // silently carrying it would hide which.
    for unit in [WorkUnit::Instructions, WorkUnit::Rows, WorkUnit::DecodedBytes,
                 WorkUnit::TileBytes, WorkUnit::Groups, WorkUnit::Matches, WorkUnit::Outputs,
                 WorkUnit::Artifacts, WorkUnit::Effects, WorkUnit::CallDepth] {
        let claimed = PAKVM_NODES.iter().any(|&n| match admit(n) {
            PakVmAdmission::Admitted(s) => s.work_formula.units().contains(&unit),
            PakVmAdmission::Refused(_) => false,
        });
        if !claimed {
            findings.push(format!(
                "authored work unit {:?} is accounted by no PakVM node family", unit));
        }
    }
}

/// The SyncBat authority firewall is complete and closed.
///
/// Runs the same `admit_crossing` the specification declares. It does not
/// recompute the legal-crossing table here: a checker carrying its own copy would
/// be a second owner of the law, and two copies of one guess agree perfectly.
pub(crate) fn check_syncbat_firewall(findings: &mut Vec<String>) {
    use architecture::SyncBatPlane;
    use syncbat_firewall::*;

    // Every authority has exactly one owning plane, and every plane owns at
    // least one authority. A plane owning nothing is not a plane; an authority
    // owned by nobody has no one to refuse its misuse.
    for &authority in SYNCBAT_AUTHORITIES {
        let owner = authority.owner();
        if !SyncBatPlane::ALL.contains(&owner) {
            findings.push(format!("{authority:?} is owned by a plane outside SyncBat"));
        }
    }
    for &plane in SyncBatPlane::ALL {
        if !SYNCBAT_AUTHORITIES.iter().any(|a| a.owner() == plane) {
            findings.push(format!("SyncBat plane {plane:?} owns no authority"));
        }
        if plane.package() != architecture::PackageId::SyncBat {
            findings.push(format!("SyncBat plane {plane:?} claims package {}", plane.package().cargo_name()));
        }
        if plane.authored_ownership().is_empty() {
            findings.push(format!("SyncBat plane {plane:?} names no authored ownership"));
        }
    }
    for i in 0..SYNCBAT_AUTHORITIES.len() {
        for j in (i + 1)..SYNCBAT_AUTHORITIES.len() {
            if SYNCBAT_AUTHORITIES[i] == SYNCBAT_AUTHORITIES[j] {
                findings.push(format!("authority {:?} is listed twice", SYNCBAT_AUTHORITIES[i]));
            }
        }
    }

    // Every declared crossing must itself be lawful: it must move an authority
    // its sender owns, between two distinct planes, and admit through the very
    // function production calls. A table entry that its own admission refuses
    // would be law nobody could obey.
    for crossing in SYNCBAT_LEGAL_CROSSINGS {
        if crossing.law.is_empty() {
            findings.push(format!(
                "the {:?} -> {:?} crossing names no authored law", crossing.from, crossing.to));
        }
        if crossing.carries.owner() != crossing.from {
            findings.push(format!(
                "the declared {:?} -> {:?} crossing carries {:?}, which {:?} does not own",
                crossing.from, crossing.to, crossing.carries, crossing.from));
            continue;
        }
        let origin = if crossing.carries.requires_semantic_origin() {
            Some(semantic_origin_for(crossing.carries))
        } else {
            None
        };
        match admit_crossing(crossing.from, crossing.to, crossing.carries, origin) {
            CrossingAdmission::Admitted(_) => {}
            CrossingAdmission::Refused(why) => findings.push(format!(
                "the declared {:?} -> {:?} crossing does not admit: {why}",
                crossing.from, crossing.to)),
        }
    }

    // A semantic authority names its PakVM origin BECAUSE it leaves the plane
    // that owns the ISA. One that no crossing carries would require an origin
    // for a value no other plane can ever receive: the requirement becomes
    // ceremony and the execution route the ISA depends on stops existing.
    //
    // This is the only rule that notices a lawful crossing being deleted. Every
    // loop above iterates the whitelist, so removing an entry does not fail
    // them -- it simply gives them less to say, which is exactly how a law
    // disappears while its proofs stay green.
    // The carrier must be REQUIRED, not merely present. A semantic route that is
    // deleted OR quietly downgraded to Optional both fail here: the ISA depends
    // on the route existing in the accepted machine, and Optional means "may be
    // absent".
    for &authority in SYNCBAT_AUTHORITIES {
        if authority.requires_semantic_origin()
            && !SYNCBAT_LEGAL_CROSSINGS.iter().any(|c| {
                c.carries == authority && c.posture == CrossingPosture::Required
            })
        {
            findings.push(format!(
                "{authority:?} must name a PakVM origin but no required crossing carries it off \
                 {:?}, so admitted node meaning has no lawful route out of its own plane",
                authority.owner()));
        }
    }

    check_syncbat_requiredness(findings);

    // docs/08 names five forbidden crossings by example. Each must be refused
    // through the production API, and the refusal must be the intended one.
    for (what, from, to, carries, want) in [
        // "PakVM advancing a durable checkpoint"
        ("pakvm advances a durable checkpoint", SyncBatPlane::PakVm, SyncBatPlane::Port,
         SyncBatAuthority::PhysicalAttempt,
         "the sending plane does not own the authority it is exercising"),
        // "Bvisor minting semantic restart authorization"
        ("bvisor mints semantic restart authority", SyncBatPlane::Bvisor, SyncBatPlane::Runtime,
         SyncBatAuthority::RetryRestartAuthority,
         "the sending plane does not own the authority it is exercising"),
        // "runtime bypassing Bvisor to execute a program"
        ("runtime bypasses bvisor to execute", SyncBatPlane::Runtime, SyncBatPlane::Port,
         SyncBatAuthority::PhysicalAttempt,
         "the sending plane does not own the authority it is exercising"),
        // "PakVM calling filesystem/network/clock directly"
        ("pakvm reaches the host directly", SyncBatPlane::PakVm, SyncBatPlane::Port,
         SyncBatAuthority::TypedEffectRequest,
         "no lawful crossing carries this authority between these planes"),
        // "world redefining BatPak-owned IDs or schema law"
        ("world redefines owned identity", SyncBatPlane::World, SyncBatPlane::PakVm,
         SyncBatAuthority::SemanticNodeInterpretation,
         "the sending plane does not own the authority it is exercising"),
        // attempt evidence must not satisfy a logical receipt
        ("attempt evidence satisfies a semantic receipt", SyncBatPlane::Bvisor,
         SyncBatPlane::Runtime, SyncBatAuthority::SemanticReceipt,
         "the sending plane does not own the authority it is exercising"),
        // runtime must not fabricate attempt evidence
        ("runtime fabricates attempt evidence", SyncBatPlane::Runtime, SyncBatPlane::PakVm,
         SyncBatAuthority::AttemptEvidence,
         "the sending plane does not own the authority it is exercising"),
        // world must not become a service locator for host capability
        ("world wires around the port", SyncBatPlane::World, SyncBatPlane::Port,
         SyncBatAuthority::PhysicalAttempt,
         "the sending plane does not own the authority it is exercising"),
    ] {
        let origin = if carries.requires_semantic_origin() {
            Some(semantic_origin_for(carries))
        } else {
            None
        };
        match admit_crossing(from, to, carries, origin) {
            CrossingAdmission::Admitted(_) => {
                findings.push(format!("the firewall admits a forbidden crossing: {what}"))
            }
            CrossingAdmission::Refused(why) if why != want => findings.push(format!(
                "{what} is refused, but for the wrong reason: {why:?} rather than {want:?}")),
            CrossingAdmission::Refused(_) => {}
        }
    }
}

/// Requiredness as connectivity, recomputed rather than trusted.
///
/// A Required crossing must be needed, and "needed" is not a stamp: it is that
/// the accepted turn cannot complete without it. The turn is a round trip. The
/// logical decider (the plane owning `LogicalResult`) authorizes work outward to
/// the execution planes and every plane returns its result or evidence back to
/// that same decider. So two obligations, both derived from authored ownership
/// rather than from any second policy table:
///
///   - every plane can reach the logical root by Required crossings (its work
///     returns), and
///   - the logical root can reach every plane except the composition source (the
///     plane owning `CompositionAndInstanceIdentity`, which precedes the turn).
///
/// Deleting a Required crossing, or downgrading one to Optional, drops it from
/// the Required graph and severs one of those reaches. Optional crossings are
/// excluded on purpose: "may be absent" means the machine must stand without
/// them, so relying on one for connectivity would be the very confusion this
/// posture exists to prevent.
fn check_syncbat_requiredness(findings: &mut Vec<String>) {
    use architecture::SyncBatPlane;
    use syncbat_firewall::*;

    let owner_of = |a: SyncBatAuthority| a.owner();
    let root = owner_of(SyncBatAuthority::LogicalResult);
    let source = owner_of(SyncBatAuthority::CompositionAndInstanceIdentity);

    // Reachability over Required crossings only. Small fixed graph; a linear
    // relaxation to a fixed point needs no allocation beyond a bitset.
    let reaches = |start: SyncBatPlane, goal: SyncBatPlane| -> bool {
        let mut seen = [false; 5];
        let idx = |p: SyncBatPlane| SyncBatPlane::ALL.iter().position(|&q| q == p).unwrap();
        seen[idx(start)] = true;
        let mut changed = true;
        while changed {
            changed = false;
            for c in SYNCBAT_LEGAL_CROSSINGS {
                // Only legal required edges carry connectivity: an illegal
                // substitute is flagged elsewhere and must not reconnect a graph
                // a deleted required route left broken.
                if c.posture == CrossingPosture::Required
                    && c.carries.owner() == c.from
                    && seen[idx(c.from)]
                    && !seen[idx(c.to)]
                {
                    seen[idx(c.to)] = true;
                    changed = true;
                }
            }
        }
        seen[idx(goal)]
    };

    for &plane in SyncBatPlane::ALL {
        if !reaches(plane, root) {
            findings.push(format!(
                "SyncBat plane {plane:?} cannot reach the logical root {root:?} by required \
                 crossings, so its result or evidence has no lawful route home; a required \
                 route is missing or was downgraded to optional"));
        }
        if plane != source && !reaches(root, plane) {
            findings.push(format!(
                "the logical root {root:?} cannot reach SyncBat plane {plane:?} by required \
                 crossings, so it cannot authorize the work that plane owns; a required route \
                 is missing or was downgraded to optional"));
        }
    }

    // An Optional crossing must be genuinely optional: there must be an alternate
    // required route delivering the same authority. If it is the sole carrier,
    // its absence would sever the machine, so "Optional" is a misclassification.
    for c in SYNCBAT_LEGAL_CROSSINGS {
        if c.posture == CrossingPosture::Optional
            && !SYNCBAT_LEGAL_CROSSINGS.iter().any(|o| {
                !core::ptr::eq(o, c)
                    && o.carries == c.carries
                    && o.posture == CrossingPosture::Required
            })
        {
            findings.push(format!(
                "the {:?} -> {:?} crossing is optional but is the sole route carrying {:?}; \
                 deleting it would sever the machine, so it cannot be optional",
                c.from, c.to, c.carries));
        }
    }
}

/// Origin law, exercised where the origin must be chosen rather than derived.
///
/// The forbidden-example loop above computes each origin from
/// `requires_semantic_origin()`, so it can never propose a MISMATCHED one. These
/// cases are exactly the mismatches, and they are the ones that matter: a pure
/// node emitting an effect request is an effect appearing inside a program the
/// validator cleared as pure.
pub(crate) fn check_syncbat_origin_law(findings: &mut Vec<String>) {
    use architecture::SyncBatPlane;
    use pakvm_isa::PakVmNodeId;
    use syncbat_firewall::*;

    for (what, from, to, carries, origin, want) in [
        // A pure node may not emit an effect request. Literal computes a value
        // and admits as Pure, so the effect posture is the only thing refusing.
        ("a pure node emits an effect request", SyncBatPlane::PakVm, SyncBatPlane::Bvisor,
         SyncBatAuthority::TypedEffectRequest, Some(PakVmNodeId::Literal),
         "a node that does not admit as Effectful emits an effect request"),
        // A semantic authority must carry its origin, not travel anonymously.
        ("an interpretation names no origin", SyncBatPlane::PakVm, SyncBatPlane::Runtime,
         SyncBatAuthority::SemanticNodeInterpretation, None,
         "a semantic authority names no PakVM origin"),
        // A physical authority cannot borrow an origin to look semantic.
        ("attempt evidence wears a semantic origin", SyncBatPlane::Bvisor,
         SyncBatPlane::Runtime, SyncBatAuthority::AttemptEvidence, Some(PakVmNodeId::Append),
         "a non-semantic authority names a PakVM origin it cannot have"),
    ] {
        match admit_crossing(from, to, carries, origin) {
            CrossingAdmission::Admitted(_) => {
                findings.push(format!("the firewall admits a forbidden crossing: {what}"))
            }
            CrossingAdmission::Refused(why) if why != want => findings.push(format!(
                "{what} is refused, but for the wrong reason: {why:?} rather than {want:?}")),
            CrossingAdmission::Refused(_) => {}
        }
    }
}

/// A node whose admitted posture suits an authority, for exercising the firewall.
/// Effect requests need an effectful node; interpretation takes any admitted one.
fn semantic_origin_for(authority: syncbat_firewall::SyncBatAuthority) -> pakvm_isa::PakVmNodeId {
    use pakvm_isa::PakVmNodeId;
    use syncbat_firewall::SyncBatAuthority;
    match authority {
        SyncBatAuthority::TypedEffectRequest => PakVmNodeId::Append,
        _ => PakVmNodeId::Compare,
    }
}
