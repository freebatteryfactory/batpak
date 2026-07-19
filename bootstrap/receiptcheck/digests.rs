use spec::bootstrap_qualification::{Sha256Digest, Tier0ArtifactEvidence, Tier0ReceiptKind};
use std::fs;
use std::path::{Path, PathBuf};
use crate::artifact::{ArtifactDoc, ParsedSource};
use crate::bundle::{probe_python, probe_tool, validate_fixture_manifest};
use crate::hashing::{sha256, tree_digest};
use crate::modes::VerifyPaths;

pub(crate) fn recompute_and_compare(doc: &ArtifactDoc, paths: &VerifyPaths) -> Result<(), String> {
    // Source-block digests.
    match &doc.source {
        ParsedSource::GitCheckout {
            spec_manifest_digest,
            workflow_path,
            workflow_digest,
            ..
        } => {
            compare_file_digest(
                &paths.root.join("SPEC.sha256"),
                *spec_manifest_digest,
                "spec-manifest-digest",
            )?;
            compare_file_digest(
                &paths.root.join(workflow_path),
                *workflow_digest,
                "workflow-digest",
            )?;
        }
        ParsedSource::FrozenExport {
            spec_manifest_digest,
            export_tree_digest,
        } => {
            compare_file_digest(
                &paths.root.join("SPEC.sha256"),
                *spec_manifest_digest,
                "spec-manifest-digest",
            )?;
            let actual = tree_digest(&paths.root)?;
            if actual != *export_tree_digest {
                return Err(digest_mismatch("export-tree-digest", *export_tree_digest, actual));
            }
        }
    }

    // Toolchain digests and identities.
    compare_file_digest(
        &paths.root.join("rust-toolchain.toml"),
        doc.toolchain.toolchain_file_digest,
        "toolchain-file-digest",
    )?;
    let (rustc_release, rustc_commit) = probe_tool("rustc", &["-vV"])?;
    if rustc_release != doc.toolchain.rustc_release {
        return Err(format!(
            "rustc-release mismatch: artifact {}, actual {}",
            doc.toolchain.rustc_release.render(),
            rustc_release.render()
        ));
    }
    if rustc_commit != doc.toolchain.rustc_commit {
        return Err(format!(
            "rustc-commit mismatch: artifact {}, actual {}",
            doc.toolchain.rustc_commit.render(),
            rustc_commit.render()
        ));
    }
    let (cargo_release, cargo_commit) = probe_tool("cargo", &["-Vv"])?;
    if cargo_release != doc.toolchain.cargo_release {
        return Err(format!(
            "cargo-release mismatch: artifact {}, actual {}",
            doc.toolchain.cargo_release.render(),
            cargo_release.render()
        ));
    }
    if cargo_commit != doc.toolchain.cargo_commit {
        return Err(format!(
            "cargo-commit mismatch: artifact {}, actual {}",
            doc.toolchain.cargo_commit.render(),
            cargo_commit.render()
        ));
    }

    // Bootstrap runtime: probe the EXACT interpreter that ran the Python gates
    // (never a search) and require CPython at the artifact's claimed release.
    let (python_impl, python_release) = probe_python(&paths.python_executable)?;
    if python_impl != "cpython" {
        return Err(format!(
            "python-implementation mismatch: artifact requires cpython, actual {python_impl:?}"
        ));
    }
    if python_release != doc.python_release {
        return Err(format!(
            "python-release mismatch: artifact {}, actual {}",
            doc.python_release.render(),
            python_release.render()
        ));
    }

    // Per-receipt artifact evidence.
    for r in &doc.receipts {
        match r.evidence {
            Tier0ArtifactEvidence::FixtureSet { digest } => {
                let manifest = paths.evidence.join("law-fixtures.manifest");
                validate_fixture_manifest(&manifest)?;
                compare_file_digest(&manifest, digest, "fixture-set digest")?;
            }
            Tier0ArtifactEvidence::Executable { digest } => {
                compare_file_digest(
                    &executable_path(&paths.evidence, r.kind),
                    digest,
                    "executable digest",
                )?;
            }
            Tier0ArtifactEvidence::ExecutableAndOutputTree {
                executable_digest,
                output_tree_digest,
            } => {
                compare_file_digest(
                    &executable_path(&paths.evidence, r.kind),
                    executable_digest,
                    "executable digest",
                )?;
                let actual = tree_digest(&paths.evidence.join("gate0-candidate"))?;
                if actual != output_tree_digest {
                    return Err(digest_mismatch(
                        "materializer output-tree digest",
                        output_tree_digest,
                        actual,
                    ));
                }
            }
        }
    }

    Ok(())
}

fn executable_path(evidence: &Path, kind: Tier0ReceiptKind) -> PathBuf {
    let base = evidence.join("executables").join(kind.slug());
    // The bundle carries a bare name on unix and a `.exe` on Windows; prefer
    // whichever exists so the verifier is host-agnostic.
    let exe = base.with_extension("exe");
    if exe.is_file() { exe } else { base }
}

fn compare_file_digest(path: &Path, claimed: Sha256Digest, label: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let actual = Sha256Digest::from_bytes(sha256(&bytes));
    if actual != claimed {
        return Err(digest_mismatch(label, claimed, actual));
    }
    Ok(())
}

fn digest_mismatch(label: &str, claimed: Sha256Digest, actual: Sha256Digest) -> String {
    format!(
        "{label} mismatch: artifact claims {}, actual {}",
        claimed.render(),
        actual.render()
    )
}

