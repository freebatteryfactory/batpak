use spec::{architecture, bootstrap_output, toolchain};
use std::fmt::Write as _;
use crate::plan::{no_std_capable, opt_in_profiles};

pub(crate) fn workspace_manifest() -> String {
    let mut out = String::new();
    out.push_str("# Generated from spec/bootstrap_output.rs, spec/architecture.rs, and spec/toolchain.rs by bootstrap/materialize.rs.\n");
    // The resolver, edition, and MSRV floor come from the typed toolchain
    // owner (5.5E3a). A literal here would be a second authority the audit
    // scans for and refuses.
    let _ = writeln!(
        out,
        "[workspace]\nresolver = \"{}\"\nmembers = [",
        toolchain::TOOLCHAIN.cargo_resolver.spelling()
    );
    for package in architecture::PACKAGES {
        let _ = writeln!(out, "  \"{}\",", package.id.workspace_path());
    }
    out.push_str("]\n\n[workspace.package]\n");
    let _ = writeln!(out, "version = \"{}\"", architecture::WORKSPACE_VERSION);
    let _ = writeln!(
        out,
        "edition = \"{}\"\nrust-version = \"{}\"",
        toolchain::TOOLCHAIN.edition.spelling(),
        toolchain::TOOLCHAIN.rust_version_floor.render()
    );
    let _ = writeln!(
        out,
        "license = \"{}\"\nrepository = \"{}\"",
        bootstrap_output::WORKSPACE_LICENSE,
        bootstrap_output::WORKSPACE_REPOSITORY
    );
    out.push_str("\n[workspace.dependencies]\n");
    for package in architecture::PACKAGES {
        // A no_std-capable package is declared default-features = false at
        // the workspace level: Cargo lets a member re-enable defaults but
        // never disable them, and the no_std qualification depends on the
        // disable being expressible on the syncbat -> batpak edge.
        let suffix = if no_std_capable(package.id) { ", default-features = false" } else { "" };
        let _ = writeln!(
            out,
            "{} = {{ path = \"{}\"{suffix} }}",
            package.id.cargo_name(),
            package.id.workspace_path()
        );
    }
    out.push_str("\n[workspace.lints.rust]\nunsafe_op_in_unsafe_fn = \"deny\"\nunused_must_use = \"deny\"\n\n");
    // expect_used is deliberately NOT globally denied: TestPak tests use
    // contextual .expect(). There is no blanket `as`-cast ban either; pointer
    // and layout casts belong to the compiler-assumption ledger and the future
    // TestPak AST gate (DEC-067).
    out.push_str("[workspace.lints.clippy]\ndbg_macro = \"deny\"\ntodo = \"deny\"\nunimplemented = \"deny\"\nunwrap_used = \"deny\"\npanic = \"deny\"\nprint_stdout = \"deny\"\nprint_stderr = \"deny\"\n");
    out.push_str("wildcard_enum_match_arm = \"deny\"\ncast_possible_truncation = \"deny\"\ncast_sign_loss = \"deny\"\nmissing_errors_doc = \"deny\"\nlarge_enum_variant = \"deny\"\nclone_on_ref_ptr = \"deny\"\nneedless_pass_by_value = \"deny\"\n");
    out
}

pub(crate) fn package_manifest(package: &architecture::PackageSpec) -> String {
    let mut out = String::new();
    out.push_str("# Gate-0 skeleton generated from the signed architecture seed.\n");
    out.push_str("[package]\n");
    let _ = writeln!(out, "name = \"{}\"", package.id.cargo_name());
    out.push_str("version.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n");
    if package.class == architecture::PackageClass::DevOnly
        || package.class == architecture::PackageClass::Example
    {
        out.push_str("publish = false\n");
    }

    match bootstrap_output::target_kind(package.id) {
        bootstrap_output::Gate0PackageTargetKind::ProcMacroLibrary => {
            out.push_str("\n[lib]\nproc-macro = true\n");
        }
        bootstrap_output::Gate0PackageTargetKind::Binary => {
            if let Some(name) = bootstrap_output::binary_target(package.id) {
                let _ = writeln!(
                    out,
                    "\n[[bin]]\nname = \"{name}\"\npath = \"{}\"",
                    bootstrap_output::source_suffix(package.id)
                );
            }
        }
        bootstrap_output::Gate0PackageTargetKind::Library
        | bootstrap_output::Gate0PackageTargetKind::ExampleBinary => {}
    }

    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut dev = Vec::new();
    for edge in architecture::EDGES.iter().filter(|edge| edge.importer == package.id) {
        match edge.class {
            architecture::EdgeClass::Required => required.push(edge),
            architecture::EdgeClass::OptionalProfile => optional.push(edge),
            architecture::EdgeClass::DevOnly => dev.push(edge),
        }
    }

    if no_std_capable(package.id) {
        // Default profile is usable native std (DEC-047). no_std consumers opt
        // out with default-features = false. std must not become a junk drawer:
        // the wasm-host adapters stay behind their own explicit opt-in
        // profiles, derived from the qualification ledger, and a required edge
        // between two no_std-capable packages forwards std explicitly so
        // --no-default-features cannot be reactivated by an internal edge.
        out.push_str("\n[features]\ndefault = [\"std\"]\n");
        let forwards: Vec<String> = required
            .iter()
            .filter(|edge| no_std_capable(edge.importee))
            .map(|edge| format!("\"{}/std\"", edge.importee.cargo_name()))
            .collect();
        let _ = writeln!(out, "std = [{}]", forwards.join(", "));
        for profile in opt_in_profiles(package.id) {
            let _ = writeln!(out, "{profile} = []");
        }
    } else if !optional.is_empty() {
        out.push_str("\n[features]\ndefault = []\n");
        for edge in &optional {
            let _ = writeln!(out, "{} = [\"dep:{}\"]", edge.profile, edge.importee.cargo_name());
        }
    }

    if !required.is_empty() || !optional.is_empty() || (package.class == architecture::PackageClass::DevOnly && !dev.is_empty()) {
        out.push_str("\n[dependencies]\n");
        // The workspace declares no_std-capable dependencies with defaults
        // off; a std consumer re-enables them explicitly, and a no_std
        // importer inherits the off state so --no-default-features cannot be
        // reactivated through an internal edge.
        let dep_suffix = |importee: architecture::PackageId| {
            if no_std_capable(importee) && !no_std_capable(package.id) {
                ", default-features = true"
            } else {
                ""
            }
        };
        for edge in required {
            let _ = writeln!(
                out,
                "{} = {{ workspace = true{} }}",
                edge.importee.cargo_name(),
                dep_suffix(edge.importee)
            );
        }
        for edge in optional {
            let _ = writeln!(
                out,
                "{} = {{ workspace = true, optional = true{} }}",
                edge.importee.cargo_name(),
                dep_suffix(edge.importee)
            );
        }
        if package.class == architecture::PackageClass::DevOnly {
            for edge in &dev {
                let _ = writeln!(
                    out,
                    "{} = {{ workspace = true{} }}",
                    edge.importee.cargo_name(),
                    dep_suffix(edge.importee)
                );
            }
        }
    }
    if package.class != architecture::PackageClass::DevOnly && !dev.is_empty() {
        out.push_str("\n[dev-dependencies]\n");
        for edge in dev {
            let _ = writeln!(out, "{} = {{ workspace = true }}", edge.importee.cargo_name());
        }
    }
    out.push_str("\n[lints]\nworkspace = true\n");
    out
}

pub(crate) fn package_readme(package: &architecture::PackageSpec) -> String {
    format!(
        "# {}\n\nGate-0 package skeleton.\n\n**Authority:** {}\n\nThe normative owner and dependency facts live in `spec/architecture.rs`. This file does not widen that role.\n",
        package.id.cargo_name(), package.role
    )
}

pub(crate) fn source_door(package: &architecture::PackageSpec) -> String {
    match bootstrap_output::target_kind(package.id) {
        bootstrap_output::Gate0PackageTargetKind::Library => library_source(package),
        bootstrap_output::Gate0PackageTargetKind::ProcMacroLibrary => proc_macro_source(package),
        bootstrap_output::Gate0PackageTargetKind::Binary => binary_source(package),
        bootstrap_output::Gate0PackageTargetKind::ExampleBinary => example_source(package),
    }
}

fn library_source(package: &architecture::PackageSpec) -> String {
    let crate_name = package.id.cargo_name().replace('-', "_");
    if package.id == architecture::PackageId::BatPak {
        return format!(
            "#![cfg_attr(not(feature = \"std\"), no_std)]\n#![deny(missing_docs)]\n//! Semantic and durable BatPak core.\n\nextern crate alloc;\n\n/// Gate-0 marker for the `{crate_name}` package skeleton.\npub const PACKAGE_ID: &str = \"{}\";\n",
            package.id.cargo_name()
        );
    }
    if package.id == architecture::PackageId::SyncBat {
        // The module list derives from the typed plane inventory; a
        // handwritten list here would be a second plane authority.
        let mut modules: Vec<&str> = architecture::SyncBatPlane::ALL
            .iter()
            .map(|plane| plane.module_name())
            .collect();
        modules.sort_unstable();
        let mut out = String::from(
            "#![cfg_attr(not(feature = \"std\"), no_std)]\n#![deny(missing_docs)]\n//! SyncBat runtime, PakVM, Bvisor, world, and port planes.\n\nextern crate alloc;\n\n",
        );
        for module in modules {
            let _ = writeln!(out, "pub mod {module};");
        }
        return out;
    }
    format!(
        "#![deny(missing_docs)]\n//! Gate-0 package skeleton for `{}`.\n\n/// Stable package identity used only by the bootstrap skeleton.\npub const PACKAGE_ID: &str = \"{}\";\n",
        package.id.cargo_name(), package.id.cargo_name()
    )
}

fn proc_macro_source(package: &architecture::PackageSpec) -> String {
    // A proc-macro crate exports no ordinary public constant: its Gate-0
    // front door is empty and documented, and the real macros land only at
    // their signed gate.
    format!(
        "#![deny(missing_docs)]\n//! Proc-macro front door for `{}`.\n//!\n//! Gate-0 publishes no macros: the contract compiler drives this crate and\n//! every exported macro lands at its signed gate. A proc-macro crate cannot\n//! export ordinary items, so this file is intentionally empty.\n",
        package.id.cargo_name()
    )
}

fn binary_source(package: &architecture::PackageSpec) -> String {
    format!(
        "//! Thin product command adapter for `{}`.\n\nfn main() {{}}\n",
        package.id.cargo_name()
    )
}

fn example_source(package: &architecture::PackageSpec) -> String {
    // Gate-0 placeholder. Real family demos replace it; every example must emit
    // observable output and depend only on public production APIs.
    format!(
        "//! Gate-0 placeholder example for `{name}`.\n\nfn main() {{\n    println!(\"{name}: gate-0 placeholder\");\n}}\n",
        name = package.id.cargo_name()
    )
}

pub(crate) fn plane_module_source(plane: architecture::SyncBatPlane) -> String {
    // The description is the plane's authored docs/08 ownership sentence: a
    // materializer must not own a second description of what a plane means.
    let name = plane.module_name();
    let ownership = plane.authored_ownership();
    format!(
        "//! SyncBat `{name}` plane: {ownership}.\n\n/// Gate-0 marker. Semantic types and transitions land only at their signed gate.\npub const PLANE_ID: &str = \"{name}\";\n"
    )
}

pub(crate) fn plane_directory_readme(plane: architecture::SyncBatPlane) -> String {
    let name = plane.module_name();
    let ownership = plane.authored_ownership();
    format!(
        "# `{name}` plane\n\nOwner: {ownership}.\n\nThe root `{name}.rs` file owns the public module and primary type spine. This directory holds same-concept implementation files when the owning gate lands.\n"
    )
}

pub(crate) fn toolchain_manifest() -> String {
    // The workspace toolchain selection is the SAME deterministic projection
    // the tracked root file carries: one owner, two consumers. Since 5.5E4a
    // the tracked bytes open with the registered provenance line
    // (spec/generated_views.rs RustToolchain), so both consumers project the
    // TRACKED form.
    toolchain::TOOLCHAIN.tracked_root_toolchain_toml()
}

pub(crate) fn justfile() -> String {
    // Only commands that WORK inside the isolated candidate. Seed auditing and
    // freezing are the signed seed repository's commands, documented in
    // bootstrap/README.md; a candidate must not advertise doors painted onto a
    // wall (5.5E5a). The no_std and smoke targets derive from the typed owners.
    let mut out = String::new();
    out.push_str("# Discoverability only. Typed command authority moves to TestPak.\n\n");
    out.push_str("check:\n    cargo check --workspace --all-targets\n\ncheck-no-std:\n");
    for package in architecture::PACKAGES {
        if no_std_capable(package.id) {
            let _ = writeln!(
                out,
                "    cargo check -p {} --no-default-features",
                package.id.cargo_name()
            );
        }
    }
    for package in architecture::PACKAGES {
        if bootstrap_output::target_kind(package.id)
            == bootstrap_output::Gate0PackageTargetKind::ExampleBinary
        {
            if let Some(bin) = bootstrap_output::binary_target(package.id) {
                let _ = write!(
                    out,
                    "\nsmoke:\n    cargo run -p {} --bin {}\n",
                    package.id.cargo_name(),
                    bin
                );
            }
        }
    }
    out
}

// --- Publication -------------------------------------------------------------

