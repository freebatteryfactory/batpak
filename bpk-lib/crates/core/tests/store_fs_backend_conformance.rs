//! MemFs default-lane runtime falsifier (#168).
//!
//! PROVES a real `Store` runs its full life cycle over a pure in-memory
//! backend — open → append → durable wait → read → receipt verify → close →
//! REOPEN (cold-start rehydration) — without ever touching the host
//! filesystem. This is issue #168's checklist executed natively; the premise
//! it falsified ("SimFs is an in-memory StoreFs") is replaced by an actual
//! one, and the store's directory lock + diagnostics seam are exercised over
//! the virtual backend too.
//!
//! The documented `StoreFs` backend-contract corpus itself now lives at
//! `batpak::store::conformance` (feature `dangerous-test-hooks`), driven by
//! `tests/store_fs_conformance_corpus.rs` — including the corpus-bites RED
//! fixture. This file deliberately keeps only the DEFAULT-LANE Store-over-MemFs
//! falsifiers so the runtime path stays proven without any feature.

use std::path::Path;
use std::sync::Arc;

use batpak::prelude::*;
use batpak::store::{MemFs, StoreError};

#[derive(serde::Serialize, serde::Deserialize, EventPayload)]
#[batpak(category = 0xF, type_id = 22)]
struct MemProbe {
    value: i64,
}

#[test]
fn store_runs_end_to_end_on_a_purely_in_memory_backend() -> Result<(), Box<dyn std::error::Error>> {
    // The #168 falsifier, natively: no tempdir, no host path — the "data
    // dir" exists only inside the MemFs tree. Clone the backend so the
    // second open sees the same tree (Arc-shared state).
    let fs = MemFs::new();
    let data_dir = Path::new("/virtual/store");

    let config = StoreConfig::new(data_dir)
        .with_fs(Arc::new(fs.clone()))
        .with_sync_every_n_events(1);
    let store = Store::open(config)?;

    let coord = Coordinate::new("entity:mem", "scope:e2e")?;
    let receipt = store.append_typed(&coord, &MemProbe { value: 41 })?;
    let fetched = store.get(receipt.event_id)?;
    assert_eq!(fetched.event.header.event_id, receipt.event_id);
    assert!(
        store.verify_append_receipt(&receipt).is_valid(),
        "a store on a purely in-memory backend must verify its own receipt"
    );
    store.close()?;

    // REOPEN over the same in-memory tree: cold-start rehydration (segment
    // scan / cold-start artifacts / dir lock) must run entirely through the
    // seam and recover the appended event.
    let reopened = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone())))?;
    let recovered = reopened.get(receipt.event_id)?;
    assert_eq!(
        recovered.event.header.event_id, receipt.event_id,
        "reopen over the in-memory tree must recover the appended event"
    );
    let second = reopened.append_typed(&coord, &MemProbe { value: 42 })?;
    assert!(
        reopened.verify_append_receipt(&second).is_valid(),
        "appends after an in-memory reopen must keep verifying"
    );
    reopened.close()?;

    Ok(())
}

#[test]
fn in_memory_store_directory_lock_excludes_a_second_open() -> Result<(), Box<dyn std::error::Error>>
{
    // The runtime IS the lock for a virtual backend: while one store holds
    // the in-process registry entry, a second open over the same tree and
    // path must fail with the exact StoreLocked variant.
    let fs = MemFs::new();
    let data_dir = Path::new("/virtual/locked");

    let store = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone())))?;
    let second = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone())));
    match second {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: a second open over a live in-memory store must be excluded",
            )
            .into())
        }
        Err(error) => assert!(
            matches!(error, StoreError::StoreLocked { .. }),
            "expected StoreLocked, got {error}"
        ),
    }
    store.close()?;
    Ok(())
}

/// Issue #171: `Store::diagnostics()` must resolve platform evidence THROUGH the
/// configured backend, never the host filesystem. This covers both failure
/// shapes. First, a `data_dir` that ALSO exists on the host (a real tempdir):
/// the pre-fix mmap probe created a `NamedTempFile` there and reported
/// `FileBacked`. Second, a PURELY virtual `data_dir` (`/virtual/...`, absent on
/// the host): the pre-fix `path_status` used host `metadata`, saw `NotFound`,
/// and reported the existing virtual store as `Unknown`. After the fix
/// `path_status` and the mmap probe both route through `fs`, so a MemFs store
/// reports `ObservedUnsupported` (never `FileBacked`/`Unknown`) and never
/// touches the host.
#[test]
fn diagnostics_over_memfs_reports_virtual_mmap_evidence_not_file_backed() {
    let host_dir = tempfile::tempdir().expect("real host tempdir");
    let data_dirs: [&Path; 2] = [host_dir.path(), Path::new("/virtual/diag-store")];

    for data_dir in data_dirs {
        let fs = MemFs::new();
        let config = StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone()));
        let store = Store::open(config).expect("open MemFs store");

        let diagnostics = store.diagnostics();
        assert_eq!(
            diagnostics.platform_evidence.store_path.mmap_index,
            batpak::store::stats::MmapEvidence::ObservedUnsupported,
            "MemFs diagnostics must report ObservedUnsupported for data_dir {data_dir:?}, \
             never FileBacked (host tempfile probe) or Unknown (host path_status)"
        );

        store.close().expect("close");
    }
}
