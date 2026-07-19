use spec::bootstrap_output;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use crate::invalid;

/// Publish the plan at the output root. Exactly two successes exist: Created
/// (the output was absent and a complete staged tree was renamed into place)
/// and Unchanged (the output already carried exactly the planned tree and
/// zero writes happened). Everything else refuses without touching the final
/// path.
pub(crate) fn publish(
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
