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
//! `campaign-verify` (F5, R4) independently recomputes and validates the
//! mini-supernova campaign-evidence bundle:
//! ```text
//! receiptcheck campaign-verify <bundle>
//!     --judge-root <dir> --envelope <file> --source-commit <40-hex>
//! ```

use std::env;

#[path = "receiptcheck/modes.rs"] mod modes;
#[path = "receiptcheck/artifact.rs"] mod artifact;
#[path = "receiptcheck/digests.rs"] mod digests;
#[path = "receiptcheck/bundle.rs"] mod bundle;
#[path = "receiptcheck/hashing.rs"] mod hashing;
#[path = "receiptcheck/campaign.rs"] mod campaign;

use crate::campaign::mode_campaign_verify;
use crate::modes::{mode_compare, mode_policy, mode_verify};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);
    let result = match mode {
        Some("policy") => mode_policy(),
        Some("verify") => mode_verify(&args[2..]),
        Some("compare") => mode_compare(&args[2..]),
        Some("campaign-verify") => mode_campaign_verify(&args[2..]),
        _ => Err(
            "usage: receiptcheck policy | receiptcheck verify <artifact> \
             --root <root> --evidence <bundle> --python-executable <py> [--upload-ready] \
             | receiptcheck compare --candidate-artifact <t0> --candidate-evidence <bundle> \
             --candidate-run-metadata <meta> --cleanroom-artifact <t0> \
             --cleanroom-evidence <bundle> --cleanroom-run-metadata <meta> \
             --root <root> --python-executable <py> [--require-promotion-confirmation] \
             | receiptcheck campaign-verify <bundle> --judge-root <dir> \
             --envelope <file> --source-commit <sha>"
                .to_owned(),
        ),
    };
    if let Err(message) = result {
        eprintln!("receiptcheck: FAIL — {message}");
        std::process::exit(1);
    }
}
