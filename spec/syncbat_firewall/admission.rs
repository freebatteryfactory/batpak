use crate::architecture::SyncBatPlane;
use crate::pakvm_isa::{admit, EffectPosture, PakVmAdmission, PakVmNodeId};

use super::inventory::SYNCBAT_LEGAL_CROSSINGS;
use super::types::{AdmittedCrossing, CrossingAdmission, SyncBatAuthority, SyncBatCrossing};

fn declared_crossing(
    from: SyncBatPlane,
    to: SyncBatPlane,
    carries: SyncBatAuthority,
) -> Option<&'static SyncBatCrossing> {
    let mut i = 0;
    while i < SYNCBAT_LEGAL_CROSSINGS.len() {
        let c = &SYNCBAT_LEGAL_CROSSINGS[i];
        if c.from == from && c.to == to && c.carries == carries {
            return Some(c);
        }
        i += 1;
    }
    None
}

/// Admit one authority crossing, or refuse it with the reason.
///
/// This is the whole production API. There is no injection seam and none is
/// needed: the caller already proposes the crossing, so a hostile fixture can
/// propose an unlawful one through the same door production uses. Nothing here
/// can be exercised in a test that cannot be exercised in production, which is
/// the point — a firewall with a test-only entrance is a firewall with an
/// entrance.
pub fn admit_crossing(
    from: SyncBatPlane,
    to: SyncBatPlane,
    carries: SyncBatAuthority,
    origin: Option<PakVmNodeId>,
) -> CrossingAdmission {
    if from == to {
        return CrossingAdmission::Refused(
            "a plane does not cross to itself; that is an internal transition",
        );
    }
    // Ownership first. This is the rule that does the real work, and it is
    // derived rather than listed: docs/08's forbidden examples all reduce to a
    // plane exercising an authority it does not own. Bvisor minting semantic
    // restart, runtime fabricating attempt evidence, and world redefining
    // BatPak-owned identity are the same defect wearing three coats.
    if carries.owner() != from {
        return CrossingAdmission::Refused(
            "the sending plane does not own the authority it is exercising",
        );
    }
    let declared = match declared_crossing(from, to, carries) {
        Some(c) => c,
        None => {
            return CrossingAdmission::Refused(
                "no lawful crossing carries this authority between these planes",
            )
        }
    };
    // A semantic authority names the admitted node it came from. Without this,
    // "the ISA is the only execution route" would be a sentence in a document
    // rather than a condition on a value.
    let origin = match (carries.requires_semantic_origin(), origin) {
        (true, None) => {
            return CrossingAdmission::Refused("a semantic authority names no PakVM origin")
        }
        (false, Some(_)) => {
            return CrossingAdmission::Refused(
                "a non-semantic authority names a PakVM origin it cannot have",
            )
        }
        (false, None) => None,
        (true, Some(node)) => {
            // ISA-ONLY EXECUTION. The origin must be a node the D4c1 authority
            // admits. A fabricated identity cannot reach this point: PakVmNodeId
            // has no constructor beyond its authored variants, so the type
            // forecloses the value. What admission adds is that the node's spec
            // must still resolve.
            match admit(node) {
                PakVmAdmission::Refused(_) => {
                    return CrossingAdmission::Refused(
                        "the named PakVM origin does not admit into the semantic ISA",
                    )
                }
                PakVmAdmission::Admitted(spec) => {
                    // Only an effectful node may emit an effect request. A pure
                    // or observational node doing so would be an effect appearing
                    // in a program the validator cleared as pure.
                    if matches!(carries, SyncBatAuthority::TypedEffectRequest)
                        && !matches!(spec.effect, EffectPosture::Effectful)
                    {
                        return CrossingAdmission::Refused(
                            "a node that does not admit as Effectful emits an effect request",
                        );
                    }
                    Some(node)
                }
            }
        }
    };
    CrossingAdmission::Admitted(AdmittedCrossing {
        from,
        to,
        carries,
        posture: declared.posture,
        origin,
        law: declared.law,
        seal: (),
    })
}
