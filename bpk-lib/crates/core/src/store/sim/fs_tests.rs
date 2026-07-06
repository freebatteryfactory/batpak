//! Unit tests for [`super::SimFs`] — split out of `fs.rs` to keep that
//! production file under the structural line cap. Loaded via `#[path]`, so
//! `super` is the `sim::fs` module (imports resolve through `use super::*`).

use super::*;

#[test]
fn same_seed_same_fsync_drop_schedule() {
    let a = SimFs::new(99, 4);
    let b = SimFs::new(99, 4);
    let pa: Vec<_> = (0..64).map(|_| a.state.fsync_dropped()).collect();
    let pb: Vec<_> = (0..64).map(|_| b.state.fsync_dropped()).collect();
    assert_eq!(
        pa, pb,
        "PROPERTY: identical seeds produce identical fsync-drop schedules"
    );
}

#[test]
fn crash_truncates_to_durable_length() {
    let dir = tempfile::tempdir().expect("tmpdir");
    // Never drop syncs here so the durability is purely the unsynced tail.
    let fs = SimFs::new(1, 0);
    let path = dir.path().join("seg.fbat");
    let mut file = fs.create_new_file(&path).expect("create");
    file.write_all(b"durable").expect("write durable");
    file.sync_all().expect("honored sync");
    let durable = fs.durable_len(&path);
    // Write more, do NOT route a sync through the fault layer: this tail
    // must be lost on crash. Flush the real bytes through the platform
    // seam (the structural gate forbids a bare `.sync_all()` outside
    // src/store/platform) so the tail is genuinely on disk before the
    // crash truncates it back to durable_len.
    file.write_all(b"-and-lost-tail").expect("write tail");
    crate::store::platform::sync::sync_file_all_io(
        file.as_std_file()
            .expect("SimFs over RealFs mints real files"),
    )
    .expect("flush real bytes to disk");
    fs.crash();
    let recovered = crate::store::platform::fs::metadata(&path)
        .expect("stat")
        .len();
    assert_eq!(
        recovered, durable,
        "PROPERTY: a crash truncates the real file to its last durable (synced) length"
    );
    assert_eq!(
        recovered,
        b"durable".len() as u64,
        "PROPERTY: only the synced prefix survives the crash"
    );
}

#[test]
fn dropped_fsync_does_not_advance_durable_length() {
    let dir = tempfile::tempdir().expect("tmpdir");
    // Always drop syncs (1-in-1): durable length must never advance.
    let fs = SimFs::new(7, 1);
    let path = dir.path().join("seg.fbat");
    let mut file = fs.create_new_file(&path).expect("create");
    file.write_all(b"unsynced").expect("write");
    crate::store::platform::sync::sync_file_all_io(
        file.as_std_file()
            .expect("SimFs over RealFs mints real files"),
    )
    .expect("flush real bytes");
    file.sync_all()
        .expect("dropped sync still returns Ok to the store");
    assert_eq!(
        fs.durable_len(&path),
        0,
        "PROPERTY: a dropped sync returns Ok but never advances the durable length"
    );
    fs.crash();
    assert_eq!(
        crate::store::platform::fs::metadata(&path)
            .expect("stat")
            .len(),
        0,
        "PROPERTY: an all-dropped-sync file loses its entire tail on crash"
    );
}

#[test]
fn remove_dir_all_removes_the_directory_entry_over_every_inner() {
    // SimFs is a fault LAYER; directory removal must remove the ENTRY, not
    // merely clear contents like the trait default. Otherwise fork/snapshot
    // cleanup over a SimFs-wrapped backend reports a cursor directory
    // removed while it survives, leaving stale destination state. Proven
    // over both a real-file and an in-memory inner — delegation must hold
    // regardless of backend.
    let real_dir = tempfile::tempdir().expect("tmpdir");
    let cases: Vec<(SimFs, std::path::PathBuf)> = vec![
        (
            SimFs::layered(0x5117_D111, 0, Arc::new(RealFs)),
            real_dir.path().join("store"),
        ),
        (
            SimFs::layered(
                0x5117_D222,
                0,
                Arc::new(crate::store::platform::mem_fs::MemFs::new()),
            ),
            std::path::PathBuf::from("/virtual/sim-remove-dir"),
        ),
    ];

    for (fs, root) in cases {
        let dir = root.join("cursors");
        fs.create_dir_all(&dir).expect("seed directory");
        let child = dir.join("resume.ckpt");
        let mut file = fs.create_new_file(&child).expect("create child");
        file.write_all(b"checkpoint").expect("write child");
        file.sync_all().expect("sync child");
        drop(file);

        assert!(
            fs.remove_dir_all_if_present(&dir)
                .expect("remove existing directory"),
            "an existing directory reports removed"
        );
        // The ENTRY is gone: the child reads NotFound and a second removal
        // is a clean no-op. The trait-default gap would keep the empty
        // directory alive and report Ok(true) again here.
        assert!(
            matches!(fs.read(&child), Err(error) if error.kind() == io::ErrorKind::NotFound),
            "a child under a removed directory must be gone"
        );
        assert!(
            !fs.remove_dir_all_if_present(&dir)
                .expect("second removal of an absent directory"),
            "an already-removed directory reports Ok(false) — the entry is truly gone"
        );
    }
}

#[test]
fn crash_truncates_the_virtual_inner_even_when_a_host_file_shadows_the_path() {
    // A blind host truncate can succeed against an unrelated same-named HOST
    // file when the inner is virtual, skipping the seam rewrite — leaving
    // the virtual file at its full pre-crash length and mutating that host
    // file. Reproduce the collision: a real host file AND a MemFs file at
    // the same path, under a SimFs layered over MemFs.
    let host_dir = tempfile::tempdir().expect("host tmpdir");
    let path = host_dir.path().join("seg.fbat");

    // A decoy host file at `path` — a blind host truncate would hit THIS.
    let real = RealFs;
    {
        let mut decoy = real.create_new_file(&path).expect("create host decoy");
        decoy
            .write_all(b"HOST-DECOY-must-not-be-touched")
            .expect("write decoy");
    }

    // The real store lives in MemFs (Arc-shared tree), layered under SimFs.
    let mem = crate::store::platform::mem_fs::MemFs::new();
    mem.create_dir_all(host_dir.path())
        .expect("seed the virtual data dir");
    let sim = SimFs::layered(0x5117_C0DE, 0, Arc::new(mem.clone()));

    let mut file = sim.create_new_file(&path).expect("create virtual file");
    file.write_all(b"durable").expect("write durable prefix");
    file.sync_all().expect("honored sync records durable_len");
    assert_eq!(
        sim.durable_len(&path),
        7,
        "the durable prefix is the synced 'durable' bytes"
    );
    // Append an unsynced tail — it must be lost on crash.
    file.write_all(b"-and-lost-tail").expect("write tail");
    drop(file);

    sim.crash();

    // The VIRTUAL file is truncated to its durable prefix, read back through
    // the inner MemFs — not left full because a host file shadowed the path.
    assert_eq!(
        mem.read(&path).expect("read virtual file"),
        b"durable",
        "SimFs crash must truncate the virtual inner via the seam, not skip it"
    );
    // The unrelated host file is untouched — the crash never truncated it.
    assert_eq!(
        real.read(&path).expect("read host decoy"),
        b"HOST-DECOY-must-not-be-touched",
        "a virtual-inner crash must not mutate an unrelated same-named host file"
    );
}

#[test]
fn a_dropped_staged_sync_loses_the_publish_on_crash() {
    // A staged atomic publish (cursor checkpoint, keyset, visibility ranges,
    // cold-start artifact) whose staged `sync_all` is DROPPED must not survive a
    // crash — the metadata bytes were never durable. `fsync_drop_one_in = 1`
    // drops every sync, so this staged sync is dropped.
    let mem = crate::store::platform::mem_fs::MemFs::new();
    let dir = std::path::Path::new("/virtual/staged-drop");
    mem.create_dir_all(dir).expect("seed dir");
    let sim = SimFs::layered(0xDACE_D40F, 1, Arc::new(mem.clone()));

    let final_path = dir.join("cursor.ckpt");
    let mut staged = sim.named_temp_in(dir).expect("stage");
    staged.write_all(b"checkpoint-bytes").expect("write staged");
    staged
        .sync_all()
        .expect("staged sync (dropped by the schedule)");
    let admission = crate::store::platform::sync::admit_current_parent_dir_sync()
        .expect("mint parent-dir-sync admission");
    staged.persist(&final_path, admission).expect("publish");

    // The publish landed on the medium...
    assert_eq!(
        mem.read(&final_path).expect("read published"),
        b"checkpoint-bytes"
    );
    sim.crash();
    // ...but a DROPPED staged sync means the crash loses it (zero durable prefix).
    assert!(
        mem.read(&final_path).expect("read after crash").is_empty(),
        "a dropped staged sync must lose the publish on crash, not survive it"
    );
}

#[test]
fn an_honored_staged_sync_survives_a_crash() {
    // The dual: with no sync drops, the honored staged sync makes the publish
    // durable, so it survives the crash intact.
    let mem = crate::store::platform::mem_fs::MemFs::new();
    let dir = std::path::Path::new("/virtual/staged-honored");
    mem.create_dir_all(dir).expect("seed dir");
    let sim = SimFs::layered(0xDACE_5A7E, 0, Arc::new(mem.clone()));

    let final_path = dir.join("cursor.ckpt");
    let mut staged = sim.named_temp_in(dir).expect("stage");
    staged.write_all(b"checkpoint-bytes").expect("write staged");
    staged.sync_all().expect("staged sync (honored)");
    let admission = crate::store::platform::sync::admit_current_parent_dir_sync()
        .expect("mint parent-dir-sync admission");
    staged.persist(&final_path, admission).expect("publish");

    sim.crash();
    assert_eq!(
        mem.read(&final_path).expect("read after crash"),
        b"checkpoint-bytes",
        "an honored staged sync makes the publish durable across a crash"
    );
}

#[test]
fn mmap_probe_scratch_removal_does_not_consume_a_remove_file_fault() {
    // An mmap-capability probe (platform evidence collection) creates and removes
    // a scratch file through the seam. Under SimFs that scratch removal must NOT
    // consume an armed RemoveFile fault meant for a real store remove — otherwise
    // a fault-injection experiment that also collects evidence loses determinism.
    let dir = tempfile::tempdir().expect("tmpdir");
    // Arm the RemoveFile fault on its FIRST occurrence, over a real inner.
    let sim = SimFs::new(0x0BE1_F00D, 0).with_fault_on(CrashOp::RemoveFile, 1);

    // A real mmap-evidence probe: create_new_file + two scratch removes. Over a
    // real inner it maps a real file (FileBacked), and the exempted scratch
    // removes do NOT advance the RemoveFile schedule.
    let evidence = crate::store::platform::evidence::mmap_evidence_for_store_path(dir.path(), &sim);
    assert_eq!(
        evidence,
        crate::store::stats::MmapEvidence::FileBacked,
        "the probe over a real inner reports FileBacked without consuming the fault"
    );

    // The armed RemoveFile fault is STILL pending: the first REAL remove faults.
    let victim = dir.path().join("segment.fbat");
    {
        let mut file = sim.create_new_file(&victim).expect("create victim");
        file.write_all(b"x").expect("write victim");
    }
    assert!(
        sim.remove_file(&victim).is_err(),
        "the armed RemoveFile fault must still fire on the first real remove — the \
         probe's scratch removes must not have consumed it"
    );
}
