use super::{deny_split, integrity};
use crate::bench;
use crate::util::cargo;
use crate::BenchSurface;
use anyhow::Result;

pub(crate) fn ci() -> Result<()> {
    integrity("doctor", ["--strict"])?;
    integrity("traceability-check", [])?;
    integrity("structural-check", [])?;
    cargo(["fmt", "--check"])?;
    cargo([
        "clippy",
        "--all-features",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])?;
    deny_split()?;
    cargo(["nextest", "run", "--profile", "ci", "--all-features"])?;
    cargo(["test", "--doc", "--all-features"])?;
    cargo(["check", "--all-features"])?;
    cargo(["check", "--no-default-features"])?;
    bench::bench_compile(BenchSurface::Neutral)?;
    bench::bench_compile(BenchSurface::Native)
}

pub(crate) fn perf_gates() -> Result<()> {
    cargo([
        "nextest",
        "run",
        "--test",
        "perf_gates",
        "--all-features",
        "--run-ignored",
        "only",
    ])
}
