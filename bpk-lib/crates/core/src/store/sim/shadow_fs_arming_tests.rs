//! Integration + crash-harness arming tests for [`super::ShadowFs`] — split
//! out via `#[path]`, so `super` is the `sim::shadow_fs` module and the shared
//! fixtures come from the sibling namespace-truth island (`super::tests`).
//!
//! This island holds the memfs-parity fail-closed edges, the end-to-end
//! `Store`-over-`ShadowFs` integration, and the master contract's R1
//! crash-harness ARMING controls (`arm_op_error_at`, `set_freeze_on_fault`,
//! `arm_file_sync_error`, `mutation_ops`, `arm_crash_at_op`). Every armed
//! fixture asserts `armed_faults_fired()` advanced — a drop that never fired
//! is a vacuous fixture.

use super::tests::{fresh, write_durable};
use super::{ShadowFs, SimOpKind};
use crate::coordinate::Coordinate;
use crate::store::platform::fs::StoreFs;
use crate::store::platform::handle::StoreFile;
use crate::store::sim::fs::CrashOp;
use crate::store::sim::recovery::{recovered_user_events, KIND};
use crate::store::{Store, StoreConfig, StoreError};
use std::io;
use std::path::Path;
use std::sync::Arc;

#[test]
fn memfs_parity_fail_closed_edges_have_exact_error_kinds(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("/shadow/parity");
    let fs = fresh(root);
    let path = root.join("f.fbat");
    write_durable(&fs, &path, b"data");

    // Create-new is exclusive: a second create on the live name fails closed.
    let dup = match fs.create_new_file(&path) {
        Ok(_) => {
            return Err(io::Error::other("PROPERTY: create_new on an existing name must fail").into())
        }
        Err(e) => e,
    };
    let StoreError::Io(dup_io) = dup else {
        return Err(io::Error::other(format!("expected Io, got {dup:?}")).into());
    };
    assert!(
        matches!(dup_io.kind(), io::ErrorKind::AlreadyExists),
        "wrong kind for create-new collision: {dup_io:?}"
    );

    // A missing parent fails closed with NotFound.
    let orphan = root.join("no-such-dir/child.fbat");
    let missing = match fs.create_new_file(&orphan) {
        Ok(_) => {
            return Err(io::Error::other("PROPERTY: create under a missing parent must fail").into())
        }
        Err(e) => e,
    };
    let StoreError::Io(missing_io) = missing else {
        return Err(io::Error::other(format!("expected Io, got {missing:?}")).into());
    };
    assert!(
        matches!(missing_io.kind(), io::ErrorKind::NotFound),
        "wrong kind for missing parent: {missing_io:?}"
    );

    // Reading a directory fails closed with a NON-NotFound kind (C22 parity).
    assert!(
        matches!(fs.read(root), Err(ref e) if e.kind() != io::ErrorKind::NotFound),
        "PROPERTY: reading a directory through the whole-file seam must fail with a non-NotFound kind"
    );

    // Renaming an absent source is NotFound.
    assert!(
        matches!(
            fs.rename(&root.join("absent.fbat"), &root.join("dest.fbat")),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound
        ),
        "PROPERTY: renaming an absent source fails closed with NotFound"
    );
    Ok(())
}

#[test]
fn store_runs_end_to_end_over_shadow_fs() -> Result<(), Box<dyn std::error::Error>> {
    // A real `Store` over ShadowFs: open → keyed append → close → reopen
    // (cold-start) must recover the event, proving the backend is Store-ready
    // before the namespace matrix builds on it.
    let shadow = ShadowFs::new();
    let data_dir = Path::new("/shadow/e2e");

    let config = StoreConfig::new(data_dir)
        .with_fs(Arc::new(shadow.clone()))
        .with_sync_every_n_events(1);
    let store = Store::open(config).expect("open store over ShadowFs");
    let coord = Coordinate::new("entity:shadow", "scope:e2e").expect("coord");
    let receipt = store
        .append(&coord, KIND, &serde_json::json!({ "seq": 1 }))
        .expect("append over ShadowFs");
    assert!(
        store.verify_append_receipt(&receipt).is_valid(),
        "a store over ShadowFs must verify its own receipt"
    );
    store.close().expect("clean close");

    let reopened = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(shadow.clone())))
        .expect("reopen over the same shadow tree");
    assert_eq!(
        recovered_user_events(&reopened).len(),
        1,
        "PROPERTY: a clean-closed store over ShadowFs recovers its committed event on reopen"
    );
    reopened.close().expect("close reopened");
    Ok(())
}

// ─────────────────────── R1 crash-harness arming controls ──────────────────

#[test]
fn arm_op_error_at_targets_by_path_suffix() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("/shadow/op-at");
    let fs = fresh(root);
    let keep = root.join("keep.fbat");
    let doomed = root.join("doomed.fbat");
    write_durable(&fs, &keep, b"keep");
    write_durable(&fs, &doomed, b"doomed");

    // Only the first rename whose source ends with "doomed.fbat" fails.
    fs.arm_op_error_at(CrashOp::Rename, "doomed.fbat", 1);

    // A non-matching rename is unaffected and does not consume the strike.
    fs.rename(&keep, &root.join("keep-moved.fbat"))
        .expect("a non-matching rename succeeds");

    let torn = match fs.rename(&doomed, &root.join("doomed-moved.fbat")) {
        Ok(()) => {
            return Err(io::Error::other("PROPERTY: the suffix-matched rename must fail").into())
        }
        Err(e) => e,
    };
    assert!(
        torn.kind() != io::ErrorKind::NotFound,
        "an injected fault, not a missing source: {torn:?}"
    );
    assert_eq!(
        fs.read(&doomed)?,
        b"doomed",
        "PROPERTY: a torn rename leaves the source intact (nothing applied)"
    );
    assert!(
        fs.armed_faults_fired() >= 1,
        "anti-vacuity: the targeted fault fired"
    );
    Ok(())
}

#[test]
fn freeze_on_fault_poisons_subsequent_mutations() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("/shadow/freeze");
    let fs = fresh(root);
    let path = root.join("a.fbat");
    write_durable(&fs, &path, b"a-bytes");

    fs.set_freeze_on_fault(true);
    fs.arm_op_error(CrashOp::Rename, 1);

    assert!(
        fs.rename(&path, &root.join("b.fbat")).is_err(),
        "PROPERTY: the armed rename fires"
    );

    // Freeze poisons every subsequent MUTATING op until crash().
    let poisoned = match fs.create_new_file(&root.join("c.fbat")) {
        Ok(_) => {
            return Err(
                io::Error::other("PROPERTY: a frozen fs must refuse subsequent mutations").into(),
            )
        }
        Err(e) => e,
    };
    let StoreError::Io(_) = poisoned else {
        return Err(io::Error::other(format!("expected an Io poison, got {poisoned:?}")).into());
    };

    // Reads are unaffected by the freeze.
    assert_eq!(
        fs.read(&path)?,
        b"a-bytes",
        "PROPERTY: freeze poisons mutations but never reads"
    );
    Ok(())
}

#[test]
fn arm_file_sync_error_surfaces_on_the_nth_sync() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("/shadow/file-sync-err");
    let fs = fresh(root);
    let path = root.join("seg.fbat");
    fs.arm_file_sync_error(1);

    let mut file = fs.create_new_file(&path).expect("create");
    file.write_all(b"payload").expect("write");
    let injected = match file.sync_all() {
        Ok(()) => {
            return Err(io::Error::other("PROPERTY: the armed handle sync must fail").into())
        }
        Err(e) => e,
    };
    assert!(
        injected.kind() != io::ErrorKind::NotFound,
        "an injected sync error, not a lookup failure: {injected:?}"
    );
    assert!(
        fs.armed_faults_fired() >= 1,
        "anti-vacuity: the file-sync error fired"
    );
    Ok(())
}

#[test]
fn mutation_ops_records_the_ordered_mutating_op_log() {
    let root = Path::new("/shadow/ops-log");
    let fs = fresh(root);
    let path = root.join("seg.fbat");
    let moved = root.join("seg-moved.fbat");
    {
        let mut file = fs.create_new_file(&path).expect("create");
        file.write_all(b"x").expect("write");
        file.sync_all().expect("sync");
    }
    fs.sync_parent_dir(&path).expect("parent sync");
    fs.rename(&path, &moved).expect("rename");
    fs.remove_file(&moved).expect("remove");

    let ops = fs.mutation_ops();
    let kinds: Vec<SimOpKind> = ops.iter().map(|op| op.kind).collect();
    for expected in [
        SimOpKind::CreateNew,
        SimOpKind::Write,
        SimOpKind::SyncFile,
        SimOpKind::SyncParentDir,
        SimOpKind::Rename,
        SimOpKind::RemoveFile,
    ] {
        assert!(
            kinds.contains(&expected),
            "PROPERTY: the mutation log records every mutating primitive, missing {expected:?}"
        );
    }

    let rename_op = ops
        .iter()
        .find(|op| op.kind == SimOpKind::Rename)
        .expect("a rename op in the log");
    assert_eq!(
        rename_op.to.as_deref(),
        Some(moved.as_path()),
        "PROPERTY: a rename op records its destination in `to`"
    );

    let position = |kind: SimOpKind| {
        kinds
            .iter()
            .position(|observed| *observed == kind)
            .expect("a recorded op position")
    };
    assert!(
        position(SimOpKind::CreateNew) < position(SimOpKind::Rename),
        "PROPERTY: the log preserves order — create precedes rename"
    );
    assert!(
        position(SimOpKind::Rename) < position(SimOpKind::RemoveFile),
        "PROPERTY: the log preserves order — rename precedes remove"
    );
}

#[test]
fn arm_crash_at_op_fails_and_poisons_at_the_absolute_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("/shadow/crash-at-op");
    let path = root.join("seg.fbat");

    // Probe run: pin the absolute index of the parent-sync op in the schedule.
    let probe = fresh(root);
    {
        let mut file = probe.create_new_file(&path).expect("probe create");
        file.write_all(b"x").expect("probe write");
        file.sync_all().expect("probe sync");
    }
    probe.sync_parent_dir(&path).expect("probe parent sync");
    let boundary = probe
        .mutation_ops()
        .iter()
        .position(|op| op.kind == SimOpKind::SyncParentDir)
        .expect("a parent-sync op in the probe schedule");

    // Replay the SAME op order; the op at the armed absolute index tears.
    let fs = fresh(root);
    fs.arm_crash_at_op(boundary);
    {
        let mut file = fs.create_new_file(&path).expect("create");
        file.write_all(b"x").expect("write");
        file.sync_all().expect("sync");
    }
    let at_boundary = match fs.sync_parent_dir(&path) {
        Ok(()) => {
            return Err(
                io::Error::other("PROPERTY: the op at the armed absolute index must fail").into(),
            )
        }
        Err(e) => e,
    };
    let StoreError::Io(_) = at_boundary else {
        return Err(io::Error::other(format!("expected Io at the boundary, got {at_boundary:?}")).into());
    };

    // It poisons regardless of the freeze flag: the next mutation also fails.
    assert!(
        fs.remove_file(&path).is_err(),
        "PROPERTY: arm_crash_at_op poisons subsequent mutations independent of freeze"
    );
    assert!(
        fs.armed_faults_fired() >= 1,
        "anti-vacuity: the boundary crash fired"
    );
    Ok(())
}
