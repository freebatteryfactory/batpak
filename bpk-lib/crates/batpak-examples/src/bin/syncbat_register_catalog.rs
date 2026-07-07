//! # syncbat_register_catalog
//!
//! **Witnesses:** the durable syncbat operation-register catalog survives a
//! restart. Operation descriptors are persisted as durable batpak rows through
//! [`syncbat::StoreRegisterCatalog`]; after the process "restarts" (the store is
//! closed and reopened from the same directory) the register is REBUILT from
//! those rows via [`syncbat::rebuild_register_from_store`] — no in-memory state
//! carries over. The rebuilt register is asserted byte-for-byte equal to the
//! original catalog.
//!
//! This is the register-catalog analogue of event sourcing: the store is the
//! source of truth for "which operations exist", and the register is a
//! projection that can always be reconstructed.
//!
//! Run: `cargo run -p batpak-examples --bin syncbat_register_catalog`

use std::io::Write;
use std::sync::Arc;

use batpak::coordinate::Coordinate;
use batpak::store::{Store, StoreConfig};
use syncbat::{
    rebuild_register_from_store, EffectClass, OperationDescriptor, Register, StoreRegisterCatalog,
};

/// The operation descriptors the catalog persists. Mechanisms, not meanings:
/// three synthetic operation names spanning the effect classes.
fn seed_operations() -> [OperationDescriptor; 3] {
    [
        OperationDescriptor::new(
            "catalog.read.v1",
            EffectClass::Inspect,
            "catalog.read.input.v1",
            "catalog.read.output.v1",
            "receipt.catalog.read.v1",
        ),
        OperationDescriptor::new(
            "catalog.write.v1",
            EffectClass::Compute,
            "catalog.write.input.v1",
            "catalog.write.output.v1",
            "receipt.catalog.write.v1",
        ),
        OperationDescriptor::new(
            "catalog.scan.v1",
            EffectClass::Inspect,
            "catalog.scan.input.v1",
            "catalog.scan.output.v1",
            "receipt.catalog.scan.v1",
        ),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();

    let dir = tempfile::tempdir()?;
    // `sync_every_n_events(1)` makes each catalog row durable the instant it is
    // appended, so a reopened store sees every row even without a graceful close.
    let config = || StoreConfig::new(dir.path()).with_sync_every_n_events(1);

    // The register coordinate: one durable stream that carries the catalog rows.
    let coordinate = Coordinate::new("register:operations", "scope:syncbat")?;

    // The in-memory register we intend to persist. This is our expected value.
    let operations = seed_operations();
    let expected = Register::from_operations(operations.iter().cloned())?;
    let _ = writeln!(
        out,
        "built a register with {} operation(s): {}",
        expected.len(),
        expected.names().collect::<Vec<_>>().join(", "),
    );

    // -- Persist the register to durable batpak rows through the catalog writer --
    let store = Arc::new(Store::open(config())?);
    let catalog = StoreRegisterCatalog::new(Arc::clone(&store), coordinate.clone());
    let receipts = catalog.persist_register(&expected)?;
    let _ = writeln!(
        out,
        "persisted {} catalog row(s) (last at global sequence {})",
        receipts.len(),
        receipts
            .last()
            .map(|r| r.global_sequence)
            .unwrap_or_default(),
    );

    // -- "Restart": drop the catalog + store handle, then gracefully close --
    drop(catalog);
    let store =
        Arc::try_unwrap(store).map_err(|_| "register catalog still holds a live store handle")?;
    store.close()?;
    let _ = writeln!(out, "closed the store (simulated process restart)");

    // -- Reopen from the same directory with a FRESH store; nothing carried over --
    let reopened = Store::open(config())?;
    let rebuilt = rebuild_register_from_store(&reopened, &coordinate)?;
    let _ = writeln!(
        out,
        "rebuilt {} operation(s) from durable rows: {}",
        rebuilt.len(),
        rebuilt.names().collect::<Vec<_>>().join(", "),
    );

    // -- Assert the rebuilt register matches the original catalog exactly --
    assert_eq!(
        rebuilt.as_map(),
        expected.as_map(),
        "register rebuilt from the store must equal the persisted catalog",
    );
    // Spot-check one descriptor round-tripped with its full schema wiring intact.
    let read = rebuilt
        .descriptor("catalog.read.v1")
        .ok_or("rebuilt register is missing catalog.read.v1")?;
    assert_eq!(read.input_schema_ref(), "catalog.read.input.v1");
    assert_eq!(read.output_schema_ref(), "catalog.read.output.v1");
    assert_eq!(read.receipt_kind(), "receipt.catalog.read.v1");
    let _ = writeln!(
        out,
        "OK: register REBUILDS from the store and matches the original",
    );

    reopened.close()?;
    Ok(())
}
