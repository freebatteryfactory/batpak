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

use spec::architecture;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[path = "materialize/plan.rs"] mod plan;
#[path = "materialize/render.rs"] mod render;
#[path = "materialize/publish.rs"] mod publish;

use crate::plan::materialize;

/// The exact `SPEC.sha256` bytes this materializer was compiled against
/// (5.5E5a). A binary compiled from seed A cannot casually claim seed B:
/// the supplied seed must carry this same manifest before any plan is
/// built. This is not a replacement for `freeze --check`; it binds the
/// binary to the source it was born from.
pub(crate) const COMPILED_SEED_MANIFEST: &str = include_str!("../SPEC.sha256");

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

pub(crate) fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

