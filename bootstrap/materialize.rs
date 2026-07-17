#![deny(warnings)]

// The typed specification is a LIBRARY (spec/lib.rs, 5.5E2): this binary
// links it instead of textually mounting modules, which also retires the
// module-resolution trap the old #[path] mounts carried.
//
// Since 5.5E5 the materializer is a ONE-TIME FACTORY with two explicit
// roots: it reads and validates the signed seed, and it publishes exactly
// one isolated Gate-0 workspace candidate at the output root. It never
// writes the seed, a tracked source path, or an implicit current
// directory; the only successful dispositions are Created and Unchanged,
// and a divergent existing output is refused, never repaired in place.
use spec::{architecture, bootstrap_output, toolchain};

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The exact `SPEC.sha256` bytes this materializer was compiled against
/// (5.5E5a). A binary compiled from seed A cannot casually claim seed B:
/// the supplied seed must carry this same manifest before any plan is
/// built. This is not a replacement for `freeze --check`; it binds the
/// binary to the source it was born from.
const COMPILED_SEED_MANIFEST: &str = include_str!("../SPEC.sha256");

fn main() {
    let (seed, output) = match parse_args(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("materialize: FAIL: {error}");
            eprintln!("usage: materialize --seed <signed-seed-root> --output <candidate-output-root>");
            std::process::exit(2);
        }
    };
    match materialize(&seed, &output) {
        Ok((disposition, planned)) => println!(
            "materialize: PASS {disposition:?} {} v{} ({} packages, {} edges, {} qualification profiles; {planned} planned files)",
            architecture::REPOSITORY_NAME,
            architecture::SPEC_VERSION,
            architecture::PACKAGES.len(),
            architecture::EDGES.len(),
            architecture::QUALIFICATION_PROFILES.len(),
        ),
        Err(error) => {
            eprintln!("materialize: FAIL: {error}");
            std::process::exit(1);
        }
    }
}

/// Two explicit roots. There is no default `"."`, no positional form, and no
/// omitted output: an implicit root is exactly how the pre-5.5E5 materializer
/// built a workspace inside the inspector's kitchen.
fn parse_args(args: &[String]) -> io::Result<(PathBuf, PathBuf)> {
    let mut seed: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" if i + 1 < args.len() && seed.is_none() => {
                seed = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--output" if i + 1 < args.len() && output.is_none() => {
                output = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                return Err(invalid(format!("unexpected or repeated argument {other:?}")));
            }
        }
    }
    let seed = seed.ok_or_else(|| invalid("the --seed root is required".into()))?;
    let output = output.ok_or_else(|| invalid("the --output root is required".into()))?;
    for (label, path) in [("seed", &seed), ("output", &output)] {
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(invalid(format!(
                "the {label} root {} contains unresolved parent traversal",
                path.display()
            )));
        }
    }
    if !seed.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("the seed root {} does not exist", seed.display()),
        ));
    }
    let seed_abs = fs::canonicalize(&seed)?;
    let output_parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .ok_or_else(|| invalid("the output parent cannot be resolved".into()))?;
    let output_name = output
        .file_name()
        .ok_or_else(|| invalid("the output root names no directory".into()))?
        .to_owned();
    let output_parent_abs = fs::canonicalize(&output_parent).map_err(|_| {
        invalid(format!(
            "the output parent {} cannot be resolved",
            output_parent.display()
        ))
    })?;
    let output_abs = output_parent_abs.join(&output_name);
    if output_abs == seed_abs {
        return Err(invalid("the seed and output roots are the same path".into()));
    }
    if output_abs.starts_with(&seed_abs) {
        return Err(invalid("the output root is inside the seed root".into()));
    }
    if seed_abs.starts_with(&output_abs) {
        return Err(invalid("the seed root is inside the output root".into()));
    }
    Ok((seed_abs, output_abs))
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn materialize(
    seed: &Path,
    output: &Path,
) -> io::Result<(bootstrap_output::Gate0OutputDisposition, usize)> {
    validate_seed(seed)?;
    let plan = build_plan()?;
    let disposition = publish(output, &plan)?;
    Ok((disposition, plan.len()))
}

fn validate_seed(root: &Path) -> io::Result<()> {
    // The supplied seed must be the exact seed this binary was compiled from.
    let seed_manifest = fs::read_to_string(root.join("SPEC.sha256")).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the seed root carries no SPEC.sha256 manifest",
        )
    })?;
    if seed_manifest != COMPILED_SEED_MANIFEST {
        return Err(invalid(
            "the seed manifest differs from the SPEC.sha256 this materializer was \
             compiled against; a binary cannot claim a seed it was not born from"
                .into(),
        ));
    }
    for required in architecture::REQUIRED_DOCS {
        if !root.join(required).is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("required seed file is missing: {required}"),
            ));
        }
    }
    for forbidden in architecture::FORBIDDEN_TARGET_PATHS {
        if root.join(forbidden).exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("forbidden target path exists: {forbidden}"),
            ));
        }
    }

    let packages: std::collections::BTreeSet<&str> =
        architecture::PACKAGES.iter().map(|p| p.id.cargo_name()).collect();
    let layers: BTreeMap<&str, u8> =
        architecture::PACKAGES.iter().map(|p| (p.id.cargo_name(), p.layer)).collect();
    if packages.len() != architecture::PACKAGES.len() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "duplicate package name"));
    }
    for edge in architecture::EDGES {
        if !packages.contains(edge.importer.cargo_name()) || !packages.contains(edge.importee.cargo_name()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown package in edge {} -> {}", edge.importer.cargo_name(), edge.importee.cargo_name()),
            ));
        }
        if edge.profile.trim().is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty edge profile"));
        }
        if let (Some(importer_layer), Some(importee_layer)) = (layers.get(edge.importer.cargo_name()), layers.get(edge.importee.cargo_name())) {
            if importer_layer <= importee_layer {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("dependency direction violation {}(L{}) -> {}(L{})", edge.importer.cargo_name(), importer_layer, edge.importee.cargo_name(), importee_layer),
                ));
            }
        }
    }
    for profile in architecture::QUALIFICATION_PROFILES {
        if !packages.contains(profile.package.cargo_name())
            || profile.profile.trim().is_empty()
            || profile.requirement.trim().is_empty()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid qualification profile {}:{}", profile.package.cargo_name(), profile.profile),
            ));
        }
    }
    Ok(())
}

// --- Planning ----------------------------------------------------------------

/// The complete expected output, constructed before any filesystem write:
/// normalized POSIX-relative path -> exact bytes. A duplicate or malformed
/// planned path refuses here, before a single directory exists.
fn build_plan() -> io::Result<BTreeMap<String, Vec<u8>>> {
    let mut plan: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    // Case-fold identity: the plan's BTreeMap is case-sensitive, but the
    // authoritative host is not, so two paths differing only in case are a
    // collision on the platform that matters.
    let mut folded: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut admit = |path: String, bytes: String| -> io::Result<()> {
        if !bootstrap_output::is_portable_gate0_relative_path(&path) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("planned path {path:?} is not a portable output-root-relative path"),
            ));
        }
        if !folded.insert(path.to_ascii_lowercase()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("planned path {path:?} case-fold collides with another planned path"),
            ));
        }
        if plan.insert(path.clone(), bytes.into_bytes()).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("planned path {path:?} is planned twice"),
            ));
        }
        Ok(())
    };

    for &artifact in bootstrap_output::Gate0RootArtifact::ALL {
        let bytes = match artifact {
            bootstrap_output::Gate0RootArtifact::WorkspaceManifest => workspace_manifest(),
            bootstrap_output::Gate0RootArtifact::RustToolchain => toolchain_manifest(),
            bootstrap_output::Gate0RootArtifact::Justfile => justfile(),
        };
        admit(artifact.relative_path().to_owned(), bytes)?;
    }

    for package in architecture::PACKAGES {
        let base = package.id.workspace_path();
        for &family in bootstrap_output::Gate0PackageArtifact::ALL {
            let (suffix, bytes) = match family {
                bootstrap_output::Gate0PackageArtifact::Manifest => {
                    ("Cargo.toml".to_owned(), package_manifest(package))
                }
                bootstrap_output::Gate0PackageArtifact::Readme => {
                    ("README.md".to_owned(), package_readme(package))
                }
                bootstrap_output::Gate0PackageArtifact::SourceDoor => (
                    bootstrap_output::source_suffix(package.id).to_owned(),
                    source_door(package),
                ),
            };
            admit(format!("{base}/{suffix}"), bytes)?;
        }
    }

    let syncbat_base = architecture::PackageId::SyncBat.workspace_path();
    for &plane in architecture::SyncBatPlane::ALL {
        let module = plane.module_name();
        for &family in bootstrap_output::Gate0PlaneArtifact::ALL {
            let (path, bytes) = match family {
                bootstrap_output::Gate0PlaneArtifact::Module => (
                    format!("{syncbat_base}/src/{module}.rs"),
                    plane_module_source(plane),
                ),
                bootstrap_output::Gate0PlaneArtifact::DirectoryReadme => (
                    format!("{syncbat_base}/src/{module}/README.md"),
                    plane_directory_readme(plane),
                ),
            };
            admit(path, bytes)?;
        }
    }
    Ok(plan)
}

// --- Rendering ---------------------------------------------------------------

/// Packages that carry a `std` feature at all: exactly those admitted for a
/// no_std + alloc semantic qualification. Derived, never listed by name.
fn no_std_capable(id: architecture::PackageId) -> bool {
    architecture::QUALIFICATION_PROFILES.iter().any(|q| {
        q.package == id && q.environment == architecture::QualificationEnvironment::NoStdAlloc
    })
}

/// The named opt-in feature rows a package carries: every admitted
/// qualification profile beyond the semantic (no_std) and native (std)
/// identities. Derived from the profile ledger, never a handwritten list.
fn opt_in_profiles(id: architecture::PackageId) -> Vec<&'static str> {
    architecture::QUALIFICATION_PROFILES
        .iter()
        .filter(|q| {
            q.package == id
                && q.environment != architecture::QualificationEnvironment::NoStdAlloc
                && q.environment != architecture::QualificationEnvironment::NativeStd
        })
        .map(|q| q.profile)
        .collect()
}

fn workspace_manifest() -> String {
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

fn package_manifest(package: &architecture::PackageSpec) -> String {
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

fn package_readme(package: &architecture::PackageSpec) -> String {
    format!(
        "# {}\n\nGate-0 package skeleton.\n\n**Authority:** {}\n\nThe normative owner and dependency facts live in `spec/architecture.rs`. This file does not widen that role.\n",
        package.id.cargo_name(), package.role
    )
}

fn source_door(package: &architecture::PackageSpec) -> String {
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

fn plane_module_source(plane: architecture::SyncBatPlane) -> String {
    // The description is the plane's authored docs/08 ownership sentence: a
    // materializer must not own a second description of what a plane means.
    let name = plane.module_name();
    let ownership = plane.authored_ownership();
    format!(
        "//! SyncBat `{name}` plane: {ownership}.\n\n/// Gate-0 marker. Semantic types and transitions land only at their signed gate.\npub const PLANE_ID: &str = \"{name}\";\n"
    )
}

fn plane_directory_readme(plane: architecture::SyncBatPlane) -> String {
    let name = plane.module_name();
    let ownership = plane.authored_ownership();
    format!(
        "# `{name}` plane\n\nOwner: {ownership}.\n\nThe root `{name}.rs` file owns the public module and primary type spine. This directory holds same-concept implementation files when the owning gate lands.\n"
    )
}

fn toolchain_manifest() -> String {
    // The workspace toolchain selection is the SAME deterministic projection
    // the tracked root file carries: one owner, two consumers. Since 5.5E4a
    // the tracked bytes open with the registered provenance line
    // (spec/generated_views.rs RustToolchain), so both consumers project the
    // TRACKED form.
    toolchain::TOOLCHAIN.tracked_root_toolchain_toml()
}

fn justfile() -> String {
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

/// Publish the plan at the output root. Exactly two successes exist: Created
/// (the output was absent and a complete staged tree was renamed into place)
/// and Unchanged (the output already carried exactly the planned tree and
/// zero writes happened). Everything else refuses without touching the final
/// path.
fn publish(
    output: &Path,
    plan: &BTreeMap<String, Vec<u8>>,
) -> io::Result<bootstrap_output::Gate0OutputDisposition> {
    if output.symlink_metadata().is_ok() {
        verify_exact(output, plan)?;
        return Ok(bootstrap_output::Gate0OutputDisposition::Unchanged);
    }

    // Build the complete tree in a fresh sibling staging directory, verify
    // it, and rename the whole directory to the output path. A failure at any
    // point removes the staging tree and leaves the final path absent.
    let staging = sibling_staging_path(output)?;
    let publish_result = (|| -> io::Result<()> {
        fs::create_dir(&staging)?;
        for (relative, bytes) in plan {
            let target = staging.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, bytes)?;
        }
        verify_exact(&staging, plan)?;
        fs::rename(&staging, output)?;
        Ok(())
    })();
    if publish_result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    publish_result?;
    Ok(bootstrap_output::Gate0OutputDisposition::Created)
}

fn sibling_staging_path(output: &Path) -> io::Result<PathBuf> {
    let name = output
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("the output root names no directory".into()))?;
    let staging = output.with_file_name(format!(".{name}.g0-staging"));
    if staging.symlink_metadata().is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("the staging path {} already exists", staging.display()),
        ));
    }
    Ok(staging)
}

/// The root must contain exactly the planned tree: every planned file with
/// exact bytes, no extra file, no directory beyond those the planned paths
/// imply, and no symlink anywhere. A divergent tree is described, never
/// repaired.
fn verify_exact(root: &Path, plan: &BTreeMap<String, Vec<u8>>) -> io::Result<()> {
    let meta = root.symlink_metadata()?;
    if meta.is_symlink() {
        return Err(invalid(format!("the output root {} is a symlink", root.display())));
    }
    if !meta.is_dir() {
        return Err(invalid(format!("the output root {} is not a directory", root.display())));
    }

    let mut implied_dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for relative in plan.keys() {
        let mut prefix = String::new();
        for part in relative.split('/').collect::<Vec<_>>().split_last().map(|(_, init)| init).unwrap_or(&[]) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            implied_dirs.insert(prefix.clone());
        }
    }

    let mut found: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = path.symlink_metadata()?;
            let relative = path
                .strip_prefix(root)
                .map_err(|_| invalid("walked outside the output root".into()))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if meta.is_symlink() {
                return Err(invalid(format!("the output carries a symlink at {relative}")));
            }
            if meta.is_dir() {
                if plan.contains_key(&relative) {
                    return Err(invalid(format!(
                        "a directory sits where the planned file {relative} belongs"
                    )));
                }
                if !implied_dirs.contains(&relative) {
                    return Err(invalid(format!(
                        "the output carries an extra directory {relative} the plan does not imply"
                    )));
                }
                stack.push(path);
            } else {
                found.insert(relative, path);
            }
        }
    }

    for (relative, path) in &found {
        match plan.get(relative) {
            None => {
                return Err(invalid(format!("the output carries an extra file {relative}")));
            }
            Some(expected) => {
                if &fs::read(path)? != expected {
                    return Err(invalid(format!(
                        "the output file {relative} does not match its planned bytes"
                    )));
                }
            }
        }
    }
    for relative in plan.keys() {
        if !found.contains_key(relative) {
            return Err(invalid(format!("the output is missing the planned file {relative}")));
        }
    }
    Ok(())
}
