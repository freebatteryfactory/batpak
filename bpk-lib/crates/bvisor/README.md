# bvisor

Sync-first **boundary supervisor**: a platform-agnostic boundary contract (types
+ a fail-closed planner) with **real** confinement backends —
Linux (landlock / seccomp / cgroups, via a single-threaded confinement launcher)
and Wasm (the [wasmi](https://crates.io/crates/wasmi) interpreter + WASI
preview1). The `Backend` trait carries **zero OS code and zero BatPak writes**;
backends only *observe*, the runner *seals* a report, and the host *persists* it.

```rust
use bvisor::{run_confined, BoundarySpec, WasmBackend};
use std::sync::Arc;

// One call does registry -> plan -> run, fail-closed:
let report = run_confined(&spec, Arc::new(WasmBackend::new()))?;
```

The Wasm backend is CVE-clean and tokio-free (pure-Rust interpreter, no JIT), and
warms a content-addressed compiled-module cache so repeated runs skip the
dominant setup cost. Admission is fail-closed: a spec a backend cannot back is
refused before any guest runs; controlled terminals ride the report's outcome.

The OS-backend structs (`LinuxBackend`, `WasmBackend`) are gated behind
`backend-linux` / `backend-wasm` features. Licensed under MIT OR Apache-2.0.
