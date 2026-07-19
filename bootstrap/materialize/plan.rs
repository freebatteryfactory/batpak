use spec::{architecture, bootstrap_output};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use crate::render::{justfile, package_manifest, package_readme, plane_directory_readme, plane_module_source, source_door, toolchain_manifest, workspace_manifest};
use crate::publish::publish;
use crate::{invalid, COMPILED_SEED_MANIFEST};

pub(crate) fn materialize(
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
pub(crate) fn no_std_capable(id: architecture::PackageId) -> bool {
    architecture::QUALIFICATION_PROFILES.iter().any(|q| {
        q.package == id && q.environment == architecture::QualificationEnvironment::NoStdAlloc
    })
}

/// The named opt-in feature rows a package carries: every admitted
/// qualification profile beyond the semantic (no_std) and native (std)
/// identities. Derived from the profile ledger, never a handwritten list.
pub(crate) fn opt_in_profiles(id: architecture::PackageId) -> Vec<&'static str> {
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

