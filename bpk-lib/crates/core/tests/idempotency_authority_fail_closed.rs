//! GAUNT-IDEMPOTENCY-AUTHORITY (#189): the durable idempotency window is an
//! AUTHORITY all the way through admission, cold start, and compaction commit.
//!
//! PROVES: INV-IDEMPOTENCY-DURABLE-WINDOW (fail-closed) and
//! INV-IDEMPOTENCY-AUTHORITY-ATOMIC-COMPACTION — after retention eviction a
//! corrupt sidecar can never turn a retry into a second append; a missing
//! sidecar on a keyed store is authority LOSS, not a fresh store; admission is
//! by the `store.meta`-authorized image-generation TOKEN, so an unauthorized
//! image is refused whatever its frontier claims — an old self image (stale),
//! another lineage's image (foreign), a diverged sibling at the same covered
//! sequence, a FARTHER-AHEAD sibling, and an anchor-colliding sibling
//! (diverged elsewhere + same final keyed event) are all rejected. (The
//! compaction old-or-new commit half lives in
//! `idempotency_authority_compaction.rs`, and the per-boundary crash walk lives
//! in `compaction_crash_windows.rs` and `workload_crash_during_maintenance.rs`.)
//! CATCHES: the issue's named admission mutants — Invalid->Missing collapse,
//! removal of the expectation check, token comparison weakened to frontier
//! arithmetic (the at-or-past superset hole), and accepting keyed traffic on
//! unproven authority.
//! SEEDED: real stores with tiny segments; deterministic key constants; byte
//! transplants between sibling directories.

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::{EntityIdType, IdempotencyKey};
use batpak::store::{
    AppendOptions, CompactionConfig, CompactionStrategy, IdempAuthorityForeignKind, Store,
    StoreConfig, StoreError,
};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 4);
const IDEMP_FILENAME: &str = "index.idemp";

fn coord() -> Coordinate {
    Coordinate::new("entity:authority", "scope:fail-closed").expect("valid coord")
}

fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
}

fn append_keyed(store: &Store, key: u128) -> batpak::store::AppendReceipt {
    let payload_tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    store
        .append_with_options(
            &coord(),
            KIND,
            &serde_json::json!({ "k": payload_tag }),
            AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
        )
        .expect("keyed append")
}

fn key_event_count(store: &Store, key: u128) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_kind() == KIND && e.event_id().as_u128() == key)
        .count()
}

/// Retention strategy that evicts EVERY user event of `KIND` (keeps only the
/// batch/system markers), forcing keyed event frames out of the store.
fn evict_all_user_events() -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Retention(Box::new(|stored| {
            stored.event.header.event_kind != KIND
        })),
        min_segments: 1,
    }
}

/// Force sealed segments then run the evicting retention compaction; assert
/// the keyed event frame is really gone from the live index.
fn evict_key_via_compaction(store: &Store, key: u128) {
    for i in 0..8 {
        let _ = store
            .append(&coord(), KIND, &serde_json::json!({ "filler": i }))
            .expect("append filler event");
    }
    let (_result, _report) = store
        .compact(&evict_all_user_events())
        .expect("retention compaction");
    let survivors = store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_id().as_u128() == key)
        .count();
    assert_eq!(
        survivors, 0,
        "PRECONDITION: retention compaction evicted the keyed event frame"
    );
}

// ---------------------------------------------------------------------------
// Fixture 1 (+6): corrupt authority after retention eviction — the retry can
// NEVER append a second event; keyed traffic stays blocked until the
// authority is restored, then the original receipt comes back.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_authority_after_retention_never_reappends_key() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new().expect("tempdir");
    let key = 0x1111_2222_3333_4444_5555_6666_7777_8888u128;

    let first = {
        let store = Store::open(config(dir.path())).expect("open");
        let first = append_keyed(&store, key);
        evict_key_via_compaction(&store, key);
        store.close().expect("close");
        first
    };

    // The durable map is now the ONLY replay authority for `key`. Corrupt it.
    let path = dir.path().join(IDEMP_FILENAME);
    let healthy = std::fs::read(&path)?;
    let mut corrupt = healthy.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 0xFF;
    std::fs::write(&path, &corrupt)?;

    // Reopen refuses — no keyed append can be admitted on unproven authority.
    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): corrupt authority after eviction must refuse open — an \
                 admitted store would treat the retry as new and double-append",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityCorrupt { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };

    // Restore the authority: the retry returns the ORIGINAL receipt identity.
    std::fs::write(&path, &healthy)?;
    let store = Store::open(config(dir.path())).expect("restored authority admits the store");
    let replay = append_keyed(&store, key);
    assert_eq!(
        replay.global_sequence, first.global_sequence,
        "retry after restore returns the original sequence"
    );
    assert_eq!(
        u128::from(replay.event_id),
        u128::from(first.event_id),
        "retry after restore returns the original event id"
    );
    assert_eq!(
        replay.content_hash, first.content_hash,
        "retry after restore returns the original content hash"
    );
    assert_eq!(
        key_event_count(&store, key),
        0,
        "the evicted frame stays evicted — the retry re-appended NOTHING"
    );
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// Fixture 2: deleting an expected sidecar differs from a fresh store.
// ---------------------------------------------------------------------------
#[test]
fn missing_expected_authority_is_not_a_fresh_store() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0xAAAA_BBBB_CCCC_DDDD_EEEE_FFFF_0000_1111u128);
        store.close().expect("close");
    }
    std::fs::remove_file(dir.path().join(IDEMP_FILENAME))?;

    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): a deleted sidecar on a keyed store is authority LOSS, not a \
                 fresh store — open must refuse",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityMissing { path } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some(IDEMP_FILENAME),
        "the refusal names the expected sidecar; got {}",
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Fixture 3: a CRC-valid but STALE image is rejected via its coverage
// frontier.
// ---------------------------------------------------------------------------
#[test]
fn stale_crc_valid_authority_image_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(IDEMP_FILENAME);

    {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x1000_0000_0000_0000_0000_0000_0000_0001u128);
        store.close().expect("close");
    }
    let old_image = std::fs::read(&path)?;

    {
        let store = Store::open(config(dir.path())).expect("reopen");
        let _ = append_keyed(&store, 0x2000_0000_0000_0000_0000_0000_0000_0002u128);
        store.close().expect("close");
    }

    // Roll the sidecar back to the older, still-CRC-valid image.
    std::fs::write(&path, &old_image)?;

    let err =
        match Store::open(config(dir.path())) {
            Ok(_) => return Err(std::io::Error::other(
                "PROPERTY (#189): a stale CRC-valid image must be rejected through its coverage \
                 frontier — keys committed after it would silently vanish from the contract",
            )
            .into()),
            Err(e) => e,
        };
    let StoreError::IdempotencyAuthorityStale {
        image_covered,
        expected_covered,
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        image_covered < expected_covered,
        "the refusal reports the frontier regression ({image_covered} < {expected_covered})"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Foreign images: another store's image (lineage), and a diverged sibling
// fork's image at the same covered sequence (compound history anchor).
// ---------------------------------------------------------------------------
#[test]
fn foreign_lineage_image_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let a_dir = TempDir::new().expect("tempdir a");
    let b_dir = TempDir::new().expect("tempdir b");
    for dir in [&a_dir, &b_dir] {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x3000_0000_0000_0000_0000_0000_0000_0003u128);
        store.close().expect("close");
    }
    // Transplant B's (CRC-valid) image into A.
    std::fs::copy(
        b_dir.path().join(IDEMP_FILENAME),
        a_dir.path().join(IDEMP_FILENAME),
    )?;

    let err = match Store::open(config(a_dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): another lineage's authority image must be rejected",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityForeign { kind } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        matches!(kind, IdempAuthorityForeignKind::Lineage),
        "a transplanted other-store image is a LINEAGE mismatch; got {kind:?}"
    );
    Ok(())
}

#[test]
fn diverged_sibling_fork_image_is_rejected_via_history_anchor(
) -> Result<(), Box<dyn std::error::Error>> {
    // Sibling forks SHARE the lineage id (owner ruling, #205), so lineage +
    // numeric frontier alone would admit a transplanted sibling image. The
    // image-generation token is what refuses it (the sibling's publication was
    // never authorized by THIS store's metadata); the diagnostic anchor —
    // equal covered sequence, diverged event id — classifies the refusal as a
    // HISTORY-ANCHOR transplant rather than a stale self image.
    let a_dir = TempDir::new().expect("tempdir a");
    let f_dir = TempDir::new().expect("fork dir");

    {
        let store = Store::open(config(a_dir.path())).expect("open");
        let _ = append_keyed(&store, 0x4000_0000_0000_0000_0000_0000_0000_0004u128);
        store.fork(f_dir.path()).expect("fork");
        store.close().expect("close");
    }

    // Equalize the sibling frontiers with unkeyed filler FIRST, then diverge
    // each with its own keyed event at the SAME recorded position: the two
    // images end anchor-equal in covered sequence, so the transplant below
    // cannot be caught by frontier comparison — only the unauthorized token,
    // classified by the diverged event id at that sequence.
    let a = Store::open(config(a_dir.path())).expect("reopen a");
    let f = Store::open(config(f_dir.path())).expect("open fork");
    for _ in 0..64 {
        let (ga, gf) = (a.stats().global_sequence, f.stats().global_sequence);
        if ga == gf {
            break;
        }
        let lagging = if ga < gf { &a } else { &f };
        let _ = lagging
            .append(&coord(), KIND, &serde_json::json!({"filler": "top-up"}))
            .expect("top-up filler append");
    }
    let leveled = (a.stats().global_sequence, f.stats().global_sequence);
    assert_eq!(
        leveled.0, leveled.1,
        "PRECONDITION: both siblings sit at the same frontier before the divergent keyed appends"
    );
    let _ = append_keyed(&a, 0x5000_0000_0000_0000_0000_0000_0000_0005u128);
    let _ = append_keyed(&f, 0x6000_0000_0000_0000_0000_0000_0000_0006u128);
    a.close().expect("close a");
    f.close().expect("close fork");

    // Transplant the fork's image into A.
    std::fs::copy(
        f_dir.path().join(IDEMP_FILENAME),
        a_dir.path().join(IDEMP_FILENAME),
    )?;

    let err = match Store::open(config(a_dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): a diverged sibling fork's image must be rejected — its \
                 publication token was never authorized by this store's metadata",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityForeign { kind } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        matches!(kind, IdempAuthorityForeignKind::HistoryAnchor),
        "a same-frontier sibling transplant is a HISTORY-ANCHOR divergence; got {kind:?}"
    );
    Ok(())
}

#[test]
fn farther_ahead_sibling_image_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // The blind "at-or-past = legal superset" admission hole: a sibling fork
    // that advanced FARTHER than this store shares the lineage and carries an
    // anchor past the local expectation, so frontier arithmetic would admit
    // its transplanted image — returning receipts for operations that only
    // ever happened in the sibling. The generation token refuses it: the
    // fork's publication was never authorized by A's metadata.
    let a_dir = TempDir::new().expect("tempdir a");
    let f_dir = TempDir::new().expect("fork dir");

    {
        let store = Store::open(config(a_dir.path())).expect("open");
        let _ = append_keyed(&store, 0x4000_0000_0000_0000_0000_0000_0000_0004u128);
        store.fork(f_dir.path()).expect("fork");
        store.close().expect("close");
    }

    // Advance ONLY the fork, well past A's frontier.
    {
        let f = Store::open(config(f_dir.path())).expect("open fork");
        let _ = append_keyed(&f, 0x6000_0000_0000_0000_0000_0000_0000_0006u128);
        let _ = append_keyed(&f, 0x7000_0000_0000_0000_0000_0000_0000_0007u128);
        for _ in 0..8 {
            let _ = f
                .append(&coord(), KIND, &serde_json::json!({"filler": "ahead"}))
                .expect("advance the fork");
        }
        f.close().expect("close fork");
    }

    // Transplant the farther-ahead sibling image into A.
    std::fs::copy(
        f_dir.path().join(IDEMP_FILENAME),
        a_dir.path().join(IDEMP_FILENAME),
    )?;

    let err = match Store::open(config(a_dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): a farther-ahead sibling image must be rejected — \
                 at-or-past frontier arithmetic must never admit an unauthorized image",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityForeign { kind } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        matches!(kind, IdempAuthorityForeignKind::HistoryAnchor),
        "a farther-ahead sibling transplant classifies as HISTORY-ANCHOR foreign; got {kind:?}"
    );
    Ok(())
}

#[test]
fn diverge_elsewhere_then_append_same_final_keyed_event_rejects_transplant(
) -> Result<(), Box<dyn std::error::Error>> {
    // Per-entity hash chains are NOT a commitment to full store history: two
    // siblings can diverge in one entity, then append the SAME final keyed
    // event (same key → same event id) to another entity at the same
    // sequence — the anchors collide as far as frontier/event-id comparison
    // can see. Only the generation token distinguishes the histories.
    let a_dir = TempDir::new().expect("tempdir a");
    let f_dir = TempDir::new().expect("fork dir");
    let elsewhere = Coordinate::new("entity:elsewhere", "scope:fail-closed").expect("valid coord");

    {
        let store = Store::open(config(a_dir.path())).expect("open");
        let _ = append_keyed(&store, 0x4000_0000_0000_0000_0000_0000_0000_0004u128);
        store.fork(f_dir.path()).expect("fork");
        store.close().expect("close");
    }

    // Diverge ONLY in another entity (same event count, different payloads).
    let a = Store::open(config(a_dir.path())).expect("reopen a");
    let f = Store::open(config(f_dir.path())).expect("open fork");
    let _ = a
        .append(&elsewhere, KIND, &serde_json::json!({"branch": "a"}))
        .expect("diverge a elsewhere");
    let _ = f
        .append(&elsewhere, KIND, &serde_json::json!({"branch": "f"}))
        .expect("diverge fork elsewhere");
    // Level the frontiers (lifecycle events skew them), then both append the
    // IDENTICAL final keyed event at the SAME recorded position.
    for _ in 0..64 {
        let (ga, gf) = (a.stats().global_sequence, f.stats().global_sequence);
        if ga == gf {
            break;
        }
        let lagging = if ga < gf { &a } else { &f };
        let _ = lagging
            .append(&elsewhere, KIND, &serde_json::json!({"filler": "level"}))
            .expect("leveling filler append");
    }
    let leveled = (a.stats().global_sequence, f.stats().global_sequence);
    assert_eq!(
        leveled.0, leveled.1,
        "PRECONDITION: equal frontiers before the identical final keyed appends"
    );
    let same_key = 0x5000_0000_0000_0000_0000_0000_0000_0005u128;
    let _ = append_keyed(&a, same_key);
    let _ = append_keyed(&f, same_key);
    a.close().expect("close a");
    f.close().expect("close fork");

    // Transplant the fork's image into A.
    std::fs::copy(
        f_dir.path().join(IDEMP_FILENAME),
        a_dir.path().join(IDEMP_FILENAME),
    )?;

    let err = match Store::open(config(a_dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): an anchor-colliding sibling image (diverged elsewhere, same \
                 final keyed event) must be rejected by the unauthorized generation token",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityForeign { kind } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        matches!(kind, IdempAuthorityForeignKind::HistoryAnchor),
        "an anchor-colliding sibling transplant is still HISTORY-ANCHOR foreign; got {kind:?}"
    );
    Ok(())
}
