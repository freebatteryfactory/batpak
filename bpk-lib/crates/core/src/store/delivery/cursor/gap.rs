use std::collections::VecDeque;
use std::num::NonZeroUsize;

/// Configuration for in-memory write-to-deliver gap observation.
///
/// A zero-capacity enabled configuration is structurally unrepresentable:
///
/// ```compile_fail
/// # use batpak::store::CursorGapConfig;
/// CursorGapConfig::Enabled { capacity: 0 };
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorGapConfig {
    /// Gap observation disabled.
    #[default]
    Disabled,
    /// Retain up to `capacity` gap observations before dropping the oldest.
    Enabled {
        /// Non-zero retained observation capacity.
        capacity: NonZeroUsize,
    },
}

/// A substrate-detectable write-to-deliver gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GapObservation {
    /// First sequence the cursor expected to deliver next.
    pub expected_sequence: u64,
    /// First visible delivered sequence after the skipped interval.
    pub delivered_sequence: u64,
    /// Half-open cancelled visibility ranges `[start, end)` intersecting the
    /// skipped interval.
    pub cancelled_ranges: Vec<(u64, u64)>,
}

#[derive(Clone, Debug)]
pub(super) struct GapBuffer {
    capacity: usize,
    observations: VecDeque<GapObservation>,
}

impl GapBuffer {
    pub(super) fn new_nonzero(capacity: NonZeroUsize) -> Self {
        let capacity = capacity.get();
        Self {
            capacity,
            observations: VecDeque::with_capacity(capacity),
        }
    }

    pub(super) fn push(&mut self, observation: GapObservation) {
        if self.observations.len() == self.capacity {
            self.observations.pop_front();
        }
        self.observations.push_back(observation);
    }

    pub(super) fn take_all(&mut self) -> Vec<GapObservation> {
        self.observations.drain(..).collect()
    }
}
