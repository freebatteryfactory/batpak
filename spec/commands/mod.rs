//! The three command namespaces (5.5E3e).
//!
//! A command's NAMESPACE IDENTITY and its INVOKED SEMANTIC AUTHORITY are
//! separate axes. Three separate closed vocabularies — never one mega enum,
//! never a universal CommandId, never a namespace-tagged payload:
//!
//! ```text
//! ProductCommand    batpak CLI verbs (lowercase tokens)
//! BatQlSourceMode   ASK/DO language modes (uppercase keywords) — language
//!                   law, never CLI verbs; DO is never a top-level command
//! TestPakCommand    testpak repository CLI verbs (lowercase tokens)
//! ```
//!
//! The enum variant IS the stable typed identity. The same surface token is
//! lawful across distinct namespaces (`batpak inspect` / `testpak inspect`);
//! uniqueness is enforced within each namespace, never globally. No raw
//! string mints an internal command identity, and the types do not
//! substitute for one another.
//!
//! docs/26 owns command-plane doctrine and projects both CLI inventories;
//! the BatQL companion owns ASK/DO semantics and projects the source-mode
//! inventory. CLI, justfile, completion, MCP, and help surfaces remain
//! future generated consumers of these tables.

mod types;

pub use types::{BatQlSourceMode, CommandAuthority, ProductCommand, TestPakCommand};
