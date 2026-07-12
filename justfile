set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Integration tests (netbat TCP listeners, temp dirs) need a host temp root.
# Cursor sandboxes and some CI images block binds under the default TMPDIR;
# override by exporting TMPDIR before `just`.
export TMPDIR := env_var_or_default("TMPDIR", "/tmp")

default:
    just --list

help:
    just --list

list:
    just --list

doctor:
    cd bpk-lib; cargo xtask doctor

install-hooks:
    cd bpk-lib; cargo xtask install-hooks

traceability:
    cd bpk-lib; cargo xtask traceability

structural:
    cd bpk-lib; cargo xtask structural

# `structural` already runs the full detector suite (layout, boundary,
# stale-paths are discoverability aliases over the same check), so inspect
# runs it once — not once per alias.
inspect:
    cd bpk-lib; cargo xtask structural
    cd bpk-lib; cargo xtask architecture-ir
    cd bpk-lib; cargo xtask architecture-ir --check
    cd bpk-lib; cargo xtask ast-grep

layout:
    cd bpk-lib; cargo xtask layout

boundary:
    cd bpk-lib; cargo xtask boundary

stale-paths:
    cd bpk-lib; cargo xtask stale-paths

disk-audit:
    cd bpk-lib; cargo xtask disk-audit

clean-generated:
    cd bpk-lib; cargo xtask clean-generated

package-leak-scan:
    cd bpk-lib; cargo xtask package-leak-scan --allow-dirty

check:
    cd bpk-lib; cargo xtask check

test:
    cd bpk-lib; cargo xtask test

clippy:
    cd bpk-lib; cargo xtask clippy

fmt:
    cd bpk-lib; cargo xtask fmt

deny:
    cd bpk-lib; cargo xtask deny

bench-compile:
    cd bpk-lib; cargo xtask bench --compile

perf-gates:
    cd bpk-lib; cargo xtask perf-gates

loom:
    cd bpk-lib; cargo xtask loom

template-freshness:
    cd bpk-lib; cargo xtask template-freshness

staged-diff:
    cd bpk-lib; cargo xtask staged-diff

release-manifest:
    cd bpk-lib; cargo xtask release-manifest

# Keep mutation surfaces aligned with the compiled feature set:
# the blake3 hash helper is excluded from no-default runs, and the
# clock-only fallback helper is excluded from all-features runs.
mutants-smoke:
    cd bpk-lib; cargo xtask mutants smoke

mutants-full:
    cd bpk-lib; cargo xtask mutants full

ledger-list:
    cd bpk-lib; cargo xtask factory-ledger list

context:
    cd bpk-lib; cargo xtask context

ledger-run command *args:
    cd bpk-lib; cargo xtask factory-ledger run -- {{command}} {{args}}

ledger-run-gate gate command *args:
    cd bpk-lib; cargo xtask factory-ledger run --gate {{gate}} -- {{command}} {{args}}

ci-fast:
    cd bpk-lib; cargo xtask ci-fast

ci-windows:
    cd bpk-lib; cargo xtask ci-windows-surface

wasm-surface:
    cd bpk-lib; cargo xtask wasm-surface

ci:
    cd bpk-lib; cargo xtask ci

verify:
    cd bpk-lib; cargo xtask preflight

seal:
    cd bpk-lib; cargo xtask check-version-pins
    cd bpk-lib; cargo xtask evidence-audit
    cd bpk-lib; cargo xtask release-status --strict --active
    cd bpk-lib; cargo xtask release-manifest --strict

ship mode="dry":
    just ship-{{mode}}

ship-dry:
    cd bpk-lib; cargo xtask release --dry-run

ship-real:
    cd bpk-lib; cargo xtask release

pre-commit:
    cd bpk-lib; cargo xtask pre-commit

# `cover` is report-only (no threshold). The blocking threshold-80 floor runs
# in CI as the `ci-fast-coverage` lane (`cargo xtask ci-fast --lane coverage`),
# which also uploads the same run's report bundle — one instrumented build
# serves both. To enforce that same floor locally, use the `cover-check`
# recipe two entries down.
cover:
    cd bpk-lib; cargo xtask cover

cover-check:
    cd bpk-lib; cargo xtask cover --ci --threshold 80

cover-json:
    cd bpk-lib; cargo xtask cover --json

bench: bench-neutral

bench-neutral:
    cd bpk-lib; cargo xtask bench --surface neutral

bench-native:
    cd bpk-lib; cargo xtask bench --surface native

bench-save surface="neutral":
    cd bpk-lib; cargo xtask bench --surface {{surface}} --save

bench-compare surface="neutral":
    cd bpk-lib; cargo xtask bench --surface {{surface}} --compare

fuzz:
    cd bpk-lib; cargo xtask fuzz

fuzz-deep:
    cd bpk-lib; cargo xtask fuzz --deep

chaos:
    cd bpk-lib; cargo xtask chaos

chaos-deep:
    cd bpk-lib; cargo xtask chaos --deep

fuzz-chaos:
    cd bpk-lib; cargo xtask fuzz-chaos

stress:
    cd bpk-lib; cargo xtask stress

docs:
    cd bpk-lib; cargo xtask docs --open

doc: docs

cargo +args:
    cd bpk-lib; cargo {{args}}
