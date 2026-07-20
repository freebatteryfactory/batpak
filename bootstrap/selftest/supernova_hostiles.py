"""Hostile battery for the mini-supernova campaign rehearsal (docs/24).

Every refusal law in the rehearsal harness is proven live here, and the
independent verifier is proven two-sided (a tampered bundle is refused; a
blinded verifier goes silent, which the battery catches). Split from
supernova.py under the ratified split-on-next-edit size rule; the laws
themselves stay owned by supernova.py -- this module only exercises them.
"""
from __future__ import annotations

import re
import shutil
from pathlib import Path

from . import supernova_bundle as sb
from . import supernova_subject as subj


def hostiles(paths: dict, work: Path, *, admit_failure_evidence,
             judge_freshness, promote_guard, validate_disjoint) -> list[str]:
    """The refusal laws are passed in by supernova.py (their owner) so the
    module graph stays a DAG: this battery depends only on the bundle and
    subject carriers, never back on the harness."""
    findings: list[str] = []

    def expect_refusal(name: str, produced: str | None, needle: str) -> None:
        if produced is None or needle not in produced:
            findings.append(f"{name} FAILED (wanted {needle!r}, got {produced!r})")

    # Judge mutation invalidates evidence (and prior evidence stays intact).
    judge_copy = work / "judge-mutant"
    shutil.copytree(paths["judge"], judge_copy)
    frozen = sb.tree_digest(judge_copy)
    if judge_freshness(judge_copy, frozen) is not None:
        findings.append("judge_freshness misfired on an unmutated judge")
    policy = judge_copy / "policy.txt"
    policy.write_text(policy.read_text(encoding="utf-8") + "# mutated\n",
                      encoding="utf-8", newline="\n")
    expect_refusal("judge_mutation_invalidates_evidence",
                   judge_freshness(judge_copy, frozen), "StaleByJudgeChange")

    # Search/holdout overlap is refused before evaluation.
    expect_refusal("search_holdout_overlap_is_refused",
                   validate_disjoint(subj.SEARCH_VECTORS,
                                     subj.SEARCH_VECTORS.split("----")[0]
                                     + "----\n" + subj.HOLDOUT_VECTORS),
                   "search/holdout overlap")

    # A scaffold cannot realize/promote.
    scaffold = {"posture": "Scaffold", "change_class": "RealizationPreserving",
                "dependencies": []}
    expect_refusal("scaffold_cannot_close_realization",
                   promote_guard(scaffold, set()), "scaffold cannot close realization")

    # A fabricated red status is refused: the failure must carry a real
    # compiler diagnostic from a real nonzero boundary.
    expect_refusal("fake_status_red_is_refused",
                   admit_failure_evidence("status = red", 1),
                   "no compiler diagnostic")
    expect_refusal("zero_exit_failure_is_refused",
                   admit_failure_evidence("error[E0308]: mismatched types", 0),
                   "exit code zero")
    if admit_failure_evidence("error[E0308]: mismatched types", 1) is not None:
        findings.append("a genuine compiler failure was refused as evidence")

    # The bundle verifier is two-sided: a tampered judge root is refused by
    # the REAL receiptcheck with a named finding, and a receiptcheck whose
    # judge-digest guard is blinded admits the identical tamper -- silence
    # the battery must catch.
    rc, err = sb.build_receiptcheck(paths["rustc"], paths["target_triple"], work)
    if rc is None:
        findings.append(f"hostile verifier battery unavailable: {err}")
        return findings
    tampered = work / "judge-tampered"
    shutil.copytree(paths["judge"], tampered)
    victim = tampered / "policy.txt"
    victim.write_text(victim.read_text(encoding="utf-8") + "# tampered\n",
                      encoding="utf-8", newline="\n")
    ok, out = sb.campaign_verify(rc, paths["bundle"], tampered, paths["envelope"],
                                 paths["source_commit"], paths["nursery"],
                                 paths["evidence_root"])
    if ok:
        findings.append("receiptcheck ADMITTED a bundle over a tampered judge root")
    elif "judge-root digest" not in out:
        findings.append("tampered judge refused for the wrong reason: "
                        f"{out.strip()[:160]}")
    neutered, err = sb.neutered_receiptcheck(
        paths["rustc"], paths["target_triple"], work,
        "if recomputed.render() != doc.judge_root_digest {",
        "if false {")
    if neutered is None:
        findings.append(f"verifier neuter did not build: {err}")
    else:
        ok, _out = sb.campaign_verify(neutered, paths["bundle"], tampered,
                                      paths["envelope"], paths["source_commit"],
                                      paths["nursery"], paths["evidence_root"])
        if not ok:
            findings.append("the blinded verifier still refused; the neuter "
                            "proves nothing about the judge-digest guard")

    # E7 perimeter probes, each on COPIES of the real nursery/evidence so the
    # rehearsal's own artifacts stay untouched.
    def verify_with(nursery, bundle=None):
        return sb.campaign_verify(rc, bundle or paths["bundle"], paths["judge"],
                                  paths["envelope"], paths["source_commit"],
                                  nursery, paths["evidence_root"])

    # (a) One tampered nursery-manifest byte is refused (byte identity with
    # the bundle-embedded manifest), and a verifier whose identity guard is
    # blinded admits the identical tamper -- the second new neuter.
    n_tampered = work / "nursery-tampered"
    shutil.copytree(paths["nursery"], n_tampered)
    victim = sorted(n_tampered.glob("*/manifest"))[0]
    text = victim.read_text(encoding="utf-8")
    flipped = text[:-2] + ("0" if text[-2] != "0" else "1") + "\n"
    victim.write_text(flipped, encoding="utf-8", newline="\n")
    _ok, out = verify_with(n_tampered)
    expect_refusal("nursery_manifest_tamper_is_refused", out,
                   "does not match its bundle-embedded manifest")
    blinded, err = sb.neutered_receiptcheck(
        paths["rustc"], paths["target_triple"], work,
        "if nursery_text != embedded_text {",
        "if nursery_text != embedded_text && false {")
    if blinded is None:
        findings.append(f"manifest-identity neuter did not build: {err}")
    else:
        ok, _out = sb.campaign_verify(blinded, paths["bundle"], paths["judge"],
                                      paths["envelope"], paths["source_commit"],
                                      n_tampered, paths["evidence_root"])
        if not ok:
            findings.append("the blinded verifier still refused the manifest "
                            "tamper; the neuter proves nothing about the "
                            "byte-identity guard")

    # (b) An unreferenced authority-looking file in the nursery is refused.
    n_rogue = work / "nursery-rogue"
    shutil.copytree(paths["nursery"], n_rogue)
    (n_rogue / "rogue.manifest").write_text("BATPAK-CANDIDATE-MANIFEST/2\n",
                                            encoding="utf-8", newline="\n")
    _ok, out = verify_with(n_rogue)
    expect_refusal("unreferenced_authority_file_is_refused", out,
                   "authority-looking")

    # (c) Deleting one referenced receipt (a promotion) leaves its terminal
    # claim unresolved and is refused.
    n_gone = work / "nursery-receipt-gone"
    shutil.copytree(paths["nursery"], n_gone)
    victim = next((p for p in sorted(n_gone.glob("*/receipts/*.receipt"))
                   if "\nkind promotion\n" in p.read_text(encoding="utf-8")),
                  None)
    if victim is None:
        findings.append("no promotion receipt exists to delete; the perimeter "
                        "probe cannot run")
    else:
        victim.unlink()
        _ok, out = verify_with(n_gone)
        expect_refusal("deleted_promotion_receipt_is_refused", out,
                       "not represented by a promotion receipt")

    # (d) Rewriting the frontier section to claim Qualified for the scaffold
    # candidate is refused by the Scaffold+Qualified coherence law.
    bundle_text = Path(paths["bundle"]).read_text(encoding="utf-8")
    scaffold_id = None
    current = None
    for line in bundle_text.splitlines():
        if line.startswith("candidate-id "):
            current = line.split(" ")[1]
        if line == "realization-posture Scaffold":
            scaffold_id = current
    if scaffold_id is None:
        findings.append("no scaffold candidate exists in the bundle; the "
                        "coherence probe cannot run")
    else:
        forged = work / "bundle-scaffold-qualified"
        old = f"frontier {scaffold_id} state=Unqualified"
        if old not in bundle_text:
            findings.append("the scaffold candidate's frontier line is not "
                            "Unqualified; the coherence probe cannot run")
        else:
            forged.write_text(
                bundle_text.replace(old, f"frontier {scaffold_id} state=Qualified"),
                encoding="utf-8", newline="\n")
            _ok, out = verify_with(paths["nursery"], bundle=forged)
            expect_refusal("scaffold_qualified_frontier_is_refused", out,
                           "Scaffold candidate")

    # E7-closeout RECEIPT/3 probes: the strict verifier is proven two-sided
    # on the new laws, on COPIES of the real nursery/bundle.
    def receipt_with(nursery: Path, needle: str) -> Path | None:
        return next((p for p in sorted(nursery.glob("*/receipts/*.receipt"))
                     if needle in p.read_text(encoding="utf-8")), None)

    def readdress(victim: Path, text: str) -> None:
        victim.unlink()
        (victim.parent / (sb.sha_hex(text) + ".receipt")).write_text(
            text, encoding="utf-8", newline="\n")

    # (e) A forged receipt source-frontier is refused (the receipt and the
    # manifest must name the SAME commitment), and a verifier whose equality
    # guard is blinded admits the identical forgery -- the closeout neuter.
    n_forged = work / "nursery-frontier-forged"
    shutil.copytree(paths["nursery"], n_forged)
    victim = receipt_with(n_forged, "\nkind escalation\n")
    if victim is None:
        findings.append("no escalation receipt exists to forge; the "
                        "source-frontier probe cannot run")
    else:
        text = victim.read_text(encoding="utf-8")
        value = re.search(r"^source-frontier ([0-9a-f]{64})$", text, re.M).group(1)
        flipped = value[:-1] + ("0" if value[-1] != "0" else "1")
        readdress(victim, text.replace(f"source-frontier {value}",
                                       f"source-frontier {flipped}"))
        _ok, out = verify_with(n_forged)
        expect_refusal("forged_source_frontier_is_refused", out,
                       "must name the same commitment")
        blinded, err = sb.neutered_receiptcheck(
            paths["rustc"], paths["target_triple"], work,
            "if receipt_frontier != candidate.source_frontier {",
            "if false {")
        if blinded is None:
            findings.append(f"source-frontier neuter did not build: {err}")
        else:
            ok, _out = sb.campaign_verify(blinded, paths["bundle"], paths["judge"],
                                          paths["envelope"], paths["source_commit"],
                                          n_forged, paths["evidence_root"])
            if not ok:
                findings.append("the blinded verifier still refused the forged "
                                "source-frontier; the neuter proves nothing "
                                "about the equality guard")

    # (f) A contradictory second terminal receipt WITHOUT supersedes-receipt
    # is refused (implicit terminal precedence is rejected).
    n_second = work / "nursery-second-terminal"
    shutil.copytree(paths["nursery"], n_second)
    victim = receipt_with(n_second, "\nkind refusal\n")
    if victim is None:
        findings.append("no refusal receipt exists; the second-terminal probe "
                        "cannot run")
    else:
        text = victim.read_text(encoding="utf-8")
        cause = re.search(r"^cause (\S+)$", text, re.M).group(1)
        other = next(c for c in ("compile-refusal", "unrealized-obligation-open")
                     if c != cause)
        second = text.replace(f"cause {cause}", f"cause {other}", 1)
        (victim.parent / (sb.sha_hex(second) + ".receipt")).write_text(
            second, encoding="utf-8", newline="\n")
        _ok, out = verify_with(n_second)
        expect_refusal("second_terminal_without_supersedes_is_refused", out,
                       "un-superseded terminal receipts")

    # (g) Deleting a receipt a promotion's target-evidence references leaves
    # the promotion denominator unrecomputable and is refused.
    n_ref = work / "nursery-target-evidence-gone"
    shutil.copytree(paths["nursery"], n_ref)
    victim = receipt_with(n_ref, "\nkind promotion\n")
    ref = victim and re.search(r"^target-evidence \S+ ([0-9a-f]{64})",
                               victim.read_text(encoding="utf-8"), re.M)
    if not ref:
        findings.append("no promotion target-evidence reference exists; the "
                        "denominator probe cannot run")
    else:
        (victim.parent / (ref.group(1) + ".receipt")).unlink()
        _ok, out = verify_with(n_ref)
        expect_refusal("deleted_target_evidence_ref_is_refused", out,
                       "resolves to no receipt in")

    # (h) Omitting one Dependency closure edge whose manifest fact stands is
    # refused by the NEW fact->edge completeness sweep.
    edge = re.search(r"^edge [0-9a-f]{64} [0-9a-f]{64} Dependency$",
                     bundle_text, re.M)
    if edge is None:
        findings.append("no Dependency closure edge exists; the fact->edge "
                        "probe cannot run")
    else:
        forged = work / "bundle-edge-omitted"
        forged.write_text(bundle_text.replace(edge.group(0) + "\n", "", 1),
                          encoding="utf-8", newline="\n")
        _ok, out = verify_with(paths["nursery"], bundle=forged)
        expect_refusal("omitted_closure_edge_is_refused", out,
                       "closure omits the Dependency edge")

    # E7-closeout release-seal (CL-10) probes: the CandidatePromotionSet seal
    # binds the canonical, sorted set of promoted CandidateId VALUES, and the
    # verifier RE-DERIVES that set from the effective, un-superseded promotion
    # receipts and requires exact set + canonical-order equality. The envelope
    # lives inside the evidence root (exempt only via --envelope), so each
    # tamper runs on a COPY of the evidence root with the forged envelope
    # written in place; the nursery stays the real one.
    ev_seal = work / "evidence-seal"
    shutil.copytree(paths["evidence_root"], ev_seal)
    seal_bundle = ev_seal / Path(paths["bundle"]).name
    seal_env = ev_seal / Path(paths["envelope"]).name
    env_text = Path(paths["envelope"]).read_text(encoding="utf-8")
    promo_rows = sorted(ln.split(" ", 2)[2] for ln in env_text.splitlines()
                        if ln.startswith("row CandidatePromotionSet "))

    def reseal(new_rows: list[str]) -> str:
        """Replace the CandidatePromotionSet count + rows, every other envelope
        byte preserved."""
        out_lines: list[str] = []
        skipping = False
        for ln in env_text.splitlines():
            if ln.startswith("seal CandidatePromotionSet rows "):
                out_lines.append(f"seal CandidatePromotionSet rows {len(new_rows)}")
                out_lines.extend(f"row CandidatePromotionSet {r}" for r in new_rows)
                skipping = True
                continue
            if skipping and ln.startswith("row CandidatePromotionSet "):
                continue
            skipping = False
            out_lines.append(ln)
        return "".join(line + "\n" for line in out_lines)

    def verify_seal(text: str):
        seal_env.write_text(text, encoding="utf-8", newline="\n")
        return sb.campaign_verify(rc, seal_bundle, paths["judge"], seal_env,
                                  paths["source_commit"], paths["nursery"], ev_seal)

    if len(promo_rows) < 2:
        findings.append("the release seal names fewer than two promoted "
                        "candidates; the CandidatePromotionSet probes cannot run")
    else:
        # (a) OMISSION: drop one promoted id -> the missing-promoted finding,
        # and a verifier whose set-equality guard is blinded ADMITS the tamper.
        omitted = promo_rows[1:]
        _ok, out = verify_seal(reseal(omitted))
        expect_refusal("seal_omits_promoted_candidate_is_refused", out,
                       "omits promoted candidate")
        seal_env.write_text(reseal(omitted), encoding="utf-8", newline="\n")
        blinded, err = sb.neutered_receiptcheck(
            paths["rustc"], paths["target_triple"], work,
            "if !rows_set.contains(id) {",
            "if !rows_set.contains(id) && false {")
        if blinded is None:
            findings.append(f"promotion-set neuter did not build: {err}")
        else:
            ok, _out = sb.campaign_verify(blinded, seal_bundle, paths["judge"],
                                          seal_env, paths["source_commit"],
                                          paths["nursery"], ev_seal)
            if not ok:
                findings.append("the blinded verifier still refused the omitted "
                                "promotion set; the neuter proves nothing about "
                                "the set-equality guard")

        # (b) INJECTION: add a token that is no bundle candidate (a stray
        # receipt-address-shaped id) -> the no-bundle-candidate finding. `f`*64
        # sorts after every real id, so the canonical order stays intact and
        # the set-membership guard is what bites.
        stray = "f" * 64
        _ok, out = verify_seal(reseal(promo_rows + [stray]))
        expect_refusal("seal_injects_noncandidate_is_refused", out,
                       "is no bundle candidate")

        # (c) SUBSTITUTION: swap a promoted id for a different (unpromoted)
        # candidate id -> the unpromoted-candidate finding.
        cand_ids = [ln.split(" ")[1] for ln in bundle_text.splitlines()
                    if ln.startswith("candidate-begin ")]
        unpromoted = sorted(set(cand_ids) - set(promo_rows))
        if not unpromoted:
            findings.append("every candidate is promoted; the substitution "
                            "probe cannot run")
        else:
            substituted = sorted(set(promo_rows[1:]) | {unpromoted[0]})
            _ok, out = verify_seal(reseal(substituted))
            expect_refusal("seal_substitutes_unpromoted_is_refused", out,
                           "names unpromoted candidate")

        # (d) DUPLICATE: repeat a promoted id -> the duplicate finding (the
        # duplicate guard bites before the ordering guard).
        _ok, out = verify_seal(reseal(promo_rows + [promo_rows[0]]))
        expect_refusal("seal_duplicate_id_is_refused", out,
                       "duplicates candidate")

        # (e) NONCANONICAL: the correct set in the wrong order -> the
        # canonical-order finding.
        unsorted = [promo_rows[1], promo_rows[0], *promo_rows[2:]]
        _ok, out = verify_seal(reseal(unsorted))
        expect_refusal("seal_noncanonical_order_is_refused", out,
                       "not in canonical")
    return findings
