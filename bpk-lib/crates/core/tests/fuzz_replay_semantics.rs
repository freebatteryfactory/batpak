//! GAUNT-PROOF-OF-PROOF (#197) — semantic regression receipts for the two
//! critical fuzz targets.
//! PROVES: the committed fuzz/regressions fixtures are semantically load-bearing:
//!   each one discriminates the cured decision core from the pre-fix forbidden
//!   behavior (pre-#192 SIDX claim escalation; pre-#189 idemp blind admission).
//! CATCHES: ceremonial fixtures (bytes that no longer reach the decision core),
//!   and any regression that re-admits the forbidden outcome.
//! SEEDED: exact committed fixture bytes, loaded from fuzz/regressions/.
//!
//! Both tests are ProductionFlip fixtures (registered in
//! `tools/integrity/src/gate_registry.rs`, bitten by `cargo xtask
//! prove-gates-bite`): under `--cfg gauntlet_red_fixture` each half asserts the
//! OLD forbidden outcome on the exact committed bytes, so it FAILS against the
//! cured core. The fixture-rot preconditions (a real claim past a non-empty
//! verified prefix; a CRC that genuinely mismatches) are asserted in BOTH cfg
//! halves so a rotted fixture reds regardless of the flip.
#![cfg(feature = "dangerous-test-hooks")]

use std::path::Path;

/// Read a committed regression fixture from `fuzz/regressions/<target>/<file>`,
/// resolved from this test crate's `CARGO_MANIFEST_DIR` (`bpk-lib/crates/core`).
fn regression_bytes(target: &str, file: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fuzz")
        .join("regressions")
        .join(target)
        .join(file);
    std::fs::read(&path).expect("committed fixture must exist")
}

#[test]
fn sidx_manifest_regression_is_semantically_load_bearing() {
    let bytes = regression_bytes("sidx_manifest", "RED-torn-tail-claim-escalation");
    let r = batpak::__fuzz::__fuzz_sidx_manifest_resolution(&bytes)
        .expect("in-memory fixture pipeline cannot Io-fail");
    // Fixture-rot preconditions: the input must still be load-bearing — a real
    // claim strictly past a non-empty verified prefix.
    let walk = r
        .walk_frames_end
        .expect("fixture must produce a walkable prefix");
    let claim = r.claim.expect("fixture must carry a footer claim");
    assert!(r.walk_count > 0, "fixture must keep at least one intact frame");
    assert_ne!(claim, walk, "fixture must claim past the verified prefix");

    #[cfg(not(gauntlet_red_fixture))]
    {
        // Cured law: table-blind recovery; strict refuses the non-empty prefix.
        assert_eq!(
            r.permissive_frames_end,
            Some(walk),
            "no-authority-escalation: permissive must land on the walk end"
        );
        assert!(
            r.strict_refused,
            "strict posture must refuse a non-empty prefix"
        );
    }
    #[cfg(gauntlet_red_fixture)]
    {
        // RED half: assert the OLD (forbidden, pre-#192) self-authentication
        // outcome — recovery lands on the untrusted claim. MUST FAIL against
        // the cured core (proven by `cargo xtask prove-gates-bite`).
        assert_eq!(
            r.permissive_frames_end,
            Some(claim),
            "RED FIXTURE: asserts the illegal claim-trusting recovery"
        );
    }
}

#[test]
fn idemp_image_regression_is_semantically_load_bearing() {
    let bytes = regression_bytes("idemp_image", "RED-crc-mismatch-blind-admission");
    // Fixture-rot preconditions: header parses at a supported version and the
    // stored CRC really mismatches the body.
    assert_eq!(&bytes[..6], b"FBATID");
    let version = u16::from_le_bytes([bytes[6], bytes[7]]);
    assert!((1..=batpak::store::IDEMP_VERSION).contains(&version));
    let stored = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    assert_ne!(stored, crc32fast::hash(&bytes[12..]), "CRC must mismatch");

    let outcome = batpak::__fuzz::__fuzz_idemp_image(&bytes);
    #[cfg(not(gauntlet_red_fixture))]
    {
        let err = match outcome {
            Ok(admitted) => unreachable!(
                "PROPERTY: corrupt authority must refuse, admitted {admitted} entries"
            ),
            Err(e) => e,
        };
        assert!(
            matches!(
                err,
                batpak::store::StoreError::IdempotencyAuthorityCorrupt { .. }
            ),
            "wrong refusal variant: {err:?}"
        );
    }
    #[cfg(gauntlet_red_fixture)]
    {
        // RED half: assert the OLD (forbidden, pre-#189) blind admission —
        // corrupt image admitted as empty. MUST FAIL against the cured core.
        let admitted =
            outcome.expect("RED FIXTURE: asserts the illegal blind admission");
        assert_eq!(admitted, 0, "RED FIXTURE: old core admitted an empty image");
    }
}
