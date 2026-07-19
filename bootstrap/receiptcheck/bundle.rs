use spec::bootstrap_qualification::{self as bq, PythonRelease, Tier0ReceiptKind, ToolchainCommit};
use spec::toolchain::RustRelease;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use crate::artifact::{parse_release, ArtifactDoc, ParsedSource};

/// A fixture manifest must be non-empty, carry six pipe-separated fields per
/// row, and never repeat a fixture identity. A row that is not terminal (wrong
/// field count) is a started-without-completion failure.
pub(crate) fn validate_fixture_manifest(path: &Path) -> Result<(), String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read fixture manifest: {e}"))?;
    let rows: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    if rows.is_empty() {
        return Err("fixture manifest is empty".to_owned());
    }
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for row in rows {
        let fields: Vec<&str> = row.split('|').map(str::trim).collect();
        if fields.len() != 6 {
            return Err(format!(
                "fixture manifest row is not a terminal six-field row: {row:?}"
            ));
        }
        let identity = fields[0].to_owned();
        if seen.insert(identity.clone(), ()).is_some() {
            return Err(format!("duplicate fixture identity {identity:?}"));
        }
    }
    Ok(())
}

/// Probe `rustc -vV` / `cargo -Vv` and parse the `release:` and `commit-hash:`
/// lines into typed values.
pub(crate) fn probe_tool(tool: &str, args: &[&str]) -> Result<(RustRelease, ToolchainCommit), String> {
    let out = Command::new(tool)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run {tool} {args:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{tool} {args:?} exited nonzero"));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut release: Option<RustRelease> = None;
    let mut commit: Option<ToolchainCommit> = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("release: ") {
            release = Some(parse_release(v.trim())?);
        }
        if let Some(v) = line.strip_prefix("commit-hash: ") {
            commit = ToolchainCommit::from_hex(v.trim()).ok();
        }
    }
    let release = release.ok_or_else(|| format!("{tool} reported no release"))?;
    let commit = commit.ok_or_else(|| format!("{tool} reported no usable commit-hash"))?;
    Ok((release, commit))
}

/// Probe the EXACT interpreter path (never a `python`/`python3` search): its
/// implementation name and release (5.5E6c2). receiptcheck asks the interpreter
/// selftest / setup-python selected, so the verifier and the producer agree.
pub(crate) fn probe_python(python: &Path) -> Result<(String, PythonRelease), String> {
    let out = Command::new(python)
        .args([
            "-c",
            "import sys; print(sys.implementation.name); \
             print(sys.version_info.major, sys.version_info.minor, sys.version_info.micro)",
        ])
        .output()
        .map_err(|e| format!("cannot run python {}: {e}", python.display()))?;
    if !out.status.success() {
        return Err(format!("python {} exited nonzero", python.display()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let implementation = lines
        .next()
        .ok_or("python reported no implementation")?
        .trim()
        .to_owned();
    let version_line = lines.next().ok_or("python reported no version")?;
    let parts: Vec<&str> = version_line.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(format!(
            "python version line {version_line:?} is not MAJOR MINOR MICRO"
        ));
    }
    let major = parts[0].parse().map_err(|_| "bad python major".to_owned())?;
    let minor = parts[1].parse().map_err(|_| "bad python minor".to_owned())?;
    let patch = parts[2].parse().map_err(|_| "bad python micro".to_owned())?;
    Ok((implementation, PythonRelease { major, minor, patch }))
}

// ===========================================================================
// Exact evidence-bundle shape (5.5E6c2): no unmanifested material
// ===========================================================================

/// Enforce the exact admitted evidence-bundle shape. The bundle carries EXACTLY
/// the admitted files — no compiler scratch (`.pdb`/`.lib`), no unmanifested
/// material, no symlink or special file, no case-folded duplicate.
/// `run-metadata.t0` is CONDITIONAL: forbidden in the core/supplemental posture,
/// required in the upload-ready hosted posture. `gate0-candidate/` internals stay
/// bound by the materializer output-tree digest.
pub(crate) fn check_bundle_shape(evidence: &Path, doc: &ArtifactDoc, upload_ready: bool) -> Result<(), String> {
    let hosted =
        doc.hosted_run.is_some() && matches!(doc.source, ParsedSource::GitCheckout { .. });
    let run_metadata_required = upload_ready && hosted;
    let allowed_files = ["qualification.t0", "law-fixtures.manifest"];
    let allowed_dirs = ["executables", "gate0-candidate"];

    let mut seen_lower: BTreeMap<String, String> = BTreeMap::new();
    let mut saw_run_metadata = false;
    for entry in fs::read_dir(evidence)
        .map_err(|e| format!("cannot read evidence bundle {}: {e}", evidence.display()))?
    {
        let entry = entry.map_err(|e| format!("bundle entry error: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry
            .file_type()
            .map_err(|e| format!("cannot stat bundle entry {name}: {e}"))?;
        if ft.is_symlink() {
            return Err(format!("evidence bundle contains a symlink: {name}"));
        }
        if let Some(prev) = seen_lower.insert(name.to_lowercase(), name.clone()) {
            return Err(format!("evidence bundle case-folds {prev:?} and {name:?}"));
        }
        if ft.is_dir() {
            if !allowed_dirs.contains(&name.as_str()) {
                return Err(format!("unadmitted directory in evidence bundle: {name}"));
            }
        } else if ft.is_file() {
            if name == "run-metadata.t0" {
                saw_run_metadata = true;
                if !run_metadata_required {
                    return Err(
                        "a core/supplemental evidence bundle must not carry run-metadata.t0"
                            .to_owned(),
                    );
                }
            } else if !allowed_files.contains(&name.as_str()) {
                return Err(format!("unmanifested file in evidence bundle: {name}"));
            }
        } else {
            return Err(format!("evidence bundle entry {name} is neither file nor dir"));
        }
    }
    if run_metadata_required && !saw_run_metadata {
        return Err("upload-ready hosted bundle is missing run-metadata.t0".to_owned());
    }

    // executables/ holds exactly one host-suffixed exe per non-fixture-set kind
    // (derived from Tier0ReceiptKind::ALL, never a hardcoded list).
    let exe_dir = evidence.join("executables");
    let admitted_exes: Vec<&str> = Tier0ReceiptKind::ALL
        .iter()
        .filter(|k| k.artifact_policy() != bq::Tier0ArtifactPolicy::FixtureSet)
        .map(|k| k.slug())
        .collect();
    let mut seen_exe_lower: BTreeMap<String, String> = BTreeMap::new();
    for entry in fs::read_dir(&exe_dir)
        .map_err(|e| format!("cannot read executables/ {}: {e}", exe_dir.display()))?
    {
        let entry = entry.map_err(|e| format!("executables/ entry error: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry
            .file_type()
            .map_err(|e| format!("cannot stat executables/ entry {name}: {e}"))?;
        if ft.is_symlink() {
            return Err(format!("executables/ contains a symlink: {name}"));
        }
        if !ft.is_file() {
            return Err(format!("executables/ contains a non-file entry: {name}"));
        }
        if let Some(prev) = seen_exe_lower.insert(name.to_lowercase(), name.clone()) {
            return Err(format!("executables/ case-folds {prev:?} and {name:?}"));
        }
        // Accept `<slug>` or `<slug>.exe`; refuse a stray `.pdb`/`.lib`, an extra
        // exe, or any other scratch output.
        let stem = name.strip_suffix(".exe").unwrap_or(&name);
        if !admitted_exes.contains(&stem) {
            return Err(format!("unmanifested file in executables/: {name}"));
        }
    }
    for slug in &admitted_exes {
        let base = exe_dir.join(slug);
        if !base.is_file() && !base.with_extension("exe").is_file() {
            return Err(format!("executables/ is missing the {slug} gate binary"));
        }
    }
    Ok(())
}

// ===========================================================================
// Deterministic tree digest (sorted paths, entry types, content hashes)
// ===========================================================================

