//! PROVES: durable operation-status writes go through the store, not a no-op.
//! CATCHES: the diff-scoped surviving mutant on
//! `StoreOperationStatusSink::record_started -> Ok(())`, which would silently
//! drop the status append while still reporting success.
//! SEEDED: a tempfile-backed batpak store and a fixed operation name.
//!
//! ROUND 2 (WP-D) —
//! PROVES: `BuildError`'s `Display` renders every variant's exact
//! distinguishing message, interpolating the offending name (and validation
//! message where present).
//! CATCHES: error.rs:121 `<impl Display for BuildError>::fmt ->
//! Ok(Default::default())`, which would render every build failure as the
//! empty string.
//! SEEDED: fixed operation/handler/module names and validation messages.

use std::sync::Arc;

use batpak::prelude::*;
use batpak::store::{Store, StoreConfig};
use syncbat::{operation_status_entity, BuildError, StoreOperationStatusSink};

fn test_store() -> (Arc<Store>, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let store = Store::open(
        StoreConfig::new(dir.path())
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )
    .expect("open store");
    (Arc::new(store), dir)
}

#[test]
fn record_started_appends_exactly_one_started_fact() {
    let (store, _dir) = test_store();
    let sink = StoreOperationStatusSink::new(Arc::clone(&store));

    sink.record_started("ping", "receipt.ping.v1")
        .expect("record_started should append");

    let entity = operation_status_entity("ping").expect("status entity");
    let hits = store.query(&Region::entity(entity.as_str()));
    assert_eq!(
        hits.len(),
        1,
        "record_started must append one fact through the store"
    );
}

/// error.rs:121: `Display for BuildError` must render each variant's exact
/// message — the `Ok(Default::default())` mutant writes nothing at all.
#[test]
fn build_error_display_renders_exact_variant_messages() {
    assert_eq!(
        BuildError::duplicate_operation("mod.a.echo").to_string(),
        "operation `mod.a.echo` is already registered"
    );
    assert_eq!(
        BuildError::duplicate_handler("mod.a.echo").to_string(),
        "handler for operation `mod.a.echo` is already registered"
    );
    assert_eq!(
        BuildError::missing_descriptor("mod.a.echo").to_string(),
        "handler `mod.a.echo` has no matching operation descriptor"
    );
    assert_eq!(
        BuildError::missing_handler("mod.a.echo").to_string(),
        "operation `mod.a.echo` has no registered handler"
    );
    assert_eq!(
        BuildError::invalid_module("mod.a", "name has a trailing dot").to_string(),
        "module `mod.a` is invalid: name has a trailing dot"
    );
    assert_eq!(
        BuildError::invalid_operation("mod.a.echo", "name has a trailing dot").to_string(),
        "operation `mod.a.echo` is invalid: name has a trailing dot"
    );
    assert_eq!(
        BuildError::invalid_handler("mod.a.echo", "name has a trailing dot").to_string(),
        "handler `mod.a.echo` is invalid: name has a trailing dot"
    );
    assert_eq!(
        BuildError::MissingReceiptSink.to_string(),
        "core has no receipt sink: a sinkless core silently drops every runtime receipt. Wire \
         one with CoreBuilder::receipt_sink, or opt out explicitly with \
         CoreBuilder::without_receipts"
    );
}
