//! The generated-view registry (5.5E4a): the closed denominator of every
//! repository-generated view.
//!
//! Owned by BP-SELF-EXPLAINING-1 (docs/28). This registry describes repository
//! GENERATION METADATA only: which generated views exist, which authority
//! source each serializes, where each lands, what surface form it takes, and
//! which bootstrap generator owns the mechanical projection. It never owns or
//! completes the semantic facts a view renders — every rendered fact remains
//! owned by the view's named authority source.
//!
//! The enum variant is the logical view identity: there is no
//! `GeneratedViewId`, no `ProjectionId`, and no universal repository artifact
//! identity. `ALL` is the sole inventory and `spec()` the sole fact function —
//! no parallel row table. `bootstrap/project.py` builds every projection plan
//! by iterating this registry, and `bootstrap/audit.py` independently proves
//! that the registered embedded target instances equal the actual generated
//! markers in the tracked corpus. A future generated view must enter this
//! registry before it may land.

mod registry;
mod types;

pub use registry::*;
pub use types::*;
