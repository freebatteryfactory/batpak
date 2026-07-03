use crate::store::write::writer::WriterCommand;
use crate::store::{Store, StoreError};

/// Test helper: trigger a panic in the writer thread to exercise restart_policy.
/// Returns Ok(()) if the panic command was sent and acknowledged by the writer.
/// After calling this, the writer will panic and (if restart_policy allows) restart.
/// Wait briefly after calling to let the restart complete before sending more commands.
#[doc(hidden)]
impl Store {
    /// Test-only: trigger a panic in the writer thread to exercise restart_policy.
    pub fn panic_writer_for_test(&self) -> Result<(), StoreError> {
        let (tx, rx) = flume::bounded(1);
        self.state
            .0
            .tx
            .send(WriterCommand::PanicForTest { respond: tx })
            .map_err(|_| StoreError::WriterCrashed)?;
        let _ = rx.recv_timeout(std::time::Duration::from_millis(500));
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }
}

#[cfg(all(test, feature = "dangerous-test-hooks"))]
mod tests {
    use crate::coordinate::Coordinate;
    use crate::event::EventKind;
    use crate::store::{RestartPolicy, Store, StoreConfig, StoreError};

    /// `panic_writer_for_test` must ACTUALLY crash the writer thread — a
    /// `-> Ok(())` no-op body would leave the writer healthy. Under
    /// `RestartPolicy::Once`, a first panic is absorbed but a second exhausts
    /// the budget, so a subsequent append must surface `WriterCrashed`. If the
    /// helper never panicked, that append would keep succeeding forever.
    #[test]
    fn panic_writer_for_test_really_crashes_the_writer() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let config = StoreConfig::new(dir.path()).with_restart_policy(RestartPolicy::Once);
        let store = Store::open(config).expect("open store");
        let coord = Coordinate::new("entity:panic", "scope:panic").expect("coord");
        let kind = EventKind::custom(0xF, 0x1);

        store
            .panic_writer_for_test()
            .expect("send first panic command");
        // A second panic exhausts RestartPolicy::Once; the writer thread then dies.
        let _ = store.panic_writer_for_test();

        // Poll for the crash rather than sleeping a fixed amount — the writer
        // takes a nondeterministic moment to process the panic and exit.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let observed: Option<StoreError> = loop {
            match store.append(&coord, kind, &serde_json::json!({ "n": 1 })) {
                Err(e) => break Some(e),
                Ok(_) if std::time::Instant::now() >= deadline => break None,
                Ok(_) => std::thread::yield_now(),
            }
        };
        let err = observed.expect(
            "PROPERTY: after two panics under RestartPolicy::Once, append must fail — a no-op \
             panic_writer_for_test would leave the writer alive and this append would succeed",
        );
        assert!(
            matches!(err, StoreError::WriterCrashed),
            "PROPERTY: append after an exhausted restart budget must surface WriterCrashed, got {err:?}"
        );
    }
}
