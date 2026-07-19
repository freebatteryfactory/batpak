//! Frozen coverage of the live legacy invariant catalog.

mod inventory;
mod types;

pub use types::{CoverageDisposition, LegacyInvariantCoverage};
pub use inventory::{COVERAGE, EXPECTED_COVERAGE_ROWS, LEGACY_SOURCE_COMMIT, SOURCE_INVARIANT_IDS};
