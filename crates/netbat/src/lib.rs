#![warn(missing_docs)]
//! Thin sync-first server/network boundary exposure layer.
//!
//! `netbat` is intentionally metadata-only: nb exposes, sb dispatches, bp
//! records. This crate can describe server-facing modules, endpoints, and
//! route tables around [`syncbat`] modules or cores, but it does not own
//! dispatch, run handlers, choose runtime decisions, or write batpak records.
//!
//! The crate is designed to be imported as:
//!
//! ```rust
//! use netbat as nb;
//! ```

/// Stable crate-layer rule for docs, diagnostics, and tests.
pub const LAYER_RULE: &str = "nb exposes, sb dispatches, bp records";

/// A syncbat operation exposed at a server/network boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    operation_name: String,
    path: String,
}

impl Endpoint {
    /// Create an endpoint for an operation and boundary path.
    #[must_use]
    pub fn new(operation_name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            operation_name: operation_name.into(),
            path: path.into(),
        }
    }

    /// Stable syncbat operation name exposed by this endpoint.
    #[must_use]
    pub fn operation_name(&self) -> &str {
        &self.operation_name
    }

    /// Boundary path associated with this endpoint.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// A mounted boundary route.
///
/// A route maps boundary metadata to a syncbat operation name. It is not a
/// dispatcher and carries no transport server implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    method: &'static str,
    endpoint: Endpoint,
}

impl Route {
    /// Create a route with a stable method label and endpoint.
    #[must_use]
    pub fn new(method: &'static str, endpoint: Endpoint) -> Self {
        Self { method, endpoint }
    }

    /// Stable method label for the boundary route.
    #[must_use]
    pub fn method(&self) -> &'static str {
        self.method
    }

    /// Endpoint exposed by this route.
    #[must_use]
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Stable syncbat operation name exposed by this route.
    #[must_use]
    pub fn operation_name(&self) -> &str {
        self.endpoint.operation_name()
    }

    /// Boundary path associated with this route.
    #[must_use]
    pub fn path(&self) -> &str {
        self.endpoint.path()
    }
}

/// Server-facing wrapper for a data-oriented syncbat module.
///
/// `ServerModule` owns the syncbat module descriptor so it can be mounted into
/// a [`syncbat::CoreBuilder`] later by the caller. It only derives route
/// metadata from operation descriptors.
pub struct ServerModule {
    module: syncbat::Module,
    routes: Vec<Route>,
}

impl ServerModule {
    /// Wrap a syncbat module and expose each operation under `base_path`.
    ///
    /// Paths are formed as `{base_path}/{operation_name}` with a single slash
    /// between the base and the operation name.
    #[must_use]
    pub fn expose(module: syncbat::Module, base_path: impl AsRef<str>) -> Self {
        let base_path = normalize_base_path(base_path.as_ref());
        let routes = module
            .operations()
            .map(|(name, _)| Route::new("CALL", Endpoint::new(name, format!("{base_path}/{name}"))))
            .collect();

        Self { module, routes }
    }

    /// Wrapped syncbat module descriptor.
    #[must_use]
    pub fn module(&self) -> &syncbat::Module {
        &self.module
    }

    /// Stable module name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.module.name()
    }

    /// Exposed routes derived from the module operation descriptors.
    #[must_use]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Number of exposed operations.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.module.operation_count()
    }

    /// Consume the wrapper and return the syncbat module descriptor.
    #[must_use]
    pub fn into_module(self) -> syncbat::Module {
        self.module
    }
}

/// Minimal server-boundary registry.
///
/// `Server` stores exposed modules and route metadata. It deliberately has no
/// method that invokes a handler or mutates a [`syncbat::Core`].
#[derive(Default)]
pub struct Server {
    modules: Vec<ServerModule>,
}

impl Server {
    /// Create an empty server-boundary registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mount server-facing module metadata.
    pub fn mount(&mut self, module: ServerModule) -> &mut Self {
        self.modules.push(module);
        self
    }

    /// Mounted server-facing modules.
    #[must_use]
    pub fn modules(&self) -> &[ServerModule] {
        &self.modules
    }

    /// Iterate all exposed routes in mount order.
    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.modules.iter().flat_map(|module| module.routes())
    }

    /// Build an introspection report over mounted module metadata.
    #[must_use]
    pub fn introspect(&self) -> Introspection {
        introspect_modules(&self.modules)
    }
}

/// Introspection report for exposed boundary metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Introspection {
    /// Number of exposed modules.
    pub module_count: usize,
    /// Number of exposed operations.
    pub operation_count: usize,
    /// Number of exposed routes.
    pub route_count: usize,
    /// Human-readable layer rule.
    pub layer_rule: &'static str,
}

/// Build an introspection report over server-facing module metadata.
#[must_use]
pub fn introspect_modules(modules: &[ServerModule]) -> Introspection {
    let operation_count = modules
        .iter()
        .map(ServerModule::operation_count)
        .sum::<usize>();
    let route_count = modules
        .iter()
        .map(|module| module.routes().len())
        .sum::<usize>();

    Introspection {
        module_count: modules.len(),
        operation_count,
        route_count,
        layer_rule: LAYER_RULE,
    }
}

/// Borrowed health check over a syncbat core's mounted operation descriptors.
///
/// This report is descriptor-only. It does not invoke handlers or claim
/// transport readiness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreHealth {
    /// Operation names present in the borrowed syncbat core.
    pub mounted_operations: Vec<String>,
    /// Operation names absent from the borrowed syncbat core.
    pub missing_operations: Vec<String>,
    /// Human-readable layer rule.
    pub layer_rule: &'static str,
}

impl CoreHealth {
    /// Return true when every inspected operation name is mounted.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.missing_operations.is_empty()
    }
}

/// Inspect whether named operations are mounted in a borrowed syncbat core.
///
/// This is a boundary health/introspection helper only; syncbat remains the
/// owner of dispatch and batpak remains the owner of durable records.
#[must_use]
pub fn inspect_core_operations<I, S>(core: &syncbat::Core, operation_names: I) -> CoreHealth
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut mounted_operations = Vec::new();
    let mut missing_operations = Vec::new();

    for name in operation_names {
        let name = name.as_ref();
        if core.contains_operation(name) {
            mounted_operations.push(name.to_owned());
        } else {
            missing_operations.push(name.to_owned());
        }
    }

    CoreHealth {
        mounted_operations,
        missing_operations,
        layer_rule: LAYER_RULE,
    }
}

fn normalize_base_path(base_path: &str) -> String {
    let trimmed = base_path.trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}
