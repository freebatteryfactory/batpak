//! The DEC-075 composition: single-writer dual-axis reconciliation.
//!
//! Every coordinate this file binds is owned elsewhere — durable order by the
//! journal law (LEG-001/067, docs/05), chronology by the time contract
//! (docs/16, DEC-061), turn and attempt identity by the runtime and Bvisor
//! contracts (docs/08/09), receipts and reconciliation posture by docs/14 —
//! and this file redeclares none of them. What it owns is the COMPOSITION:
//! which coordinates exist, what role each plays in reconciling the logical
//! and physical books, which axes a receipt preserves without inference, and
//! which signals may decide a retry. The lesson this composition encodes is
//! the old `HlcPoint` autopsy (issue #208): one type may carry fields
//! efficiently, but one NAME may never own multiple ordering authorities.
//!
//! `docs/02_SYSTEM_MODEL.md` owns the end-to-end prose law; its coordinate
//! and retry tables are generated projections of the constants below.

mod inventory;
mod types;

pub use types::{
    DOUBLE_ENTRY_AXES, DoubleEntryAxis, RETRY_SIGNALS, ReconciliationCoordinate,
    ReconciliationRole, RetrySignal,
};
pub use inventory::RECONCILIATION_COORDINATES;
