# hostbat

Generic deterministic **module host** for the BatPak family: mount
content-identified modules over a [`syncbat`](https://crates.io/crates/syncbat)
`Core` and drive them with a deterministic startup/shutdown hook schedule.

`hostbat` adds what a raw `syncbat::Core` has no concept of — content identity
(`H_module` per module + a `HostFingerprint`), modules that bundle operations +
guard + hooks + jobs as one mountable unit, a generic `Supervisor`, and the
host-control effect axis. It **does not** reimplement dispatch, receipts, or
admission — those stay in `syncbat`; it lowers mounted modules into one
`syncbat::CoreBuilder` and delegates invocation to the composed `Core`.

```rust
use hostbat::{HostBuilder, HostModule};

let module = HostModule::builder("my.module", 1)
    .operation(descriptor, handler)?
    .build()?;
let mut host = HostBuilder::new().mount(module)?.build()?;
let out = host.invoke("my.op", input)?;
```

Part of the sync-first, append-only BatPak stack. Licensed under MIT OR
Apache-2.0.
