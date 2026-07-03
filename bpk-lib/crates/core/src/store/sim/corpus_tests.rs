//! Unit tests for [`super`] (split to keep `corpus.rs` under the file-size cap).

use super::super::recovery::run;
use super::{check_graduation, run_corpus_sweep, Boundary, FaultModeLabel, GraduationRefusal};

#[test]
fn fault_mode_label_round_trips_through_serialized_form() {
    for (label, expect) in [
        (FaultModeLabel::HonestDiskCrash, "HonestDiskCrash"),
        (
            FaultModeLabel::LyingDiskFsyncDrop { one_in: 3 },
            "LyingDiskFsyncDrop",
        ),
        (FaultModeLabel::CrashBeforeFsync, "CrashBeforeFsync"),
    ] {
        assert_eq!(label.as_str(), expect, "label must serialize stably");
        let parsed = FaultModeLabel::parse(label.as_str()).expect("label must re-parse");
        assert_eq!(
            parsed.as_str(),
            label.as_str(),
            "parse∘as_str must be identity on the label"
        );
    }
    assert!(
        FaultModeLabel::parse("NotARealMode").is_none(),
        "unknown labels must not parse"
    );
    assert_eq!(
        FaultModeLabel::parse_with_rate("LyingDiskFsyncDrop", Some(0)),
        Some(FaultModeLabel::LyingDiskFsyncDrop { one_in: 1 }),
        "a zero drop-rate must clamp to the legal floor of 1"
    );
    assert!(
        FaultModeLabel::parse_with_rate("LyingDiskFsyncDrop", None).is_none(),
        "a lying-disk row without a drop-rate column must be rejected"
    );
}

#[test]
fn boundary_round_trips_through_serialized_form() {
    for boundary in Boundary::ALL {
        let parsed = Boundary::parse(boundary.as_str()).expect("boundary must re-parse");
        assert_eq!(
            parsed, boundary,
            "parse∘as_str must be identity on every boundary"
        );
    }
    assert!(
        Boundary::parse("NotABoundary").is_none(),
        "unknown boundary labels must not parse"
    );
}

#[test]
fn graduation_refuses_nondeterministic_seed() -> Result<(), Box<dyn std::error::Error>> {
    let seed = 0xC000_0001;
    let steps = 48;
    let first = run(seed, steps).map_err(std::io::Error::other)?;
    let mismatched = run(seed, steps + 1).map_err(std::io::Error::other)?;
    if first.digest == mismatched.digest {
        return Err(std::io::Error::other(
            "PROPERTY: distinct step counts should diverge for this fixture",
        )
        .into());
    }
    let refusal = GraduationRefusal::NonDeterministic {
        seed,
        first: first.digest,
        second: mismatched.digest,
    };
    assert!(
        refusal.to_string().contains("non-deterministic"),
        "refusal must name non-determinism: {refusal}"
    );
    Ok(())
}

#[test]
fn graduation_accepts_deterministic_legal_seed() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = check_graduation(0xC000_0002, 64, "writer-commit", "L4")
        .map_err(|r| std::io::Error::other(format!("PROPERTY: legal seed must graduate: {r}")))?;
    assert_eq!(candidate.entry.seam_touched, "writer-commit");
    assert_eq!(candidate.entry.assurance_level, "L4");
    let again = check_graduation(0xC000_0002, 64, "writer-commit", "L4")
        .map_err(|r| std::io::Error::other(format!("PROPERTY: replay must re-graduate: {r}")))?;
    assert_eq!(
        candidate.entry.op_trace_digest, again.entry.op_trace_digest,
        "PROPERTY: digest must be stable across graduation calls"
    );
    Ok(())
}

#[test]
fn committed_corpus_seed_digest_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = check_graduation(48104590831, 96, "writer-commit", "L4").map_err(|r| {
        std::io::Error::other(format!(
            "PROPERTY: committed corpus seed must graduate: {r}"
        ))
    })?;
    assert_eq!(
        candidate.entry.op_trace_digest, 101_395_256_710_529_115,
        "PROPERTY: committed corpus digest for seed 48104590831 / 96 steps must be stable"
    );
    Ok(())
}

#[test]
fn sweep_emits_candidates_for_legal_seeds() -> Result<(), Box<dyn std::error::Error>> {
    let (ok, bad) = run_corpus_sweep(&[0xC000_0003, 0xC000_0004], 48, "writer-commit", "L4");
    if ok.len() != 2 {
        return Err(std::io::Error::other(format!(
            "PROPERTY: expected two graduates, got {} ok and {} refused",
            ok.len(),
            bad.len()
        ))
        .into());
    }
    Ok(())
}

#[test]
fn empty_seam_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    match check_graduation(0xC000_0005, 32, "", "L4") {
        Ok(_) => {
            return Err(
                std::io::Error::other("PROPERTY: empty seam_touched must be refused").into(),
            )
        }
        Err(GraduationRefusal::EmptySeam { .. }) => {}
        Err(other) => {
            return Err(std::io::Error::other(format!(
                "PROPERTY: expected EmptySeam refusal, got {other}"
            ))
            .into())
        }
    }
    Ok(())
}

#[test]
fn corpus_outcome_parses_every_label_including_canonical_refusal() {
    use super::CorpusOutcome;
    assert_eq!(
        CorpusOutcome::parse("CommittedPrefix"),
        Some(CorpusOutcome::CommittedPrefix)
    );
    assert_eq!(
        CorpusOutcome::parse("RolledBack"),
        Some(CorpusOutcome::RolledBack)
    );
    assert_eq!(
        CorpusOutcome::parse("CanonicalRefusal"),
        Some(CorpusOutcome::CanonicalRefusal),
        "PROPERTY: the CanonicalRefusal label must round-trip through parse — deleting its \
         match arm would make a corrupt-refusal corpus row unparseable"
    );
    assert!(
        CorpusOutcome::parse("NotAnOutcome").is_none(),
        "unknown outcome labels must not parse"
    );
}

#[test]
fn graduation_records_the_replayed_outcome_not_the_placeholder() {
    use super::{
        check_graduation_for, CorpusOracle, CorpusOutcome, FaultModeLabel, GraduationCell,
    };
    // A CrashBeforeFsync abort at the SingleAppendFrame boundary recovers as
    // RolledBack (committed corpus seed 1677852675). The graduated candidate
    // must carry THAT replayed outcome, not the `CommittedPrefix` placeholder
    // the working entry is seeded with — dropping the `outcome: first.outcome`
    // field would silently ship the placeholder.
    let candidate = check_graduation_for(GraduationCell {
        seed: 1677852675,
        steps: 96,
        fault_mode: FaultModeLabel::CrashBeforeFsync,
        boundary: Boundary::parse("SingleAppendFrame"),
        seam_touched: "single-append-frame",
        assurance_level: "L4",
        oracle: CorpusOracle::StoreRecovery,
        import_kind: None,
    })
    .expect("PROPERTY: the RolledBack fixture must graduate deterministically");
    assert_eq!(
        candidate.entry.outcome,
        CorpusOutcome::RolledBack,
        "PROPERTY: graduation must record the REPLAYED outcome (RolledBack), not the \
         CommittedPrefix placeholder"
    );
}

#[test]
fn verify_corpus_row_accepts_the_committed_digest_and_rejects_a_drifted_one() {
    use super::verify_corpus_row;
    // The committed honest-disk row (seed 48104590831 / 96 steps) is current.
    verify_corpus_row(
        48_104_590_831,
        96,
        "HonestDiskCrash",
        101_395_256_710_529_115,
    )
    .expect("PROPERTY: the committed digest must still replay identically");
    // A drifted digest must be REPORTED — a `-> Ok(())` body on either
    // verify_corpus_row or its delegate verify_corpus_row_cell would mask it.
    let err = verify_corpus_row(48_104_590_831, 96, "HonestDiskCrash", 0xDEAD_BEEF)
        .expect_err("PROPERTY: a drifted digest must be reported, not silently accepted");
    assert!(
        err.contains("expected digest"),
        "the report must name the digest drift, got: {err}"
    );
}

#[test]
fn assert_corpus_rows_current_refuses_an_empty_row_set() {
    use super::{assert_corpus_rows_current, CorpusRowDescriptor};
    let rows: &[CorpusRowDescriptor<'_>] = &[];
    let err = assert_corpus_rows_current(rows)
        .expect_err("PROPERTY: an empty corpus row set must be refused");
    assert!(
        err.contains("non-empty"),
        "the report must name the empty-corpus violation, got: {err}"
    );
}

#[test]
fn assert_corpus_currency_refuses_an_empty_corpus() {
    use super::{assert_corpus_currency, CorpusEntry};
    let entries: &[CorpusEntry] = &[];
    let err =
        assert_corpus_currency(entries).expect_err("PROPERTY: an empty corpus must be refused");
    assert!(
        err.contains("non-empty"),
        "the report must name the empty-corpus violation, got: {err}"
    );
}
