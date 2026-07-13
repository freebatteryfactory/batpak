//! Unit tests for [`super::ShadowFs`] — split out via `#[path]`, so `super`
//! is the `sim::shadow_fs` module and `use super::*` resolves `ShadowFs`,
//! `SimOp`, and `SimOpKind`.
//!
//! This island holds the namespace-truth axis: the mandate's seven
//! namespace-durability STATES (each constructed deterministically from the
//! two-truths model, then validated after `crash()`) plus the model
//! properties that generalize them (determinism, honored-remove finality,
//! orphan-stage absence, lock-registry reset). The memfs-parity, store
//! integration, and R1 crash-harness arming controls live in the sibling
//! `arming_tests` island.

use super::ShadowFs;
use crate::store::platform::fs::StoreFs;
use crate::store::platform::handle::{StagedFile, StoreFile};
use crate::store::platform::sync::admit_current_parent_dir_sync;
use crate::store::sim::fs::CrashOp;
use std::io;
use std::path::Path;

/// A fresh backend with its data dir seeded (dirs are durable-on-create).
pub(super) fn fresh(root: &Path) -> ShadowFs {
    let fs = ShadowFs::new();
    fs.create_dir_all(root).expect("seed data dir");
    fs
}

/// Create `path` with `bytes` fully synced AND its name made durable (one
/// honored parent-dir sync). The post-condition is a segment that a crash
/// alone would preserve intact — the durable baseline the state tests perturb.
pub(super) fn write_durable(fs: &ShadowFs, path: &Path, bytes: &[u8]) {
    let mut file = fs.create_new_file(path).expect("create durable file");
    file.write_all(bytes).expect("write durable bytes");
    file.sync_all().expect("honored handle sync");
    drop(file);
    fs.sync_parent_dir(path).expect("honored parent-dir sync");
}

// ─────────────────────────── mandate states 1–7 ───────────────────────────

#[test]
fn state1_staged_name_durable_final_absent_after_crash() {
    let root = Path::new("/shadow/state1");
    let fs = fresh(root);
    let staged = root.join("segment.compact-new");
    let final_path = root.join("segment.fbat");

    let mut stage = fs.named_temp_in(root).expect("stage");
    stage.write_all(b"replacement").expect("write staged");
    stage.sync_all().expect("staged sync honored");
    let admission = admit_current_parent_dir_sync().expect("mint admission");
    stage.persist(&staged, admission).expect("publish staged name");

    fs.crash();

    assert!(
        fs.is_name_durable(&staged),
        "PROPERTY: a staged name published with an honored implicit parent sync is durable"
    );
    assert!(
        !fs.is_name_durable(&final_path),
        "PROPERTY: a final name never created is absent after the crash"
    );
    assert_eq!(
        fs.durable_byte_len(&staged),
        Some(b"replacement".len() as u64),
        "PROPERTY: the durable staged file carries its full synced bytes"
    );
}

#[test]
fn state2_final_rename_visible_parent_sync_dropped_recovers_old_name() {
    let root = Path::new("/shadow/state2");
    let fs = fresh(root);
    let old = root.join("a.fbat");
    let new = root.join("b.fbat");
    write_durable(&fs, &old, b"payload-a");

    fs.arm_parent_sync_drop(fs.parent_syncs_observed() + 1);
    fs.rename(&old, &new).expect("rename visible");
    fs.sync_parent_dir(&new)
        .expect("dropped parent sync still returns Ok (lying dir)");

    fs.crash();

    assert!(
        fs.is_name_durable(&old) && !fs.is_name_durable(&new),
        "PROPERTY: a rename whose parent sync was dropped is not durable — the old name survives"
    );
    assert!(
        fs.armed_faults_fired() >= 1,
        "anti-vacuity: the armed parent-sync drop actually fired"
    );
}

#[test]
fn state3_source_rename_visible_not_durable_recovers_source() {
    let root = Path::new("/shadow/state3");
    let fs = fresh(root);
    let src = root.join("source.fbat");
    let relocated = root.join("source.compact-src");
    write_durable(&fs, &src, b"source-seg");

    fs.arm_parent_sync_drop(fs.parent_syncs_observed() + 1);
    fs.rename(&src, &relocated).expect("relocate the source aside");
    fs.sync_parent_dir(&relocated)
        .expect("dropped parent sync still returns Ok");

    fs.crash();

    assert!(
        fs.is_name_durable(&src) && !fs.is_name_durable(&relocated),
        "PROPERTY: a source relocation whose parent sync dropped is not durable"
    );
    assert_eq!(
        fs.read(&src).expect("the source name resurrects"),
        b"source-seg",
        "PROPERTY: the recovered source carries its full durable bytes (rollback stays readable)"
    );
    assert!(fs.armed_faults_fired() >= 1, "anti-vacuity: the drop fired");
}

#[test]
fn state4_source_removal_not_durable_resurrects_durable_prefix() {
    let root = Path::new("/shadow/state4");
    let fs = fresh(root);
    let src = root.join("source.fbat");

    let mut file = fs.create_new_file(&src).expect("create source");
    file.write_all(b"012345").expect("write synced prefix");
    file.sync_all().expect("sync the 6-byte prefix");
    file.write_all(b"6789").expect("write the unsynced tail");
    drop(file);
    fs.sync_parent_dir(&src).expect("name durable");

    fs.arm_parent_sync_drop(fs.parent_syncs_observed() + 1);
    fs.remove_file(&src).expect("remove visible");
    fs.sync_parent_dir(&src)
        .expect("the removal's parent sync is dropped");

    fs.crash();

    assert_eq!(
        fs.durable_byte_len(&src),
        Some(6),
        "PROPERTY: a resurrected name is bound to the inode's DURABLE byte prefix, not its visible tail"
    );
    assert_eq!(
        fs.read(&src).expect("the removed name resurrects"),
        b"012345",
        "PROPERTY: resurrection bytes are exactly the synced prefix — never the torn tail"
    );
    assert!(fs.armed_faults_fired() >= 1, "anti-vacuity: the drop fired");
}

#[test]
fn state5_stale_staged_and_final_names_both_present_after_crash() {
    let root = Path::new("/shadow/state5");
    let fs = fresh(root);
    let final_path = root.join("segment.fbat");
    let staged = root.join("segment.compact-new");
    write_durable(&fs, &final_path, b"old-final");

    let mut stage = fs.named_temp_in(root).expect("stage");
    stage.write_all(b"staged-new").expect("write staged");
    stage.sync_all().expect("staged sync honored");
    let admission = admit_current_parent_dir_sync().expect("mint admission");
    stage.persist(&staged, admission).expect("publish staged name");

    fs.crash();

    assert!(
        fs.is_name_durable(&final_path) && fs.is_name_durable(&staged),
        "PROPERTY: names made durable in different protocol steps both survive — the stale-pair state"
    );
}

#[test]
fn state6_interrupted_replacement_publication_leaves_final_absent() {
    let root = Path::new("/shadow/state6");
    let fs = fresh(root);
    let final_path = root.join("segment.fbat");
    fs.arm_op_error(CrashOp::PersistTemp, 1);

    let mut stage = fs.named_temp_in(root).expect("stage");
    stage.write_all(b"new").expect("write staged");
    stage.sync_all().expect("staged sync honored");
    let admission = admit_current_parent_dir_sync().expect("mint admission");
    assert!(
        stage.persist(&final_path, admission).is_err(),
        "PROPERTY: an armed PersistTemp fault tears the publish"
    );

    assert!(
        matches!(fs.read(&final_path), Err(ref e) if e.kind() == io::ErrorKind::NotFound),
        "PROPERTY: a torn publish installs nothing — the final name is absent even before crash"
    );
    fs.crash();
    assert!(
        !fs.is_name_durable(&final_path),
        "PROPERTY: the never-published name is absent after crash"
    );
    assert!(fs.armed_faults_fired() >= 1, "anti-vacuity: the fault fired");
}

#[test]
fn state7_dropped_parent_sync_at_each_transition_loses_all_names() {
    let root = Path::new("/shadow/state7");
    let fs = fresh(root);
    fs.arm_parent_sync_drop_all();

    for name in ["a.fbat", "b.fbat"] {
        let path = root.join(name);
        let mut file = fs.create_new_file(&path).expect("create");
        file.write_all(b"bytes").expect("write");
        file.sync_all().expect("sync handle (bytes durable, name is not)");
        drop(file);
        fs.sync_parent_dir(&path)
            .expect("every parent sync is dropped by drop_all");
    }

    fs.crash();

    assert!(
        fs.durable_entries(root).is_empty(),
        "PROPERTY: with every parent sync dropped, no created name is durable"
    );
    assert!(
        fs.visible_entries(root).is_empty(),
        "PROPERTY: crash sets visible := durable — the lost names are gone from both truths"
    );
    assert!(
        fs.armed_faults_fired() >= 2,
        "anti-vacuity: both dropped parent syncs fired"
    );
}

// ─────────────────────── model properties (beyond the 7) ───────────────────

#[test]
fn crash_outcome_is_deterministic_for_identical_op_sequences() {
    let build = || {
        let root = Path::new("/shadow/determinism");
        let fs = fresh(root);
        let keep = root.join("keep.fbat");
        let torn = root.join("torn.fbat");
        write_durable(&fs, &keep, b"keep-bytes");

        let mut file = fs.create_new_file(&torn).expect("create torn");
        file.write_all(b"synced").expect("write synced");
        file.sync_all().expect("sync handle");
        file.write_all(b"unsynced").expect("write tail");
        drop(file);
        fs.arm_parent_sync_drop(fs.parent_syncs_observed() + 1);
        fs.sync_parent_dir(&torn).expect("dropped");
        fs.crash();

        (
            fs.visible_entries(root),
            fs.durable_entries(root),
            fs.durable_byte_len(&keep),
        )
    };

    assert_eq!(
        build(),
        build(),
        "PROPERTY: identical construction + op order yields a byte-identical crash outcome"
    );
}

#[test]
fn honored_remove_never_resurrects() {
    let root = Path::new("/shadow/honored-remove");
    let fs = fresh(root);
    let path = root.join("gone.fbat");
    write_durable(&fs, &path, b"gone");

    fs.remove_file(&path).expect("remove visible");
    fs.sync_parent_dir(&path)
        .expect("HONORED parent sync makes the removal durable");
    fs.crash();

    assert!(
        !fs.is_name_durable(&path),
        "PROPERTY: a removal with an honored parent sync is durable — the name never resurrects"
    );
    assert!(
        matches!(fs.read(&path), Err(ref e) if e.kind() == io::ErrorKind::NotFound),
        "PROPERTY: the durably-removed name reads NotFound after crash"
    );
}

#[test]
fn unpersisted_stage_never_appears_after_crash() {
    let root = Path::new("/shadow/orphan-stage");
    let fs = fresh(root);
    let target = root.join("orphan.fbat");
    {
        let mut stage = fs.named_temp_in(root).expect("stage");
        stage.write_all(b"never-published").expect("write staged");
        stage.sync_all().expect("staged sync");
        // Dropped WITHOUT persist: the bytes were buffered privately.
    }
    fs.crash();

    assert!(
        matches!(fs.read(&target), Err(ref e) if e.kind() == io::ErrorKind::NotFound),
        "PROPERTY: an abandoned stage never installs a name"
    );
    assert!(
        fs.durable_entries(root).is_empty(),
        "PROPERTY: an abandoned stage leaves no durable entry"
    );
}

#[test]
fn crash_clears_the_lock_registry() {
    let root = Path::new("/shadow/lock");
    let fs = fresh(root);
    let lock = root.join("store.lock");

    let held = fs.try_lock_store_dir(&lock).expect("first lock call");
    assert!(held.is_some(), "PROPERTY: the first acquisition succeeds");
    let second = fs.try_lock_store_dir(&lock).expect("second lock call");
    assert!(
        second.is_none(),
        "PROPERTY: a held store lock excludes a second owner"
    );

    fs.crash();

    let after = fs.try_lock_store_dir(&lock).expect("lock call after crash");
    assert!(
        after.is_some(),
        "PROPERTY: crash (process death) clears the lock registry so a reopen can re-acquire"
    );
    drop(held);
}
