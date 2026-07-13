//! `Display` render bodies for the authority-sidecar refusals (durable
//! idempotency authority + store metadata). Split from `display.rs` to keep
//! that file within the absolute production size cap (split-don't-bump);
//! `StoreError::fmt_authority_refusal` delegates here.

pub(super) fn fmt_idemp_authority_corrupt(
    f: &mut std::fmt::Formatter<'_>,
    path: &std::path::Path,
    kind: &super::IdempAuthorityCorruption,
) -> std::fmt::Result {
    write!(
        f,
        "durable idempotency authority at {} is unreadable ({kind}); failing closed: after \
         retention eviction this sidecar is the only proof a keyed retry is not a new command, \
         so damage is never treated as an empty set — restore the file from a backup or a \
         healthy replica of the store directory",
        path.display()
    )
}

pub(super) fn fmt_idemp_authority_missing(
    f: &mut std::fmt::Formatter<'_>,
    path: &std::path::Path,
) -> std::fmt::Result {
    write!(
        f,
        "durable idempotency authority {} is missing although this store's metadata records a \
         durable-authority expectation; an absent sidecar on a keyed store is authority LOSS, \
         not a fresh store — restore index.idemp from a backup or a healthy replica of the \
         store directory",
        path.display()
    )
}

pub(super) fn fmt_idemp_authority_foreign(
    f: &mut std::fmt::Formatter<'_>,
    kind: &super::IdempAuthorityForeignKind,
) -> std::fmt::Result {
    write!(
        f,
        "durable idempotency authority image belongs to a different history than this store's \
         ({kind}); refusing to admit it — restore this store's own image from a backup or a \
         healthy replica"
    )
}

pub(super) fn fmt_idemp_authority_stale(
    f: &mut std::fmt::Formatter<'_>,
    image_covered: u64,
    expected_covered: u64,
) -> std::fmt::Result {
    write!(
        f,
        "durable idempotency authority image covers global sequence {image_covered} but this \
         store's metadata expects coverage through {expected_covered}; a stale image would \
         silently drop keys committed after it was written — restore a current image from a \
         backup or a healthy replica"
    )
}

pub(super) fn fmt_store_metadata_corrupt(
    f: &mut std::fmt::Formatter<'_>,
    path: &std::path::Path,
    kind: &super::StoreMetaCorruption,
) -> std::fmt::Result {
    write!(
        f,
        "store metadata at {} is unreadable ({kind}); failing closed: store.meta carries the \
         store's lineage identity and durable-authority expectations, so damage is never treated \
         as absence — restore the file from a backup or a healthy replica of the store directory \
         (do not delete it; a remint would reset the identity)",
        path.display()
    )
}

pub(super) fn fmt_store_metadata_missing(
    f: &mut std::fmt::Formatter<'_>,
    path: &std::path::Path,
) -> std::fmt::Result {
    write!(
        f,
        "store metadata {} is missing although the idempotency sidecar proves this store was \
         already migrated (format v2+); refusing to remint the lineage identity — restore \
         store.meta from a backup or a healthy replica of the store directory",
        path.display()
    )
}

pub(super) fn fmt_idemp_restore_refused(
    f: &mut std::fmt::Formatter<'_>,
    reason: &super::IdempotencyRestoreRefusal,
) -> std::fmt::Result {
    write!(
        f,
        "idempotency-authority restore refused ({reason}); the export is admitted only when this \
         store's metadata authorizes its generation token or a fresh generation can be \
         corroborated and minted — restore a matching-lineage, current export, or take a new \
         export from a healthy store"
    )
}
