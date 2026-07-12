#![no_main]
//! GAUNT-SIDX-NO-SELF-AUTH (#192) — `sidx_manifest` semantic-mutation target.
//!
//! Drives `batpak::__fuzz::__fuzz_sidx_manifest(&[u8])`, a PROPERTY harness (not
//! a pure decoder): the shim builds a VALID `[frames][SDX3 footer]` pair, applies
//! the input as a semantic mutation script over the SIDX entry table (forged
//! offsets/lengths, removed rows, duplicated forged siblings, a torn last frame,
//! an overridden footer claim) while keeping the table well-formed — so inputs
//! reach the untrusted-recovery DECISION core instead of dying in bounds checks —
//! and asserts the no-authority-escalation law in-shim against the table-blind
//! CRC walk as ground truth:
//!
//! * permissive recovery lands EXACTLY on the walk's verified prefix end,
//!   whatever the table claims (no false loss, no extension past verified bytes);
//! * strict posture refuses EVERY non-empty prefix (no table content is a
//!   recovery ticket — no false safety);
//! * an empty prefix recovers under both postures;
//! * a walk-level refusal (mid-stream shape) refuses under both postures.
//!
//! Pure random bytes would only exercise the footer bounds checks; this target
//! manufactures PLAUSIBLE LIES, which is what the issue's proof obligation asks
//! for. A violated law panics inside the shim → libFuzzer crash in real decision
//! logic. A typed `Err` (Io from the in-memory writer path) is a legal outcome.

use libfuzzer_sys::fuzz_target;

use batpak::__fuzz::__fuzz_sidx_manifest;

fuzz_target!(|data: &[u8]| {
    match __fuzz_sidx_manifest(data) {
        Ok(classification) => {
            let _ = classification;
        }
        Err(err) => {
            let _ = format!("{err:?}");
        }
    }
});
