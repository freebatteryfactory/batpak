//! Store-to-store event import by re-application.

use crate::coordinate::Region;
use crate::id::{EntityIdType, IdempotencyKey};
use crate::store::index::IndexEntry;
use crate::store::{
    AppendOptions, AppendReceipt, BatchAppendItem, CausationRef, EncodedBytes, ExtensionKey, Open,
    Store, StoreError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Predicate used to skip source events during import.
pub type ImportFilter = Box<dyn Fn(&IndexEntry) -> bool + Send + Sync>;

/// Caller-owned identity for an import source log.
///
/// A non-empty opaque label that, together with the source event id, forms the
/// deterministic import idempotency key. The serde form is transparent: the
/// wire representation is identical to the inner string.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceNamespace(String);

impl SourceNamespace {
    /// Construct a source namespace, validating that it is non-empty.
    ///
    /// # Errors
    /// Returns [`StoreError::Configuration`] if the namespace is empty.
    pub fn new(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        if value.is_empty() {
            return Err(StoreError::Configuration(
                "import source_namespace must be non-empty".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Borrow the namespace as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SourceNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Source event selector for [`Store::import_events`].
#[derive(Clone, Debug)]
#[must_use]
pub struct ImportSelector {
    region: Region,
    after_global_sequence: Option<u64>,
}

impl ImportSelector {
    /// Select every visible source event.
    pub fn all() -> Self {
        Self {
            region: Region::all(),
            after_global_sequence: None,
        }
    }

    /// Select visible source events matching `region`.
    pub fn region(region: Region) -> Self {
        Self {
            region,
            after_global_sequence: None,
        }
    }

    /// Select visible source events strictly after `after_global_sequence`.
    pub fn after(after_global_sequence: u64) -> Self {
        Self {
            region: Region::all(),
            after_global_sequence: Some(after_global_sequence),
        }
    }

    /// Add or replace the exclusive source global-sequence resume point.
    pub fn with_after_global_sequence(mut self, after_global_sequence: u64) -> Self {
        self.after_global_sequence = Some(after_global_sequence);
        self
    }

    /// Borrow the region predicate used by this selector.
    pub fn region_ref(&self) -> &Region {
        &self.region
    }

    /// Return the exclusive source global-sequence resume point, if configured.
    pub fn after_global_sequence(&self) -> Option<u64> {
        self.after_global_sequence
    }
}

impl Default for ImportSelector {
    fn default() -> Self {
        Self::all()
    }
}

/// Options controlling [`Store::import_events`].
#[must_use]
pub struct ImportOptions {
    source_namespace: SourceNamespace,
    chunk_size: usize,
    filter: Option<ImportFilter>,
}

impl ImportOptions {
    /// Construct import options with the required caller-owned source namespace.
    ///
    /// # Errors
    /// Returns [`StoreError::Configuration`] if the namespace is empty.
    pub fn new(source_namespace: impl Into<String>) -> Result<Self, StoreError> {
        Ok(Self {
            source_namespace: SourceNamespace::new(source_namespace)?,
            chunk_size: 256,
            filter: None,
        })
    }

    /// Derive a namespace from a source data-directory path.
    ///
    /// This is an explicit opt-in convenience for local tooling. Durable import
    /// identity is still caller policy: passing a stable opaque namespace is
    /// preferred when the same logical source can move paths.
    ///
    /// # Errors
    /// Returns [`StoreError::Configuration`] if the path cannot be
    /// canonicalized or encoded as a namespace.
    pub fn with_source_namespace_from_data_dir(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let canonical =
            crate::store::platform::fs::canonicalize(path.as_ref()).map_err(|error| {
                StoreError::Configuration(format!(
                    "source namespace path {} could not be canonicalized: {error}",
                    path.as_ref().display()
                ))
            })?;
        let digest = crate::evidence::content_hash(canonical.as_os_str().as_encoded_bytes());
        Self::new(format!("data-dir:{}", hex_lower(&digest)))
    }

    /// Set the preferred chunk size. The import path clamps it to the
    /// destination store's configured batch maximum at execution time.
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size.max(1);
        self
    }

    /// Attach a caller predicate. Returning `false` skips the source entry.
    pub fn with_filter(mut self, filter: ImportFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Borrow the source namespace.
    pub fn source_namespace(&self) -> &SourceNamespace {
        &self.source_namespace
    }

    /// Return the preferred chunk size before destination clamping.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

/// Report returned by [`Store::import_events`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportReport {
    /// Events newly appended to the destination store.
    pub imported: u64,
    /// Events already present by deterministic import idempotency key.
    pub deduplicated: u64,
    /// Source events skipped because their kind is substrate-reserved.
    pub skipped_reserved: u64,
    /// Source events skipped by the caller filter.
    pub skipped_filtered: u64,
    /// Source events skipped because they were our own prior import output for
    /// this source namespace (chain re-import guard; see INV-IMPORT-NO-RUNAWAY).
    pub skipped_reimported: u64,
    /// Highest source global sequence observed by the selector.
    pub source_high_watermark: Option<u64>,
}

/// Schema version for the import provenance receipt extension.
pub const IMPORT_PROVENANCE_SCHEMA_VERSION: u16 = 1;

/// Signed receipt-extension body recording the source lineage of an import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImportProvenance {
    /// Extension schema version.
    pub schema_version: u16,
    /// Caller-supplied namespace for the source log.
    pub source_namespace: SourceNamespace,
    /// Source event id, as a raw `u128`.
    pub source_event_id: u128,
    /// Source global sequence.
    pub source_global_sequence: u64,
    /// Source event kind raw encoding.
    pub source_kind: u16,
    /// Source event content hash, covering the raw payload bytes.
    pub source_content_hash: [u8; 32],
}

impl ImportProvenance {
    fn encode_extension(&self) -> Result<EncodedBytes, StoreError> {
        crate::encoding::to_bytes(self).map_err(|error| StoreError::Serialization(Box::new(error)))
    }
}

/// Decode import provenance from an append receipt when present.
#[must_use]
pub fn provenance(receipt: &AppendReceipt) -> Option<ImportProvenance> {
    provenance_from_extensions(&receipt.extensions)
}

/// Decode import provenance from a receipt-extension map when present.
#[must_use]
pub fn provenance_from_extensions(
    extensions: &BTreeMap<ExtensionKey, EncodedBytes>,
) -> Option<ImportProvenance> {
    extensions
        .get(&import_provenance_extension_key())
        .and_then(|bytes| crate::encoding::from_bytes(bytes).ok())
}

/// Wire magic for compat-matrix and forward-compat gates (`import_provenance.fbip`).
pub(crate) const IMPORT_PROVENANCE_WIRE_MAGIC: &[u8; 6] = b"FBATIP";

/// Encode import provenance using the shared compat wire framing:
/// `magic(6) | version(2 le) | crc32(4 le over body) | body(to_vec_named)`.
///
/// # Errors
/// MessagePack encoding failure from `rmp-serde`.
pub fn encode_import_provenance_wire(provenance: &ImportProvenance) -> Result<Vec<u8>, StoreError> {
    let body_bytes = crate::encoding::to_bytes(provenance)
        .map_err(|error| StoreError::Serialization(Box::new(error)))?;
    let crc = crc32fast::hash(&body_bytes);
    let mut bytes = Vec::with_capacity(12 + body_bytes.len());
    bytes.extend_from_slice(IMPORT_PROVENANCE_WIRE_MAGIC);
    bytes.extend_from_slice(&provenance.schema_version.to_le_bytes());
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes.extend_from_slice(&body_bytes);
    Ok(bytes)
}

/// Decode import provenance from compat wire framing.
///
/// # Errors
/// Returns [`StoreError::ImportProvenanceFutureVersion`] when the wire or body schema
/// version exceeds [`IMPORT_PROVENANCE_SCHEMA_VERSION`], or a configuration /
/// serialization error for corrupt framing.
pub fn decode_import_provenance_wire(bytes: &[u8]) -> Result<ImportProvenance, StoreError> {
    if bytes.len() < 12 || bytes.get(..6) != Some(IMPORT_PROVENANCE_WIRE_MAGIC) {
        return Err(StoreError::Configuration(
            "import provenance wire framing is invalid".into(),
        ));
    }
    let found = u16::from_le_bytes(
        bytes[6..8]
            .try_into()
            .map_err(|_| StoreError::Configuration("import provenance version slice".into()))?,
    );
    if found > IMPORT_PROVENANCE_SCHEMA_VERSION {
        return Err(StoreError::ImportProvenanceFutureVersion {
            found,
            supported: IMPORT_PROVENANCE_SCHEMA_VERSION,
        });
    }
    let expected_crc = u32::from_le_bytes(
        bytes[8..12]
            .try_into()
            .map_err(|_| StoreError::Configuration("import provenance crc slice".into()))?,
    );
    let body_bytes = &bytes[12..];
    if crc32fast::hash(body_bytes) != expected_crc {
        return Err(StoreError::Configuration(
            "import provenance wire crc mismatch".into(),
        ));
    }
    let provenance: ImportProvenance = crate::encoding::from_bytes(body_bytes)
        .map_err(|error| StoreError::Serialization(Box::new(error)))?;
    if provenance.schema_version > IMPORT_PROVENANCE_SCHEMA_VERSION {
        return Err(StoreError::ImportProvenanceFutureVersion {
            found: provenance.schema_version,
            supported: IMPORT_PROVENANCE_SCHEMA_VERSION,
        });
    }
    Ok(provenance)
}

pub(crate) fn import_provenance_extension_key() -> ExtensionKey {
    ExtensionKey::reserved("batpak.import.provenance")
}

pub(crate) fn import_events<S: crate::store::StoreState>(
    destination: &Store<Open>,
    source: &Store<S>,
    selector: &ImportSelector,
    options: &ImportOptions,
) -> Result<ImportReport, StoreError> {
    let destination_batch_max = usize::try_from(destination.config.batch.max_size)
        .unwrap_or(usize::MAX)
        .max(1);
    let chunk_size = options.chunk_size.max(1).min(destination_batch_max).max(1);
    let mut after = selector.after_global_sequence;
    let pre_import_frontier = destination.frontier().visible_hlc.global_sequence;
    // Bound the import to the source frontier captured at call time. Without this,
    // a same-store import (source == destination) would keep paginating into the
    // events it just appended — they carry higher global sequences and fresh import
    // keys, so they would re-import endlessly until a disk/idempotency limit.
    let import_ceiling = source.frontier().visible_hlc.global_sequence;
    let mut report = ImportReport::default();

    loop {
        let page = source.query_entries_after(&selector.region, after, chunk_size);
        if page.is_empty() {
            break;
        }
        after = page.last().map(IndexEntry::global_sequence);

        let mut new_items = Vec::new();
        let mut reached_ceiling = false;
        for entry in page {
            if entry.global_sequence() > import_ceiling {
                // Past the call-time source frontier: stop before re-importing
                // events appended by this import itself (same-store guard).
                reached_ceiling = true;
                break;
            }
            report.source_high_watermark = Some(
                report
                    .source_high_watermark
                    .unwrap_or(0)
                    .max(entry.global_sequence()),
            );
            if entry.event_kind().is_reserved() {
                report.skipped_reserved = report.skipped_reserved.saturating_add(1);
                continue;
            }
            if let Some(filter) = options.filter.as_ref() {
                if !filter(&entry) {
                    report.skipped_filtered = report.skipped_filtered.saturating_add(1);
                    continue;
                }
            }

            // Self-chain guard (INV-IMPORT-NO-RUNAWAY): a source event that already
            // carries import provenance for THIS source_namespace is our own prior
            // import output for the same logical stream. It aliases an original
            // source identity that is also part of this selection (and is itself
            // deduplicated below), so re-importing the copy would amplify the stream
            // on every clean repeat pass. Skip the copy without counting it: the
            // original it aliases carries the dedup credit. This is what makes a
            // completed same-store import idempotent against itself.
            if source_event_is_self_chained(source, &entry, &options.source_namespace)? {
                report.skipped_reimported = report.skipped_reimported.saturating_add(1);
                continue;
            }

            let key = import_key(&options.source_namespace, entry.event_id().as_u128());
            if import_key_already_present(destination, key) {
                report.deduplicated = report.deduplicated.saturating_add(1);
                continue;
            }

            let raw = source.read_raw(entry.event_id())?;
            let provenance = ImportProvenance {
                schema_version: IMPORT_PROVENANCE_SCHEMA_VERSION,
                source_namespace: options.source_namespace.clone(),
                source_event_id: entry.event_id().as_u128(),
                source_global_sequence: entry.global_sequence(),
                source_kind: entry.event_kind().as_raw_u16(),
                source_content_hash: raw.event.header.content_hash,
            };
            let append_options = AppendOptions::new()
                .with_idempotency(key)
                .with_correlation(raw.event.header.correlation_id)
                .with_extension(
                    import_provenance_extension_key(),
                    provenance.encode_extension()?,
                );
            new_items.push(BatchAppendItem::from_msgpack_bytes(
                raw.coordinate,
                raw.event.header.event_kind,
                raw.event.payload,
                append_options,
                CausationRef::None,
            ));
        }

        if !new_items.is_empty() {
            let receipts = destination.append_batch(new_items)?;
            for receipt in receipts {
                if receipt.global_sequence < pre_import_frontier {
                    report.deduplicated = report.deduplicated.saturating_add(1);
                } else {
                    report.imported = report.imported.saturating_add(1);
                }
            }
        }

        if reached_ceiling {
            break;
        }
    }

    Ok(report)
}

fn import_key(source_namespace: &SourceNamespace, source_event_id: u128) -> IdempotencyKey {
    let source_event_id = format!("{source_event_id:032x}");
    IdempotencyKey::for_operation(
        "batpak.import",
        &[source_namespace.as_str(), &source_event_id],
    )
}

fn import_key_already_present(destination: &Store<Open>, key: IdempotencyKey) -> bool {
    destination.index.idemp.get(key.as_u128()).is_some()
        || destination.index.get_by_id(key.as_u128()).is_some()
}

/// True when `entry` already carries import provenance recorded under
/// `source_namespace` — i.e. it is a copy this import chain previously produced
/// for the same logical source. Such copies alias an original source identity
/// and must not be re-imported (INV-IMPORT-NO-RUNAWAY).
fn source_event_is_self_chained<S: crate::store::StoreState>(
    source: &Store<S>,
    entry: &IndexEntry,
    source_namespace: &SourceNamespace,
) -> Result<bool, StoreError> {
    let extensions = source.reader.read_receipt_extensions(&entry.disk_pos())?;
    Ok(provenance_from_extensions(&extensions)
        .is_some_and(|provenance| provenance.source_namespace == *source_namespace))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinate::Coordinate;
    use crate::event::EventKind;
    use crate::store::{AppendOptions, StoreConfig};

    #[test]
    fn hex_lower_is_exact_lowercase() {
        // Exact value pins both nibbles of every byte: high nibble (0xA, 0xC),
        // low nibble (0xB, 0xD), and the all-zero byte. A constant-return
        // mutant (`String::new()` or `"xyzzy".into()`) cannot reproduce this.
        assert_eq!(hex_lower(&[0xAB, 0xCD, 0x01, 0x00]), "abcd0100");
    }

    /// Covers the public `provenance(&AppendReceipt)` wrapper (the fn at the top
    /// of this file) by capturing a REAL `AppendReceipt` whose extension envelope
    /// carries genuine import provenance, then decoding it through the wrapper.
    /// The `provenance -> None` mutant cannot satisfy the `Some(p)` field
    /// assertions. `provenance(&receipt)` is called directly so the wrapper —
    /// not `provenance_from_extensions` — is the covered seam.
    #[test]
    fn provenance_wrapper_decodes_real_import_receipt() {
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let source = Store::open(StoreConfig::new(source_dir.path())).expect("open source");
        let dest_dir = tempfile::tempdir().expect("dest tempdir");
        let dest = Store::open(StoreConfig::new(dest_dir.path())).expect("open dest");

        let coord = Coordinate::new("entity:prov:wrapper", "scope:import").expect("coord");
        let kind = EventKind::custom(0xF, 0x8A);
        drop(
            source
                .append(&coord, kind, &serde_json::json!({"n": 1}))
                .expect("source append"),
        );

        // Drive a real import so the source event is genuinely re-applied.
        let options = ImportOptions::new("source-prov-wrapper").expect("options");
        let report =
            import_events(&dest, &source, &ImportSelector::all(), &options).expect("import");
        assert_eq!(report.imported, 1, "exactly one source event must import");

        // Rebuild the SAME provenance the import wrote, from the real source
        // entry + raw bytes, and persist it on a real receipt via append. This
        // yields a genuine `AppendReceipt` carrying the import provenance
        // extension — the exact envelope shape an imported event receipt holds.
        let source_entry = source.by_entity("entity:prov:wrapper")[0].clone();
        let raw = source
            .read_raw(source_entry.event_id())
            .expect("read source raw");
        let key = import_key(
            options.source_namespace(),
            source_entry.event_id().as_u128(),
        );
        let provenance_body = ImportProvenance {
            schema_version: IMPORT_PROVENANCE_SCHEMA_VERSION,
            source_namespace: options.source_namespace().clone(),
            source_event_id: source_entry.event_id().as_u128(),
            source_global_sequence: source_entry.global_sequence(),
            source_kind: source_entry.event_kind().as_raw_u16(),
            source_content_hash: raw.event.header.content_hash,
        };
        let append_options = AppendOptions::new().with_idempotency(key).with_extension(
            import_provenance_extension_key(),
            provenance_body
                .encode_extension()
                .expect("encode provenance"),
        );
        let receipt = dest
            .append_with_options(
                &Coordinate::new("entity:prov:wrapper:receipt", "scope:import").expect("coord"),
                kind,
                &serde_json::json!({"n": 1}),
                append_options,
            )
            .expect("append with provenance extension");

        let decoded = provenance(&receipt).expect("wrapper must decode import provenance");
        assert_eq!(
            decoded.source_event_id,
            source_entry.event_id().as_u128(),
            "wrapper-decoded source_event_id must match the source event"
        );
        assert_eq!(
            decoded.source_namespace.as_str(),
            "source-prov-wrapper",
            "wrapper-decoded source_namespace must match the configured source namespace"
        );

        source.close().expect("close source");
        dest.close().expect("close dest");
    }

    #[test]
    fn append_level_replay_receipt_below_the_frontier_counts_as_deduplicated() {
        use crate::store::index::idemp::IdempEntry;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let kind = EventKind::custom(0xE, 0x31);
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let source = Store::open(StoreConfig::new(source_dir.path())).expect("open source");
        let coord = Coordinate::new("entity:import-replay", "scope:import").expect("coord");
        let receipt_one = source
            .append(&coord, kind, &serde_json::json!({"n": 1}))
            .expect("append source event 1");
        let receipt_two = source
            .append(&coord, kind, &serde_json::json!({"n": 2}))
            .expect("append source event 2");

        // Destination: one pre-existing event so the pre-import frontier sits
        // strictly ABOVE the fabricated replay sequence (0) — `<` vs `==` on
        // that comparison is exactly the mutant under test.
        let dest_dir = tempfile::tempdir().expect("dest tempdir");
        let dest = Store::open(StoreConfig::new(dest_dir.path())).expect("open dest");
        drop(
            dest.append(
                &Coordinate::new("entity:pre-existing", "scope:import").expect("coord"),
                kind,
                &serde_json::json!({"seed": true}),
            )
            .expect("append destination seed event"),
        );
        assert!(
            dest.frontier().visible_hlc.global_sequence >= 1,
            "scenario shape: the pre-import frontier must be strictly above sequence 0"
        );

        let options = ImportOptions::new("replay-ns")
            .expect("options")
            .with_chunk_size(3);
        let key_one = import_key(options.source_namespace(), receipt_one.event_id.as_u128());
        let key_two = import_key(options.source_namespace(), receipt_two.event_id.as_u128());

        // On the SECOND filter consultation (source event 2) — after event 1
        // passed its own key check but BEFORE the page's append_batch — plant
        // durable idempotency entries for BOTH keys, recorded at sequence 0.
        // The writer's preflight then replays event 1 from the durable store
        // with global_sequence 0 (strictly below the frontier) while event 2
        // is skipped by its own key check. This is the append-level replay
        // race the receipt-counting loop classifies.
        let fabricate = |key: u128| IdempEntry {
            key,
            event_id: key,
            global_sequence: 0,
            disk_pos_segment: 0,
            disk_pos_offset: 0,
            disk_pos_length: 1,
            content_hash: [0; 32],
            prev_hash: [0; 32],
            entity: "entity:import-replay".to_owned(),
            scope: "scope:import".to_owned(),
            kind,
            recorded_global_sequence: 0,
            event_evicted: false,
            receipt_extensions: BTreeMap::new(),
        };
        let index = Arc::clone(&dest.index);
        let calls = Arc::new(AtomicUsize::new(0));
        let filter_calls = Arc::clone(&calls);
        let entry_one = fabricate(key_one.as_u128());
        let entry_two = fabricate(key_two.as_u128());
        let options = options.with_filter(Box::new(move |_entry| {
            if filter_calls.fetch_add(1, Ordering::SeqCst) == 1 {
                index.idemp.record(entry_one.clone());
                index.idemp.record(entry_two.clone());
            }
            true
        }));

        let report =
            import_events(&dest, &source, &ImportSelector::all(), &options).expect("import");

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "scenario shape: the filter must see exactly the two source events"
        );
        assert_eq!(
            report.deduplicated, 2,
            "one dedup from event 2's key check plus one from event 1's append-level \
             replay receipt at global_sequence 0 (strictly below the frontier)"
        );
        assert_eq!(
            report.imported, 0,
            "a replay receipt strictly below the pre-import frontier is never counted as imported"
        );

        source.close().expect("close source");
        dest.close().expect("close dest");
    }
}

/// Cure island for the wire-framing decode guards, the durable-idempotency
/// dedup predicate, and the append-level replay frontier boundary. Kept as a
/// second `#[cfg(test)]` island so the existing one stays within the inline
/// test-island budget.
#[cfg(test)]
mod wire_and_dedup_cure_tests {
    use super::*;
    use crate::coordinate::Coordinate;
    use crate::event::EventKind;
    use crate::store::index::idemp::IdempEntry;
    use crate::store::{Store, StoreConfig};

    #[test]
    fn decode_wire_at_exactly_twelve_bytes_reaches_the_body_decoder() {
        // A 12-byte frame (magic|version|crc-of-empty|EMPTY body) is exactly the
        // `< 12` boundary: the strict `<` must let it PAST the framing guard so
        // the empty body then fails MessagePack decode (Serialization). The
        // `<= 12` mutant rejects it earlier as invalid framing (Configuration).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(IMPORT_PROVENANCE_WIRE_MAGIC);
        bytes.extend_from_slice(&IMPORT_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
        bytes.extend_from_slice(&crc32fast::hash(&[]).to_le_bytes());
        assert_eq!(
            bytes.len(),
            12,
            "premise: the crafted frame is exactly 12 bytes"
        );
        let err = decode_import_provenance_wire(&bytes).expect_err("empty body must not decode");
        assert!(
            matches!(err, StoreError::Serialization(_)),
            "a 12-byte frame must pass framing and fail at body decode (kills `< -> <=`), got {err:?}"
        );
    }

    #[test]
    fn decode_wire_below_twelve_bytes_with_valid_magic_is_invalid_framing() {
        // A frame with valid magic but only 8 bytes (< 12) must be rejected as
        // invalid framing. Kills `< -> ==` and `|| -> &&`: both would let a
        // valid-magic short frame slip past the length guard.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(IMPORT_PROVENANCE_WIRE_MAGIC);
        bytes.extend_from_slice(&IMPORT_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
        assert_eq!(bytes.len(), 8, "premise: valid magic, 8 bytes total (< 12)");
        let err = decode_import_provenance_wire(&bytes).expect_err("short frame must be rejected");
        assert!(
            matches!(&err, StoreError::Configuration(msg) if msg.contains("framing is invalid")),
            "a short but valid-magic frame must be invalid framing, got {err:?}"
        );
    }

    #[test]
    fn decode_wire_accepts_an_older_body_schema_version() {
        // schema_version 0 is an OLDER (<= supported) body version and must decode
        // successfully. The `> -> <` mutant on the body-version guard rejects it
        // as a future version.
        let provenance = ImportProvenance {
            schema_version: 0,
            source_namespace: SourceNamespace::new("older-ns").expect("ns"),
            source_event_id: 0xABCD,
            source_global_sequence: 7,
            source_kind: 0x1234,
            source_content_hash: [0x9; 32],
        };
        let wire = encode_import_provenance_wire(&provenance).expect("encode");
        let decoded =
            decode_import_provenance_wire(&wire).expect("an older body schema version must decode");
        assert_eq!(
            decoded.schema_version, 0,
            "an older body schema version round-trips unchanged (kills `> -> <`)"
        );
        assert_eq!(decoded.source_event_id, 0xABCD);
    }

    #[test]
    fn import_key_present_via_idempotency_store_alone() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let dest = Store::open(StoreConfig::new(dir.path())).expect("open");
        let ns = SourceNamespace::new("dedup-ns").expect("ns");
        // The import key lives ONLY in the durable idempotency store, never as a
        // committed event id in the index. `import_key_already_present` must still
        // report present: the `||` short-circuits on the idemp hit. The `&&`
        // mutant, requiring BOTH stores to hold the key, returns false here.
        assert!(
            !import_key_already_present(&dest, import_key(&ns, 0xFEED)),
            "premise: the key is absent before recording"
        );
        let key_raw = import_key(&ns, 0xFEED).as_u128();
        dest.index.idemp.record(IdempEntry {
            key: key_raw,
            event_id: key_raw,
            global_sequence: 3,
            disk_pos_segment: 0,
            disk_pos_offset: 0,
            disk_pos_length: 1,
            content_hash: [0; 32],
            prev_hash: [0; 32],
            entity: "entity:dedup".to_owned(),
            scope: "scope:dedup".to_owned(),
            kind: EventKind::custom(0xE, 0x01),
            recorded_global_sequence: 3,
            event_evicted: false,
            receipt_extensions: BTreeMap::new(),
        });
        assert!(
            import_key_already_present(&dest, import_key(&ns, 0xFEED)),
            "an idempotency-store-only key must count as present (kills `|| -> &&`)"
        );
        dest.close().expect("close");
    }

    #[test]
    fn append_level_replay_receipt_at_the_frontier_counts_as_imported() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let kind = EventKind::custom(0xE, 0x32);
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let source = Store::open(StoreConfig::new(source_dir.path())).expect("open source");
        let coord = Coordinate::new("entity:import-frontier", "scope:import").expect("coord");
        let receipt_one = source
            .append(&coord, kind, &serde_json::json!({ "n": 1 }))
            .expect("append source event 1");
        let receipt_two = source
            .append(&coord, kind, &serde_json::json!({ "n": 2 }))
            .expect("append source event 2");

        let dest_dir = tempfile::tempdir().expect("dest tempdir");
        let dest = Store::open(StoreConfig::new(dest_dir.path())).expect("open dest");
        drop(
            dest.append(
                &Coordinate::new("entity:pre-existing", "scope:import").expect("coord"),
                kind,
                &serde_json::json!({ "seed": true }),
            )
            .expect("append destination seed event"),
        );
        // The append-level replay for event 1 is planted at EXACTLY the
        // pre-import frontier, so its receipt's global_sequence equals the
        // frontier. The strict `<` classifies it as imported (frontier < frontier
        // is false); the `<=` mutant would misclassify it as deduplicated.
        let frontier = dest.frontier().visible_hlc.global_sequence;
        assert!(
            frontier >= 1,
            "premise: the pre-import frontier is above origin"
        );

        let options = ImportOptions::new("frontier-ns")
            .expect("options")
            .with_chunk_size(3);
        let key_one = import_key(options.source_namespace(), receipt_one.event_id.as_u128());
        let key_two = import_key(options.source_namespace(), receipt_two.event_id.as_u128());

        let fabricate = |key: u128, seq: u64| IdempEntry {
            key,
            event_id: key,
            global_sequence: seq,
            disk_pos_segment: 0,
            disk_pos_offset: 0,
            disk_pos_length: 1,
            content_hash: [0; 32],
            prev_hash: [0; 32],
            entity: "entity:import-frontier".to_owned(),
            scope: "scope:import".to_owned(),
            kind,
            recorded_global_sequence: seq,
            event_evicted: false,
            receipt_extensions: BTreeMap::new(),
        };
        let index = Arc::clone(&dest.index);
        let calls = Arc::new(AtomicUsize::new(0));
        let filter_calls = Arc::clone(&calls);
        // Plant event 1's durable key AT the frontier and event 2's below it, on
        // the SECOND filter consultation — after event 1 has passed its own key
        // check and been queued, but before the page's append_batch.
        let entry_one = fabricate(key_one.as_u128(), frontier);
        let entry_two = fabricate(key_two.as_u128(), 0);
        let options = options.with_filter(Box::new(move |_entry| {
            if filter_calls.fetch_add(1, Ordering::SeqCst) == 1 {
                index.idemp.record(entry_one.clone());
                index.idemp.record(entry_two.clone());
            }
            true
        }));

        let report =
            import_events(&dest, &source, &ImportSelector::all(), &options).expect("import");

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "scenario shape: the filter must see exactly the two source events"
        );
        assert_eq!(
            report.imported, 1,
            "event 1's replay receipt at global_sequence == the pre-import frontier is imported \
             (frontier < frontier is false), not deduplicated (kills `< -> <=`)"
        );
        assert_eq!(
            report.deduplicated, 1,
            "only event 2's key check dedups; event 1's replay is classified imported"
        );

        source.close().expect("close source");
        dest.close().expect("close dest");
    }
}
