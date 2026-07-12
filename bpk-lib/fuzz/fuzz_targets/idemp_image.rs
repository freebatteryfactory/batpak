#![no_main]
//! GAUNT-IDEMPOTENCY-AUTHORITY (#189) — `idemp_image` target.
//!
//! Drives `batpak::__fuzz::__fuzz_idemp_image(&[u8])`, a file-path decoder over
//! the durable idempotency authority image (`index.idemp`): header magic +
//! version + CRC + msgpack body (v1 bare entries, v2 lineage + compound anchor
//! + entries), followed by the ADMISSION checks (lineage match, coverage
//! frontier, history-anchor divergence) under both an unbound and a bound
//! expectation. A crafted image must be a typed `StoreError`
//! (`IdempotencyAuthorityCorrupt`/`Stale`/`Foreign`/`FutureVersion`), never a
//! panic — corruption is not absence, and it is not a crash either.
//!
//! S1 contract: (a) never panic; (b) no unbounded allocation — the run's
//! `-rss_limit_mb` backstops the msgpack body decode; (c) a valid image parses
//! `Ok`; (d) every damaged/foreign/stale shape returns its typed refusal.

use libfuzzer_sys::fuzz_target;

use batpak::__fuzz::__fuzz_idemp_image;

fuzz_target!(|data: &[u8]| {
    match __fuzz_idemp_image(data) {
        Ok(entry_count) => {
            // A decoded image cannot honestly hold more entries than bytes.
            assert!(
                entry_count <= data.len(),
                "admitted entry count must never exceed the file length (no amplification)"
            );
        }
        Err(err) => {
            let _ = format!("{err:?}");
        }
    }
});
