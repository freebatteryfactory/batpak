#![allow(clippy::panic)]

use netbat as nb;
use syncbat::{Core, EffectClass, Handler, HandlerResult, Module, OperationDescriptor};

const PING: OperationDescriptor = OperationDescriptor::new(
    "ping",
    EffectClass::Inspect,
    "schema.ping.input.v1",
    "schema.ping.output.v1",
    "receipt.ping.v1",
);

struct PingHandler;

impl Handler for PingHandler {
    fn handle(&mut self, input: &[u8], _cx: &mut syncbat::Cx<'_>) -> HandlerResult {
        Ok(input.to_vec())
    }
}

#[test]
fn exposes_syncbat_module_as_boundary_routes_without_dispatch() {
    let module = Module::from_operations("health", [PING]).expect("module builds");
    let server_module = nb::ServerModule::expose(module, "/nb");

    assert_eq!(server_module.name(), "health");
    assert_eq!(server_module.operation_count(), 1);
    assert_eq!(server_module.routes().len(), 1);
    assert_eq!(server_module.routes()[0].method(), "CALL");
    assert_eq!(server_module.routes()[0].operation_name(), "ping");
    assert_eq!(server_module.routes()[0].path(), "/nb/ping");
}

#[test]
fn server_introspection_reports_modules_routes_and_layer_rule() {
    let module = Module::from_operations("health", [PING]).expect("module builds");
    let mut server = nb::Server::new();
    server.mount(nb::ServerModule::expose(module, "api"));

    let report = server.introspect();

    assert_eq!(report.module_count, 1);
    assert_eq!(report.operation_count, 1);
    assert_eq!(report.route_count, 1);
    assert_eq!(report.layer_rule, "nb exposes, sb dispatches, bp records");
    assert_eq!(server.routes().count(), 1);
}

#[test]
fn inspects_borrowed_syncbat_core_without_invoking_handlers() {
    let mut builder = Core::builder();
    builder.register(PING, PingHandler).expect("register");
    let core = builder.build().expect("core builds");

    let health = nb::inspect_core_operations(&core, ["ping", "missing"]);

    assert!(!health.is_healthy());
    assert_eq!(health.mounted_operations, vec!["ping"]);
    assert_eq!(health.missing_operations, vec!["missing"]);
    assert_eq!(health.layer_rule, nb::LAYER_RULE);
}
