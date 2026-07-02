//! Round-4 (Cluster A) mutation kills for the ALL-FEATURES repo-wide shard —
//! crypto-shred keyset granularity persistence plus the seeded-sim replay-seed,
//! canonical-refusal, and fault-mode-label seams.
//!
//! PROVES: (1) a persisted `PerCategory` key-scope granularity round-trips
//! through the durable keyset file — the reloaded store recovers the key under
//! its original per-category scope and the pre-flush ciphertext still opens —
//! and a MISMATCHED configured granularity is refused fail-closed NAMING
//! PerCategory as the persisted side; (2) `fork_fault_replay_seed` returns the
//! caller's default when `BATPAK_SEED` is absent and always agrees with the
//! shared `seed_from_env` entry point (`replay_seed`) for distinct defaults;
//! (3) every `MatrixCell.mode` label in the public recovery-matrix report
//! carries the exact stable per-mode string, in matrix order.
//! CATCHES: keyscope/persist.rs:95 deletion of the `DISC_PER_CATEGORY` match
//! arm in `granularity_from_disc` (a PerCategory keyset would reload as
//! "unknown discriminant 2" — an accidental total crypto-shred refusal);
//! fork_recovery.rs:138 `fork_fault_replay_seed -> 0` (replay would silently
//! pin every fork-fault run to seed 0); recovery_matrix.rs:605
//! `mode_label -> String::new()` (every matrix report cell would lose its
//! fault-mode identity).
//! SEEDED: deterministic — fixed keyset nonces/scopes; the matrix-label sweep
//! reuses the known-legal inline witness seed 0x5EED_B301; no wall-clock, no
//! sleeps, no env mutation (`std::env::set_var` is banned repo-wide as
//! thread-unsafe — the env leg is pinned structurally against the shared seed
//! source instead).
//!
//! NOTE (round-4 survivor NOT killed here): recovery.rs:181 match guard
//! `is_canonical_refusal(&error) -> false` in the honest-disk B2 `drive` is a
//! defensively dead arm for every input reachable through
//! `run_seeded_recovery(seed, steps)`: the driver is honest-disk only
//! (`SimFs::new(seed, 0)`, no injector wired), fsyncs happen at frame-aligned
//! writer quiesce points, so `SimFs::crash` truncation always leaves a
//! well-formed prefix and the reopen cannot refuse. A 3072-run differential
//! sweep (seeds 0..768 x steps {4, 8, 48, 96}) with the mutant hand-applied
//! was byte-identical to the real code (zero reopen errors on either side).
//! The LIVE refusal guard is the distinct site recovery_matrix.rs:333,
//! exercised by the B3 crash-before-fsync cells.

/// KILLS keyscope/persist.rs:95 — delete match arm `DISC_PER_CATEGORY =>
/// Some(KeyScopeGranularity::PerCategory)` in `granularity_from_disc`. The
/// round-2 kill pins the whole-function `-> Some(Default)` stub via PerEvent;
/// this PER-ARM deletion needs the PerCategory-specific pin: with the arm
/// gone, disc 2 falls through to `None` and a freshly flushed PerCategory
/// keyset refuses to load as "unknown key-scope granularity discriminant 2".
#[cfg(feature = "payload-encryption")]
mod keyset_per_category {
    use batpak::coordinate::Coordinate;
    use batpak::event::EventKind;
    use batpak::id::EventId;
    use batpak::store::{scope_for, KeyScope, KeyScopeGranularity, KeyStore, StoreError};

    const NONCE: [u8; 24] = [0x4C; 24];

    fn scope(granularity: KeyScopeGranularity, entity: &str) -> KeyScope {
        let coord = Coordinate::new(entity, "scope:keyset-mk4").expect("coordinate");
        scope_for(
            granularity,
            &coord,
            EventKind::custom(0xF, 2),
            EventId::from(11_u128),
        )
    }

    #[test]
    fn a_percategory_granularity_round_trips_and_a_mismatch_names_percategory() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let gran = KeyScopeGranularity::PerCategory;
        assert_ne!(
            gran,
            KeyScopeGranularity::default(),
            "fixture must exercise the NON-default PerCategory granularity"
        );
        let target = scope(gran, "entity:per-category");

        let mut store = KeyStore::new(gran);
        let ciphertext = store
            .get_or_create(&target)
            .expect("mint per-category key")
            .seal(&NONCE, b"aad-mk4", b"per-category round-trip")
            .expect("seal");
        store.flush(dir.path()).expect("flush PerCategory keyset");

        // THE KILL: with the DISC_PER_CATEGORY arm deleted, this load refuses
        // as "unknown key-scope granularity discriminant 2" instead of
        // rehydrating — an accidental total crypto-shred of the keyset.
        let reloaded = KeyStore::load(dir.path(), gran)
            .expect("PROPERTY: a persisted PerCategory granularity must round-trip");
        assert_eq!(reloaded.key_count(), 1, "the per-category key recovers");
        assert_eq!(
            reloaded
                .get(&target)
                .expect("the recovered key is filed under its original per-category scope")
                .open(&NONCE, b"aad-mk4", &ciphertext)
                .expect("recovered key opens the pre-flush ciphertext")
                .as_slice(),
            b"per-category round-trip",
        );

        // Fail-closed leg: a mismatched configured granularity must refuse
        // NAMING PerCategory as the persisted side. The arm-deletion mutant
        // cannot produce this reason either — it reports discriminant 2 as
        // unknown before the mismatch check is ever reached.
        let mismatched = KeyStore::load(dir.path(), KeyScopeGranularity::PerEntity)
            .err()
            .expect("PROPERTY: a granularity mismatch must fail the load closed");
        assert!(
            matches!(
                &mismatched,
                StoreError::KeysetCorrupt { reason }
                    if reason.contains(
                        "granularity PerEntity does not match persisted keyset granularity \
                         PerCategory"
                    )
            ),
            "PROPERTY: the mismatch refusal must be KeysetCorrupt naming PerCategory as the \
             persisted granularity; got {mismatched:?}"
        );
    }
}

/// KILLS the seeded-sim survivors (dangerous-test-hooks surface): the fork
/// replay-seed passthrough, the canonical-refusal guard in the honest-disk
/// recovery driver, and the recovery-matrix mode labels.
#[cfg(feature = "dangerous-test-hooks")]
mod sim_replay_and_labels {
    use batpak::__sim::{fork_fault_replay_seed, replay_seed, run_recovery_matrix};

    /// KILLS fork_recovery.rs:138 `fork_fault_replay_seed -> 0`. Default leg:
    /// absent `BATPAK_SEED`, the helper returns the caller's default (tolerant
    /// of a replay run that exports the variable, mirroring the
    /// `seed_from_env` unit-test pattern — the process env is never mutated,
    /// `std::env::set_var` is banned repo-wide as thread-unsafe). Env leg,
    /// pinned structurally: the helper must agree with `replay_seed` (the
    /// shared `seed_from_env` entry) for DISTINCT defaults under WHATEVER env
    /// state the process has — a `-> 0` stub disagrees the moment the shared
    /// source returns anything nonzero.
    #[test]
    fn fork_fault_replay_seed_returns_the_default_and_tracks_the_shared_seed_source() {
        let default_a = 0xF00F_5EED_u64;
        let default_b = 0x0B2_0B3_0B4_u64;

        let chosen = fork_fault_replay_seed(default_a);
        assert!(
            chosen == default_a || std::env::var("BATPAK_SEED").is_ok(),
            "PROPERTY: absent BATPAK_SEED, fork_fault_replay_seed returns the supplied \
             default 0x{default_a:X}; got 0x{chosen:X}"
        );
        assert_eq!(
            fork_fault_replay_seed(default_a),
            replay_seed(default_a),
            "PROPERTY: fork_fault_replay_seed delegates to the shared BATPAK_SEED source \
             (default 0x{default_a:X})"
        );
        assert_eq!(
            fork_fault_replay_seed(default_b),
            replay_seed(default_b),
            "PROPERTY: fork_fault_replay_seed delegates to the shared BATPAK_SEED source \
             (default 0x{default_b:X})"
        );
    }

    /// KILLS recovery_matrix.rs:605 `mode_label -> String::new()`. The mode
    /// label is a REPORT FIELD of the public matrix sweep (`MatrixCell.mode`),
    /// consumed by the recovery-oracle test to identify cells; every cell must
    /// carry its exact stable per-mode string, in matrix order.
    #[test]
    fn matrix_cells_carry_the_exact_stable_mode_labels_in_matrix_order() {
        // Seed/steps mirror the inline `every_mode_recovers_legally` witness,
        // so every cell is known-legal and the sweep returns all seven cells.
        let cells = run_recovery_matrix(0x5EED_B301, 64)
            .expect("the full fault matrix recovers legally at the witness seed");
        let labels: Vec<&str> = cells.iter().map(|cell| cell.mode.as_str()).collect();
        assert_eq!(
            labels,
            [
                "honest-disk-crash",
                "lying-disk-fsync-drop-1-in-2",
                "lying-disk-fsync-drop-1-in-5",
                "crash-before-fsync@SingleAppendFrame",
                "crash-before-fsync@BatchCommitMarker",
                "crash-before-fsync@BatchPostFsyncPrePublish",
                "crash-before-fsync@SegmentRotationCreate",
            ],
            "PROPERTY: every matrix report cell names its fault mode with the exact stable \
             label, in matrix order"
        );
    }
}
