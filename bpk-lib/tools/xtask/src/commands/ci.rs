use super::{deny_split, integrity, templates};
use crate::bench;
use crate::coverage;
use crate::publish::FAMILY_CRATES;
use crate::util::{cargo, cargo_target_dir_arg};
use crate::{BenchSurface, CiFastArgs, CiFastLane, CoverArgs, PackageLeakScanArgs, PublicApiArgs};
use anyhow::Result;

/// Early PR signal: format, clippy, checks, tests, dependency gates, machine law.
///
/// With no `--lane`, runs every lane serially (the local one-command bundle).
/// With `--lane <name>`, runs exactly one fan-out lane — this is how CI splits
/// the fast bundle into parallel jobs (one lane per runner) while `ci-fast`
/// stays a single reproducible command on a developer machine. The anti-rebury
/// assertion in tools/integrity/src/ci_parity.rs pins every lane into this
/// serial dispatch AND into an always-on ci.yml lane job, so fan-out can never
/// silently drop a gate from the default PR path.
pub(crate) fn ci_fast(args: CiFastArgs) -> Result<()> {
    let Some(lane) = args.lane else {
        ci_fast_check()?;
        ci_fast_lint()?;
        ci_fast_test()?;
        ci_fast_test_docs()?;
        ci_fast_contracts()?;
        return ci_fast_coverage();
    };
    match lane {
        CiFastLane::Check => ci_fast_check(),
        CiFastLane::Lint => ci_fast_lint(),
        CiFastLane::Test => ci_fast_test(),
        CiFastLane::TestDocs => ci_fast_test_docs(),
        CiFastLane::Contracts => ci_fast_contracts(),
        CiFastLane::Coverage => ci_fast_coverage(),
    }
}

/// Lane: version pins, format, workspace/family checks, dependency gates.
fn ci_fast_check() -> Result<()> {
    super::check_version_pins()?;
    cargo(["fmt", "--check"])?;
    run_workspace_and_family_checks()?;
    deny_split()
}

/// Lane: clippy with denied warnings across the workspace and family crates.
fn ci_fast_lint() -> Result<()> {
    run_workspace_clippy()?;
    run_family_clippy()
}

/// Lane: the workspace nextest surface — deliberately ALONE in this lane.
fn ci_fast_test() -> Result<()> {
    // `--workspace` is mandatory here for the same reason as the clippy gate:
    // without it nextest runs only `default-members` (crates/core), leaving
    // tools/integrity and the macro crates outside the test net. That
    // hole let a stale integrity self-test fixture rot undetected until a
    // workspace-wide run surfaced it.
    run_nextest_ci(["--workspace", "--all-features"])
}

/// Lane: doctests and per-family-crate test suites, split out of the nextest
/// lane. Warm-cache measurement (PR #221's run) put the nextest lane at
/// 22m44s — the fan-out critical path — with ~4 min of that being this tail;
/// splitting lets nextest and the doctest/family surfaces run on parallel
/// runners, dropping PR wall-clock to roughly the coverage lane's floor.
fn ci_fast_test_docs() -> Result<()> {
    cargo(["test", "--doc", "--all-features"])?;
    run_family_tests()
}

/// Lane: machine law and the L2+ contract gates (P1-1).
///
/// These used to live only in `ci()`/`preflight()` behind the label-gated
/// verify-linux job, which meant a public-api drift or sub-floor coverage
/// shipped without any label. They now run in `ci-fast` (the always-on default
/// lane) so EVERY Rust-touching PR enforces them. The devcontainer ships the
/// required tools, so each gate is invoked STRICT here (cargo-public-api
/// availability is still handled gracefully inside `public_api`, mirroring its
/// existing strict/advisory split). Re-burying any of these is blocked by the
/// anti-rebury assertion in tools/integrity/src/ci_parity.rs.
fn ci_fast_contracts() -> Result<()> {
    templates()?;
    integrity("traceability-check", [])?;
    integrity("structural-check", [])?;
    crate::public_api::public_api(PublicApiArgs {
        strict: true,
        check_baseline: true,
        bless_baseline: false,
    })?;
    super::package_leak_scan(PackageLeakScanArgs {
        allow_dirty: false,
        strict_language: true,
    })?;
    integrity("doctor", ["--strict"])?;
    // Tool-qualification anti-vacuity gate (P1-3): every registered gate must
    // have left a non-vacuous execution receipt. Runs AFTER structural-check so
    // the receipts it validates were just (re)generated in this same invocation.
    integrity("gauntlet-receipts-present", [])
}

/// Lane: the blocking coverage floor. The single instrumented llvm-cov run both
/// enforces `COVERAGE_FLOOR_PCT` and leaves the report artifacts under
/// `target/xtask-cover/last-run/` — CI uploads those from this same lane, so
/// the old separate advisory `coverage-baseline` job (a second full
/// instrumented recompile per PR) is retired.
fn ci_fast_coverage() -> Result<()> {
    coverage::cover(CoverArgs {
        ci: true,
        json: false,
        threshold: Some(coverage::COVERAGE_FLOOR_PCT),
    })
}

/// Full merge bundle: fast lane plus release-oriented and compile-heavy gates.
///
/// The L2+ contract gates (coverage floor, public-api baseline,
/// package-leak-scan, doctor --strict) now live in `ci_fast()` so they run on
/// the default PR path; this bundle keeps only the genuinely compile-heavy /
/// release-oriented extras on top of the fast lane. `ci_fast` already ends by
/// running `structural-check` inside the contracts lane, so this bundle does
/// not re-run it.
pub(crate) fn ci() -> Result<()> {
    ci_fast(CiFastArgs { lane: None })?;
    doc_deny_warnings()?;
    bench::bench_compile(BenchSurface::Neutral)?;
    bench::bench_compile(BenchSurface::Native)?;
    unused_deps_advisory();
    Ok(())
}

/// Check-only wasm32-unknown-unknown embedding surface (issue #164): core and
/// syncbat — plus core's `payload-encryption` feature — must keep compiling
/// for the target. Check-only by design: no test runner, JS host, or runtime
/// dependency is involved, so the lane stays cheap and never gates on wasm
/// tooling. The target itself is installed by `rust-toolchain.toml`.
pub(crate) fn wasm_surface() -> Result<()> {
    cargo([
        "check",
        "-p",
        "batpak",
        "--target",
        "wasm32-unknown-unknown",
    ])?;
    cargo([
        "check",
        "-p",
        "batpak",
        "--features",
        "payload-encryption",
        "--target",
        "wasm32-unknown-unknown",
    ])?;
    cargo([
        "check",
        "-p",
        "syncbat",
        "--target",
        "wasm32-unknown-unknown",
    ])
}

/// Native Windows surface compatibility: checks, clippy, tests, and
/// platform-sensitive fixtures.
pub(crate) fn ci_windows_surface() -> Result<()> {
    super::check_version_pins()?;
    cargo(["fmt", "--check"])?;
    run_workspace_and_family_checks()?;
    // Windows-native clippy (#178): Linux clippy cannot lint code compiled
    // out by `cfg(windows)`, so this is unique truth, not duplicate heat.
    // The dependency deny/audit gates stay Linux-only by design — dependency
    // graphs are host-independent.
    run_workspace_clippy()?;
    run_family_clippy()?;
    // `--workspace` is mandatory (#224): without it nextest runs only
    // default-members (crates/core), leaving the family crates and the new
    // conformance/namespace tests as a Windows blind spot. The complete family
    // surface must run natively here, not just crates/core.
    run_nextest_ci(["--workspace", "--all-features"])?;
    run_kind_collision_composer_fixture()
}

fn run_workspace_clippy() -> Result<()> {
    // `--workspace` is mandatory: without it cargo lints only `default-members`
    // (crates/core), silently leaving tools/integrity and tools/xtask outside
    // the clippy net. A 0.9.0 PR1 review caught 14 clippy errors hiding in
    // tools/integrity precisely because this gate skipped them.
    cargo([
        "clippy",
        "--workspace",
        "--all-features",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])
}

fn run_family_clippy() -> Result<()> {
    for package in FAMILY_CRATES {
        cargo([
            "clippy",
            "-p",
            package,
            "--no-deps",
            "--all-features",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])?;
    }
    Ok(())
}

fn run_workspace_and_family_checks() -> Result<()> {
    // `--workspace` for the same reason as run_workspace_clippy: the bare form
    // only checks default-members (crates/core), leaving the tooling crates
    // unchecked under both the default and no-default-features axes.
    cargo(["check", "--workspace", "--all-features"])?;
    cargo(["check", "--workspace", "--no-default-features"])?;
    for package in FAMILY_CRATES {
        cargo(["check", "-p", package, "--all-features"])?;
        cargo(["check", "-p", package, "--no-default-features"])?;
    }
    Ok(())
}

fn run_family_tests() -> Result<()> {
    for package in FAMILY_CRATES {
        cargo(["test", "-p", package, "--all-features"])?;
    }
    Ok(())
}

fn run_kind_collision_composer_fixture() -> Result<()> {
    cargo([
        "test",
        "--release",
        "--manifest-path",
        "crates/core/fixtures/kind-collision-composer/Cargo.toml",
    ])
}

/// Build rustdoc for every workspace member with `-D warnings`.
///
/// Uses `--workspace` (not a hardcoded crate list) so any new crate is doc-lint
/// gated automatically. `--no-deps` keeps the gate to first-party crates.
/// `--all-features` is load-bearing (#178 seam 2): the 0.10.0 release-blocking
/// stale intra-doc link lived behind `#[cfg(feature = "payload-encryption")]`
/// and hid from the default-features build — only the release `docs()` gate
/// compiled it. Feature-gated doc breakage must fail HERE, not at release.
fn doc_deny_warnings() -> Result<()> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["doc", "--workspace", "--all-features", "--no-deps"])
        .env("RUSTDOCFLAGS", "-D warnings");
    crate::util::run(cmd)
}

fn unused_deps_advisory() {
    if let Err(error) = super::unused_deps() {
        errln!("xtask ci: unused-deps advisory pass reported: {error}");
    }
}

pub(crate) fn perf_gates() -> Result<()> {
    run_nextest_ci([
        "--test",
        "perf_gates",
        "--test",
        "perf_gates_throughput_latency",
        "--test",
        "perf_gates_cold_start",
        "--test",
        "perf_gates_correctness",
        "--all-features",
        "--run-ignored",
        "only",
    ])
}

pub(crate) fn run_nextest_ci<'a, I>(args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let target_dir = cargo_target_dir_arg()?;
    let mut command: Vec<String> = vec![
        "nextest".to_owned(),
        "run".to_owned(),
        "--target-dir".to_owned(),
        target_dir,
        "--profile".to_owned(),
        "ci".to_owned(),
    ];
    command.extend(args.into_iter().map(str::to_owned));
    cargo(command.iter().map(String::as_str))
}
