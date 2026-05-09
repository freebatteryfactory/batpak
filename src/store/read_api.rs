use super::*;

impl<State> Store<State> {
    /// READ: get a single event by ID.
    ///
    /// # Errors
    /// Returns `StoreError::NotFound` if no event with that ID exists.
    /// Returns `StoreError::Io` or `StoreError::Serialization` if reading from disk fails.
    pub fn get(&self, event_id: u128) -> Result<StoredEvent<serde_json::Value>, StoreError> {
        let entry = self
            .index
            .get_by_id(event_id)
            .ok_or(StoreError::NotFound(event_id))?;
        self.reader.read_entry(&entry.disk_pos)
    }

    /// READ: fetch a single event by ID with the payload left as raw
    /// MessagePack bytes. Mirrors [`get`](Self::get) but skips the
    /// JSON-decode step, suitable for the `RawMsgpackInput` lane of a
    /// multi-event reactor.
    ///
    /// # Errors
    /// Returns `StoreError::NotFound` if no event with that ID exists.
    /// Returns `StoreError::Io` or `StoreError::Serialization` if reading
    /// from disk fails.
    pub fn get_raw(&self, event_id: u128) -> Result<StoredEvent<Vec<u8>>, StoreError> {
        let entry = self
            .index
            .get_by_id(event_id)
            .ok_or(StoreError::NotFound(event_id))?;
        self.reader.read_entry_raw(&entry.disk_pos)
    }

    /// Verify an append receipt against the store's signing-key registry and
    /// current index state.
    #[must_use]
    pub fn verify_append_receipt(&self, receipt: &AppendReceipt) -> bool {
        let Some(entry) = self.index.get_by_id(receipt.event_id) else {
            return false;
        };
        self.runtime.signing_registry.verify_append_receipt(
            receipt,
            &entry.coord,
            entry.kind,
            entry.hash_chain.prev_hash,
        )
    }

    /// Verify a persisted denial receipt against the store's signing-key
    /// registry and current index state.
    #[must_use]
    pub fn verify_denial_receipt(&self, receipt: &DenialReceipt) -> bool {
        let Some(entry) = self.index.get_by_id(receipt.event_id) else {
            return false;
        };
        self.runtime.signing_registry.verify_denial_receipt(
            receipt,
            &entry.coord,
            entry.kind,
            entry.hash_chain.prev_hash,
        )
    }

    /// READ: query by Region.
    #[must_use]
    pub fn query(&self, region: &Region) -> Vec<IndexEntry> {
        self.index.query(region)
    }

    /// READ: walk hash chain ancestors.
    pub fn walk_ancestors(
        &self,
        event_id: u128,
        limit: usize,
    ) -> Vec<StoredEvent<serde_json::Value>> {
        ancestry::walk_ancestors(self, event_id, limit)
    }

    /// PROJECT: reconstruct typed state from events, with cache support.
    ///
    /// # Errors
    /// Returns any replay, deserialization, cache, or disk-read error surfaced
    /// while reconstructing the projection state.
    pub fn project<T>(&self, entity: &str, freshness: &Freshness) -> Result<Option<T>, StoreError>
    where
        T: EventSourced + serde::Serialize + serde::de::DeserializeOwned + 'static,
        T::Input: projection::flow::ReplayInput,
    {
        projection::flow::project(self, entity, freshness)
    }

    /// Return the current per-entity generation if the entity exists.
    ///
    /// Generations advance monotonically on every insert for that entity.
    /// When entity-group overlays are disabled, this falls back to the entity
    /// stream length so callers still get a stable monotonic skip token.
    pub fn entity_generation(&self, entity: &str) -> Option<u64> {
        self.index.entity_generation(entity)
    }

    /// Project only when the entity changed since `last_seen_generation`.
    ///
    /// Returns `Ok(None)` when no change is observed. Otherwise returns the
    /// generation at which the returned state was materialized together with
    /// the freshly projected state. The returned generation is honest: a
    /// cache-hit path returns the generation at which the cache was
    /// stamped, a replay path returns the generation sampled before replay
    /// started. Callers who persist this generation as a watermark (e.g.
    /// [`ProjectionWatcher`]) will not silently consume a relevant append
    /// against stale state (F5). To preserve that property, this API treats
    /// [`Freshness::MaybeStale`] the same as [`Freshness::Consistent`].
    ///
    /// # Errors
    /// Returns any error surfaced by [`Store::project`] when the entity has
    /// changed and the projection must be rebuilt.
    pub fn project_if_changed<T>(
        &self,
        entity: &str,
        last_seen_generation: u64,
        freshness: &Freshness,
    ) -> Result<Option<(u64, Option<T>)>, StoreError>
    where
        T: EventSourced + serde::Serialize + serde::de::DeserializeOwned + 'static,
        T::Input: projection::flow::ReplayInput,
    {
        projection::flow::project_if_changed(self, entity, last_seen_generation, freshness)
    }

    /// CONVENIENCE: sugar over index.stream() for exact entity match.
    #[must_use]
    pub fn stream(&self, entity: &str) -> Vec<IndexEntry> {
        self.index.stream(entity)
    }

    /// READ: query all events in the given scope.
    #[must_use]
    pub fn by_scope(&self, scope: &str) -> Vec<IndexEntry> {
        self.query(&Region::scope(scope))
    }

    /// READ: query all events of the given event kind across all entities and scopes.
    #[must_use]
    pub fn by_fact(&self, kind: EventKind) -> Vec<IndexEntry> {
        self.query(&Region::all().with_fact(KindFilter::Exact(kind)))
    }

    /// READ (typed): query all events whose kind matches `T::KIND`.
    ///
    /// Available on both `Store<Open>` and `Store<ReadOnly>`.
    #[must_use]
    pub fn by_fact_typed<T: EventPayload>(&self) -> Vec<IndexEntry> {
        self.by_fact(T::KIND)
    }

    /// CURSOR: pull-based, ordered delivery from the in-memory index.
    ///
    /// Available on both `Store<Open>` and `Store<ReadOnly>`. This cursor is
    /// process-local only: it does not persist its position, so restart-time
    /// at-least-once semantics require the checkpoint-bound cursor worker
    /// surface rather than this constructor.
    pub fn cursor_guaranteed(&self, region: &Region) -> Cursor {
        Cursor::new(region.clone(), Arc::clone(&self.index))
    }

    /// DIAGNOSTICS
    pub fn stats(&self) -> StoreStats {
        lifecycle::stats(self)
    }

    /// Return detailed diagnostic information about the store's internal state.
    pub fn diagnostics(&self) -> StoreDiagnostics {
        lifecycle::diagnostics(self)
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

    /// Return the current operator-facing frontier view.
    pub fn frontier(&self) -> FrontierView {
        self.watermark_handle.lock().snapshot_view()
    }

    /// Return a coherent clone of the internal frontier watermarks.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_watermark_snapshot(&self) -> WatermarkSnapshot {
        self.watermark_handle.lock().snapshot()
    }

    /// Register a projection ID in the applied-frontier registry.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_register_projection(&self, projection_id: &str) {
        self.projection_registry.register(projection_id.to_owned());
    }

    /// Register the same projection ID used by `project::<T>()` for `entity`.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_register_projection_for<T: 'static>(&self, entity: &str) {
        self.projection_registry
            .register(ProjectionRegistry::id_for_type::<T>(entity));
    }

    /// Report projection progress directly for focused frontier tests.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_notify_projection_applied(&self, projection_id: &str, point: HlcPoint) {
        self.projection_registry
            .notify_applied(projection_id.to_owned(), point);
    }

    /// Remove a projection ID from the applied-frontier registry.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_unregister_projection(&self, projection_id: &str) {
        self.projection_registry.unregister(projection_id);
    }

    /// Wake frontier waiters without advancing a watermark.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_notify_watermark_waiters(&self) {
        self.watermark_handle.dangerous_notify_all();
    }
}
