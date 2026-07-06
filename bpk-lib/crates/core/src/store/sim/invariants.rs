//! The simulation's append-ordered model log and durable chain head.
//!
//! [`ModelState`] records each simulated event (sequence, chain hash, durable
//! flag), tracks the monotonic visible frontier, and exposes the durable chain
//! head. This state is the bookkeeping the seeded workload folds into its
//! determinism digest — it is not a safety oracle.
//!
//! Real-Store safety over the sim backends (hash-chain continuity, no durable
//! loss after crash/recover, canonical reopen) is proven directly against a
//! real `Store` by the fork/import/recovery DST corpus (`recovery.rs`,
//! `fork_recovery.rs`, `import_recovery.rs`, `recovery_matrix.rs`); the model
//! does not re-verify those properties on itself.

/// A single logged event in the simulation model.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SimEvent {
    /// Monotonic sequence number assigned at append.
    pub(crate) seq: u64,
    /// This event's chain hash (`fold(prev, seq, payload)` over the durable
    /// chain head at append time).
    pub(crate) hash: u64,
    /// Whether the writer acknowledged this event as durable (fsynced).
    pub(crate) durable: bool,
}

/// Running model state maintained by the seeded workload.
#[derive(Default)]
pub(crate) struct ModelState {
    /// The append-ordered event log.
    pub(crate) log: Vec<SimEvent>,
    /// Highest visible frontier observed so far (monotonic).
    pub(crate) visible_frontier: u64,
}

impl ModelState {
    /// Append `event`, advancing the frontier when it is durable. The caller
    /// links the event's chain hash to [`Self::chain_head`] (the last *durable*
    /// hash). Only durable events enter the hash chain — a torn or unsynced
    /// event is recorded for the frontier/recovery view but does not advance the
    /// chain, mirroring the real store where only committed events are chained.
    pub(crate) fn append(&mut self, event: SimEvent) {
        if event.durable {
            self.visible_frontier = self.visible_frontier.max(event.seq);
        }
        self.log.push(event);
    }

    /// The current hash-chain head: the hash of the last *durable* event, or 0
    /// for an empty (or all-non-durable) chain. Callers link the next event's
    /// chain hash to this so the durable chain stays continuous across torn
    /// writes.
    pub(crate) fn chain_head(&self) -> u64 {
        self.log
            .iter()
            .rev()
            .find(|e| e.durable)
            .map_or(0, |e| e.hash)
    }
}
