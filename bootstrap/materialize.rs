#![deny(warnings)]

#[path = "../spec/architecture.rs"]
mod architecture;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    let root = env::args().nth(1).map_or_else(|| PathBuf::from("."), PathBuf::from);
    if let Err(error) = materialize(&root) {
        eprintln!("materialize: FAIL: {error}");
        std::process::exit(1);
    }
    println!(
        "materialize: PASS {} v{} ({} packages, {} edges, {} qualification profiles)",
        architecture::REPOSITORY_NAME,
        architecture::SPEC_VERSION,
        architecture::PACKAGES.len(),
        architecture::EDGES.len(),
        architecture::QUALIFICATION_PROFILES.len(),
    );
}

fn materialize(root: &Path) -> io::Result<()> {
    validate_seed(root)?;
    write_checked(root.join("Cargo.toml"), &workspace_manifest())?;
    write_checked(root.join("rust-toolchain.toml"), toolchain_manifest())?;
    write_checked(root.join("justfile"), justfile())?;

    for package in architecture::PACKAGES {
        let package_root = root.join(package.path);
        fs::create_dir_all(package_root.join("src"))?;
        write_checked(package_root.join("Cargo.toml"), &package_manifest(package))?;
        write_checked(package_root.join("README.md"), &package_readme(package))?;
        if package.class == architecture::PackageClass::BinaryAdapter {
            write_checked(package_root.join("src/main.rs"), &binary_source(package))?;
        } else {
            write_checked(package_root.join("src/lib.rs"), &library_source(package))?;
        }
    }

    materialize_syncbat_planes(root)?;
    Ok(())
}

fn validate_seed(root: &Path) -> io::Result<()> {
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

    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.package).collect();
    let layers: BTreeMap<&str, u8> = architecture::PACKAGES.iter().map(|p| (p.package, p.layer)).collect();
    if packages.len() != architecture::PACKAGES.len() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "duplicate package name"));
    }
    for edge in architecture::EDGES {
        if !packages.contains(edge.importer) || !packages.contains(edge.importee) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown package in edge {} -> {}", edge.importer, edge.importee),
            ));
        }
        if edge.profile.trim().is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty edge profile"));
        }
        if let (Some(importer_layer), Some(importee_layer)) = (layers.get(edge.importer), layers.get(edge.importee)) {
            if importer_layer <= importee_layer {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("dependency direction violation {}(L{}) -> {}(L{})", edge.importer, importer_layer, edge.importee, importee_layer),
                ));
            }
        }
    }
    for profile in architecture::QUALIFICATION_PROFILES {
        if !packages.contains(profile.package)
            || profile.profile.trim().is_empty()
            || profile.target.trim().is_empty()
            || profile.requirement.trim().is_empty()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid qualification profile {}:{}", profile.package, profile.profile),
            ));
        }
    }
    Ok(())
}

fn workspace_manifest() -> String {
    let mut out = String::new();
    out.push_str("# Generated once from spec/architecture.rs by bootstrap/materialize.rs.\n");
    out.push_str("[workspace]\nresolver = \"3\"\nmembers = [\n");
    for package in architecture::PACKAGES {
        let _ = writeln!(out, "  \"{}\",", package.path);
    }
    out.push_str("]\n\n[workspace.package]\n");
    out.push_str("version = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"1.97\"\n");
    out.push_str("license = \"MIT OR Apache-2.0\"\nrepository = \"https://github.com/freebatteryfactory/batpak\"\n\n");
    out.push_str("[workspace.dependencies]\n");
    for package in architecture::PACKAGES {
        let _ = writeln!(out, "{} = {{ path = \"{}\" }}", package.package, package.path);
    }
    out.push_str("\n[workspace.lints.rust]\nunsafe_op_in_unsafe_fn = \"deny\"\nunused_must_use = \"deny\"\n\n");
    out.push_str("[workspace.lints.clippy]\ndbg_macro = \"deny\"\ntodo = \"deny\"\nunimplemented = \"deny\"\nunwrap_used = \"deny\"\npanic = \"deny\"\nprint_stdout = \"deny\"\nprint_stderr = \"deny\"\n");
    out
}

fn package_manifest(package: &architecture::PackageSpec) -> String {
    let mut out = String::new();
    out.push_str("# Gate-0 skeleton generated from the signed architecture seed.\n");
    out.push_str("[package]\n");
    let _ = writeln!(out, "name = \"{}\"", package.package);
    out.push_str("version.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n");
    if package.class == architecture::PackageClass::DevOnly {
        out.push_str("publish = false\n");
    }

    if package.package == "macbat" {
        out.push_str("\n[lib]\nproc-macro = true\n");
    } else if package.class == architecture::PackageClass::BinaryAdapter {
        out.push_str("\n[[bin]]\nname = \"batpak\"\npath = \"src/main.rs\"\n");
    }

    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut dev = Vec::new();
    for edge in architecture::EDGES.iter().filter(|edge| edge.importer == package.package) {
        match edge.class {
            architecture::EdgeClass::Required => required.push(edge),
            architecture::EdgeClass::OptionalProfile => optional.push(edge),
            architecture::EdgeClass::DevOnly => dev.push(edge),
        }
    }

    if package.package == "batpak" || package.package == "syncbat" {
        out.push_str("\n[features]\ndefault = []\nstd = []\n");
        if package.package == "syncbat" {
            out.push_str("browser = []\n");
        }
    } else if !optional.is_empty() {
        out.push_str("\n[features]\ndefault = []\n");
        for edge in &optional {
            let _ = writeln!(out, "{} = [\"dep:{}\"]", edge.profile, edge.importee);
        }
    }

    if !required.is_empty() || !optional.is_empty() || (package.class == architecture::PackageClass::DevOnly && !dev.is_empty()) {
        out.push_str("\n[dependencies]\n");
        for edge in required {
            let _ = writeln!(out, "{} = {{ workspace = true }}", edge.importee);
        }
        for edge in optional {
            let _ = writeln!(out, "{} = {{ workspace = true, optional = true }}", edge.importee);
        }
        if package.class == architecture::PackageClass::DevOnly {
            for edge in &dev {
                let _ = writeln!(out, "{} = {{ workspace = true }}", edge.importee);
            }
        }
    }
    if package.class != architecture::PackageClass::DevOnly && !dev.is_empty() {
        out.push_str("\n[dev-dependencies]\n");
        for edge in dev {
            let _ = writeln!(out, "{} = {{ workspace = true }}", edge.importee);
        }
    }
    out.push_str("\n[lints]\nworkspace = true\n");
    out
}

fn package_readme(package: &architecture::PackageSpec) -> String {
    format!(
        "# {}\n\nGate-0 package skeleton.\n\n**Authority:** {}\n\nThe normative owner and dependency facts live in `spec/architecture.rs`. This file does not widen that role.\n",
        package.package, package.role
    )
}

fn library_source(package: &architecture::PackageSpec) -> String {
    let crate_name = package.package.replace('-', "_");
    if package.package == "batpak" {
        return format!(
            "#![cfg_attr(not(feature = \"std\"), no_std)]\n#![deny(missing_docs)]\n//! Semantic and durable BatPak core.\n\nextern crate alloc;\n\n/// Gate-0 marker for the `{crate_name}` package skeleton.\npub const PACKAGE_ID: &str = \"{}\";\n",
            package.package
        );
    }
    if package.package == "syncbat" {
        return "#![cfg_attr(not(feature = \"std\"), no_std)]\n#![deny(missing_docs)]\n//! SyncBat runtime, PakVM, Bvisor, world, and port planes.\n\nextern crate alloc;\n\npub mod bvisor;\npub mod pakvm;\npub mod port;\npub mod runtime;\npub mod world;\n".to_owned();
    }
    format!(
        "#![deny(missing_docs)]\n//! Gate-0 package skeleton for `{}`.\n\n/// Stable package identity used only by the bootstrap skeleton.\npub const PACKAGE_ID: &str = \"{}\";\n",
        package.package, package.package
    )
}

fn binary_source(package: &architecture::PackageSpec) -> String {
    format!(
        "//! Thin product command adapter for `{}`.\n\nfn main() {{}}\n",
        package.package
    )
}

fn materialize_syncbat_planes(root: &Path) -> io::Result<()> {
    let base = root.join("crates/syncbat");
    let plane_roles: BTreeMap<&str, &str> = BTreeMap::from([
        ("runtime", "logical process, turn, checkpoint, delivery, and restart legality"),
        ("pakvm", "typed ProgramImage and WorldImage validation and interpretation"),
        ("bvisor", "capability, budget, AttemptId, supervision, and reconciliation"),
        ("world", "linking, instances, generations, interfaces, and entrypoints"),
        ("port", "typed host requests and responses"),
    ]);
    for relative in architecture::SYNCBAT_REQUIRED_PLANES {
        let path = base.join(relative);
        if relative.ends_with(".rs") {
            let name = relative.trim_start_matches("src/").trim_end_matches(".rs");
            let role = plane_roles.get(name).copied().unwrap_or("declared SyncBat plane");
            write_checked(
                path,
                &format!(
                    "//! SyncBat `{name}` plane: {role}.\n\n/// Gate-0 marker. Semantic types and transitions land only at their signed gate.\npub const PLANE_ID: &str = \"{name}\";\n"
                ),
            )?;
        } else {
            fs::create_dir_all(&path)?;
            let name = relative.trim_start_matches("src/");
            let role = plane_roles.get(name).copied().unwrap_or("declared SyncBat plane");
            write_checked(
                path.join("README.md"),
                &format!(
                    "# `{name}` plane\n\nOwner: {role}.\n\nThe root `{name}.rs` file owns the public module and primary type spine. This directory holds same-concept implementation files when the owning gate lands.\n"
                ),
            )?;
        }
    }
    Ok(())
}

fn toolchain_manifest() -> &'static str {
    "[toolchain]\nchannel = \"1.97.0\"\nprofile = \"minimal\"\ncomponents = [\"clippy\", \"rustfmt\"]\n"
}

fn justfile() -> &'static str {
    "# Discoverability only. Typed command authority moves to TestPak.\n\naudit:\n    python3 bootstrap/audit.py .\n\nfreeze-check:\n    python3 bootstrap/freeze.py . --check\n\nseedcheck:\n    mkdir -p target\n    rustc bootstrap/seedcheck.rs -o target/seedcheck\n    ./target/seedcheck .\n\ncheck:\n    cargo check --workspace --all-targets\n"
}

fn write_checked(path: PathBuf, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::read_to_string(&path) {
        Ok(existing) if existing == content => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("refusing to overwrite nonmatching file {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::write(path, content),
        Err(error) => Err(error),
    }
}
