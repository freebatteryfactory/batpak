use super::*;

impl<State: crate::store::StoreState> Store<State> {
    /// DIAGNOSTICS
    pub fn stats(&self) -> StoreStats {
        lifecycle::stats(self)
    }

    /// Return detailed diagnostic information about the store's internal state.
    pub fn diagnostics(&self) -> StoreDiagnostics {
        lifecycle::diagnostics(self)
    }

    /// The store's LINEAGE IDENTITY (#205): a stable anchor for derived state
    /// persisted OUTSIDE the store (projection sidecars, external cursors) to
    /// detect "this cache belongs to a different store lineage" on load.
    ///
    /// Semantics — read these before anchoring anything to it:
    /// - minted once, at the first writable open of the directory, and
    ///   persisted in the `store.meta` sidecar;
    /// - **copied by snapshot and copied by fork** (both copy the idempotency
    ///   authority alongside the logical history);
    /// - stable across path moves and reopen;
    /// - distinguishes UNRELATED store lineages;
    /// - it does **not** by itself detect rewind or sibling-fork divergence —
    ///   two forks of one store share the identity and may diverge. External
    ///   derived state must bind to the identity PLUS a history anchor (the
    ///   frontier sequence it was built at and the event id observed at that
    ///   sequence).
    ///
    /// # Errors
    /// [`StoreError::StoreMetadataMissing`] when the store was opened
    /// read-only over a legacy directory that predates `store.meta` (a
    /// writable open performs the one-time migration; see the never-remint
    /// law on the sidecar module).
    pub fn identity(&self) -> Result<crate::id::StoreIdentity, StoreError> {
        self.runtime
            .identity
            .get()
            .copied()
            .ok_or_else(|| StoreError::StoreMetadataMissing {
                path: self
                    .config
                    .data_dir
                    .join(crate::store::store_meta::STORE_META_FILENAME),
            })
    }

    /// Number of keys currently held in the durable idempotency store.
    ///
    /// This is the persistent dedup authority that survives retention
    /// compaction, cold-start, and snapshot independent of event eviction. It
    /// can temporarily exceed the configured soft cap under a within-window
    /// key-rate spike (the window always wins on correctness). Exposed for
    /// diagnostics and durability tests.
    pub fn durable_idempotency_key_count(&self) -> usize {
        self.index.idemp.len()
    }

    /// Deterministic store resource evidence over stable [`StoreDiagnostics`] facts.
    ///
    /// Canonical identity excludes raw paths (uses [`store_data_dir_identity_hash`]),
    /// free-form envelope diagnostics, and timestamps outside the structured cold-start
    /// report. Metadata fields on the returned envelope are unset by default.
    ///
    /// # Errors
    /// Canonical body encoding failure while computing `body_hash`.
    pub fn store_resource_evidence_report(
        &self,
    ) -> Result<StoreResourceEvidenceReport, StoreResourceReportError> {
        store_resource_evidence_report_from_diagnostics(&lifecycle::diagnostics(self))
    }
}
