"""Manifest ordering, stale-vocabulary, legacy parity, BatQL operator surfaces,
the generated-view registry, inventory mirrors, and exact ledgers."""
from __future__ import annotations
import importlib.util
import shutil
import tempfile
from pathlib import Path
from .core import (
    HERE,
    _regen_operator_blocks,
    gate_sandbox,
    isolated_tree,
    must_replace,
)


def test_manifest_ordering(freeze) -> list[str]:
    findings: list[str] = []
    names = ["A.md", "a-child.md", "Z.md", "nested/Á.md", "nested/z.md"]
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for name in names:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode("utf-8"))
        rendered = freeze.render(root).decode("utf-8").splitlines()
    order = [line.split("  ", 1)[1] for line in rendered]
    expected = sorted(names, key=lambda value: value.encode("utf-8"))
    if order != expected:
        findings.append(f"ordering is not UTF-8-byte canonical: {order} != {expected}")
    return findings


def test_casefold_collision(audit) -> list[str]:
    findings: list[str] = []
    collisions = audit.casefold_collisions(["Foo.md", "foo.md", "bar.md"])
    if [("Foo.md", "foo.md")] != collisions:
        findings.append(f"expected Foo.md/foo.md collision, got {collisions}")
    unrelated = audit.casefold_collisions(["a.md", "b.md", "nested/a.md"])
    if unrelated:
        findings.append(f"false collision reported on distinct paths: {unrelated}")
    return findings


def test_stale_vocabulary(audit) -> list[str]:
    """Hostile fixtures for the decision-derived stale-vocabulary matcher."""
    findings: list[str] = []
    amap = {"filebat": ("DEC-003", frozenset({"DecisionLedger", "LegacyEvidence"}), "FileBat")}

    def scan(text: str, context: str) -> list[str]:
        out: list[str] = []
        audit.scan_stale_occurrences("synthetic.md", text, context, amap, out)
        return out

    if not scan("FileBat is superseded", "OrdinaryAuthoritative"):
        findings.append("stale_alias_in_authoritative_doc_is_rejected FAILED")
    if not scan("let handle = FileBat::open();", "ProductionSource"):
        findings.append("stale_alias_in_production_source_is_rejected FAILED")
    if not scan("FileBat [STALE-REF: DEC-011]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_wrong_decision_reference_is_rejected FAILED")
    if scan("FileBat [STALE-REF: DEC-003]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_killed_decision_reference_is_allowed FAILED")
    if not scan("plain text [STALE-REF: DEC-999]", "OrdinaryAuthoritative"):
        findings.append("unknown_stale_reference_is_rejected FAILED")
    if scan("FileBat appears in the ledger", "DecisionLedger"):
        findings.append("stale_alias_in_allowed_context_passes FAILED")
    return findings


def test_stale_derivation(audit, root) -> list[str]:
    """The matcher is derived from spec/dispositions.rs, not a hand list."""
    findings: list[str] = []
    problems: list[str] = []
    amap = audit.parse_stale_vocabulary(root, problems)
    if problems:
        findings.append(f"parse_stale_vocabulary reported problems on the real seed: {problems}")
    if len(amap) < 20:
        findings.append(f"generated_stale_table_must_match_decisions: derived only {len(amap)} aliases")
    owner = amap.get("filebat")
    if not owner or owner[0] != "DEC-003":
        findings.append("FileBat alias is not derived to its killing decision DEC-003")
    if "made-up-stale-term" in amap:
        findings.append("alias_removed_from_decision_disappears_from_all_views FAILED")

    # 5.5E2: the context vocabulary is closed in BOTH directions — the enum
    # declares exactly what the scanner speaks, and the two never-permissive
    # defaults cannot be allow-listed by a row.
    def context_probe(name, old, new, needle):
        tmp = gate_sandbox([("spec/dispositions.rs", old, new)])
        try:
            problems: list[str] = []
            audit.parse_stale_vocabulary(tmp, problems)
            if not any(needle in p for p in problems):
                findings.append(f"{name} FAILED (wanted {needle!r}, got {problems!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    context_probe(
        "allow_listing_production_source_is_rejected",
        "stale_allowed_contexts: &[StaleContext::DecisionLedger, StaleContext::RejectionRecord, "
        'StaleContext::SupersessionGuide, StaleContext::LegacyEvidence], replacement_contract: '
        'Some(".fbat remains the BatPak-owned journal format")',
        "stale_allowed_contexts: &[StaleContext::DecisionLedger, StaleContext::RejectionRecord, "
        "StaleContext::SupersessionGuide, StaleContext::LegacyEvidence, "
        'StaleContext::ProductionSource], replacement_contract: '
        'Some(".fbat remains the BatPak-owned journal format")',
        "allow-lists the default context ProductionSource")
    context_probe(
        "dropping_a_default_context_variant_is_rejected",
        "    /// DEFAULT context for every other authoritative surface. Never permissive.\n"
        "    OrdinaryAuthoritative,\n",
        "",
        "the scanner's closed vocabulary")
    context_probe(
        "reintroducing_a_dead_context_variant_is_rejected",
        "    MigrationCompatibility,\n",
        "    MigrationCompatibility,\n    TestFixture,\n",
        "the scanner's closed vocabulary")
    return findings


def test_legacy_manifest_parity(audit) -> list[str]:
    """Hostile fixtures for the D-1 declaration-parity denominator.

    The denominator is measured against declared invariant IDs, so a structural
    key that merely resembles an invariant name (the generated INV-CATALOG
    marker) must never be counted as a declaration.
    """
    findings: list[str] = []
    base = ["INV-A", "INV-B", "INV-C"]
    check = audit.legacy_manifest_findings

    if check(base, base, 3):
        findings.append("exact_declaration_parity_should_pass FAILED")
    if not check(base + ["INV-CATALOG"], base, 3):
        findings.append("structural_INV_token_counted_as_declaration_is_rejected FAILED")
    if not check(base, ["INV-A", "INV-B"], 3):
        findings.append("declared_invariant_without_coverage_is_rejected FAILED")
    if not check(base, base + ["INV-GHOST"], 3):
        findings.append("coverage_for_undeclared_invariant_is_rejected FAILED")
    if not check(base, base, 4):
        findings.append("denominator_count_mismatch_is_rejected FAILED")
    if not check(base + ["INV-A"], base, 3):
        findings.append("duplicate_declaration_id_is_rejected FAILED")
    return findings


def _sample_operators() -> list[dict[str, str]]:
    """Resolved operator rows in the typed 5.5E3j shape (canonical/alias from
    OperatorSyntax; no word_surface, symbol_surface, or formatting fields)."""
    base = {
        "semantic_op": "s", "input_sorts": "a", "result_sort": "b", "overflow": "o",
        "exception": "e", "mutation_classes": "m", "numeric_support": "ExactSupported",
    }
    return [
        {**base, "id": "OP-ADD", "class": "Arithmetic", "shape": "SymbolOnly",
         "canonical": "+", "alias": "",
         "arity": "Binary", "fixity": "Infix", "precedence": "60", "semantic_op": "add",
         "typing": "OperatorTypingRule::PercentAdjustment, OperatorTypingRule::SameUnit",
         "associativity": "Left", "exactness": "Exact", "spoken": "plus"},
        {**base, "id": "OP-EQ", "class": "Comparison", "shape": "WordWithSymbolAlias",
         "canonical": "IS", "alias": "=",
         "arity": "Binary", "fixity": "Infix", "precedence": "50",
         "typing": "OperatorTypingRule::SameSortComparison",
         "associativity": "NonAssociative", "exactness": "Exact", "spoken": "is"},
    ]


def test_batql(audit, project) -> list[str]:
    """Named hostile fixtures for the BatQL operator/grammar/proof surfaces."""
    findings: list[str] = []
    ops = _sample_operators()
    companion = (HERE.parent / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # 1 & 2: proof-disposition axis must keep the exact five states
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("missing_legacy_weak_on_authoritative_surface_is_rejected")
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("legacy_weak_collapsed_by_projection_is_rejected")

    # 3 & 4: grammar reference closure and unique ownership
    undefined, _, _ = audit.batql_grammar_closure_findings(["foo\n  := bar baz"])
    if not undefined:
        fail("undefined_grammar_leaf_is_rejected")
    dup, _, _ = audit.batql_grammar_closure_findings(['foo\n  := "x"\n\nfoo\n  := "y"'])
    if not any("more than one owning" in item for item in dup):
        fail("duplicate_grammar_owner_is_rejected")

    # 5: MATCH requires an initial step plus at least one THEN step
    if "match_step (THEN match_step)+" not in companion or "match_step (THEN match_step)*" in companion:
        fail("one_step_match_is_rejected")

    # 6: kernel declaration clauses appear once, in canonical order
    order = ["FROM KERNEL kernel_ref", "purity_clause", "determinism_clause", "capability_clause", "cost_clause"]
    positions = [companion.find(token) for token in order]
    if -1 in positions or positions != sorted(positions):
        fail("reordered_or_duplicated_kernel_clauses_are_rejected")

    # 7 & 8 & 9: operator fact discipline
    if not any("duplicate OperatorId" in x for x in audit.batql_operator_fact_findings(ops + [dict(ops[0])])):
        fail("duplicate_operator_id_is_rejected")
    clash = {**ops[0], "id": "OP-ADD-CLONE"}
    if not any("claimed by" in x for x in audit.batql_operator_fact_findings([ops[0], clash])):
        fail("ambiguous_token_plus_fixity_ownership_is_rejected")
    missing_field = {**ops[0], "spoken": ""}
    if not any("missing field spoken" in x for x in audit.batql_operator_fact_findings([missing_field])):
        fail("missing_required_operatorspec_field_is_rejected")

    # 10, 11, 12: projection parity is sensitive to alteration, missing, and extra rows
    expected = audit.batql_render_catalog(ops)
    if expected == expected.replace("Exact", "EXACT", 1):
        fail("altered_generated_operator_projection_is_rejected")
    if expected == audit.batql_render_catalog([ops[0]]):
        fail("missing_generated_operator_row_is_rejected")
    if expected == audit.batql_render_catalog(ops + [{**ops[0], "id": "OP-XTRA"}]):
        fail("extra_generated_operator_row_is_rejected")

    # 13: canonical order comes from OperatorId::ALL, so generated output
    # FOLLOWS row order — a reorder is visible, never silently re-sorted by
    # a Python ranking table (5.5E3j; the CLASS_RANK maps are dead).
    if audit.batql_render_catalog(ops) == audit.batql_render_catalog(list(reversed(ops))):
        fail("generated_output_follows_operator_id_all_order")

    # Typed legality rules (5.5E1): the rules are the operand/result authority,
    # their placement is law, and the two phantom sorts stay dead.
    if not any("declares no typing rules" in x
               for x in audit.batql_operator_fact_findings([{**ops[0], "typing": ""}])):
        fail("operator_without_typing_rules_is_rejected")
    if not any("only subtraction" in x for x in audit.batql_operator_fact_findings(
            [{**ops[0], "typing": "OperatorTypingRule::WallObservationDifference"}])):
        fail("wall_difference_outside_subtraction_is_rejected")
    if not any("must carry exactly" in x for x in audit.batql_operator_fact_findings(
            [{**ops[1], "typing": "OperatorTypingRule::TruthUnary"}])):
        fail("comparison_with_foreign_typing_rule_is_rejected")
    if audit.phantom_sort_findings(HERE.parent):
        fail("phantom_sort_baseline_is_silent")
    with isolated_tree() as tmp:
        p = tmp / "companion/BATQL_LANGUAGE.md"
        p.write_text(p.read_text(encoding="utf-8")
                     + "\nDecimalMoney is back.\n", encoding="utf-8")
        if not any("phantom sort" in x for x in audit.phantom_sort_findings(tmp)):
            fail("a_phantom_sort_cannot_reenter_the_corpus")
    # A mutated rule regenerates a DIFFERENT matrix: the block is semantic, not
    # decorative. Dropping the percent-difference rule must change the render.
    stripped = {**ops[0], "id": "OP-SUB", "canonical": "-", "semantic_op": "subtract",
                "typing": "OperatorTypingRule::SameUnit"}
    ruled = {**stripped,
             "typing": "OperatorTypingRule::PercentDifference, OperatorTypingRule::SameUnit"}
    if audit.batql_render_typing([stripped]) == audit.batql_render_typing([ruled]):
        fail("typing_rule_mutation_changes_the_generated_matrix")

    # 14: the independent generator and auditor projections must agree
    if (
        project.render_catalog(ops) != audit.batql_render_catalog(ops)
        or project.render_surfaces(ops) != audit.batql_render_surfaces(ops)
        or project.render_grammar(ops) != audit.batql_render_grammar(ops)
        or project.render_projection(ops) != audit.batql_render_projection(ops)
        or project.render_numeric(ops) != audit.batql_render_numeric(ops)
    ):
        fail("generator_auditor_disagreement_turns_gate_red")

    # 15: FLOOR/CEILING without the language-change record turns the gate red
    modes = "rounding_mode\n  := HALF_EVEN | HALF_UP | DOWN | UP | FLOOR | CEILING\n"
    if audit.batql_language_change_findings(modes + "\nLanguage-change record (BatQL V1)\n"):
        fail("language_change_record_present_passes")
    if not audit.batql_language_change_findings(modes):
        fail("floor_ceiling_without_language_change_record_is_rejected")

    return findings


def _sample_operator_model() -> dict:
    """A minimal coherent typed operator model (one operator per class) in the
    exact shape audit.batql_parse_operator_model produces."""
    row = {
        "semantic_op": "s", "arity": "Binary", "fixity": "Infix",
        "precedence": "50", "associativity": "Left",
        "typing": "OperatorTypingRule::SameSortComparison", "input_sorts": "a",
        "result_sort": "b", "exactness": "Exact", "overflow": "o",
        "exception": "e", "spoken": "sp", "mutation_classes": "m",
        "numeric_support": "ExactSupported",
    }
    ids = ("Multiply", "Equal", "Not")
    return {
        "src": "",
        "id_type": "OperatorId",
        "id_variants": list(ids), "id_all": list(ids),
        "id_token": {"Multiply": "OP-MUL", "Equal": "OP-EQ", "Not": "OP-NOT"},
        "id_owner": {v: "BP-BATQL-LANGUAGE-1" for v in ids},
        "id_basis": {v: "DEC-060" for v in ids},
        "word_type": "OperatorWordSurface",
        "word_variants": ["Is", "Not"], "word_all": ["Is", "Not"],
        "word_token": {"Is": "IS", "Not": "NOT"},
        "word_owner": {"Is": "BP-BATQL-LANGUAGE-1", "Not": "BP-BATQL-LANGUAGE-1"},
        "word_basis": {"Is": "DEC-060", "Not": "DEC-060"},
        "symbol_type": "OperatorSymbolSurface",
        "symbol_variants": ["Multiply", "Equal"], "symbol_all": ["Multiply", "Equal"],
        "symbol_token": {"Multiply": "*", "Equal": "="},
        "symbol_owner": {"Multiply": "BP-BATQL-LANGUAGE-1", "Equal": "BP-BATQL-LANGUAGE-1"},
        "symbol_basis": {"Multiply": "DEC-060", "Equal": "DEC-060"},
        "rows": [
            {**row, "variant": "Multiply", "class": "Arithmetic",
             "shape": "SymbolOnly", "inner": "OperatorSymbolSurface::Multiply",
             "semantic_op": "multiply",
             "typing": "OperatorTypingRule::DimensionalByDimensionless"},
            {**row, "variant": "Equal", "class": "Comparison",
             "shape": "WordWithSymbolAlias",
             "inner": "OperatorWordSurface::Is, OperatorSymbolSurface::Equal",
             "associativity": "NonAssociative"},
            {**row, "variant": "Not", "class": "Logical", "shape": "WordOnly",
             "inner": "OperatorWordSurface::Not", "fixity": "Prefix",
             "arity": "Unary", "typing": "OperatorTypingRule::TruthUnary",
             "numeric_support": "NotApplicable"},
        ],
    }


def test_operator_surfaces(audit, project) -> list[str]:
    """Named hostile fixtures for the typed operator identity and
    source-surface closure (5.5E3j): OperatorId, the closed word and symbol
    surface inventories, OperatorSyntax, the class/shape law, the DEC-060
    admission boundary, projection provenance, and the retired raw fields."""
    findings: list[str] = []
    root = HERE.parent
    CONTRACTS = {"BP-BATQL-LANGUAGE-1"}
    DECISIONS = {"DEC-060"}

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def mfind(model) -> list[str]:
        return audit.batql_operator_model_findings(model)

    def rfind(rows) -> list[str]:
        return audit.batql_operator_fact_findings(rows)

    def resolved(model) -> list[dict]:
        return audit.batql_resolve_operator_rows(model, [])

    def batql(tmp) -> list[str]:
        out: list[str] = []
        audit.check_batql(tmp, out)
        return out

    def probe(name, edits, validator, needle):
        tmp = gate_sandbox(edits)
        try:
            expect(name, validator(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # The synthetic model itself is lawful; a red baseline would let every
    # hostile below pass for the wrong reason.
    base = _sample_operator_model()
    baseline = mfind(base) + rfind(resolved(base)) \
        + audit.batql_operator_authority_findings(base, CONTRACTS, DECISIONS)
    if baseline:
        fail(f"typed_operator_model_baseline_is_red ({baseline!r})")

    # Identity/row parity in OperatorId::ALL order.
    m = _sample_operator_model()
    m["id_all"] = ["Multiply", "Equal"]
    expect("operator_id_missing_from_all_is_rejected", mfind(m),
           "is declared but missing from OperatorId::ALL")
    m = _sample_operator_model()
    m["rows"] = m["rows"][:2]
    expect("operator_id_without_operator_row_is_rejected", mfind(m),
           "OperatorId::Not has no OperatorSpec row")
    m = _sample_operator_model()
    m["rows"] = m["rows"] + [dict(m["rows"][0])]
    expect("duplicate_operator_id_row_is_rejected", mfind(m),
           "duplicate OperatorSpec row for OperatorId::Multiply")
    m = _sample_operator_model()
    m["rows"] = list(reversed(m["rows"]))
    expect("operator_row_order_must_equal_operator_id_all", mfind(m),
           "OPERATORS row order does not equal OperatorId::ALL")

    # The closed surface inventories.
    m = _sample_operator_model()
    m["word_all"] = ["Is"]
    expect("operator_word_surface_missing_from_all_is_rejected", mfind(m),
           "missing from OperatorWordSurface::ALL")
    m = _sample_operator_model()
    m["symbol_all"] = ["Multiply"]
    expect("operator_symbol_surface_missing_from_all_is_rejected", mfind(m),
           "missing from OperatorSymbolSurface::ALL")
    m = _sample_operator_model()
    for key in ("word_variants", "word_all"):
        m[key] = m[key] + ["Or"]
    m["word_token"]["Or"] = "OR"
    m["word_owner"]["Or"] = "BP-BATQL-LANGUAGE-1"
    m["word_basis"]["Or"] = "DEC-060"
    expect("orphan_operator_word_surface_is_rejected", mfind(m),
           "word surface Or is adopted by no OperatorSpec row")
    m = _sample_operator_model()
    for key in ("symbol_variants", "symbol_all"):
        m[key] = m[key] + ["Add"]
    m["symbol_token"]["Add"] = "+"
    m["symbol_owner"]["Add"] = "BP-BATQL-LANGUAGE-1"
    m["symbol_basis"]["Add"] = "DEC-060"
    expect("orphan_operator_symbol_surface_is_rejected", mfind(m),
           "symbol surface Add is adopted by no OperatorSpec row")
    m = _sample_operator_model()
    m["word_token"]["Not"] = "IS"
    expect("duplicate_operator_word_token_is_rejected", mfind(m),
           "duplicate OperatorWordSurface token 'IS'")
    m = _sample_operator_model()
    m["symbol_token"]["Equal"] = "*"
    expect("duplicate_operator_symbol_token_is_rejected", mfind(m),
           "duplicate OperatorSymbolSurface token '*'")
    m = _sample_operator_model()
    m["word_token"]["Not"] = ""
    expect("empty_operator_word_token_is_rejected", mfind(m),
           "OperatorWordSurface::Not declares an empty token")
    m = _sample_operator_model()
    m["symbol_token"]["Equal"] = ""
    expect("empty_operator_symbol_token_is_rejected", mfind(m),
           "OperatorSymbolSurface::Equal declares an empty token")
    m = _sample_operator_model()
    m["word_token"]["Not"] = "!"
    expect("punctuation_cannot_enter_the_word_inventory", mfind(m),
           "punctuation can never enter the word inventory")
    m = _sample_operator_model()
    m["symbol_token"]["Equal"] = "EQ"
    expect("a_word_cannot_enter_the_symbol_inventory", mfind(m),
           "a word can never enter the symbol inventory")

    # Owner and basis resolution across all three families.
    m = _sample_operator_model()
    m["id_owner"]["Multiply"] = "BP-NOBODY-1"
    expect("operator_id_with_unknown_owner_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "names owner BP-NOBODY-1, which no declared contract owns")
    m = _sample_operator_model()
    m["word_owner"]["Is"] = "BP-NOBODY-1"
    expect("operator_surface_with_unknown_owner_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "OperatorWordSurface::Is names owner BP-NOBODY-1")
    m = _sample_operator_model()
    m["id_basis"]["Multiply"] = "DEC-999"
    expect("operator_id_with_dangling_basis_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "admission basis DEC-999, which no declared decision owns")
    m = _sample_operator_model()
    m["symbol_basis"]["Equal"] = "DEC-999"
    expect("operator_surface_with_dangling_basis_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "OperatorSymbolSurface::Equal names admission basis DEC-999")

    # The class/shape law is total.
    rows = resolved(_sample_operator_model())
    arithmetic, comparison, logical = rows[0], rows[1], rows[2]
    expect("arithmetic_operator_must_be_symbol_only",
           rfind([{**arithmetic, "shape": "WordWithSymbolAlias", "alias": "*",
                   "canonical": "TIMES"}]),
           "its class law requires SymbolOnly")
    expect("comparison_operator_must_be_word_with_symbol_alias",
           rfind([{**comparison, "shape": "WordOnly", "alias": ""}]),
           "its class law requires WordWithSymbolAlias")
    expect("comparison_symbol_cannot_become_canonical",
           rfind([{**comparison, "shape": "SymbolOnly", "canonical": "=", "alias": ""}]),
           "its class law requires WordWithSymbolAlias")
    expect("logical_operator_must_be_word_only",
           rfind([{**logical, "shape": "SymbolOnly", "canonical": "!"}]),
           "its class law requires WordOnly")
    expect("logical_operator_cannot_gain_a_symbol_alias",
           rfind([{**logical, "shape": "WordWithSymbolAlias", "alias": "!"}]),
           "its class law requires WordOnly")

    # Surface/fixity ownership: one surface under one fixity names one
    # operator; the same surface under a DISTINCT fixity is lawful.
    expect("same_surface_and_fixity_cannot_name_two_operators",
           rfind([comparison, {**comparison, "id": "OP-EQ2"}]),
           "claimed by")
    lawful = rfind([{**comparison, "fixity": "Prefix"},
                    {**comparison, "id": "OP-EQ2", "fixity": "Infix"}])
    if any("claimed by" in f for f in lawful):
        fail("same_surface_under_distinct_fixity_is_lawful")

    # A comparison alias stays attached to its one canonical OperatorId.
    m = _sample_operator_model()
    m["id_variants"] = m["id_all"] = m["id_all"] + ["NotEqual"]
    m["id_token"]["NotEqual"] = "OP-NE"
    m["id_owner"]["NotEqual"] = "BP-BATQL-LANGUAGE-1"
    m["id_basis"]["NotEqual"] = "DEC-060"
    m["word_variants"] = m["word_all"] = m["word_all"] + ["IsNot"]
    m["word_token"]["IsNot"] = "IS NOT"
    m["word_owner"]["IsNot"] = "BP-BATQL-LANGUAGE-1"
    m["word_basis"]["IsNot"] = "DEC-060"
    m["rows"] = m["rows"] + [{**m["rows"][1], "variant": "NotEqual",
                              "inner": "OperatorWordSurface::IsNot, OperatorSymbolSurface::Equal"}]
    expect("comparison_alias_must_lower_to_the_same_operator_id", mfind(m),
           "an alias stays attached to one OperatorId")

    # The DEC-060 admission boundary names the typed surface law and the
    # exact V1 comparison pairs.
    probe("dec060_must_name_the_operator_surface_boundary",
          [("spec/dispositions.rs",
            "and OperatorSyntax the closed source-shape law",
            "and a closed source-shape law")],
          batql, "must name the operator surface boundary")
    probe("dec060_must_name_the_exact_comparison_pairs",
          [("spec/dispositions.rs",
            "IS LESS THAN with <,",
            "IS LESS THAN with <>,")],
          batql, "do not equal the typed word/symbol mapping")

    # A comparison-pair swap in the typed source is refused by the DEC-060
    # mapping law even AFTER every projection is regenerated: the hostile
    # must fail for the syntax law, not for a stale block.
    tmp = gate_sandbox([
        ("spec/operators.rs",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsLessThan, OperatorSymbolSurface::LessThan)",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsLessThan, OperatorSymbolSurface::AtMost)"),
        ("spec/operators.rs",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsAtMost, OperatorSymbolSurface::AtMost)",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsAtMost, OperatorSymbolSurface::LessThan)"),
    ])
    try:
        _regen_operator_blocks(project, tmp)
        expect("comparison_pair_swap_is_rejected_after_regeneration", batql(tmp),
               "do not equal the typed word/symbol mapping")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # The formatter law and the spoken fence.
    probe("formatter_must_emit_the_canonical_surface",
          [("companion/BATQL_LANGUAGE.md", "| OP-EQ | `IS` | is |", "| OP-EQ | `=` | is |")],
          batql, "not its canonical surface")
    probe("spoken_projection_cannot_enter_operator_grammar",
          [("companion/BATQL_LANGUAGE.md",
            "logical_operator\n  := AND | OR", "logical_operator\n  := AND | OR | times")],
          batql, "narration is never source syntax")
    probe("arithmetic_spoken_word_cannot_become_source_syntax",
          [("companion/BATQL_LANGUAGE.md",
            'arithmetic_operator\n  := "*" | "/" | "+" | "-"',
            'arithmetic_operator\n  := "*" | "/" | "+" | "-" | plus')],
          batql, "narration is never source syntax")

    # The retired raw fields and the empty-string sentinel may not return.
    probe("raw_word_surface_field_cannot_return",
          [("spec/operators.rs", 'semantic_op: "multiply"',
            'word_surface: "*", semantic_op: "multiply"')],
          batql, "retired raw surface field word_surface may not return")
    probe("raw_symbol_surface_field_cannot_return",
          [("spec/operators.rs", 'semantic_op: "equal"',
            'symbol_surface: "=", semantic_op: "equal"')],
          batql, "retired raw surface field symbol_surface may not return")
    probe("duplicate_formatting_field_cannot_return",
          [("spec/operators.rs", 'spoken: "times"',
            'formatting: "*", spoken: "times"')],
          batql, "retired raw surface field formatting may not return")
    probe("empty_symbol_sentinel_cannot_return",
          [("spec/operators.rs", 'spoken: "or"', 'spoken: ""')],
          batql, "empty-string surface sentinel may not return")

    # Projection drift: each fixture edits ONLY the generated document.
    probe("operator_surface_projection_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "| OP-NE | word-with-symbol | `IS NOT` | `!=` |",
            "| OP-NE | word-with-symbol | `!=` | `IS NOT` |")],
          batql, "generated block OPERATORS-SURFACES does not match")
    probe("operator_catalog_projection_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "| OP-MUL | Arithmetic | Binary |", "| OP-MUL | Arithmetic | Unary |")],
          batql, "generated block OPERATORS-CATALOG does not match")
    probe("operator_grammar_projection_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            'arithmetic_operator\n  := "*"', 'arithmetic_operator\n  := "**"')],
          batql, "generated block OPERATORS-GRAMMAR does not match")

    # Provenance: every operator projection's BEGIN marker names the typed
    # source and the generator byte-exactly.
    probe("operator_projection_source_marker_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "OPERATORS-PROJECTION:BEGIN generated from spec/operators.rs by bootstrap/project.py",
            "OPERATORS-PROJECTION:BEGIN generated from spec/operators.rs by bootstrap/render.py")],
          batql, "OPERATORS-PROJECTION BEGIN marker does not name")
    probe("operator_typing_source_marker_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "OPERATORS-TYPING:BEGIN generated from spec/operators.rs",
            "OPERATORS-TYPING:BEGIN generated from spec/architecture.rs")],
          batql, "OPERATORS-TYPING BEGIN marker does not name")
    probe("operator_numeric_source_marker_drift_is_rejected",
          [("docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md",
            "OPERATORS-NUMERIC:BEGIN generated from spec/operators.rs",
            "OPERATORS-NUMERIC:BEGIN generated from spec/numeric.rs")],
          batql, "OPERATORS-NUMERIC BEGIN marker does not name")

    # The identity closure: the residue row rests on the REAL typed owner,
    # and OperatorId can never re-enter the general identity catalogs.
    tmp = gate_sandbox([("spec/operators.rs", "pub enum OperatorId {",
                         "pub enum OperatorTag {")])
    try:
        expect("operator_id_residue_requires_a_real_typed_owner",
               batql(tmp), "no public typed OperatorId owner")
        expect("operator_id_residue_requires_a_real_typed_owner (wrapper-guard parity)",
               audit.identity_catalog_findings(tmp),
               "EXISTING_TYPED_OWNER_SPELLINGS lists OperatorId, which no spec")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    probe("operator_id_cannot_enter_the_general_identity_catalog",
          [("spec/identities.rs",
            'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("OperatorId", "BP-CRYPTO-SECRET-1")')],
          audit.identity_catalog_findings,
          "duplicates a spelling that already has a typed spec owner")

    # E3j1: the documentary layer may fence operator admission, never deny
    # value existence — DEC-069/docs/37 own first-class Qualified
    # Approximation TODAY. The ordinary parser strips comments, so this law
    # executes over the raw source.
    operators_src = (root / "spec/operators.rs").read_text(encoding="utf-8")
    for stale in ("No Approximate value exists yet.",
                  "No `Approximate` value exists until a profile ships."):
        mutated = must_replace(
            operators_src, "Approximate numeric values already exist as typed",
            stale, "operator_docs_cannot_deny_existing_approximate_values")
        expect("operator_docs_cannot_deny_existing_approximate_values",
               audit.batql_operator_doc_findings(mutated),
               "approximate values exist; no current ordinary operator admits raw")
    expect("operator_docs_cannot_revive_superseded_chronology",
           audit.batql_operator_doc_findings(
               operators_src + "\n// those arrive with 5.5B2\n"),
           "approximate values exist; no current ordinary operator admits raw")
    # GREEN: the corrected module, Exactness, and NumericSupport doctrines
    # agree with DEC-069/docs/37 while every real OperatorSpec row stays
    # exact-only — value vocabulary exists, operator admission does not.
    doc_green = audit.batql_operator_doc_findings(operators_src)
    admitted_rows = [op["id"] for op in resolved(audit.batql_parse_operator_model(root))
                     if op["numeric_support"] == "QualifiedProfileOnly"]
    if doc_green or admitted_rows:
        fail("operator_docs_distinguish_value_existence_from_operator_admission "
             f"({doc_green!r}, {admitted_rows!r})")

    # GREEN structural growth — count-freedom evidence ONLY. A sandbox-only
    # synthetic comparison operator with typed identity, both surfaces, an
    # explicit owner and basis, DEC-060 naming, authored companion adoption,
    # and regenerated projections passes every gate: no frozen numeric count
    # guards the inventory. This does NOT admit the operator into real BatQL.
    tmp = gate_sandbox([
        ("spec/operators.rs", "    Not,\n    And,\n    Or,\n}",
         "    Not,\n    And,\n    Or,\n    Congruent,\n}"),
        ("spec/operators.rs", "        OperatorId::Or,\n    ];",
         "        OperatorId::Or,\n        OperatorId::Congruent,\n    ];"),
        ("spec/operators.rs", '            OperatorId::Or => "OP-OR",\n        }',
         '            OperatorId::Or => "OP-OR",\n            OperatorId::Congruent => "OP-CONG",\n        }'),
        ("spec/operators.rs",
         '            OperatorId::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n        }',
         '            OperatorId::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n            OperatorId::Congruent => ContractId("BP-BATQL-LANGUAGE-1"),\n        }'),
        ("spec/operators.rs",
         '            OperatorId::Or => DecisionId("DEC-060"),\n        }',
         '            OperatorId::Or => DecisionId("DEC-060"),\n            OperatorId::Congruent => DecisionId("DEC-060"),\n        }'),
        ("spec/operators.rs", "    Not,\n    And,\n    Or,\n}",
         "    Not,\n    And,\n    Or,\n    IsRoughly,\n}"),
        ("spec/operators.rs", "        OperatorWordSurface::Or,\n    ];",
         "        OperatorWordSurface::Or,\n        OperatorWordSurface::IsRoughly,\n    ];"),
        ("spec/operators.rs", '            OperatorWordSurface::Or => "OR",\n        }',
         '            OperatorWordSurface::Or => "OR",\n            OperatorWordSurface::IsRoughly => "IS ROUGHLY",\n        }'),
        ("spec/operators.rs",
         '            OperatorWordSurface::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n        }',
         '            OperatorWordSurface::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n            OperatorWordSurface::IsRoughly => ContractId("BP-BATQL-LANGUAGE-1"),\n        }'),
        ("spec/operators.rs",
         '            OperatorWordSurface::Or => DecisionId("DEC-060"),\n        }',
         '            OperatorWordSurface::Or => DecisionId("DEC-060"),\n            OperatorWordSurface::IsRoughly => DecisionId("DEC-060"),\n        }'),
        ("spec/operators.rs", "    MoreThan,\n    AtLeast,\n}",
         "    MoreThan,\n    AtLeast,\n    Congruent,\n}"),
        ("spec/operators.rs", "        OperatorSymbolSurface::AtLeast,\n    ];",
         "        OperatorSymbolSurface::AtLeast,\n        OperatorSymbolSurface::Congruent,\n    ];"),
        ("spec/operators.rs", '            OperatorSymbolSurface::AtLeast => ">=",\n        }',
         '            OperatorSymbolSurface::AtLeast => ">=",\n            OperatorSymbolSurface::Congruent => "~",\n        }'),
        ("spec/operators.rs",
         '            OperatorSymbolSurface::AtLeast => ContractId("BP-BATQL-LANGUAGE-1"),\n        }',
         '            OperatorSymbolSurface::AtLeast => ContractId("BP-BATQL-LANGUAGE-1"),\n            OperatorSymbolSurface::Congruent => ContractId("BP-BATQL-LANGUAGE-1"),\n        }'),
        ("spec/operators.rs",
         '            OperatorSymbolSurface::AtLeast => DecisionId("DEC-060"),\n        }',
         '            OperatorSymbolSurface::AtLeast => DecisionId("DEC-060"),\n            OperatorSymbolSurface::Congruent => DecisionId("DEC-060"),\n        }'),
        ("spec/operators.rs", "numeric_support: NumericSupport::NotApplicable },\n];",
         "numeric_support: NumericSupport::NotApplicable },\n"
         '    OperatorSpec { id: OperatorId::Congruent, class: OperatorClass::Comparison, '
         "syntax: OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsRoughly, "
         'OperatorSymbolSurface::Congruent), semantic_op: "congruent", arity: Arity::Binary, '
         "fixity: Fixity::Infix, precedence: 50, associativity: Associativity::NonAssociative, "
         'typing: &[OperatorTypingRule::SameSortComparison], input_sorts: "exact same-sort pair", '
         'result_sort: "Truth with TypedMargin", exactness: Exactness::Exact, overflow: "none", '
         'exception: "cross-sort comparison is a type error", spoken: "is roughly", '
         'mutation_classes: "precedence, comparison-direction, margin-sign, parenthesis", '
         "numeric_support: NumericSupport::ExactSupported },\n];"),
        ("spec/dispositions.rs", "IS AT LEAST with >=, and NOT AND OR are word-only",
         "IS AT LEAST with >=, IS ROUGHLY with ~, and NOT AND OR are word-only"),
        ("companion/BATQL_LANGUAGE.md",
         "Spoken narration is not parser syntax: the spoken projection narrates the typed tree and is never accepted source.",
         "Spoken narration is not parser syntax: the spoken projection narrates the typed tree and is never accepted source.\n\n"
         "A sandbox-only synthetic comparison `IS ROUGHLY` (compact alias `~`) may be "
         "added here as structural count-freedom evidence."),
    ])
    try:
        _regen_operator_blocks(project, tmp)
        grown = batql(tmp)
        if grown:
            fail(f"synthetic_operator_structural_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    return findings


def test_generated_views(audit, project) -> list[str]:
    """Named hostile fixtures for the generated-view registry (5.5E4a):
    spec/generated_views.rs is the closed denominator of every repository-
    generated view, the projector dispatches only through it, and the
    universal marker census leaves no unregistered block in the wallpaper."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def sandbox(edits):
        # gate_sandbox carries the bootstrap sources since 5.5E4b, so the
        # registry auditor's dispatcher-parity read resolves in every sandbox.
        return gate_sandbox(edits)

    def probe(name, edits, needle, validator=None):
        tmp = sandbox(edits)
        try:
            got = (validator or audit.generated_view_findings)(tmp)
            expect(name, got, needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.generated_view_findings(root):
        fail(f"generated_view_registry_passes_on_the_real_seed "
             f"({audit.generated_view_findings(root)!r})")

    GV = "spec/generated_views/registry.rs"
    PY = "bootstrap/project.py"
    GATES_ROW = "        GeneratedView::GateInventory,\n"

    # Registry structure.
    probe("generated_view_missing_from_all_is_rejected",
          [(GV, GATES_ROW, "")],
          "is declared but missing from GeneratedView::ALL")
    probe("generated_view_without_spec_arm_is_rejected",
          [(GV, "GeneratedView::GateInventory => GeneratedViewSpec {",
            "GeneratedView::GateInventoryRenamed => GeneratedViewSpec {")],
          "GeneratedView::GateInventory has no parseable spec() arm")
    probe("duplicate_generated_view_name_is_rejected",
          [(GV, GATES_ROW, GATES_ROW + GATES_ROW)],
          "GeneratedView::ALL repeats GateInventory")
    probe("generated_view_without_authority_source_is_rejected",
          [(GV, 'authority_sources: &["spec/gates.rs"],', "authority_sources: &[],")],
          "generated view GateInventory names no authority source")
    probe("generated_view_with_missing_authority_source_is_rejected",
          [(GV, 'authority_sources: &["spec/gates.rs"],',
            'authority_sources: &["spec/gates_missing.rs"],')],
          "names authority source spec/gates_missing.rs, which does not exist")
    probe("generated_view_with_missing_static_target_is_rejected",
          [(GV, 'GeneratedViewTarget::Static(&["docs/25_IMPLEMENTATION_GATES.md"])',
            'GeneratedViewTarget::Static(&["docs/25_MISSING.md"])')],
          "targets docs/25_MISSING.md, which does not exist")

    # Surface/target shape.
    probe("embedded_view_without_marker_is_rejected",
          [(GV, 'marker: Some("GATE-INVENTORY"),', "marker: None,")],
          "embedded generated view GateInventory carries no marker")
    probe("standalone_view_with_embedded_marker_is_rejected",
          [(GV,
            'target: GeneratedViewTarget::Static(&["rust-toolchain.toml"]),\n'
            "                surface: GeneratedViewSurface::StandaloneFile,\n"
            "                marker: None,",
            'target: GeneratedViewTarget::Static(&["rust-toolchain.toml"]),\n'
            "                surface: GeneratedViewSurface::StandaloneFile,\n"
            '                marker: Some("RUST-TOOLCHAIN"),')],
          "standalone generated view RustToolchain carries an embedded marker")
    probe("corpus_frontmatter_with_static_target_is_rejected",
          [(GV,
            "target: GeneratedViewTarget::EligibleMarkdownCorpus,\n"
            "                surface: GeneratedViewSurface::CorpusFrontmatter,",
            'target: GeneratedViewTarget::Static(&["docs/00_CONSTITUTION.md"]),\n'
            "                surface: GeneratedViewSurface::CorpusFrontmatter,")],
          "must target the eligible markdown corpus, never static paths")
    probe("duplicate_target_marker_claim_is_rejected",
          [(GV, 'marker: Some("MUTATION-RESULTS"),', 'marker: Some("MUTATION-LANES"),')],
          "both claim marker MUTATION-LANES in docs/12_TESTPAK.md")
    if any("STALE-VOCAB" in f or "StaleVocabulary" in f
           for f in audit.generated_view_findings(root)):
        fail("multi_target_stale_vocabulary_is_lawful")

    # The universal marker census.
    GATES_DOC = "docs/25_IMPLEMENTATION_GATES.md"
    GATES_BEGIN = ("<!-- GATE-INVENTORY:BEGIN generated from spec/gates.rs by "
                   "bootstrap/project.py; do not edit -->")
    probe("unregistered_generated_begin_marker_is_rejected",
          [(GATES_DOC, GATES_BEGIN,
            "<!-- ROGUE-VIEW:BEGIN generated from spec/gates.rs by "
            "bootstrap/project.py; do not edit -->\nrogue\n"
            "<!-- ROGUE-VIEW:END -->\n\n" + GATES_BEGIN)],
          "generated marker ROGUE-VIEW is not a registered generated view")
    probe("orphan_generated_end_marker_is_rejected",
          [(GATES_DOC, GATES_BEGIN, "<!-- ORPHANED:END -->\n\n" + GATES_BEGIN)],
          "ORPHANED has an END with no matching BEGIN")
    probe("mismatched_generated_end_marker_is_rejected",
          [(GATES_DOC, "<!-- GATE-INVENTORY:END -->", "<!-- GATE-INVENTORYX:END -->")],
          "GATE-INVENTORY has a BEGIN with no matching END")
    probe("duplicate_generated_marker_in_one_target_is_rejected",
          [(GATES_DOC, GATES_BEGIN,
            GATES_BEGIN + "\nrogue\n<!-- GATE-INVENTORY:END -->\n\n" + GATES_BEGIN)],
          "generated marker GATE-INVENTORY appears more than once")
    probe("registered_marker_missing_from_target_is_rejected",
          [(GATES_DOC, "GATE-INVENTORY:BEGIN", "GATE-INVENTORYX:BEGIN"),
           (GATES_DOC, "GATE-INVENTORY:END", "GATE-INVENTORYX:END")],
          "registered generated marker GATE-INVENTORY is absent from its target")
    probe("generated_marker_source_drift_is_rejected",
          [(GATES_DOC, "GATE-INVENTORY:BEGIN generated from spec/gates.rs",
            "GATE-INVENTORY:BEGIN generated from spec/architecture.rs")],
          "names source spec/architecture.rs, not its registered authority source")
    probe("generated_marker_generator_drift_is_rejected",
          [(GATES_DOC, "GATE-INVENTORY:BEGIN generated from spec/gates.rs by bootstrap/project.py",
            "GATE-INVENTORY:BEGIN generated from spec/gates.rs by bootstrap/render.py")],
          "names generator bootstrap/render.py, not bootstrap/project.py")

    # Dispatcher parity and the registry bypass fence. The dispatcher literal
    # lives in the project package since SW3c; the shim carries no map.
    PYPKG = "bootstrap/project/__init__.py"
    probe("projector_dispatcher_omission_is_rejected",
          [(PYPKG, '    "GateInventory": render_gate_inventory,\n', "")],
          "registered generated view GateInventory has no projector dispatcher entry")
    probe("unregistered_projector_dispatcher_entry_is_rejected",
          [(PYPKG, '    "GateInventory": render_gate_inventory,\n',
            '    "GateInventory": render_gate_inventory,\n'
            '    "RogueView": render_gate_inventory,\n')],
          "projector dispatcher entry RogueView serializes no registered view")
    probe("manual_projection_plan_cannot_bypass_registry",
          [(PY, '\nif __name__ == "__main__":',
            '\n_bypass = block_pattern("ROGUE-VIEW")\n\nif __name__ == "__main__":')],
          "manual projection plan for 'ROGUE-VIEW'")

    # Standalone provenance.
    probe("guarantee_graph_generated_from_drift_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "generated_from: spec/invariants.rs;",
            "generated_from: spec/wrong.rs;")],
          "not its registry row")
    probe("guarantee_graph_generator_drift_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "generated_by: bootstrap/project.py",
            "generated_by: bootstrap/render.py")],
          "not bootstrap/project.py")
    probe("rust_toolchain_provenance_comment_missing_is_rejected",
          [("rust-toolchain.toml",
            "# generated from spec/toolchain.rs by bootstrap/project.py; do not edit\n[toolchain]",
            "[toolchain]")],
          "missing or drifted provenance comment")

    # Registry self-inclusion and the corpus-membership floor.
    probe("corpus_epoch_membership_cannot_leave_the_registry",
          [(GV, "        GeneratedView::CorpusEpochMembership,\n", "")],
          "CorpusEpochMembership may not leave the registry")
    probe("generated_view_registry_must_include_itself",
          [(GV, "        GeneratedView::GeneratedViewRegistry,\n", "")],
          "must include itself")
    probe("generated_view_registry_projection_drift_is_rejected",
          [("docs/28_SELF_EXPLAINING_REPOSITORY.md",
            "| GateInventory | embedded-block |",
            "| GateInventory | standalone-file |")],
          "generated block GENERATED-VIEW-REGISTRY does not match")
    probe("generated_view_registry_marker_drift_is_rejected",
          [("docs/28_SELF_EXPLAINING_REPOSITORY.md",
            "GENERATED-VIEW-REGISTRY:BEGIN generated from spec/generated_views.rs",
            "GENERATED-VIEW-REGISTRY:BEGIN generated from spec/architecture.rs")],
          "not its registered authority source spec/generated_views.rs")

    # The retired hybrid fence and the projector's documentary honesty.
    probe("contract_kind_hybrid_fence_cannot_return",
          [("docs/06_MACBAT.md",
            "What each admitting law means, in authored prose",
            "The admitted kinds, each with its admitting law:\n\n```text\n"
            "Error LEG-047 stale hybrid\n```\n\n"
            "What each admitting law means, in authored prose")],
          "hybrid admitted-kinds fence may not return",
          validator=audit.contract_kind_findings)
    probe("projector_docs_cannot_deny_guarantee_graph_generation",
          [(PY, "project.py never generates decision text itself.",
            "project.py never generates decision text itself, and does not "
            "generate the Guarantee Graph.")],
          "denies generating the Guarantee Graph")
    probe("projector_status_cannot_report_only_operator_blocks",
          [(PY, '\nif __name__ == "__main__":',
            '\n_legacy_banner = "operator blocks current"\n\nif __name__ == "__main__":')],
          "status reports only operator blocks")

    # GREEN structural registry growth — evidence ONLY. A sandbox-only future
    # view with a fresh source, a temporary target, a complete arm, and a
    # dispatcher entry stays green after regeneration through its OWN
    # projector: no frozen numeric count guards the registry. This does not
    # justify a real new generated surface.
    tmp = sandbox([
        (GV, "    GeneratedViewRegistry,\n}",
         "    GeneratedViewRegistry,\n    FutureLedger,\n}"),
        (GV, "        GeneratedView::GeneratedViewRegistry,\n    ];",
         "        GeneratedView::GeneratedViewRegistry,\n"
         "        GeneratedView::FutureLedger,\n    ];"),
        (GV, '            GeneratedView::GeneratedViewRegistry => "GeneratedViewRegistry",\n        }',
         '            GeneratedView::GeneratedViewRegistry => "GeneratedViewRegistry",\n'
         '            GeneratedView::FutureLedger => "FutureLedger",\n        }'),
        (GV,
         'marker: Some("GENERATED-VIEW-REGISTRY"),\n'
         "                generator: BootstrapToolId::ProjectPy,\n"
         "            },",
         'marker: Some("GENERATED-VIEW-REGISTRY"),\n'
         "                generator: BootstrapToolId::ProjectPy,\n"
         "            },\n"
         "            GeneratedView::FutureLedger => GeneratedViewSpec {\n"
         '                authority_sources: &["spec/future_ledger.rs"],\n'
         '                target: GeneratedViewTarget::Static(&["docs/FUTURE_LEDGER.md"]),\n'
         "                surface: GeneratedViewSurface::EmbeddedBlock,\n"
         '                marker: Some("FUTURE-LEDGER"),\n'
         "                generator: BootstrapToolId::ProjectPy,\n"
         "            },"),
        ("bootstrap/project/__init__.py",
         '    "GeneratedViewRegistry": render_generated_view_registry,\n}',
         '    "GeneratedViewRegistry": render_generated_view_registry,\n'
         '    "FutureLedger": lambda root: "sandbox-only future ledger body",\n}'),
    ])
    try:
        (tmp / "spec/future_ledger.rs").write_bytes(
            b"// sandbox-only future ledger source (registry-growth evidence)\n")
        (tmp / "docs/FUTURE_LEDGER.md").write_bytes(
            b"# Future ledger (sandbox only)\n\n"
            b"<!-- FUTURE-LEDGER:BEGIN generated from spec/future_ledger.rs by "
            b"bootstrap/project.py; do not edit -->\n(pending)\n"
            b"<!-- FUTURE-LEDGER:END -->\n")
        _growth_pkg = tmp / "bootstrap/project/__init__.py"
        if _growth_pkg.is_file():
            spec_mod = importlib.util.spec_from_file_location(
                "project_growth_tmp", _growth_pkg,
                submodule_search_locations=[str(tmp / "bootstrap/project")])
        else:
            spec_mod = importlib.util.spec_from_file_location(
                "project_growth_tmp", tmp / "bootstrap/project.py")
        proj_tmp = importlib.util.module_from_spec(spec_mod)
        spec_mod.loader.exec_module(proj_tmp)
        regen_findings: list[str] = []
        _views, plans, _bindings = proj_tmp.build_plans(tmp, regen_findings)
        if regen_findings:
            fail(f"structural_registry_growth_is_lawful (projector refused: "
                 f"{regen_findings!r})")
        else:
            for path, original, rewritten in plans:
                if rewritten != original:
                    path.write_bytes(rewritten.encode("utf-8"))
            grown = audit.generated_view_findings(tmp)
            if grown:
                fail(f"structural_registry_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    return findings


def test_inventory_mirrors(audit, project) -> list[str]:
    """Named hostile fixtures for the repository inventory projections
    (5.5E4b): package/edge/profile/bundle/Tier0 mirrors are generated
    derivations of their typed owners, retired authored mirrors stay retired,
    and a denominator never impersonates a run result."""
    findings: list[str] = []
    root = HERE.parent
    AR = "spec/architecture.rs"
    RM, D3, D29 = ("README.md", "docs/03_REPOSITORY_AND_PACKAGES.md",
                   "docs/29_STATUS_AND_SUPERSESSION.md")
    D28 = "docs/28_SELF_EXPLAINING_REPOSITORY.md"
    D23 = "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md"

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def sandbox(edits):
        # gate_sandbox carries the root-level documents and bootstrap sources
        # since 5.5E4b, so the inventory projections resolve in every sandbox.
        # It also resolves any concept-door edit (SW5) to the submodule that
        # carries the needle, so AR edits below may name the architecture door.
        return gate_sandbox(edits)

    def probe(name, edits, needle, validator=None):
        tmp = sandbox(edits)
        try:
            expect(name, (validator or audit.inventory_mirror_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.inventory_mirror_findings(root):
        fail(f"inventory_mirrors_pass_on_the_real_seed "
             f"({audit.inventory_mirror_findings(root)!r})")

    # Documentary class inventories.
    probe("package_class_missing_from_all_is_rejected",
          [(AR, "        PackageClass::Example,\n    ];", "    ];")],
          "PackageClass::Example is declared but missing from PackageClass::ALL")
    probe("duplicate_package_class_spelling_is_rejected",
          [(AR, 'PackageClass::Example => "example",', 'PackageClass::Example => "production",')],
          "duplicate PackageClass spelling 'production'")
    probe("edge_class_missing_from_all_is_rejected",
          [(AR, "        EdgeClass::DevOnly,\n    ];", "    ];")],
          "EdgeClass::DevOnly is declared but missing from EdgeClass::ALL")
    probe("duplicate_edge_class_spelling_is_rejected",
          [(AR, 'EdgeClass::OptionalProfile => "optional-profile",',
            'EdgeClass::OptionalProfile => "required",')],
          "duplicate EdgeClass spelling 'required'")

    # Every generated inventory row is positional law: a dropped, reordered,
    # or reclassified row — and any smuggled module — is projection drift.
    PKG_NEEDLE = "generated block PACKAGE-INVENTORY does not match its typed derivation"
    EX_ROW = ("| batpak-examples | example | 5 | examples | public-surface witness; "
              "runnable demos over production APIs only; owns no semantic law and "
              "depends on no dev tooling |")
    for name, rel, old, new, needle in (
        ("package_inventory_missing_batpak_examples_is_rejected", RM,
         EX_ROW + "\n", "", PKG_NEEDLE),
        ("package_inventory_order_drift_is_rejected", D3,
         "| macbat-compiler | production | 0 | crates/macbat/compiler | pure Rust contract compiler |\n| macbat | production | 1 |",
         "| macbat | production | 1 | crates/macbat/macros | proc-macro front door |\n| macbat-compiler | production | 0 |",
         PKG_NEEDLE),
        ("package_inventory_class_drift_is_rejected", RM,
         "| batpak-cli | binary-adapter |", "| batpak-cli | production |", PKG_NEEDLE),
        ("package_inventory_layer_drift_is_rejected", RM,
         "| macbat | production | 1 |", "| macbat | production | 2 |", PKG_NEEDLE),
        ("package_inventory_path_drift_is_rejected", RM,
         "| batpak | production | 2 | crates/batpak |",
         "| batpak | production | 2 | crates/batpak2 |", PKG_NEEDLE),
        ("package_inventory_role_drift_is_rejected", RM,
         "pure Rust contract compiler |", "impure contract compiler |", PKG_NEEDLE),
        ("internal_syncbat_plane_cannot_enter_package_inventory", RM,
         EX_ROW, EX_ROW + "\n| pakvm | production | 3 | crates/syncbat | plane |",
         PKG_NEEDLE),
        ("muterprater_cannot_enter_package_inventory", D3,
         EX_ROW, EX_ROW + "\n| muterprater | dev-only | 6 | crates/testpak | module |",
         PKG_NEEDLE),
        ("package_edge_missing_from_projection_is_rejected", D3,
         "| macbat | macbat-compiler | required | compile |\n", "",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_extra_projection_row_is_rejected", D3,
         "| batpak-examples | batql | required | example |",
         "| batpak-examples | batql | required | example |\n| batql | netbat | required | rogue |",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_order_drift_is_rejected", D3,
         "| macbat | macbat-compiler | required | compile |\n| batpak | macbat | required | derive |",
         "| batpak | macbat | required | derive |\n| macbat | macbat-compiler | required | compile |",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_class_drift_is_rejected", D3,
         "| batpak-cli | netbat | optional-profile | serve |",
         "| batpak-cli | netbat | required | serve |",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_profile_drift_is_rejected", D3,
         "| macbat | macbat-compiler | required | compile |",
         "| macbat | macbat-compiler | required | compile2 |",
         "generated block PACKAGE-EDGES does not match"),
        ("qualification_profile_missing_browser_storage_is_rejected", D3,
         "| batpak | browser-storage |", "| batpak | browser-storage-gone |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_profile_order_drift_is_rejected", D3,
         "| batpak | semantic | no_std + alloc |", "| batpak | zzz-semantic | no_std + alloc |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_environment_drift_is_rejected", D3,
         "| syncbat | semantic | no_std + alloc |", "| syncbat | semantic | std |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_gate_drift_is_rejected", D3,
         "| batpak | semantic | no_std + alloc | G0/G5 |",
         "| batpak | semantic | no_std + alloc | G5 |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_requirement_drift_is_rejected", D3,
         "storage-port law compile without std |", "storage-port law need std |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("bundle_inventory_package_count_tracks_package_id_all", D28,
         "| Cargo packages | 9 |", "| Cargo packages | 8 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_edge_count_tracks_edges", D28,
         "| package edges | 19 |", "| package edges | 18 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_profile_count_tracks_profiles", D28,
         "| qualification profiles | 6 |", "| qualification profiles | 5 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_seed_count_tracks_seed", D28,
         "| SEED guarantees | 32 |", "| SEED guarantees | 31 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_decision_count_tracks_decisions", D28,
         "| decision rows | 82 |", "| decision rows | 81 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_legacy_count_tracks_obligations", D28,
         "| legacy semantic obligations | 87 |", "| legacy semantic obligations | 86 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_coverage_count_tracks_source_invariant_ids", D28,
         "| legacy invariant declarations | 115 |", "| legacy invariant declarations | 107 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_operator_count_tracks_operator_id_all", D28,
         "| BatQL operators | 13 |", "| BatQL operators | 12 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_generated_view_count_tracks_registry", D28,
         f"| registered generated views | {len(audit.a_parse_generated_views(root)['order'])} |",
         "| registered generated views | 1 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_markdown_count_tracks_eligible_corpus", D28,
         "| Markdown documents | 46 |", "| Markdown documents | 45 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("tier0_receipt_missing_from_projection_is_rejected", D23,
         "| tier0-materialize | ExecutableAndOutputTree |\n", "",
         "generated block TIER0-RECEIPT-DENOMINATOR does not match"),
        ("tier0_receipt_artifact_policy_drift_is_rejected", D23,
         "| tier0-seedcheck | Executable |", "| tier0-seedcheck | FixtureSet |",
         "generated block TIER0-RECEIPT-DENOMINATOR does not match"),
        ("tier0_projection_order_drift_is_rejected", D23,
         "| tier0-law-fixtures | FixtureSet |\n| tier0-seedcheck | Executable |",
         "| tier0-seedcheck | Executable |\n| tier0-law-fixtures | FixtureSet |",
         "generated block TIER0-RECEIPT-DENOMINATOR does not match"),
    ):
        probe(name, [(rel, old, new)], needle)

    # Retired authored mirrors stay retired; converged doctrine stays authored.
    for name, rel, insertion, needle in (
        ("readme_authored_package_table_cannot_return", RM,
         "\n| `batpak` | production Cargo package | semantic core |\n",
         "retired authored mirror may not return (authored package table)"),
        ("docs03_authored_layer_inventory_cannot_return", D3,
         "\nL0  macbat-compiler\nL1  macbat\n",
         "retired authored mirror may not return (authored layer inventory)"),
        ("docs03_ascii_dependency_inventory_cannot_return", D3,
         "\nmacbat\n  ↑\nmacbat-compiler\n",
         "retired authored mirror may not return (ASCII dependency inventory)"),
        ("docs03_five_profile_mirror_cannot_return", D3,
         "\nbatpak semantic    no_std + alloc\n",
         "retired authored mirror may not return (five-profile target matrix)"),
        ("authored_bundle_counts_cannot_return", D28,
         "\n19 declared package edges\n",
         "retired authored mirror may not return (authored bundle counts)"),
        ("legacy_107_row_count_is_rejected", D28,
         "\n107-row legacy catalog coverage equivalence PASS\n",
         "retired authored mirror may not return (107-row legacy claim)"),
        ("volatile_word_count_cannot_return", D28,
         "\n31,000+ words of architecture\n",
         "retired authored mirror may not return (volatile word count)"),
        ("static_validation_pass_table_cannot_return", D23,
         "\n## Validation completed in this delivery environment\n",
         "retired authored mirror may not return (static validation PASS table)"),
        ("historical_no_rustc_claim_cannot_return", D23,
         "\nThe environment did not contain `rustc` or `cargo`.\n",
         "retired authored mirror may not return (historical no-rustc claim)"),
        ("delivery_notes_cannot_embed_current_run_ids_as_timeless_law", D23,
         "\nHosted run 29593750281 was green.\n",
         "retired authored mirror may not return (current run id embedded as timeless law)"),
    ):
        probe(name, [(rel, "\n## ", insertion + "\n## ")], needle)
    for name, rel, fragment, needle in (
        ("feature_coverage_family_cannot_masquerade_as_qualification_profile", D3,
         "They are not additional QualificationProfile identities unless they enter",
         "converged inventory doctrine absent (feature-coverage-distinction)"),
        ("authored_metadata_requires_reconciliation_epoch", D29,
         "ReconciliationEpoch corpus membership",
         "converged inventory doctrine absent (authored-metadata-epoch)"),
        ("generated_metadata_requires_exact_sources", D29,
         "generator, exact authority sources, do-not-edit posture",
         "converged inventory doctrine absent (generated-metadata-sources)"),
        ("generated_document_does_not_require_contract_id", D29,
         "does not invent a contract ID or supersession claim",
         "converged inventory doctrine absent (generated-needs-no-contract-id)"),
        ("authored_document_cannot_use_generated_metadata_shape", D29,
         "cannot claim generated status to evade the authored fields",
         "converged inventory doctrine absent (authored-cannot-claim-generated)"),
    ):
        probe(name, [(rel, fragment, "RETIRED FRAGMENT")], needle)
    # Multi-source provenance: order IS provenance.
    probe("bundle_inventory_multi_source_order_drift_is_rejected",
          [(D28, "BUNDLE-INVENTORY:BEGIN generated from spec/architecture.rs; spec/invariants.rs;",
            "BUNDLE-INVENTORY:BEGIN generated from spec/invariants.rs; spec/architecture.rs;")],
          "not its registered authority source",
          validator=audit.generated_view_findings)

    # GREEN: profile growth — a sandbox-only QualificationProfile row flows
    # mechanically into docs/03 and the bundle count with no frozen number.
    first_profile = (
        'QualificationProfile {\n        package: PackageId::BatPak,\n'
        '        profile: "semantic",\n'
        '        environment: QualificationEnvironment::NoStdAlloc,\n'
        '        gates: &[GateId::G0, GateId::G5],\n'
        '        requirement: "contracts, schemas, codecs, image values, '
        'deterministic parsing, and storage-port law compile without std",\n    },')
    tmp = sandbox([(AR, first_profile, first_profile +
                    '\n    QualificationProfile {\n        package: PackageId::BatPak,\n'
                    '        profile: "sandbox-growth",\n'
                    '        environment: QualificationEnvironment::WasmHost,\n'
                    '        gates: &[GateId::G2, GateId::G5],\n'
                    '        requirement: "sandbox-only structural growth evidence",\n    },')])
    try:
        _regen_operator_blocks(project, tmp)
        grown = audit.inventory_mirror_findings(tmp) + audit.generated_view_findings(tmp)
        doc3 = (tmp / D3).read_text(encoding="utf-8")
        if grown or "| batpak | sandbox-growth | wasm32 host | G2/G5 |" not in doc3 \
                or "| qualification profiles | 7 |" not in (tmp / D28).read_text(encoding="utf-8"):
            fail(f"qualification_profile_structural_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: one logical view, two registered target instances — the SAME
    # PACKAGE-INVENTORY bytes land in README and docs/03.
    readme_body = audit.batql_extract_block(
        (root / RM).read_text(encoding="utf-8"), "PACKAGE-INVENTORY")
    docs3_body = audit.batql_extract_block(
        (root / D3).read_text(encoding="utf-8"), "PACKAGE-INVENTORY")
    if readme_body is None or readme_body != docs3_body:
        fail("package_inventory_multi_target_bytes_are_identical")

    return findings


def test_exact_ledgers(audit, project) -> list[str]:
    """Named hostile fixtures for 5.5E4c: version-boundary convergence, the
    generated decision ledger, the generated legacy-invariant coverage matrix,
    and the exact-mirror/semantic-owner doctrine."""
    findings: list[str] = []
    root = HERE.parent
    DI, CV, ID = ("spec/dispositions.rs", "spec/legacy_invariant_coverage.rs",
                  "spec/identities.rs")
    D30, D34 = ("docs/30_DECISION_AND_REJECTION_LEDGER.md",
                "docs/34_LEGACY_INVARIANT_COVERAGE.md")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, needle, validator=None):
        tmp = gate_sandbox(edits)
        try:
            expect(name, (validator or audit.exact_ledger_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.exact_ledger_findings(root):
        fail(f"exact_ledgers_pass_on_the_real_seed "
             f"({audit.exact_ledger_findings(root)!r})")

    LEDGER = "generated block DECISION-LEDGER does not match its typed derivation"
    MATRIX = "generated block LEGACY-INVARIANT-COVERAGE does not match its typed derivation"
    DEC1 = "| DEC-001 | LOCK | HistoricalReceipt |  | Pass 1 machine-centered architecture | Base of final v1 |"
    INV1 = "| `INV-ALLOW-IS-DESIGN` | SUPERSEDE | `BP-AGENT-GUIDE and BP-GAUNTLET-1` | Keep the zero-silencer law; replace the legacy checker implementation. |"
    for name, rel, old, new, needle in (
        # Version convergence.
        ("dec064_must_name_every_version_identity", DI,
         "PakVmIsaVersion FbatFormatVersion", "FbatFormatVersion",
         "the version-separation law names its complete inventory"),
        ("dec064_cannot_name_phantom_version_identity", DI,
         "no generic Version crosses subsystem boundaries",
         "GadgetVersion is admitted; no generic Version crosses subsystem boundaries",
         "phantom version identity"),
        ("event_frame_must_use_frame_version", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "frame_version: FrameVersion", "frame_version: u16",
         "does not carry frame_version: FrameVersion"),
        ("frame_version_cannot_be_described_as_fbat_container_version",
         "docs/05_STORAGE_FBAT_AND_TILES.md",
         "FrameVersion versions the EventFrame envelope.",
         "FrameVersion versions the .fbat container.",
         "version/convergence doctrine absent (three-way-envelope-split)"),
        ("frame_version_cannot_be_described_as_payload_record_version",
         "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
         "FrameVersion is reserved for the EventFrame envelope",
         "FrameVersion is reserved for the payload record",
         "version/convergence doctrine absent (frame-version-reservation)"),
        ("fbat_format_version_cannot_be_described_as_event_frame_version",
         "docs/05_STORAGE_FBAT_AND_TILES.md",
         "FbatFormatVersion versions the surrounding .fbat container.",
         "FbatFormatVersion versions the EventFrame envelope.",
         "version/convergence doctrine absent (three-way-envelope-split)"),
        ("bat_tagged_record_version_cannot_be_described_as_container_version",
         "docs/15_SCHEMA_CODEC_AND_MIGRATION.md",
         "It does not version EventFrame or the .fbat container.",
         "It versions the .fbat container.",
         "version/convergence doctrine absent (payload-record-boundary)"),
        ("layout_version_cannot_substitute_for_schema_version",
         "docs/02_SYSTEM_MODEL.md",
         "Neither substitutes for the other.",
         "Either substitutes for the other.",
         "version/convergence doctrine absent (layout-schema-split)"),
        ("docs24_version_witness_cannot_freeze_ten", "docs/24_GAUNTLET.md",
         "Every member of spec/identities.rs VersionIdentityKind::ALL denotes a",
         "The ten declared version identities are distinct types; every one denotes a",
         "may not freeze a numeric count"),
        ("docs24_version_witness_must_consume_version_identity_all",
         "docs/24_GAUNTLET.md",
         "VersionIdentityKind::ALL denotes a", "the version catalog denotes a",
         "version/convergence doctrine absent (version-witness-consumes-all)"),
        ("docs32_cannot_use_ambiguous_frame_header_version_wording",
         "docs/32_IMPLEMENTATION_CONSTANTS.md",
         "FrameVersion values and EventFrame envelope field IDs/order",
         "frame/header magic and version numbers",
         "version/convergence doctrine absent (frame-version-constants)"),
        ("generic_version_type_is_rejected", ID,
         "pub const EXISTING_TYPED_OWNER_SPELLINGS",
         "pub struct Version(pub u32);\npub const EXISTING_TYPED_OWNER_SPELLINGS",
         "declares a generic Version type"),
        ("event_frame_version_alias_is_rejected", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "preserving three independent version identities.",
         "preserving three independent version identities. EventFrameVersion is the alias.",
         "an unadmitted version-identity alias"),
        ("fbat_frame_version_alias_is_rejected", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "preserving three independent version identities.",
         "preserving three independent version identities. FbatFrameVersion is the alias.",
         "an unadmitted version-identity alias"),
        # Decision-ledger vocabulary and drift.
        ("disposition_missing_from_all_is_rejected", DI,
         "        Disposition::RetainAsEvidence,\n    ];", "    ];",
         "Disposition::RetainAsEvidence is declared but missing from Disposition::ALL"),
        ("duplicate_disposition_spelling_is_rejected", DI,
         'Disposition::Defer => "DEFER",', 'Disposition::Defer => "KEEP",',
         "duplicate Disposition spelling 'KEEP'"),
        ("empty_disposition_meaning_is_rejected", DI,
         'Disposition::Keep => "retained as current architecture",',
         'Disposition::Keep => "",',
         "Disposition::Keep declares no or an empty meaning"),
        ("decision_class_missing_from_all_is_rejected", DI,
         "        DecisionClass::Naming,\n", "",
         "DecisionClass::Naming is declared but missing from DecisionClass::ALL"),
        ("duplicate_decision_class_spelling_is_rejected", DI,
         'DecisionClass::Naming => "Naming",', 'DecisionClass::Naming => "Capability",',
         "duplicate DecisionClass spelling 'Capability'"),
        ("decision_ledger_missing_row_is_rejected", D30, DEC1 + "\n", "", LEDGER),
        ("decision_ledger_extra_row_is_rejected", D30, DEC1,
         DEC1 + "\n| DEC-998 | KEEP | Naming | | X | Y |", LEDGER),
        ("decision_ledger_order_drift_is_rejected", D30,
         DEC1 + "\n| DEC-002 |", "| DEC-002 |\n" + DEC1 + " ", LEDGER),
        ("decision_ledger_tag_drift_is_rejected", D30,
         "| DEC-001 | LOCK |", "| DEC-001 | KEEP |", LEDGER),
        ("decision_ledger_class_drift_is_rejected", D30,
         "| DEC-001 | LOCK | HistoricalReceipt |", "| DEC-001 | LOCK | Naming |", LEDGER),
        ("decision_ledger_gate_drift_is_rejected", D30,
         "| DEC-003 | KILL | Architecture | G2 |", "| DEC-003 | KILL | Architecture | G5 |",
         LEDGER),
        ("decision_ledger_subject_drift_is_rejected", D30,
         "Pass 1 machine-centered architecture", "Pass 1 hand-centered architecture",
         LEDGER),
        ("decision_ledger_ruling_drift_is_rejected", D30,
         "| Base of final v1 |", "| Base of final v2 |", LEDGER),
        ("decision_disposition_legend_drift_is_rejected", D30,
         "| KEEP | retained as current architecture |",
         "| KEEP | retained as legacy architecture |", LEDGER),
        ("authored_decision_row_cannot_return", D30,
         "## Reopening rule",
         "| DEC-999 | KEEP | Naming | G0 | Rogue | Rogue |\n\n## Reopening rule",
         "an authored decision row may not return"),
        ("authored_disposition_inventory_cannot_return", D30,
         "## Reopening rule",
         "KEEP                 retained as current architecture\n\n## Reopening rule",
         "an authored disposition inventory may not return"),
        # Coverage vocabulary and drift.
        ("coverage_disposition_missing_from_all_is_rejected", CV,
         "        CoverageDisposition::Requalify,\n    ];", "    ];",
         "CoverageDisposition::Requalify is declared but missing"),
        ("duplicate_coverage_disposition_spelling_is_rejected", CV,
         'CoverageDisposition::Requalify => "REQUALIFY",',
         'CoverageDisposition::Requalify => "KILL",',
         "duplicate CoverageDisposition spelling 'KILL'"),
        ("empty_coverage_disposition_meaning_is_rejected", CV,
         'CoverageDisposition::Kill => "intentionally absent from the target",',
         'CoverageDisposition::Kill => "",',
         "CoverageDisposition::Kill declares no or an empty meaning"),
        ("legacy_coverage_missing_row_is_rejected", D34, INV1 + "\n", "", MATRIX),
        ("legacy_coverage_extra_row_is_rejected", D34, INV1,
         INV1 + "\n| `INV-ROGUE` | KILL | `x` | rogue |", MATRIX),
        ("legacy_coverage_order_drift_is_rejected", D34,
         INV1 + "\n| `INV-BATCH-ATOMIC-VISIBILITY` |",
         "| `INV-BATCH-ATOMIC-VISIBILITY` |\n" + INV1 + " ", MATRIX),
        ("legacy_coverage_disposition_drift_is_rejected", D34,
         "| `INV-ALLOW-IS-DESIGN` | SUPERSEDE |", "| `INV-ALLOW-IS-DESIGN` | KILL |",
         MATRIX),
        ("legacy_coverage_successor_drift_is_rejected", D34,
         "| `BP-AGENT-GUIDE and BP-GAUNTLET-1` |", "| `BP-AGENT-GUIDE` |", MATRIX),
        ("legacy_coverage_rationale_drift_is_rejected", D34,
         "Keep the zero-silencer law; replace the legacy checker implementation.",
         "Drop the zero-silencer law.", MATRIX),
        ("legacy_coverage_legend_drift_is_rejected", D34,
         "| PRESERVE | semantic law survives through named successor |",
         "| PRESERVE | semantic law may lapse |", MATRIX),
        ("legacy_coverage_source_denominator_drift_is_rejected", D34,
         "The source declaration denominator is the 115-declaration",
         "The source declaration denominator is the 107-declaration", MATRIX),
        ("authored_legacy_coverage_row_cannot_return", D34,
         "## Note: the two fusion invariants are distinct",
         "| `INV-ROGUE` | KILL | `x` | rogue |\n\n"
         "## Note: the two fusion invariants are distinct",
         "an authored coverage row may not return"),
        ("authored_coverage_disposition_inventory_cannot_return", D34,
         "## Note: the two fusion invariants are distinct",
         "PRESERVE    semantic law survives through named successor\n\n"
         "## Note: the two fusion invariants are distinct",
         "an authored coverage disposition inventory may not return"),
        # Convergence doctrine.
        ("exact_mirror_cannot_remain_hand_authored",
         "docs/28_SELF_EXPLAINING_REPOSITORY.md",
         "An equivalence checker babysitting two editable copies is not convergence.",
         "Two editable copies are acceptable.",
         "version/convergence doctrine absent (exact-mirror-clause)"),
        ("generator_cannot_complete_missing_semantic_fields",
         "docs/28_SELF_EXPLAINING_REPOSITORY.md",
         "from conventions, neighbouring rows, or Python defaults.",
         "from any convenient source.",
         "version/convergence doctrine absent (no-python-prosthetics-clause)"),
        ("subset_projection_requires_a_typed_relation",
         "docs/28_SELF_EXPLAINING_REPOSITORY.md",
         "Parsing prose headings is not an authority relation.",
         "Parsing prose headings is acceptable.",
         "version/convergence doctrine absent (subset-relation-clause)"),
        ("historical_count_cannot_masquerade_as_current_inventory", D34,
         "retained as secondary historical evidence, not a competing denominator",
         "the current competing denominator",
         "version/convergence doctrine absent (historical-framing)"),
        ("docs21_cannot_be_marked_generated_before_its_missing_fields_are_owned",
         "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
         "status: AUTHORITATIVE", "status: GENERATED",
         "may not be marked GENERATED before its missing fields are owned"),
        ("docs24_semantic_meaning_cannot_be_demoted_to_generated_projection",
         "docs/24_GAUNTLET.md",
         "status: AUTHORITATIVE", "status: GENERATED",
         "may not be marked GENERATED before its missing fields are owned"),
    ):
        probe(name, [(rel, old, new)], needle)

    # The omission hostiles must silence EVERY occurrence — the boundary
    # clauses name the identities too, which is exactly the redundancy the
    # admission law provides.
    probe("dec064_omitting_frame_version_is_rejected",
          [(DI, "BatTaggedRecordVersion FrameVersion NetBatProtocolVersion",
            "BatTaggedRecordVersion NetBatProtocolVersion"),
           (DI, "FrameVersion versions the EventFrame envelope and never the "
            ".fbat container, the payload record codec, or NetBat transport framing; ",
            "")],
          "DEC-064 forward-policy fields do not name FrameVersion")
    probe("dec064_omitting_layout_version_is_rejected",
          [(DI, "SchemaVersion LayoutVersion and Tier0QualificationArtifactVersion "
            "are distinct types",
            "SchemaVersion and Tier0QualificationArtifactVersion are distinct types"),
           (DI, "LayoutVersion versions physical organization under LayoutId "
            "and never schema meaning; ", "")],
          "DEC-064 forward-policy fields do not name LayoutVersion")

    # Marker drift flows through the universal census.
    probe("decision_ledger_marker_drift_is_rejected",
          [(D30, "DECISION-LEDGER:BEGIN generated from spec/dispositions.rs",
            "DECISION-LEDGER:BEGIN generated from spec/architecture.rs")],
          "not its registered authority source",
          validator=audit.generated_view_findings)
    probe("legacy_coverage_marker_drift_is_rejected",
          [(D34, "LEGACY-INVARIANT-COVERAGE:BEGIN generated from spec/legacy_invariant_coverage.rs",
            "LEGACY-INVARIANT-COVERAGE:BEGIN generated from spec/architecture.rs")],
          "not its registered authority source",
          validator=audit.generated_view_findings)

    # GREEN growth — structural count-freedom evidence ONLY.
    def regen_and_check(name, edits, checks, validators):
        tmp = gate_sandbox(edits)
        try:
            _regen_operator_blocks(project, tmp)
            grown: list[str] = []
            for validator in validators:
                grown += validator(tmp)
            texts = {rel: (tmp / rel).read_text(encoding="utf-8")
                     for rel, _needle in checks}
            missing = [needle for rel, needle in checks if needle not in texts[rel]]
            if grown or missing:
                fail(f"{name} ({grown!r}, missing {missing!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    regen_and_check(
        "decision_structural_growth_is_lawful",
        [(DI, "pub const DECISIONS: &[DecisionSpec] = &[",
          "pub const DECISIONS: &[DecisionSpec] = &[\n"
          '    DecisionSpec { id: "DEC-990", class: DecisionClass::HistoricalReceipt, '
          "gates: &[], disposition: Disposition::RetainAsEvidence, "
          'subject: "Sandbox growth", successor: "structural count-freedom evidence only", '
          "stale_aliases: &[], stale_allowed_contexts: &[], replacement_contract: None },")],
        [(D30, "| DEC-990 | RETAIN-AS-EVIDENCE |"),
         ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "| decision rows | 83 |")],
        [audit.exact_ledger_findings])
    regen_and_check(
        "coverage_structural_growth_is_lawful",
        [(CV, '    "INV-ZERO-WARNINGS",\n];',
          '    "INV-ZERO-WARNINGS",\n    "INV-ZZZ-SANDBOX-GROWTH",\n];'),
         (CV, "pub const COVERAGE: &[LegacyInvariantCoverage] = &[",
          "pub const COVERAGE: &[LegacyInvariantCoverage] = &[\n"
          '    LegacyInvariantCoverage { legacy_id: "INV-ZZZ-SANDBOX-GROWTH", '
          "disposition: CoverageDisposition::Kill, "
          'successor: "none", rationale: "structural count-freedom evidence only." },')],
        [(D34, "| `INV-ZZZ-SANDBOX-GROWTH` | KILL |"),
         (D34, "the 116-declaration `SOURCE_INVARIANT_IDS` manifest")],
        [audit.exact_ledger_findings])
    regen_and_check(
        "version_structural_growth_is_lawful",
        [(ID, "        VersionIdentityKind::Tier0QualificationArtifact,\n    ];",
          "        VersionIdentityKind::Tier0QualificationArtifact,\n"
          "        VersionIdentityKind::Gadget,\n    ];"),
         (ID, "    Tier0QualificationArtifact,\n}",
          "    Tier0QualificationArtifact,\n    Gadget,\n}"),
         (ID, 'VersionIdentityKind::Layout => entry!("LayoutVersion", ',
          'VersionIdentityKind::Gadget => entry!("GadgetVersion", "BP-STORAGE-TILES-1"),\n'
          '            VersionIdentityKind::Layout => entry!("LayoutVersion", '),
         (DI, "SchemaVersion LayoutVersion and Tier0QualificationArtifactVersion "
          "are distinct types",
          "SchemaVersion LayoutVersion Tier0QualificationArtifactVersion and "
          "GadgetVersion are distinct types"),
         ("docs/05_STORAGE_FBAT_AND_TILES.md",
          "preserving three independent version identities.",
          "preserving three independent version identities. GadgetVersion is a "
          "sandbox-only structural-growth identity owned here.")],
        [("docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "GadgetVersion")],
        [audit.exact_ledger_findings, audit.identity_catalog_findings])

    return findings
