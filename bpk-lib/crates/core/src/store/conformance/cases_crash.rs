//! Crash-edition conformance cases C27–C34: the `StoreFs` durability contract
//! observed across a simulated power loss.
//!
//! Each case drives the published seam (`&dyn StoreFs` + handle traits) then
//! asks the backend's [`HostileControls`](super::HostileControls) to `crash()`
//! and inspects the surviving namespace/bytes. A backend that supplies no
//! controls (e.g. `RealFs`) yields a typed [`Qualification`] — a skip is never
//! a pass. The obligation is the BACKEND's crash-legality ENVELOPE, not the
//! store's compaction protocol: C29/C30/C31/C33 accept the honest-durability
//! outcomes ("may vanish", "may resurrect", "old-or-new") because a backend
//! with stronger names/journaling legitimately lands the stronger side; only a
//! TORN state (a partial byte prefix, a mixed name set, a third body) is a
//! violation.

use std::io;

use super::{CaseCx, CaseFailure, CaseResult, CrashOp, HostileControls, Qualification, check};
use crate::store::platform::fs::StoreFs;
use crate::store::platform::handle::{StagedFile, StoreFile};
use crate::store::platform::sync::admit_current_parent_dir_sync;

/// Ten durable bytes every case syncs as the baseline payload.
const PROBE_BYTES: &[u8] = b"0123456789";
/// Six extra bytes appended WITHOUT a sync (the droppable tail).
const TAIL_BYTES: &[u8] = b"ABCDEF";
/// A distinct replacement body (different length + content from `PROBE_BYTES`).
const REPLACEMENT_BYTES: &[u8] = b"replacement!";

/// Map any `Display` failure to a case violation with a fixed context tag.
fn violation<E: std::fmt::Display>(ctx: &'static str) -> impl Fn(E) -> CaseFailure {
    move |error| CaseFailure::Violation(format!("{ctx}: {error}"))
}

/// The crash cases require hostile controls; a backend without them is
/// qualified (typed skip), never silently passed.
fn require_controls<'a>(cx: &CaseCx<'a>) -> Result<&'a dyn HostileControls, CaseFailure> {
    match cx.controls {
        Some(controls) => Ok(controls),
        None => Err(CaseFailure::Qualified(Qualification::ControlUnsupported {
            control: "crash",
            backend: String::new(),
            reason: "backend factory supplied no hostile controls".into(),
        })),
    }
}

/// The three legal post-crash shapes of a single path (see the module doc):
/// gone, present with EXACTLY the durable bytes, or a torn/erroneous third
/// state that is always a violation.
enum CrashReadState {
    Absent,
    PresentExact,
    Divergent(String),
}

fn classify_read(read: io::Result<Vec<u8>>, expect: &[u8]) -> CrashReadState {
    match read {
        Ok(bytes) if bytes == expect => CrashReadState::PresentExact,
        Ok(bytes) => CrashReadState::Divergent(format!(
            "present with {} byte(s), not the expected {} durable byte(s)",
            bytes.len(),
            expect.len()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => CrashReadState::Absent,
        Err(error) => CrashReadState::Divergent(format!("unexpected read error: {error}")),
    }
}

/// Create `path`, write `PROBE_BYTES`, sync the handle and the parent name so
/// both bytes and name are durable, then drop the handle.
fn seed_durable_probe(cx: &CaseCx<'_>, path: &std::path::Path) -> Result<(), CaseFailure> {
    let mut file = cx
        .fs
        .create_new_file(path)
        .map_err(violation("create_new_file"))?;
    file.write_all(PROBE_BYTES).map_err(violation("write_all"))?;
    file.sync_all().map_err(violation("sync_all"))?;
    drop(file);
    cx.fs
        .sync_parent_dir(path)
        .map_err(violation("sync_parent_dir"))?;
    Ok(())
}

/// C27 `crash-preserves-synced-bytes-and-durable-name`: synced bytes + a synced
/// name survive the crash intact.
pub(crate) fn crash_preserves_synced_bytes_and_durable_name(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let path = cx.root.join("crash-c27.probe");
    seed_durable_probe(cx, &path)?;
    controls.crash().map_err(CaseFailure::Qualified)?;
    let got = cx.fs.read(&path).map_err(violation("C27 read after crash"))?;
    check(
        got == PROBE_BYTES,
        format!(
            "C27: expected the {} durable byte(s) to survive, got {} byte(s)",
            PROBE_BYTES.len(),
            got.len()
        ),
    )
}

/// C28 `crash-drops-unsynced-tail`: bytes past the last sync vanish; the crash
/// yields exactly the durable prefix, never a torn tail.
pub(crate) fn crash_drops_unsynced_tail(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let path = cx.root.join("crash-c28.probe");
    let mut file = cx
        .fs
        .create_new_file(&path)
        .map_err(violation("C28 create_new_file"))?;
    file.write_all(PROBE_BYTES)
        .map_err(violation("C28 write prefix"))?;
    file.sync_all().map_err(violation("C28 sync prefix"))?;
    cx.fs
        .sync_parent_dir(&path)
        .map_err(violation("C28 sync_parent_dir"))?;
    // The unsynced tail: appended, never made durable.
    file.write_all(TAIL_BYTES)
        .map_err(violation("C28 write tail"))?;
    drop(file);
    controls.crash().map_err(CaseFailure::Qualified)?;
    let got = cx.fs.read(&path).map_err(violation("C28 read after crash"))?;
    check(
        got == PROBE_BYTES,
        format!(
            "C28: expected the {}-byte durable prefix, got {} byte(s)",
            PROBE_BYTES.len(),
            got.len()
        ),
    )
}

/// C29 `unsynced-name-may-vanish-never-torn`: a name whose parent was never
/// synced may be absent OR present with the full synced bytes — never partial.
pub(crate) fn unsynced_name_may_vanish_never_torn(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let path = cx.root.join("crash-c29.probe");
    let mut file = cx
        .fs
        .create_new_file(&path)
        .map_err(violation("C29 create_new_file"))?;
    file.write_all(PROBE_BYTES)
        .map_err(violation("C29 write_all"))?;
    file.sync_all().map_err(violation("C29 sync_all"))?;
    drop(file);
    // No parent-dir sync: the NAME's durability is deliberately unresolved.
    controls.crash().map_err(CaseFailure::Qualified)?;
    match classify_read(cx.fs.read(&path), PROBE_BYTES) {
        CrashReadState::Absent | CrashReadState::PresentExact => Ok(()),
        CrashReadState::Divergent(why) => Err(CaseFailure::Violation(format!(
            "C29: unsynced name recovered torn: {why}"
        ))),
    }
}

/// C30 `rename-without-parent-sync-crash-old-or-new`: a rename whose parent
/// sync was dropped recovers to EXACTLY ONE of the old or new name.
pub(crate) fn rename_without_parent_sync_crash_old_or_new(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let old_path = cx.root.join("crash-c30-a.probe");
    let new_path = cx.root.join("crash-c30-b.probe");
    seed_durable_probe(cx, &old_path)?;
    controls
        .drop_next_parent_dir_sync()
        .map_err(CaseFailure::Qualified)?;
    cx.fs
        .rename(&old_path, &new_path)
        .map_err(violation("C30 rename"))?;
    // The publish's parent sync is the dropped one (returns Ok, not durable).
    cx.fs
        .sync_parent_dir(&new_path)
        .map_err(violation("C30 sync_parent_dir"))?;
    controls.crash().map_err(CaseFailure::Qualified)?;
    let old_state = classify_read(cx.fs.read(&old_path), PROBE_BYTES);
    let new_state = classify_read(cx.fs.read(&new_path), PROBE_BYTES);
    if let CrashReadState::Divergent(why) = &old_state {
        return Err(CaseFailure::Violation(format!("C30: old name torn: {why}")));
    }
    if let CrashReadState::Divergent(why) = &new_state {
        return Err(CaseFailure::Violation(format!("C30: new name torn: {why}")));
    }
    let old_present = matches!(old_state, CrashReadState::PresentExact);
    let new_present = matches!(new_state, CrashReadState::PresentExact);
    check(
        old_present ^ new_present,
        format!(
            "C30: expected exactly one of old/new after crash; \
             old_present={old_present} new_present={new_present}"
        ),
    )
}

/// C31 `remove-without-parent-sync-crash-may-resurrect`: a removal whose parent
/// sync was dropped may leave the path absent OR resurrected with exact bytes.
pub(crate) fn remove_without_parent_sync_crash_may_resurrect(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let path = cx.root.join("crash-c31.probe");
    seed_durable_probe(cx, &path)?;
    controls
        .drop_next_parent_dir_sync()
        .map_err(CaseFailure::Qualified)?;
    cx.fs
        .remove_file(&path)
        .map_err(violation("C31 remove_file"))?;
    // The removal's parent sync is the dropped one: the unlink is not durable.
    cx.fs
        .sync_parent_dir(&path)
        .map_err(violation("C31 sync_parent_dir"))?;
    controls.crash().map_err(CaseFailure::Qualified)?;
    match classify_read(cx.fs.read(&path), PROBE_BYTES) {
        CrashReadState::Absent | CrashReadState::PresentExact => Ok(()),
        CrashReadState::Divergent(why) => Err(CaseFailure::Violation(format!(
            "C31: resurrection torn: {why}"
        ))),
    }
}

/// C32 `durable-remove-never-resurrects`: a removal with an HONORED parent sync
/// stays gone across the crash, always.
pub(crate) fn durable_remove_never_resurrects(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let path = cx.root.join("crash-c32.probe");
    seed_durable_probe(cx, &path)?;
    cx.fs
        .remove_file(&path)
        .map_err(violation("C32 remove_file"))?;
    // Honored parent sync (no drop armed): the unlink is durable.
    cx.fs
        .sync_parent_dir(&path)
        .map_err(violation("C32 sync_parent_dir"))?;
    controls.crash().map_err(CaseFailure::Qualified)?;
    match classify_read(cx.fs.read(&path), PROBE_BYTES) {
        CrashReadState::Absent => Ok(()),
        CrashReadState::PresentExact => Err(CaseFailure::Violation(
            "C32: a durably-removed file resurrected after crash".into(),
        )),
        CrashReadState::Divergent(why) => {
            Err(CaseFailure::Violation(format!("C32: {why}")))
        }
    }
}

/// C33 `persist-without-parent-sync-crash-old-or-new`: a staged publish whose
/// implicit parent sync was dropped recovers to old-complete or new-complete,
/// never a mixed/empty third state.
pub(crate) fn persist_without_parent_sync_crash_old_or_new(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let final_path = cx.root.join("crash-c33.probe");
    seed_durable_probe(cx, &final_path)?;
    controls
        .drop_next_parent_dir_sync()
        .map_err(CaseFailure::Qualified)?;
    let mut staged = cx
        .fs
        .named_temp_in(cx.root)
        .map_err(violation("C33 named_temp_in"))?;
    staged
        .write_all(REPLACEMENT_BYTES)
        .map_err(violation("C33 staged write_all"))?;
    staged.sync_all().map_err(violation("C33 staged sync_all"))?;
    let admission = admit_current_parent_dir_sync().map_err(violation("C33 admit parent sync"))?;
    // persist's IMPLICIT parent sync is the dropped one: the new name lands
    // visible but its durability is unresolved.
    staged
        .persist(&final_path, admission)
        .map_err(violation("C33 persist"))?;
    controls.crash().map_err(CaseFailure::Qualified)?;
    match cx.fs.read(&final_path) {
        Ok(bytes) if bytes == PROBE_BYTES => Ok(()),
        Ok(bytes) if bytes == REPLACEMENT_BYTES => Ok(()),
        Ok(bytes) => Err(CaseFailure::Violation(format!(
            "C33: persist crash left a third state ({} byte(s), neither old nor new complete)",
            bytes.len()
        ))),
        Err(error) => Err(CaseFailure::Violation(format!(
            "C33: durable final name unreadable after crash: {error}"
        ))),
    }
}

/// C34 `planted-fault-actually-fires`: the controls themselves are not vacuous
/// — an armed op fault errs, advances the fired counter, and is consumed.
pub(crate) fn planted_fault_actually_fires(cx: &CaseCx<'_>) -> CaseResult {
    let controls = require_controls(cx)?;
    let old_path = cx.root.join("crash-c34-a.probe");
    let new_path = cx.root.join("crash-c34-b.probe");
    seed_durable_probe(cx, &old_path)?;
    controls
        .fault_next_op(CrashOp::Rename)
        .map_err(CaseFailure::Qualified)?;
    match cx.fs.rename(&old_path, &new_path) {
        Ok(()) => {
            return Err(CaseFailure::Violation(
                "C34: armed rename fault did not fire — rename succeeded".into(),
            ));
        }
        Err(_expected) => {}
    }
    check(
        controls.faults_fired() >= 1,
        format!(
            "C34: fault fired but the counter did not advance (faults_fired={})",
            controls.faults_fired()
        ),
    )?;
    // The fault is consumed (nth-occurrence semantics): the next rename lands.
    cx.fs
        .rename(&old_path, &new_path)
        .map_err(violation("C34 rename after fault consumed"))?;
    Ok(())
}
