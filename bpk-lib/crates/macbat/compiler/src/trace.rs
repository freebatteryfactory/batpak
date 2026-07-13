//! Optional per-pass expansion trace. Populated only when `ExpandOptions.trace` is
//! set; otherwise every `ExpansionTrace` on an artifact is empty. Each
//! `TraceRecord` is a one-line summary of a pass, and the ordered sequence is the
//! input to the `expand` stepper example. Kept in a `Vec` for deterministic order.

/// A single pass's contribution to the trace: the pass name and a human summary.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TraceRecord {
    pub pass: &'static str,
    pub summary: String,
}

/// The ordered trace of a single expansion. Empty unless tracing was requested.
///
/// NOTE (flagged to the coordinator): the master contract §B.12 pins only the
/// private `records` field and no methods. A trace the passes cannot append to and
/// the stepper cannot read is inert, so this adds the minimal functional API
/// (`push` / `records` / `is_empty` + `Default`) needed for the passes to populate
/// it and the `expand` example to render it. No behavior beyond append-and-read.
#[derive(Clone, Default)]
pub struct ExpansionTrace {
    records: Vec<TraceRecord>,
}

impl ExpansionTrace {
    /// Append a pass record (only called on the traced path).
    pub fn push(&mut self, record: TraceRecord) {
        self.records.push(record);
    }

    /// The ordered pass records.
    #[must_use]
    pub fn records(&self) -> &[TraceRecord] {
        &self.records
    }

    /// Whether nothing was traced (the untraced default).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}
