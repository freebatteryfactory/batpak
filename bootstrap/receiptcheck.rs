#![deny(warnings)]

//! bootstrap/receiptcheck.rs — the independent Tier 0 qualification verifier
//! (5.5E6b). This binary is the AUTHORITATIVE computer of qualification
//! evidence: it links the real `spec` rlib, parses the strict
//! `qualification.t0` artifact, recomputes every digest from the bytes on disk,
//! compares them against the artifact's claims, and only then calls the typed
//! `spec::bootstrap_qualification::verify`. It is a standalone gate tool, NOT a
//! sixth Tier 0 receipt, and it never imports or shells out to selftest.py.
//!
//! The private seal on `VerifiedTier0Qualification` prevents bypass of the
//! verified shape; it does not prove the inputs were computed honestly. That
//! honest computation is exactly what this program performs.
//!
//! Modes:
//! ```text
//! receiptcheck policy
//! receiptcheck verify <qualification.t0> --root <source-root> --evidence <bundle-root>
//!     --python-executable <interpreter> [--upload-ready]
//! receiptcheck compare
//!     --candidate-artifact <t0> --candidate-evidence <bundle> --candidate-run-metadata <meta>
//!     --cleanroom-artifact <t0> --cleanroom-evidence <bundle> --cleanroom-run-metadata <meta>
//!     --root <source-root> --python-executable <interpreter> [--require-promotion-confirmation]
//! ```
//!
//! The artifact must be the bundle's own `qualification.t0`; the bundle shape is
//! exact (no unmanifested `.pdb`/scratch); the interpreter is probed at the exact
//! path (never a `python`/`python3` search) and must be CPython at the artifact's
//! release. The v2 grammar binds the builder Python runtime (5.5E6c2).
//!
//! `compare` is the wire end of the cross-run algebra (5.5E6c1): it independently
//! verifies BOTH uploaded bundles, ties each to its hosted run's own record
//! (conclusion, head SHA, repository, workflow, run id, attempt), then calls the
//! sealed `compare_runs` and `confirm_promotion`. It computes the comparison —
//! Python never parses or duplicates the comparator law.
//!
//! `campaign-verify` (F5 R4; E7 R4 hardening) independently recomputes and
//! validates the campaign-evidence bundle, dispatching on the bundle's own
//! version line — a `/3` bundle is verified live and requires the full
//! six-flag perimeter, a `/2` bundle is REFUSED as E7-mechanical historical
//! evidence (its full verifier is retired), and a `/1` bundle routes to the
//! retained historical arm under the original four flags:
//! ```text
//! receiptcheck campaign-verify <bundle>
//!     --judge-root <dir> --envelope <file> --source-commit <40-hex>
//!     --nursery-root <dir> --evidence-root <dir>
//! ```
//!
//! `e7-verify` (E7 closeout, CL-8) independently verifies the
//! BATPAK-E7-UNDERWRITING/2 opening-matrix artifact: it recomputes every
//! binding digest from the bytes on disk, refuses any nonzero opening-matrix
//! row by name, VERIFIES each row's independently produced owner receipt (no
//! literal zero is accepted without a receipt-backed owner), re-executes the
//! campaign verification core in-process over the six bound campaign inputs,
//! and recomputes the unresolved-architect-required-findings row from the
//! nursery receipts (TL-6). `e7-open` is the SOLE printer of
//! `phase6-opening-eligible`: given the cross-run stability receipt and the
//! two runs' artifacts, it recomputes the receipt's digests, independently
//! re-runs the authoritative comparison, requires every zero row 0 with
//! owner+receipt and `cross-run-stability pending` in BOTH, and only then
//! speaks the mechanical-rows opening banner:
//! ```text
//! receiptcheck e7-verify <artifact> --root <repo> --tier0-bundle <dir>
//!     --campaign-bundle <file> --judge-root <dir> --envelope <file>
//!     --source-commit <40-hex> --nursery-root <dir> --evidence-root <dir>
//! receiptcheck e7-open <stability-receipt> --own-artifact <t0>
//!     --candidate-artifact <t0> --source-commit <40-hex>
//! ```

use std::env;

#[path = "receiptcheck/modes.rs"] mod modes;
#[path = "receiptcheck/artifact.rs"] mod artifact;
#[path = "receiptcheck/digests.rs"] mod digests;
#[path = "receiptcheck/bundle.rs"] mod bundle;
#[path = "receiptcheck/hashing.rs"] mod hashing;
#[path = "receiptcheck/campaign.rs"] mod campaign;
#[path = "receiptcheck/campaign_v1.rs"] mod campaign_v1;
#[path = "receiptcheck/e7.rs"] mod e7;

use crate::campaign::mode_campaign_verify;
use crate::e7::{mode_e7_open, mode_e7_verify};
use crate::modes::{mode_compare, mode_policy, mode_verify};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);
    let result = match mode {
        Some("policy") => mode_policy(),
        Some("verify") => mode_verify(&args[2..]),
        Some("compare") => mode_compare(&args[2..]),
        Some("campaign-verify") => mode_campaign_verify(&args[2..]),
        Some("e7-verify") => mode_e7_verify(&args[2..]),
        Some("e7-open") => mode_e7_open(&args[2..]),
        _ => Err(
            "usage: receiptcheck policy | receiptcheck verify <artifact> \
             --root <root> --evidence <bundle> --python-executable <py> [--upload-ready] \
             | receiptcheck compare --candidate-artifact <t0> --candidate-evidence <bundle> \
             --candidate-run-metadata <meta> --cleanroom-artifact <t0> \
             --cleanroom-evidence <bundle> --cleanroom-run-metadata <meta> \
             --root <root> --python-executable <py> [--require-promotion-confirmation] \
             | receiptcheck campaign-verify <bundle> --judge-root <dir> \
             --envelope <file> --source-commit <sha> --nursery-root <dir> \
             --evidence-root <dir> (V1 bundles: the first three flags only) \
             | receiptcheck e7-verify <artifact> --root <repo> --tier0-bundle <dir> \
             --campaign-bundle <file> --judge-root <dir> --envelope <file> \
             --source-commit <sha> --nursery-root <dir> --evidence-root <dir> \
             | receiptcheck e7-open <stability-receipt> --own-artifact <t0> \
             --candidate-artifact <t0> --source-commit <sha>"
                .to_owned(),
        ),
    };
    if let Err(message) = result {
        eprintln!("receiptcheck: FAIL — {message}");
        std::process::exit(1);
    }
}
