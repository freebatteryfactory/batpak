use super::*;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

#[test]
fn validated_runtime_clock_wraps_direct_field_assignment() {
    let raw = Arc::new(AtomicI64::new(2_000));
    let raw_clock = {
        let raw = Arc::clone(&raw);
        Arc::new(move || raw.load(Ordering::SeqCst)) as Arc<dyn Fn() -> i64 + Send + Sync>
    };

    let mut config = StoreConfig::new("target/test-clock-wrap");
    config.clock = Some(clock_from_fn(raw_clock));

    let runtime = config.validated().expect("config validates");
    assert_eq!(runtime.now_us(), 2_000);

    raw.store(1_500, Ordering::SeqCst);
    assert_eq!(
        runtime.now_us(),
        2_000,
        "validated runtime clock must clamp direct-field regressions"
    );
}

#[test]
fn cache_now_us_clamps_negative_custom_clock_values() {
    let raw_clock = Arc::new(|| -42_i64) as Arc<dyn Fn() -> i64 + Send + Sync>;
    let mut config = StoreConfig::new("target/test-cache-clock-clamp");
    config.clock = Some(clock_from_fn(raw_clock));

    let runtime = config.validated().expect("config validates");
    assert_eq!(
        runtime.cache_now_us(),
        0,
        "projection/cache metadata clock must not persist negative timestamps"
    );
}

#[test]
fn cache_now_us_preserves_zero_custom_clock_value() {
    let raw_clock = Arc::new(|| 0_i64) as Arc<dyn Fn() -> i64 + Send + Sync>;
    let mut config = StoreConfig::new("target/test-cache-clock-zero");
    config.clock = Some(clock_from_fn(raw_clock));

    let runtime = config.validated().expect("config validates");
    assert_eq!(
        runtime.cache_now_us(),
        0,
        "PROPERTY: zero is a valid cache timestamp boundary, not a negative-clock violation"
    );
}

#[test]
fn signing_policy_required_refuses_keyless_store() {
    use crate::store::signing::SigningKey;

    // RED: SigningPolicy::Required with no signing key must FAIL validation — a
    // store that cannot sign must never silently accept unsigned receipts as valid.
    assert!(
        StoreConfig::new("target/test-signing-required-keyless")
            .with_signing_policy(SigningPolicy::Required)
            .validated()
            .is_err(),
        "Required + no key must fail closed at validation"
    );

    // GREEN: the default (Optional) policy permits a keyless store.
    assert!(
        StoreConfig::new("target/test-signing-optional-keyless")
            .validated()
            .is_ok(),
        "default Optional must permit a keyless store"
    );

    // GREEN: explicitly choosing Optional also permits a keyless store.
    assert!(
        StoreConfig::new("target/test-signing-optional-explicit")
            .with_signing_policy(SigningPolicy::Optional)
            .validated()
            .is_ok(),
        "explicit Optional must permit a keyless store"
    );

    // GREEN: Required validates once a signing key is configured.
    assert!(
        StoreConfig::new("target/test-signing-required-keyed")
            .with_signing_policy(SigningPolicy::Required)
            .with_signing_key(SigningKey::from_bytes([7u8; 32]))
            .validated()
            .is_ok(),
        "Required + a signing key must validate"
    );
}

#[test]
fn validated_accepts_documented_inclusive_upper_bounds() {
    let mut config = StoreConfig::new("target/test-config-upper-bounds");
    config.writer.pressure_retry_threshold_pct = 100;
    config.batch.max_size = 4096;

    config
        .validated()
        .expect("documented inclusive upper bounds should validate");
}

#[test]
fn validated_rejects_values_above_documented_upper_bounds() {
    let mut pressure = StoreConfig::new("target/test-config-pressure-too-high");
    pressure.writer.pressure_retry_threshold_pct = 101;
    assert!(
        matches!(
            pressure.validated(),
            Err(crate::store::StoreError::Configuration(_))
        ),
        "PROPERTY: pressure retry threshold above 100 must be rejected"
    );

    let mut batch = StoreConfig::new("target/test-config-batch-too-large");
    batch.batch.max_size = 4097;
    assert!(
        matches!(
            batch.validated(),
            Err(crate::store::StoreError::Configuration(_))
        ),
        "PROPERTY: batch.max_size above 4096 must be rejected"
    );

    let mut single_append = StoreConfig::new("target/test-config-single-append-too-large");
    single_append.single_append_max_bytes = 64 * 1024 * 1024 + 1;
    assert!(
        matches!(
            single_append.validated(),
            Err(crate::store::StoreError::Configuration(_))
        ),
        "PROPERTY: single_append_max_bytes above 64MB must be rejected"
    );

    let mut batch_bytes = StoreConfig::new("target/test-config-batch-bytes-too-large");
    batch_bytes.batch.max_bytes = 16 * 1024 * 1024 + 1;
    assert!(
        matches!(
            batch_bytes.validated(),
            Err(crate::store::StoreError::Configuration(_))
        ),
        "PROPERTY: batch.max_bytes above 16MB must be rejected"
    );
}

#[test]
fn validated_rejects_zero_payload_size_boundaries() {
    let mut single_append = StoreConfig::new("target/test-config-single-append-zero");
    single_append.single_append_max_bytes = 0;
    assert!(
        matches!(
            single_append.validated(),
            Err(crate::store::StoreError::Configuration(_))
        ),
        "PROPERTY: single_append_max_bytes of zero must be rejected"
    );

    let mut batch_bytes = StoreConfig::new("target/test-config-batch-bytes-zero");
    batch_bytes.batch.max_bytes = 0;
    assert!(
        matches!(
            batch_bytes.validated(),
            Err(crate::store::StoreError::Configuration(_))
        ),
        "PROPERTY: batch.max_bytes of zero must be rejected"
    );
}

#[test]
fn validated_config_debug_names_runtime_policy_fields() {
    let runtime = StoreConfig::new("target/test-validated-debug")
        .validated()
        .expect("config validates");
    let rendered = format!("{runtime:?}");

    assert!(
        rendered.contains("ValidatedStoreConfig")
            && rendered.contains("pressure_retry_threshold")
            && rendered.contains("group_commit_drain_budget")
            && rendered.contains("signing_registry"),
        "PROPERTY: ValidatedStoreConfig Debug must name the runtime policy fields, got: {rendered}"
    );
}

#[test]
fn process_boot_ns_is_nonzero_and_stable_in_process() {
    let clock = SystemClock::new();
    let first = clock.process_boot_ns();
    let second = clock.process_boot_ns();

    assert_ne!(
        first, 0,
        "PROPERTY: process_boot_ns must expose the captured wall-clock anchor, not zero/default"
    );
    assert_eq!(
        first, second,
        "PROPERTY: process_boot_ns must stay stable for the process lifetime"
    );
}

#[test]
fn chain_verification_accessor_returns_the_configured_non_default_policy() {
    // PROPERTY: the accessor must return the CONFIGURED policy — a
    // `Default::default()` body would read every configured `Recompute` back
    // as `Crc` and silently skip the at-open tamper check. Kills
    // `StoreConfig::chain_verification -> Default::default()`.
    let configured = StoreConfig::new("target/test-chain-verification-accessor")
        .with_chain_verification(ChainVerification::Recompute);
    assert_eq!(
        configured.chain_verification(),
        ChainVerification::Recompute,
        "PROPERTY: a configured non-default ChainVerification must be returned as-is"
    );
    assert_eq!(
        StoreConfig::new("target/test-chain-verification-default").chain_verification(),
        ChainVerification::Crc,
        "an unconfigured store trusts the per-frame CRC (the default)"
    );
}

#[test]
fn validated_process_boot_ns_delegates_to_the_runtime_clock() {
    // PROPERTY: the accessor must report the wrapped runtime clock's process
    // epoch marker, not a constant — projection/cache freshness compares this
    // marker across restarts, so a constant would alias every process. Kills
    // `ValidatedStoreConfig::process_boot_ns -> 1`.
    let runtime = StoreConfig::new("target/test-boot-ns-delegation")
        .validated()
        .expect("config validates");
    assert_eq!(
        runtime.process_boot_ns(),
        runtime.clock().process_boot_ns(),
        "PROPERTY: process_boot_ns must delegate to the runtime clock's epoch marker"
    );
    assert_ne!(
        runtime.process_boot_ns(),
        1,
        "PROPERTY: the process epoch marker is a captured wall-clock anchor, never a 1 sentinel"
    );
}

#[test]
fn now_mono_ns_advances_beyond_nonzero_sentinel() {
    let clock = SystemClock::new();
    std::thread::sleep(Duration::from_millis(1));
    let elapsed = clock.now_mono_ns();

    assert!(
        elapsed > 1,
        "PROPERTY: now_mono_ns must report elapsed nanoseconds from the process anchor, not a fixed sentinel; got {elapsed}"
    );
}

#[test]
fn duration_micros_preserves_zero_and_one_microsecond_boundaries() {
    assert_eq!(
        duration_micros(Duration::ZERO),
        0,
        "PROPERTY: zero duration must remain zero, not a default/nonzero sentinel"
    );
    assert_eq!(
        duration_micros(Duration::from_micros(1)),
        1,
        "PROPERTY: one microsecond must round-trip exactly"
    );
}

#[test]
fn has_custom_clock_reflects_clock_presence() {
    // Pins `has_custom_clock`: hardcoding it to `true` would claim a fresh
    // config carries an injected clock, breaking callers that branch on it.
    let mut config = StoreConfig::new("target/test-has-custom-clock");
    assert!(
        !config.has_custom_clock(),
        "a fresh config must report no custom clock"
    );

    let raw = Arc::new(|| 1_000i64) as Arc<dyn Fn() -> i64 + Send + Sync>;
    config.clock = Some(clock_from_fn(raw));
    assert!(
        config.has_custom_clock(),
        "a config with an injected clock must report a custom clock"
    );
}

#[test]
fn with_spawner_installs_custom_spawner_and_runs_body() {
    use crate::store::platform::spawn::{JobHandle, Spawn};
    use std::sync::atomic::AtomicBool;

    // A recording spawner proving that `with_spawner` rewires the seam: it
    // sets a flag when asked to spawn, then delegates the body to a real
    // ThreadSpawn so the join contract still holds end-to-end.
    struct RecordingSpawn {
        spawned: Arc<AtomicBool>,
        inner: crate::store::platform::spawn::ThreadSpawn,
    }
    impl Spawn for RecordingSpawn {
        fn spawn(
            &self,
            name: String,
            stack_size: Option<usize>,
            body: Box<dyn FnOnce() + Send + 'static>,
        ) -> Result<Box<dyn JobHandle>, crate::store::platform::spawn::SpawnError> {
            self.spawned.store(true, Ordering::Release);
            self.inner.spawn(name, stack_size, body)
        }
    }

    let spawned = Arc::new(AtomicBool::new(false));
    let spawner: Arc<dyn Spawn> = Arc::new(RecordingSpawn {
        spawned: Arc::clone(&spawned),
        inner: crate::store::platform::spawn::ThreadSpawn,
    });

    let config = StoreConfig::new("target/test-with-spawner").with_spawner(spawner);

    let ran = Arc::new(AtomicBool::new(false));
    let ran_for_body = Arc::clone(&ran);
    let handle = config
        .spawner()
        .spawn(
            "with-spawner-config-proof".to_string(),
            None,
            Box::new(move || ran_for_body.store(true, Ordering::Release)),
        )
        .expect("custom spawner must spawn");
    handle.join().expect("body must join Ok");

    assert!(
        spawned.load(Ordering::Acquire),
        "PROPERTY: with_spawner must route config.spawner() through the installed Spawn"
    );
    assert!(
        ran.load(Ordering::Acquire),
        "PROPERTY: the body handed to the custom spawner must run to completion"
    );
}

#[test]
fn with_fs_installs_custom_filesystem_backend() {
    use crate::store::platform::fs::{RealFs, StoreFs};
    use std::path::Path;
    use std::sync::atomic::AtomicBool;

    // A recording StoreFs proving that `with_fs` rewires the seam: it flags
    // when asked to create_dir_all, then delegates to RealFs so the production
    // op still happens (behavior-preserving delegation through the trait).
    struct RecordingFs {
        created: Arc<AtomicBool>,
        inner: RealFs,
    }
    impl StoreFs for RecordingFs {
        fn read_dir(
            &self,
            path: &Path,
        ) -> std::io::Result<Vec<crate::store::platform::fs::DirEntryInfo>> {
            self.inner.read_dir(path)
        }
        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            self.created.store(true, Ordering::Release);
            self.inner.create_dir_all(path)
        }
        fn create_new_file(
            &self,
            path: &Path,
        ) -> Result<Box<dyn crate::store::platform::fs::StoreFile>, crate::store::StoreError>
        {
            self.inner.create_new_file(path)
        }
        fn open_file(
            &self,
            path: &Path,
        ) -> std::io::Result<Box<dyn crate::store::platform::fs::StoreFile>> {
            self.inner.open_file(path)
        }
        fn sync_parent_dir(&self, path: &Path) -> Result<(), crate::store::StoreError> {
            self.inner.sync_parent_dir(path)
        }
        fn reject_symlink_leaf(
            &self,
            path: &Path,
            purpose: &str,
        ) -> Result<(), crate::store::StoreError> {
            self.inner.reject_symlink_leaf(path, purpose)
        }
        fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
            self.inner.read(path)
        }

        fn canonicalize(&self, path: &Path) -> std::io::Result<std::path::PathBuf> {
            self.inner.canonicalize(path)
        }
        fn symlink_metadata(
            &self,
            path: &Path,
        ) -> std::io::Result<crate::store::platform::fs::FileStat> {
            self.inner.symlink_metadata(path)
        }
        fn cow_copy_file(
            &self,
            from: &Path,
            to: &Path,
            preference: crate::store::CopyPreference,
        ) -> std::io::Result<crate::store::platform::fs::CowStrategyUsed> {
            self.inner.cow_copy_file(from, to, preference)
        }
        fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
            self.inner.copy(from, to)
        }
        fn metadata(&self, path: &Path) -> std::io::Result<crate::store::platform::fs::FileStat> {
            self.inner.metadata(path)
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            self.inner.rename(from, to)
        }
        fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            self.inner.remove_file(path)
        }
        fn named_temp_in(
            &self,
            dir: &Path,
        ) -> std::io::Result<Box<dyn crate::store::platform::fs::StagedFile>> {
            self.inner.named_temp_in(dir)
        }
        fn try_lock_store_dir(
            &self,
            lock_path: &Path,
        ) -> Result<
            Option<Box<dyn crate::store::platform::fs::StoreDirLockGuard>>,
            crate::store::StoreError,
        > {
            self.inner.try_lock_store_dir(lock_path)
        }
    }

    let created = Arc::new(AtomicBool::new(false));
    let fs: Arc<dyn StoreFs> = Arc::new(RecordingFs {
        created: Arc::clone(&created),
        inner: RealFs,
    });

    let config = StoreConfig::new("target/test-with-fs").with_fs(fs);

    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("seam").join("leaf");
    config
        .fs()
        .create_dir_all(&nested)
        .expect("custom fs must create the tree");

    assert!(
        created.load(Ordering::Acquire),
        "PROPERTY: with_fs must route config.fs() through the installed StoreFs"
    );
    assert!(
        nested.is_dir(),
        "PROPERTY: the installed StoreFs must still perform the real create_dir_all"
    );
}

#[test]
fn batch_accessor_returns_the_configured_batch_config() {
    // Kills `batch -> Box::leak(Box::new(Default::default()))`: the accessor must
    // reflect a configured non-default max_size (999), not the default 256.
    let configured = StoreConfig::new("target/test-batch-accessor").with_batch_max_size(999);
    assert_eq!(
        configured.batch().max_size,
        999,
        "PROPERTY: batch() must return the configured BatchConfig, not a leaked default (256)"
    );
}

#[test]
fn index_accessor_returns_the_configured_index_config() {
    // Kills `index -> Box::leak(Box::new(Default::default()))`: the default
    // incremental_projection is false; configuring true must read back true.
    let configured =
        StoreConfig::new("target/test-index-accessor").with_incremental_projection(true);
    assert!(
        configured.index().incremental_projection,
        "PROPERTY: index() must return the configured IndexConfig, not a leaked default (false)"
    );
}

#[test]
fn event_payload_validation_accessor_returns_the_configured_policy() {
    use crate::event::EventPayloadValidation;
    // Kills `event_payload_validation -> Default::default()` (FailFast): a
    // configured non-default Silent policy must be returned as-is.
    let configured = StoreConfig::new("target/test-epv-accessor")
        .with_event_payload_validation(EventPayloadValidation::Silent);
    assert_eq!(
        configured.event_payload_validation(),
        EventPayloadValidation::Silent,
        "PROPERTY: the accessor must return the configured policy, not the default (FailFast)"
    );
    assert_eq!(
        StoreConfig::new("target/test-epv-default").event_payload_validation(),
        EventPayloadValidation::FailFast,
        "the default collision policy is FailFast"
    );
}

#[test]
fn now_wall_ns_reports_the_wrapped_clock_wall_nanoseconds_not_a_sentinel() {
    // Inject a fixed microsecond clock; FnClock derives wall-ns as us * 1000, and
    // the monotonic wrapper passes the first reading through unchanged. Kills
    // `now_wall_ns -> -1 / 0 / 1`.
    let raw = Arc::new(|| 1_234_567_i64) as Arc<dyn Fn() -> i64 + Send + Sync>;
    let mut config = StoreConfig::new("target/test-now-wall-ns");
    config.clock = Some(clock_from_fn(raw));
    let runtime = config.validated().expect("config validates");
    assert_eq!(
        runtime.now_wall_ns(),
        1_234_567_000,
        "PROPERTY: now_wall_ns must report the wrapped clock's wall nanoseconds, never a -1/0/1 sentinel"
    );
}

#[test]
fn validated_accepts_exact_inclusive_byte_ceilings() {
    // single_append_max_bytes at EXACTLY 64MiB is valid (the `>` boundary); the
    // `>= 64MiB` mutant would reject the inclusive ceiling.
    let mut single = StoreConfig::new("target/test-single-append-ceiling");
    single.single_append_max_bytes = 64 * 1024 * 1024;
    single
        .validated()
        .expect("64MiB single_append_max_bytes is the inclusive ceiling");

    // batch.max_bytes at EXACTLY 16MiB is valid (the `>` boundary); the
    // `>= 16MiB` mutant would reject the inclusive ceiling.
    let mut batch = StoreConfig::new("target/test-batch-bytes-ceiling");
    batch.batch.max_bytes = 16 * 1024 * 1024;
    batch
        .validated()
        .expect("16MiB batch.max_bytes is the inclusive ceiling");
}

#[cfg(feature = "payload-encryption")]
#[test]
fn payload_encryption_getter_returns_the_configured_non_default_granularity() {
    use crate::store::keyscope::KeyScopeGranularity;
    // A fresh config reports None — encryption is opt-in.
    assert_eq!(
        StoreConfig::new("target/test-payload-enc-none").payload_encryption(),
        None,
        "a default config must report no payload encryption"
    );
    // A configured NON-DEFAULT granularity round-trips exactly. Kills
    // `payload_encryption -> None` and `-> Some(Default::default())` (PerEntity).
    assert_ne!(
        KeyScopeGranularity::PerEvent,
        KeyScopeGranularity::default(),
        "premise: PerEvent must differ from the default so Some(Default) is distinguishable"
    );
    let configured = StoreConfig::new("target/test-payload-enc-some")
        .with_payload_encryption(KeyScopeGranularity::PerEvent);
    assert_eq!(
        configured.payload_encryption(),
        Some(KeyScopeGranularity::PerEvent),
        "PROPERTY: the configured PerEvent granularity must be returned as-is, never None or the default"
    );
}

#[cfg(feature = "dangerous-test-hooks")]
#[test]
fn writer_mode_accessor_returns_the_configured_non_default_mode() {
    // Kills `writer_mode -> Default::default()` (Threaded): a configured
    // Cooperative mode must be returned as-is.
    let configured =
        StoreConfig::new("target/test-writer-mode").with_writer_mode(WriterMode::Cooperative);
    assert_eq!(
        configured.writer_mode(),
        WriterMode::Cooperative,
        "PROPERTY: writer_mode() must return the configured mode, not the default (Threaded)"
    );
    assert_eq!(
        StoreConfig::new("target/test-writer-mode-default").writer_mode(),
        WriterMode::Threaded,
        "an unconfigured store drives the writer on a thread (the default)"
    );
}
