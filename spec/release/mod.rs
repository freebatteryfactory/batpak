//! The release-seal vocabulary (DEC-058, 5.5E1): what a release receipt
//! binds. One typed inventory — docs/36 projects the full list, docs/24
//! names the gauntlet inputs, and DEC-058 references this owner instead of
//! restating a third copy. Every field is mandatory: a set-valued field is
//! present even when its set is empty (an empty set is evidence, a missing
//! field is an incomplete envelope), and which fields are set-valued is the
//! typed EmptySetPosture classification, never doc-comment prose.

mod types;
mod inventory;

pub use types::{EmptySetPosture, ReleaseSealField};
pub use inventory::RELEASE_SEAL_FIELDS;
