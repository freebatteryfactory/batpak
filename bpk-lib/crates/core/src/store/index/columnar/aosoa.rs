use super::EntryQuery;
use crate::event::EventKind;
use crate::store::index::{IndexEntry, QueryHit};
use std::collections::HashSet;
use std::sync::Arc;

/// A tile that holds up to `N` events of the **same** kind.
///
/// `repr(C, align(64))` aligns the tile to a 64-byte cache-line boundary. The
/// `kinds` column is an **inline** fixed-width `[EventKind; N]` array embedded
/// directly in the tile, so the kind values sit contiguously with no heap hop —
/// this is the locality the AoSoA64 layout exists to provide, and the shape a
/// future SIMD kind-scan needs. Live slots are `kinds[0..len]`; the trailing
/// `kinds[len..N]` slots hold [`EventKind::UNINIT`] padding and are never read
/// by the scan (it consults only `kinds[0]` of a non-empty tile and masks
/// entries by `len`).
///
/// ### Why is `entries` still a `Vec`?
///
/// Const-generic arrays of non-`Copy` types (e.g. `[Arc<IndexEntry>; N]`)
/// require `T: Default`, which `Arc<IndexEntry>` does not implement. `entries`
/// therefore stays a `Vec` pre-reserved to `N` (no reallocation over the tile's
/// lifetime). `EventKind` **is** `Copy` and has the `UNINIT` sentinel, so the
/// kind column — the hot one for kind/category queries — is the inline array.
#[repr(C, align(64))]
pub(crate) struct Tile<const N: usize> {
    /// Event kinds stored in this tile; live slots `kinds[0..len]` all share the
    /// same kind. Slots `[len..N]` hold [`EventKind::UNINIT`] padding.
    pub kinds: [EventKind; N],
    /// Full index entries parallel to `kinds[0..len]`.
    pub entries: Vec<Arc<IndexEntry>>,
    /// Number of valid elements currently stored in the tile.
    pub len: usize,
}

impl<const N: usize> Tile<N> {
    /// Create an empty tile: the inline kind column is padded to `N` with the
    /// [`EventKind::UNINIT`] sentinel, and `entries` is pre-reserved to `N`.
    pub(crate) fn new() -> Self {
        Self {
            kinds: [EventKind::UNINIT; N],
            entries: Vec::with_capacity(N),
            len: 0,
        }
    }

    /// Returns `true` when the tile has no room for another entry.
    #[inline]
    pub(crate) fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Append an entry. Panics (debug only) if the tile is already full; the
    /// `open_tiles` bookkeeping in [`AoSoAInner`] guarantees a tile is never
    /// pushed past `N`, so the inline `kinds[self.len]` write is always in range.
    pub(crate) fn push(&mut self, kind: EventKind, entry: Arc<IndexEntry>) {
        debug_assert!(!self.is_full(), "Tile<{N}>::push called on a full tile");
        self.kinds[self.len] = kind;
        self.entries.push(entry);
        self.len += 1;
    }
}

/// Internal state for tiled AoSoA layouts.
///
/// Events are bucketed into tiles by kind: every tile contains entries of a
/// single `EventKind` (matching `kinds[0]` for any non-empty tile). Each kind
/// has at most one open tile at a time; `open_tiles` maps a kind to the index
/// of its current open tile. When a tile fills, it is evicted from `open_tiles`
/// and a new tile is started on the next event of that kind.
///
/// This strategy keeps tiles full regardless of insertion order, so interleaved
/// multi-kind workloads produce the same tile density as sorted runs.
///
/// The outer `Vec` of `Tile`s is unsorted; `query_by_kind` iterates all tiles
/// and collects matching entries. Tiles are cache-line aligned, but the current
/// scan is scalar. The tile structure is the correct layout for a future SIMD
/// specialization; see the AoSoA64 variant.
pub(super) struct AoSoAInner<const N: usize> {
    pub(super) tiles: Vec<Tile<N>>,
    /// kind → index of the currently open (not yet full) tile for that kind.
    open_tiles: std::collections::HashMap<EventKind, usize>,
    // scope membership is correct-by-construction because `coord.scope` is
    // immutable post-construction; debug_assertions verifies invariant at
    // insert time.
    /// scope → entity set, same role as in SoAInner.
    scope_entities: std::collections::HashMap<Arc<str>, HashSet<Arc<str>>>,
}

impl<const N: usize> AoSoAInner<N> {
    pub(super) fn new() -> Self {
        Self {
            tiles: Vec::new(),
            open_tiles: std::collections::HashMap::new(),
            scope_entities: std::collections::HashMap::new(),
        }
    }

    pub(super) fn from_entries(entries: &[Arc<IndexEntry>]) -> Self {
        let mut built = Self::new();
        for entry in entries {
            built.push(entry);
        }
        built
    }

    /// Append one event into the appropriate tile.
    ///
    /// Each kind has at most one open tile. If the open tile for this kind is
    /// full (or none exists), a new tile is allocated and registered as open.
    pub(super) fn push(&mut self, entry: &Arc<IndexEntry>) {
        let scope: Arc<str> = entry.coord.scope_arc();
        let entity: Arc<str> = entry.coord.entity_arc();
        let kind = entry.kind;
        debug_assert_eq!(
            scope.as_ref(),
            entry.coord.scope(),
            "scope_entities bucket must match entry.coord.scope()"
        );

        match self.open_tiles.get(&kind).copied() {
            Some(idx) => {
                self.tiles[idx].push(kind, Arc::clone(entry));
                if self.tiles[idx].is_full() {
                    self.open_tiles.remove(&kind);
                }
            }
            None => {
                let new_idx = self.tiles.len();
                let mut tile = Tile::new();
                tile.push(kind, Arc::clone(entry));
                let is_full = tile.is_full();
                self.tiles.push(tile);
                if !is_full {
                    self.open_tiles.insert(kind, new_idx);
                }
            }
        }

        self.scope_entities.entry(scope).or_default().insert(entity);
    }

    fn query_hits_entries(&self, mut matches: impl FnMut(EventKind) -> bool) -> Vec<QueryHit> {
        let mut out = Vec::new();
        for tile in &self.tiles {
            if tile.len == 0 {
                continue;
            }
            if !matches(tile.kinds[0]) {
                continue;
            }
            for entry in tile.entries.iter().take(tile.len) {
                out.push(QueryHit::from_entry(entry));
            }
        }
        out
    }

    pub(super) fn query_hits_by_kind(&self, target: EventKind) -> Vec<QueryHit> {
        self.query_hits_entries(|kind| kind == target)
    }

    pub(super) fn query_hits_by_category(&self, category: u8) -> Vec<QueryHit> {
        self.query_hits_entries(|kind| kind.category() == category)
    }

    pub(super) fn query_hits_by_scope(&self, scope: &str) -> Vec<QueryHit> {
        let Some(entities) = self.scope_entities.get(scope) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for tile in &self.tiles {
            for entry in tile.entries.iter().take(tile.len) {
                if entities.contains(entry.coord.entity_arc().as_ref()) {
                    out.push(QueryHit::from_entry(entry));
                }
            }
        }
        out
    }

    pub(super) fn hits_candidates(&self, spec: &EntryQuery<'_>) -> Vec<QueryHit> {
        match spec {
            EntryQuery::Kind(kind) => self.query_hits_by_kind(*kind),
            EntryQuery::Category(category) => self.query_hits_by_category(*category),
            EntryQuery::Scope(scope) => self.query_hits_by_scope(scope),
        }
    }

    /// Execute `f` on the tile at position `idx`.
    ///
    /// Returns `None` if `idx` is out of range.
    #[cfg(test)]
    pub(crate) fn with_tile<R>(&self, idx: usize, f: impl FnOnce(&Tile<N>) -> R) -> Option<R> {
        self.tiles.get(idx).map(f)
    }

    pub(super) fn clear(&mut self) {
        self.tiles.clear();
        self.open_tiles.clear();
        self.scope_entities.clear();
    }
}
