//! # netbat_route_introspection
//!
//! **Witnesses:** the [`netbat`] route surface can DESCRIBE a boundary — enumerate
//! the routes an operation set exposes and inspect a live [`syncbat::Core`]'s
//! mounted operations — WITHOUT opening a TCP server. Pure metadata:
//!
//! - a [`syncbat::Module`] of operations is wrapped by [`netbat::ServerModule`]
//!   under a base path, mounted into a [`netbat::Server`], and introspected via
//!   [`netbat::Server::introspect`] + [`netbat::Server::routes`]; and
//! - a real `syncbat::Core` built from the same operations is inspected with
//!   [`netbat::inspect_core_operations`], reporting mounted vs missing names.
//!
//! We assert the described routes match the registered operations exactly (one
//! route per operation, path = `{base}/{operation}`), and that the core-health
//! report enumerates every mounted operation and flags an absent one.
//!
//! Run: `cargo run -p batpak-examples --bin netbat_route_introspection`

use std::collections::BTreeSet;
use std::io::Write;

use netbat as nb;
use syncbat::{Core, Ctx, EffectClass, Handler, HandlerResult, Module, OperationDescriptor};

/// A side-effect-free handler: echoes its input. Used only so the operations can
/// be mounted into a real `Core` for the health-introspection half of the demo.
struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(&mut self, input: &[u8], _cx: &mut Ctx<'_>) -> HandlerResult {
        Ok(input.to_vec())
    }
}

/// The operations exposed at the boundary. Mechanisms, not meanings.
fn operations() -> Vec<OperationDescriptor> {
    vec![
        OperationDescriptor::new(
            "route.status.v1",
            EffectClass::Inspect,
            "route.status.input.v1",
            "route.status.output.v1",
            "receipt.route.status.v1",
        ),
        OperationDescriptor::new(
            "route.submit.v1",
            EffectClass::Compute,
            "route.submit.input.v1",
            "route.submit.output.v1",
            "receipt.route.submit.v1",
        ),
        OperationDescriptor::new(
            "route.scan.v1",
            EffectClass::Inspect,
            "route.scan.input.v1",
            "route.scan.output.v1",
            "receipt.route.scan.v1",
        ),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();

    const BASE_PATH: &str = "/api";
    let descriptors = operations();
    let registered: BTreeSet<String> = descriptors.iter().map(|d| d.name().to_owned()).collect();

    // -- Describe the routes WITHOUT serving -----------------------------------
    let module = Module::from_operations("catalog", descriptors.iter().cloned())?;
    let server_module = nb::ServerModule::expose(module, BASE_PATH)?;
    let mut server = nb::Server::new();
    server.mount(server_module)?;

    let report = server.introspect();
    let _ = writeln!(
        out,
        "introspection: {} module(s), {} operation(s), {} route(s) [{}]",
        report.module_count, report.operation_count, report.route_count, report.layer_rule,
    );
    assert_eq!(report.module_count, 1);
    assert_eq!(report.operation_count, registered.len());
    assert_eq!(report.route_count, registered.len());

    // Each described route maps one registered operation at `{base}/{operation}`.
    let mut described = BTreeSet::new();
    for route in server.routes() {
        let expected_path = format!("{BASE_PATH}/{}", route.operation_name());
        assert_eq!(
            route.path(),
            expected_path,
            "route path must be base + operation name",
        );
        let _ = writeln!(
            out,
            "  route {} {} -> {}",
            route.method(),
            route.path(),
            route.operation_name(),
        );
        described.insert(route.operation_name().to_owned());
    }
    assert_eq!(
        described, registered,
        "described routes must match the registered operations exactly",
    );

    // -- Inspect a live Core's mounted operations (still no server) ------------
    let mut builder = Core::builder();
    for descriptor in &descriptors {
        builder.register(descriptor.clone(), EchoHandler)?;
    }
    builder.without_receipts();
    let core = builder.build()?;

    // Probe the registered names plus one that was never mounted.
    let probe: Vec<&str> = registered
        .iter()
        .map(String::as_str)
        .chain(std::iter::once("route.absent.v1"))
        .collect();
    let health = nb::inspect_core_operations(&core, probe);
    let _ = writeln!(
        out,
        "core health: {} mounted, {} missing (healthy={})",
        health.mounted_operations.len(),
        health.missing_operations.len(),
        health.is_healthy(),
    );
    assert_eq!(
        health
            .mounted_operations
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        registered,
        "every registered operation is mounted in the core",
    );
    assert_eq!(
        health.missing_operations,
        vec!["route.absent.v1".to_owned()],
        "the never-mounted operation is reported missing",
    );
    assert!(
        !health.is_healthy(),
        "a missing operation makes the probe unhealthy"
    );

    let _ = writeln!(
        out,
        "OK: routes described + core inspected with no TCP server",
    );
    Ok(())
}
