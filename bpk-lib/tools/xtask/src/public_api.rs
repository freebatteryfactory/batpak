use crate::util::{cargo_target_dir, repo_root, run_output};
use crate::{PublicApiArgs, SemverCheckArgs};
use anyhow::{bail, Context, Result};
use std::fs;
use std::process::Command;

struct PublicApiPackage {
    package: &'static str,
    baseline: &'static str,
    features: &'static [&'static str],
}

const PUBLIC_API_PACKAGES: &[PublicApiPackage] = &[
    PublicApiPackage {
        package: "batpak",
        baseline: "batpak.txt",
        features: &["blake3"],
    },
    PublicApiPackage {
        package: "syncbat",
        baseline: "syncbat.txt",
        features: &[],
    },
    PublicApiPackage {
        package: "netbat",
        baseline: "netbat.txt",
        features: &[],
    },
];

pub(crate) fn public_api(args: PublicApiArgs) -> Result<()> {
    let root = repo_root()?;
    let target_dir = cargo_target_dir()?.join("public-api");
    fs::create_dir_all(&target_dir).with_context(|| format!("create {}", target_dir.display()))?;
    if args.check_baseline && args.bless_baseline {
        bail!("public-api: choose either --check-baseline or --bless-baseline, not both");
    }

    if !cargo_public_api_is_available() {
        let message = "public-api: cargo-public-api is not installed; advisory run skipped";
        if args.strict {
            bail!("{message}");
        }
        eprintln!("{message}");
        return Ok(());
    }

    for package in PUBLIC_API_PACKAGES {
        let mut command = Command::new("cargo");
        command
            .current_dir(&root)
            .env("CARGO_TARGET_DIR", cargo_target_dir()?)
            .args([
                "public-api",
                "-sss",
                "--color",
                "never",
                "--package",
                package.package,
                "--manifest-path",
                "Cargo.toml",
            ]);
        if !package.features.is_empty() {
            command.arg("--features").arg(package.features.join(","));
        }

        let output = match run_output(command) {
            Ok(output) => output,
            Err(error) if !args.strict => {
                eprintln!(
                    "public-api: advisory run failed for {}: {error:#}",
                    package.package
                );
                return Ok(());
            }
            Err(error) => return Err(error),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let current = target_dir.join(package.baseline);
        let stderr_path = target_dir.join(format!("{}.stderr.txt", package.package));
        fs::write(&current, stdout.as_bytes())
            .with_context(|| format!("write {}", current.display()))?;
        fs::write(&stderr_path, stderr.as_bytes())
            .with_context(|| format!("write {}", stderr_path.display()))?;
        println!("public-api: wrote {}", current.display());

        let baseline = root.join("traceability/public_api").join(package.baseline);
        if args.bless_baseline {
            if let Some(parent) = baseline.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create {}", parent.display()))?;
            }
            fs::write(&baseline, stdout.as_bytes())
                .with_context(|| format!("write {}", baseline.display()))?;
            println!("public-api: blessed {}", baseline.display());
        }
        if args.check_baseline {
            let expected = fs::read_to_string(&baseline)
                .with_context(|| format!("read {}", baseline.display()))?;
            if expected != stdout {
                bail!(
                    "public-api baseline for {} drifted; inspect {} and refresh intentionally with `cargo xtask public-api --strict --bless-baseline`",
                    package.package,
                    current.display()
                );
            }
            println!("public-api: baseline matches {}", baseline.display());
        }
    }
    Ok(())
}

pub(crate) fn semver_check(args: SemverCheckArgs) -> Result<()> {
    let root = repo_root()?;
    let target_dir = cargo_target_dir()?.join("semver-public-api");
    fs::create_dir_all(&target_dir).with_context(|| format!("create {}", target_dir.display()))?;

    if !cargo_semver_checks_is_available() {
        let message = "semver-check: cargo-semver-checks is not installed; advisory run skipped";
        if args.strict {
            bail!("{message}");
        }
        eprintln!("{message}");
        return Ok(());
    }

    let mut command = Command::new("cargo");
    command
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", cargo_target_dir()?)
        .args([
            "semver-checks",
            "--manifest-path",
            "crates/core/Cargo.toml",
            "--package",
            "batpak",
            "--features",
            "blake3",
        ]);

    let output = match run_output(command) {
        Ok(output) => output,
        Err(error) if !args.strict => {
            let report = target_dir.join("semver-checks.txt");
            fs::write(&report, format!("{error:#}\n")).context("write semver advisory error")?;
            eprintln!(
                "semver-check: advisory run reported incompatibility or failed; see {}",
                report.display()
            );
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    fs::write(target_dir.join("semver-checks.txt"), stdout.as_bytes())
        .context("write target/semver-public-api/semver-checks.txt")?;
    fs::write(
        target_dir.join("semver-checks.stderr.txt"),
        stderr.as_bytes(),
    )
    .context("write target/semver-public-api/semver-checks.stderr.txt")?;
    println!(
        "semver-check: wrote {}",
        target_dir.join("semver-checks.txt").display()
    );
    Ok(())
}

fn cargo_public_api_is_available() -> bool {
    Command::new("cargo-public-api")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn cargo_semver_checks_is_available() -> bool {
    Command::new("cargo-semver-checks")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
