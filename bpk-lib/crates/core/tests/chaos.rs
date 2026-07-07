#![cfg(target_os = "linux")]
#![cfg(feature = "dangerous-test-hooks")]

//! PROVES: INV-CHAOS-LINUX-ONLY — the dm-flakey chaos suite (a real Linux
//! device-mapper failure boundary) compiles and runs ONLY under
//! `target_os = "linux"` plus the `dangerous-test-hooks` feature.
//! CATCHES: the privileged chaos harness leaking onto a non-Linux target or into
//! the default feature set (both cfg gates must hold for this entrypoint to build).
//! SEEDED: n/a — cfg-gated module entrypoint; determinism lives in the scenarios.

#[path = "chaos/mod.rs"]
mod chaos;
