"""Typed-owner catalog authorities: identity catalogs, commands, mutation
lanes, corpus epoch, compiler assumptions, promotion, toolchain, package
identity, contract kinds, seedcheck execution, and rust-spec compilation."""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from .core import (
    HERE,
    TOOLCHAIN_EDITION,
    _resolve_edit_carrier,
    gate_sandbox,
    isolated_tree,
    must_replace,
    receipt,
    spec_carrier,
)
from .tier0 import (
    qualify_binary,
    qualify_materializer,
)


def test_identity_catalogs(audit) -> list[str]:
    """Named hostile fixtures for the identity catalogs (5.5E3d). Four axes,
    one term one axis, owners resolve, chronology stays out, existing typed
    owners are never duplicated, docs/16 projects ordered entries AND owners,
    and the permanent corpus denominator refuses any identity-shaped term
    that resolves through none of the five classification paths."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.identity_catalog_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.identity_catalog_findings(root):
        fail("identity_catalogs_pass_on_the_real_seed")

    ID = "spec/identities.rs"
    DOC16 = "docs/16_IDENTITY_TIME_AND_NAVIGATION.md"
    # EventId was the sweep's headline restoration; deleting it again is
    # refused by the corpus denominator itself: the term is still authored
    # in docs/04, docs/05, and the companion, so it must classify.
    probe("event_id_cannot_disappear_from_identity_catalog",
          [(ID, "        IdentityKind::Event,\n", ""),
           (ID, '            IdentityKind::Event => entry!("EventId", "BP-STORAGE-TILES-1"),\n',
            "")],
          "identity-shaped term EventId appears in the authoritative corpus "
          "and resolves through none of the five classification paths")
    # A binding is a commitment to content, never the object's identity:
    # cross-axis uniqueness means the misclassification has no spelling.
    probe("content_digest_cannot_be_classified_as_object_identity",
          [(ID, 'entry!("CorrelationId", "BP-STORAGE-TILES-1")',
            'entry!("ContentDigest", "BP-STORAGE-TILES-1")')],
          "ContentDigest answers two questions: it appears in both "
          "IdentityKind and BindingKind")
    probe("commitment_cannot_be_classified_as_object_identity",
          [(ID, 'entry!("TileId", "BP-STORAGE-TILES-1")',
            'entry!("Commitment", "BP-IDENTITY-TIME-NAV-1")')],
          "Commitment answers two questions: it appears in both "
          "IdentityKind and BindingKind")
    # Chronology and navigation vocabulary is excluded by executed law.
    probe("hlc_cannot_enter_identity_catalog",
          [(ID, 'entry!("RewrapId", "BP-CRYPTO-SECRET-1")',
            'entry!("Hlc", "BP-IDENTITY-TIME-NAV-1")')],
          "Hlc is chronology/navigation vocabulary and may not enter "
          "the IdentityKind catalog")
    probe("commit_point_cannot_enter_identity_catalog",
          [(ID, 'entry!("UnitId", "BP-NUMERIC-1")',
            'entry!("CommitPoint", "BP-IDENTITY-TIME-NAV-1")')],
          "CommitPoint is chronology/navigation vocabulary and may not enter "
          "the IdentityKind catalog")
    probe("identity_kind_missing_from_all_is_rejected",
          [(ID, "        IdentityKind::Tile,\n", "")],
          "IdentityKind::Tile declares an entry but is omitted from IdentityKind::ALL")
    probe("catalog_entry_with_unknown_owner_is_rejected",
          [(ID, 'entry!("TileId", "BP-STORAGE-TILES-1")',
            'entry!("TileId", "BP-NONEXISTENT-1")')],
          "IdentityKind entry TileId names owner BP-NONEXISTENT-1, "
          "which no declared contract owns")
    probe("docs_identity_block_drift_is_rejected",
          [(DOC16, f"{'EventId':<30} BP-STORAGE-TILES-1",
            f"{'EventId':<30} BP-SYNCBAT-1")],
          "docs/16 IDENTITY-CATALOG row 7 states EventId BP-SYNCBAT-1; the "
          "typed catalog states EventId BP-STORAGE-TILES-1 at that position")
    # Existing typed owners stay owners: a duplicate variant is a wrapper
    # passport, refused by spelling.
    probe("existing_package_id_owner_cannot_be_duplicated",
          [(ID, 'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("PackageId", "BP-CRYPTO-SECRET-1")')],
          "IdentityKind entry PackageId duplicates a spelling that already "
          "has a typed spec owner")
    probe("existing_proof_row_id_owner_cannot_be_duplicated",
          [(ID, 'entry!("KeyBackendId", "BP-CRYPTO-SECRET-1")',
            'entry!("ProofRowId", "BP-CRYPTO-SECRET-1")')],
          "IdentityKind entry ProofRowId duplicates a spelling that already "
          "has a typed spec owner")
    # The standing denominator law: a new identity-shaped term authored
    # anywhere in the corpus turns red until classified through exactly one
    # of the five paths.
    probe("new_identity_shaped_term_requires_classification",
          [(DOC16, "BatPak never invents its own entropy primitive.",
            "BatPak never invents its own entropy primitive. A future "
            "WidgetTrackerId is under discussion.")],
          "identity-shaped term WidgetTrackerId appears in the authoritative "
          "corpus and resolves through none of the five classification paths")
    probe("noncataloged_term_cannot_also_be_admitted",
          [(ID, 'entry!("KeyId", "BP-CRYPTO-SECRET-1")',
            'entry!("NavigationId", "BP-IDENTITY-TIME-NAV-1")')],
          "NavigationId resolves through two paths: the residue table and "
          "the IdentityKind catalog")
    probe("owned_elsewhere_term_requires_a_live_contract_owner",
          [(ID, 'OwnedElsewhere(ContractId("BP-GATES-1"))',
            'OwnedElsewhere(ContractId("BP-GHOST-1"))')],
          "residue term GateId is owned elsewhere by BP-GHOST-1, "
          "which no declared contract owns")
    # CompressionId is a passport application stapled to DEC-063's checklist.
    # Sneaking it into a catalog while the residue row stands is the two-path
    # refusal; entry happens only by amending DEC-063 AND moving the term.
    probe("compression_id_cannot_enter_without_dec063_amendment",
          [(ID, 'entry!("FloatFormatId", "BP-NUMERIC-1")',
            'entry!("CompressionId", "BP-NUMERIC-1")')],
          "CompressionId resolves through two paths: the residue table and "
          "the IdentityKind catalog")
    probe("not_yet_admitted_term_requires_a_live_decision",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-963"))')],
          "residue term CompressionId is not yet admitted by DEC-963, "
          "which no declared decision owns")
    # A decision retained only as historical coverage has no forward policy
    # authority: the archive does not issue future visas. DEC-005 is the
    # standing Supersede row.
    probe("historical_decision_cannot_own_not_yet_admitted_term",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-005"))')],
          "residue term CompressionId is not yet admitted by DEC-005, which "
          "is retained only as historical coverage and cannot own a future "
          "admission barrier")
    # 5.5E3d1: TRUE set equality — a catalog entry with only its own
    # generated projection is an answer sheet, not an adopter. The typed
    # entry and the docs/16 row alone must refuse; add a real authored
    # sentence under the live owner contract and the same growth is lawful.
    _SESSION_GROWTH = [
        (ID, "        IdentityKind::Tile,\n",
         "        IdentityKind::Tile,\n        IdentityKind::Session,\n"),
        (ID, '            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),',
         '            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),\n'
         '            IdentityKind::Session => entry!("SessionId", "BP-STORAGE-TILES-1"),'),
        (DOC16, f"{'TileId':<30} BP-STORAGE-TILES-1\n",
         f"{'TileId':<30} BP-STORAGE-TILES-1\n"
         f"{'SessionId':<30} BP-STORAGE-TILES-1\n"),
    ]
    probe("catalog_entry_without_authored_adopter_is_rejected",
          list(_SESSION_GROWTH),
          "catalog entry SessionId appears in no authored corpus document; "
          "an entry without an authored adopter is a brochure passport")
    # Self-neutering guards: deleting the exclusion or wrapper-guard row
    # before smuggling the term in is refused by independent parity with
    # the authored docs/16 fences and the real spec declarations.
    probe("chronology_exclusion_cannot_be_removed_then_cataloged",
          [(ID, '    "Hlc",\n', ''),
           (ID, 'entry!("RewrapId", "BP-CRYPTO-SECRET-1")',
            'entry!("Hlc", "BP-IDENTITY-TIME-NAV-1")')],
          "EXCLUDED_CHRONOLOGY_AND_NAVIGATION omits Hlc, which docs/16's "
          "navigation and time-and-order fences declare")
    probe("existing_owner_guard_cannot_be_removed_then_readmitted",
          [(ID, '"PackageId", ', ''),
           (ID, 'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("PackageId", "BP-CRYPTO-SECRET-1")')],
          "EXISTING_TYPED_OWNER_SPELLINGS omits PackageId, which the spec "
          "declares as an existing typed owner")
    # One term, one path — and one ROW on that path.
    probe("duplicate_residue_term_is_rejected",
          [(ID, '        term: "GateId",\n',
            '        term: "GateId",\n        disposition: '
            'IdentityTermDisposition::OwnedElsewhere(ContractId("BP-GATES-1")),\n'
            '    },\n    NonCatalogedIdentityTerm {\n        term: "GateId",\n')],
          "residue term GateId is declared twice; one term, one path also "
          "means one row on that path")
    # Permanent is not enough: Kill is a dead passport and Keep is admitted
    # retained policy — neither is a pending application.
    probe("killed_decision_cannot_own_not_yet_admitted_term",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Kill,')],
          "residue term CompressionId is not yet admitted by DEC-063, whose "
          "Kill disposition is not a standing future-entry policy; only Lock "
          "and Defer own pending applications")
    probe("kept_decision_cannot_own_not_yet_admitted_term",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Keep,')],
          "residue term CompressionId is not yet admitted by DEC-063, whose "
          "Keep disposition is not a standing future-entry policy; only Lock "
          "and Defer own pending applications")
    # Provenance is verified independently of the renderer: a BEGIN marker
    # naming any other source is not a generated identity block at all.
    probe("identity_projection_source_marker_drift_is_rejected",
          [(DOC16,
            "<!-- IDENTITY-CATALOG:BEGIN generated from spec/identities/types.rs "
            "by bootstrap/project.py; do not edit -->",
            "<!-- IDENTITY-CATALOG:BEGIN generated from spec/identity_soup.rs "
            "by bootstrap/project.py; do not edit -->")],
          "docs/16 carries no generated IDENTITY-CATALOG block naming "
          "spec/identities/types.rs as source")
    # Owner coherence (5.5E3d2): the owner must AUTHOR the term. Each owner
    # mutation moves the generated docs/16 row in lockstep, so the hostile
    # fails for owner incoherence, never for a stale projection.
    probe("catalog_owner_must_author_the_term",
          [(ID, 'entry!("EventId", "BP-STORAGE-TILES-1")',
            'entry!("EventId", "BP-SYNCBAT-1")'),
           (DOC16, f"{'EventId':<30} BP-STORAGE-TILES-1",
            f"{'EventId':<30} BP-SYNCBAT-1")],
          "IdentityKind entry EventId names owner BP-SYNCBAT-1, whose "
          "authoritative document does not author the term")
    probe("owned_elsewhere_owner_must_author_the_term",
          [(ID, 'OwnedElsewhere(ContractId("BP-GATES-1"))',
            'OwnedElsewhere(ContractId("BP-NETBAT-1"))'),
           (DOC16, f"{'GateId':<30} {'owned elsewhere':<22} BP-GATES-1",
            f"{'GateId':<30} {'owned elsewhere':<22} BP-NETBAT-1")],
          "residue term GateId is owned elsewhere by BP-NETBAT-1, "
          "which does not author the term")
    # DEC-058 is live and Lock, so it passes existence and the disposition
    # predicate — but it is about the release seal and says nothing about
    # CompressionId. A live but unrelated authority is refused by name.
    probe("not_yet_admitted_decision_must_name_the_term",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-058"))'),
           (DOC16, f"{'CompressionId':<30} {'not yet admitted by':<22} DEC-063",
            f"{'CompressionId':<30} {'not yet admitted by':<22} DEC-058")],
          "residue term CompressionId is not yet admitted by DEC-058, whose "
          "forward-policy fields do not name the term")
    # GREEN fixture: a term may lawfully appear in many documents while its
    # canonical owner document also authors it — breadth of adoption is not
    # an ownership dispute.
    tmp = gate_sandbox([
        ("docs/14_RECEIPTS_AND_EXPLANATION.md",
         "It is structured evidence, not a debug log.",
         "It is structured evidence, not a debug log. A replay receipt names "
         "the EventId range it covered.")])
    try:
        got = audit.identity_catalog_findings(tmp)
        if got:
            fail(f"term_in_many_documents_with_authoring_owner_is_lawful "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    # GREEN fixture: the counts are observed output, never acceptance
    # thresholds. The SAME growth as the brochure hostile above, plus a real
    # authored adopter sentence under the live owner contract (docs/05,
    # BP-STORAGE-TILES-1), is lawful with no frozen number touched.
    tmp = gate_sandbox(_SESSION_GROWTH + [
        ("docs/05_STORAGE_FBAT_AND_TILES.md",
         "EventFrame identity\n",
         "EventFrame identity\nSessionId scoping\n"),
    ])
    try:
        got = audit.identity_catalog_findings(tmp)
        if got:
            fail(f"catalog_counts_are_derived_not_frozen (lawful growth "
                 f"was refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_commands(audit) -> list[str]:
    """Named hostile fixtures for the three command namespaces (5.5E3e). The
    variant is the identity; namespace identity and invoked semantic
    authority are separate axes; composition never transfers ownership."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.command_catalog_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: the real seed passes whole — which simultaneously proves the two
    # ruled lawful shapes: the same "inspect" token lives in ProductCommand
    # AND TestPakCommand (uniqueness is per namespace), and the composite
    # commands delegate to several contracts without transferring ownership.
    if audit.command_catalog_findings(root):
        fail("commands_pass_on_the_real_seed")

    CM = "spec/commands.rs"
    D26 = "docs/26_COMMAND_PLANE.md"
    CP = "companion/BATQL_LANGUAGE.md"
    probe("product_command_missing_from_inventory_is_rejected",
          [(CM, "        ProductCommand::Repl,\n    ];", "    ];")],
          "ProductCommand::Repl is omitted from ProductCommand::ALL")
    probe("testpak_command_missing_from_inventory_is_rejected",
          [(CM, "        TestPakCommand::Seal,\n    ];", "    ];")],
          "TestPakCommand::Seal is omitted from TestPakCommand::ALL")
    probe("batql_mode_missing_from_inventory_is_rejected",
          [(CM, "&[BatQlSourceMode::Ask, BatQlSourceMode::Do]",
            "&[BatQlSourceMode::Ask]")],
          "BatQlSourceMode::Do is omitted from BatQlSourceMode::ALL")
    probe("duplicate_token_within_product_namespace_is_rejected",
          [(CM, 'ProductCommand::Repl => "repl",', 'ProductCommand::Repl => "run",')],
          "ProductCommand token run is claimed twice; tokens are unique "
          "within their namespace")
    probe("do_cannot_enter_product_commands",
          [(CM, 'ProductCommand::Repl => "repl",', 'ProductCommand::Repl => "do",')],
          "ProductCommand admits do: ASK and DO are language modes, never "
          "CLI verbs, and DO is never a top-level command")
    probe("ask_cannot_enter_testpak_commands",
          [(CM, 'TestPakCommand::Forge => "forge",', 'TestPakCommand::Forge => "ask",')],
          "TestPakCommand admits ask: ASK and DO are language modes")
    # Owner coherence, with the generated row moved in lockstep so the
    # refusal is authorship, not projection staleness.
    probe("direct_command_owner_must_author_the_token",
          [(CM, 'ProductCommand::Serve => CommandAuthority::Direct(ContractId("BP-NETBAT-1")),',
            'ProductCommand::Serve => CommandAuthority::Direct(ContractId("BP-CRYPTO-SECRET-1")),'),
           (D26, "serve      direct     BP-NETBAT-1",
            "serve      direct     BP-CRYPTO-SECRET-1")],
          "ProductCommand serve cites owner BP-CRYPTO-SECRET-1, whose "
          "authoritative document does not author the token")
    probe("composite_command_owner_must_author_the_token",
          [(CM, 'ProductCommand::Verify => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),',
            'ProductCommand::Verify => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-NUMERIC-1"),'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-NUMERIC-1                 "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1")],
          "ProductCommand verify cites composition owner BP-NUMERIC-1, whose "
          "authoritative document does not author the token")
    probe("composite_command_requires_at_least_one_delegate",
          [(CM, 'delegates: &[\n                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n                ],',
            "delegates: &[],"),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-COMMAND-PLANE-1")],
          "ProductCommand verify is a composite with no delegates")
    probe("composite_command_cannot_repeat_a_delegate",
          [(CM, '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n                ],',
            '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n                ],'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1")],
          "ProductCommand verify repeats a delegate")
    probe("composition_owner_cannot_be_its_own_delegate",
          [(CM, '                    ContractId("BP-GAUNTLET-1"),\n                ],',
            '                    ContractId("BP-GAUNTLET-1"),\n'
                '                    ContractId("BP-COMMAND-PLANE-1"),\n                ],'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1 BP-COMMAND-PLANE-1")],
          "ProductCommand verify lists its composition owner as its own "
          "delegate; composition never transfers ownership")
    # The composition boundary is declared in authored docs/26 prose, and the
    # typed authority must agree in BOTH directions.
    probe("compile_cannot_collapse_to_macbat_only",
          [(CM, 'ProductCommand::Compile => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-MACBAT-1"),\n'
                '                    ContractId("BP-BATQL-ARCH-1"),\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                ],\n            },',
            'ProductCommand::Compile => CommandAuthority::Direct(ContractId("BP-MACBAT-1")),'),
           (D26, "compile    composite  BP-COMMAND-PLANE-1           "
                 "BP-MACBAT-1 BP-BATQL-ARCH-1 BP-WORLD-PORTS-1",
            "compile    direct     BP-MACBAT-1")],
          "docs/26 declares compile a composition boundary; a Direct owner "
          "would collapse it to a single sovereign")
    probe("query_cannot_silently_gain_raw_source_compilation",
          [(CM, 'ProductCommand::Query => {\n'
                '                CommandAuthority::Direct(ContractId("BP-WORLD-PORTS-1"))\n'
                '            }',
            'ProductCommand::Query => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-BATQL-ARCH-1"),\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                ],\n            },'),
           (D26, "query      direct     BP-WORLD-PORTS-1",
            "query      composite  BP-COMMAND-PLANE-1           "
                 "BP-BATQL-ARCH-1 BP-WORLD-PORTS-1")],
          "docs/26 does not declare query a composition boundary; the typed "
          "authority may not silently gain a composite")
    probe("product_inspect_cannot_claim_receipts_as_sole_owner",
          [(CM, 'ProductCommand::Inspect => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                    ContractId("BP-SCHEMA-CODEC-1"),\n'
                '                    ContractId("BP-PAKVM-ISA-1"),\n'
                '                    ContractId("BP-BVISOR-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                ],\n            },',
            'ProductCommand::Inspect => CommandAuthority::Direct(ContractId("BP-RECEIPTS-1")),'),
           (D26, "inspect    composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-SCHEMA-CODEC-1 BP-PAKVM-ISA-1 BP-BVISOR-1 BP-RECEIPTS-1",
            "inspect    direct     BP-RECEIPTS-1")],
          "docs/26 declares inspect a composition boundary")
    probe("product_verify_cannot_claim_receipts_as_sole_owner",
          [(CM, 'ProductCommand::Verify => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n'
                '                ],\n            },',
            'ProductCommand::Verify => CommandAuthority::Direct(ContractId("BP-RECEIPTS-1")),'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     direct     BP-RECEIPTS-1")],
          "docs/26 declares verify a composition boundary")
    # The ruled TestPak owners are pinned by the generated projection: moving
    # an owner in the spec without regenerating docs/26 is positional drift,
    # and regenerating docs/26 is a visible diff the promotion review reads.
    probe("testpak_inspect_is_owned_by_self_explaining_contract",
          [(CM, "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-SELF-EXPLAINING-1"))',
            "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))')],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 1 states inspect "
          "direct BP-SELF-EXPLAINING-1; the typed catalog states inspect "
          "direct BP-TESTPAK-1 at that position")
    probe("testpak_context_is_owned_by_self_explaining_contract",
          [(CM, "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-SELF-EXPLAINING-1"))',
            "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))')],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 8 states context "
          "direct BP-SELF-EXPLAINING-1; the typed catalog states context "
          "direct BP-TESTPAK-1 at that position")
    probe("fuzz_bench_and_prove_are_gauntlet_commands",
          [(CM, "TestPakCommand::Fuzz | TestPakCommand::Bench | TestPakCommand::Prove => {\n"
                '                CommandAuthority::Direct(ContractId("BP-GAUNTLET-1"))',
            "TestPakCommand::Fuzz | TestPakCommand::Bench | TestPakCommand::Prove => {\n"
                '                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))')],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 5 states fuzz "
          "direct BP-GAUNTLET-1; the typed catalog states fuzz direct "
          "BP-TESTPAK-1 at that position")
    probe("docs_product_command_order_drift_is_rejected",
          [(D26, "run        direct     BP-WORLD-PORTS-1\n"
                 "query      direct     BP-WORLD-PORTS-1",
            "query      direct     BP-WORLD-PORTS-1\n"
                 "run        direct     BP-WORLD-PORTS-1")],
          "docs/26_COMMAND_PLANE.md PRODUCT-COMMANDS row 2 states query "
          "direct BP-WORLD-PORTS-1; the typed catalog states run direct "
          "BP-WORLD-PORTS-1 at that position")
    probe("docs_testpak_command_order_drift_is_rejected",
          [(D26, "forge      direct     BP-TESTPAK-1\n"
                 "test       direct     BP-TESTPAK-1",
            "test       direct     BP-TESTPAK-1\n"
                 "forge      direct     BP-TESTPAK-1")],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 2 states test "
          "direct BP-TESTPAK-1; the typed catalog states forge direct "
          "BP-TESTPAK-1 at that position")
    probe("batql_mode_projection_drift_is_rejected",
          [(CP, "ASK        direct     BP-BATQL-LANGUAGE-1",
            "ASK        direct     BP-BATQL-ARCH-1")],
          "companion/BATQL_LANGUAGE.md BATQL-SOURCE-MODES row 1 states ASK "
          "direct BP-BATQL-ARCH-1; the typed catalog states ASK direct "
          "BP-BATQL-LANGUAGE-1 at that position")
    probe("command_projection_source_marker_drift_is_rejected",
          [(D26, "<!-- PRODUCT-COMMANDS:BEGIN generated from spec/commands/types.rs "
                 "by bootstrap/project.py; do not edit -->",
            "<!-- PRODUCT-COMMANDS:BEGIN generated from spec/command_soup.rs "
                 "by bootstrap/project.py; do not edit -->")],
          "docs/26_COMMAND_PLANE.md carries no generated PRODUCT-COMMANDS "
          "block naming spec/commands/types.rs as source")
    probe("universal_command_id_is_absent_from_the_public_spec",
          [(CM, "use crate::guarantees::ContractId;",
            "use crate::guarantees::ContractId;\npub struct CommandId;")],
          "declares a universal CommandId; command identity lives in three "
          "separate namespaces or nowhere")
    return findings


def test_mutation(audit) -> list[str]:
    """Named hostile fixtures for the canonical mutation vocabulary (5.5E3f):
    one typed owner, total classification functions, no shadow tables, no
    lettered lane nicknames."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.mutation_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.mutation_findings(root):
        fail("mutation_vocabulary_passes_on_the_real_seed")

    MU = "spec/mutation.rs"
    D12 = "docs/12_TESTPAK.md"
    probe("mutation_lane_missing_from_all_is_rejected",
          [(MU, "        MutationLane::SelectableCompiled,\n", "")],
          "MutationLane::SelectableCompiled is omitted from MutationLane::ALL")
    probe("mutation_result_missing_from_all_is_rejected",
          [(MU, "        MutationResult::TimedOut,\n", "")],
          "MutationResult::TimedOut is omitted from MutationResult::ALL")
    probe("mutation_lane_with_unknown_owner_is_rejected",
          [(MU, '| MutationLane::CompilerBacked => ContractId("BP-TESTPAK-1"),',
            '| MutationLane::CompilerBacked => ContractId("BP-GHOST-1"),')],
          "MutationLane SemanticIr cites owner BP-GHOST-1, which no declared "
          "contract owns")
    probe("mutation_lane_with_dangling_admission_basis_is_rejected",
          [(MU, '| MutationLane::CompilerBacked => DecisionId("DEC-015"),',
            '| MutationLane::CompilerBacked => DecisionId("DEC-915"),')],
          "lane SemanticIr cites admission basis DEC-915, which no declared "
          "decision owns")
    # The admission boundary must NAME what it admits: a DEC-015 amendment
    # that stops spelling a lane un-admits it.
    probe("mutation_lane_basis_must_name_the_lane",
          [("spec/dispositions.rs",
            "SemanticIr mutates BatPak-owned semantic structures",
            "The semantic lane mutates BatPak-owned semantic structures")],
          "DEC-015 forward-policy fields do not name lane SemanticIr")
    # The semantic fences.
    probe("semantic_ir_cannot_require_real_rustc_semantics",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,",
            "            MutationLane::SemanticIr => true,\n"
                "            MutationLane::SelectableCompiled => true,")],
          "SemanticIr claims real rustc semantics; the reference interpreter "
          "lane must not")
    probe("selectable_compiled_requires_real_rustc_semantics",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,")],
          "SelectableCompiled must run under real rustc semantics")
    probe("selectable_compiled_does_not_require_per_candidate_compile",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,\n"
                "            MutationLane::CompilerBacked => true,")],
          "SelectableCompiled must not require a per-candidate Rust compile")
    probe("compiler_backed_requires_per_candidate_compile",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => false,")],
          "CompilerBacked must require a per-candidate Rust compile")
    probe("lane_without_activation_evidence_is_rejected",
          [(MU, "            MutationLane::SemanticIr => true,\n"
                "            MutationLane::SelectableCompiled => true,\n"
                "            MutationLane::CompilerBacked => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,\n"
                "            MutationLane::CompilerBacked => true,")],
          "lane SemanticIr does not require activation evidence")
    probe("lane_permitting_production_profile_slots_is_rejected",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => false,",
            "            MutationLane::SemanticIr => true,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => false,")],
          "lane SemanticIr permits production-profile mutation slots")
    probe("lane_without_independent_evidence_route_is_rejected",
          [(MU, "pub const fn requires_independent_evidence_route(self) -> bool {\n"
                "        match self {\n"
                "            MutationLane::SemanticIr => true,",
            "pub const fn requires_independent_evidence_route(self) -> bool {\n"
                "        match self {\n"
                "            MutationLane::SemanticIr => false,")],
          "lane SemanticIr does not require an independent evidence route")
    probe("mutation_lane_wrong_gate_is_rejected",
          [(MU, "            MutationLane::SemanticIr => &[GateId::G3],",
            "            MutationLane::SemanticIr => &[GateId::G2],")],
          "lane SemanticIr gates ['G2'] are not exactly G3")
    # The classification teeth.
    probe("only_killed_counts_as_kill",
          [(MU, "            MutationResult::Survived => false,",
            "            MutationResult::Survived => true,")],
          "only Killed counts as a kill; Survived claims one")
    probe("only_survived_counts_as_survival",
          [(MU, "            MutationResult::Survived => true,\n"
                "            MutationResult::NotActivated => false,",
            "            MutationResult::Survived => true,\n"
                "            MutationResult::NotActivated => true,")],
          "only Survived counts as survival; NotActivated claims one")
    probe("equivalent_candidate_counts_as_neither_kill_nor_survival",
          [(MU, "            MutationResult::EquivalentCandidate => false,",
            "            MutationResult::EquivalentCandidate => true,")],
          "only Killed counts as a kill; EquivalentCandidate claims one")
    probe("unbuildable_cannot_count_as_kill",
          [(MU, "            MutationResult::Unbuildable => false,",
            "            MutationResult::Unbuildable => true,")],
          "only Killed counts as a kill; Unbuildable claims one")
    probe("every_mutation_result_stays_in_the_denominator",
          [(MU, "            MutationResult::TimedOut => true,",
            "            MutationResult::TimedOut => false,")],
          "TimedOut leaves the denominator")
    # The lettered nicknames are dead.
    probe("lane_letter_alias_is_rejected_in_authoritative_docs",
          [(D12, "### Three frozen lanes",
            "### Three frozen lanes (Lane A, Lane B, Lane C)")],
          "cites the letter alias 'Lane A' in authoritative prose")
    probe("docs_mutation_lane_projection_drift_is_rejected",
          [(D12, "SemanticIr           false   false   true    false   true    G3",
            "SemanticIr           false   true    true    false   true    G3")],
          "docs/12 MUTATION-LANES row 2 states SemanticIr false true true "
          "false true G3; the typed catalog states SemanticIr false false "
          "true false true G3 at that position")
    probe("docs_mutation_result_projection_drift_is_rejected",
          [(D12, "Killed                 true    false   true",
            "Killed                 true    true    true")],
          "docs/12 MUTATION-RESULTS row 2 states Killed true true true; the "
          "typed catalog states Killed true false true at that position")
    probe("mutation_projection_source_marker_drift_is_rejected",
          [(D12, "<!-- MUTATION-LANES:BEGIN generated from spec/mutation/types.rs "
                 "by bootstrap/project.py; do not edit -->",
            "<!-- MUTATION-LANES:BEGIN generated from spec/mutation_soup.rs "
                 "by bootstrap/project.py; do not edit -->")],
          "docs/12 carries no generated MUTATION-LANES block naming "
          "spec/mutation/types.rs as source")
    return findings


def test_corpus_epoch(audit) -> list[str]:
    """Named hostile fixtures for the corpus reconciliation epoch (5.5E3g):
    one typed owner, one current selection, corpus-wide membership, and no
    date wearing an epoch costume."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.corpus_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.corpus_findings(root):
        fail("corpus_epoch_passes_on_the_real_seed")

    CO = "spec/corpus.rs"
    D16 = "docs/16_IDENTITY_TIME_AND_NAVIGATION.md"
    _V2_ARMS = [
        (CO, "    CleanroomV1,\n}", "    CleanroomV1,\n    CleanroomV2,\n}"),
        (CO, "&[ReconciliationEpoch::CleanroomV1]",
         "&[ReconciliationEpoch::CleanroomV1, ReconciliationEpoch::CleanroomV2]"),
        (CO, 'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",',
         'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",\n'
         '            ReconciliationEpoch::CleanroomV2 => "cleanroom-v2",'),
        (CO, 'ReconciliationEpoch::CleanroomV1 => ContractId("BP-CONSTITUTION-1"),',
         'ReconciliationEpoch::CleanroomV1 => ContractId("BP-CONSTITUTION-1"),\n'
         '            ReconciliationEpoch::CleanroomV2 => ContractId("BP-CONSTITUTION-1"),'),
        (CO, 'ReconciliationEpoch::CleanroomV1 => {\n'
             '                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))\n'
             '            }',
         'ReconciliationEpoch::CleanroomV1 => {\n'
             '                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))\n'
             '            }\n'
             '            ReconciliationEpoch::CleanroomV2 => {\n'
             '                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))\n'
             '            }'),
    ]
    probe("reconciliation_epoch_missing_from_all_is_rejected",
          [(CO, "&[ReconciliationEpoch::CleanroomV1]", "&[]")],
          "ReconciliationEpoch::CleanroomV1 is omitted from ReconciliationEpoch::ALL")
    probe("current_reconciliation_epoch_must_be_declared",
          [(CO, "ReconciliationEpoch = ReconciliationEpoch::CleanroomV1;",
            "ReconciliationEpoch = ReconciliationEpoch::CleanroomV9;")],
          "CURRENT_RECONCILIATION_EPOCH names CleanroomV9, which "
          "ReconciliationEpoch::ALL does not declare")
    probe("empty_reconciliation_epoch_spelling_is_rejected",
          [(CO, 'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",',
            'ReconciliationEpoch::CleanroomV1 => "",')],
          "ReconciliationEpoch::CleanroomV1 projects an empty spelling")
    probe("duplicate_reconciliation_epoch_spelling_is_rejected",
          [_V2_ARMS[0], _V2_ARMS[1],
           (CO, 'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",',
            'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",\n'
            '            ReconciliationEpoch::CleanroomV2 => "cleanroom-v1",'),
           _V2_ARMS[3], _V2_ARMS[4]],
          "epoch spelling cleanroom-v1 is claimed twice")
    probe("reconciliation_epoch_with_unknown_owner_is_rejected",
          [(CO, 'ContractId("BP-CONSTITUTION-1")', 'ContractId("BP-GHOST-1")')],
          "epoch cleanroom-v1 cites owner BP-GHOST-1, which no declared "
          "contract owns")
    probe("reconciliation_epoch_owner_must_author_the_spelling",
          [(CO, 'ContractId("BP-CONSTITUTION-1")', 'ContractId("BP-NETBAT-1")')],
          "epoch cleanroom-v1 cites owner BP-NETBAT-1, whose authoritative "
          "document does not author ReconciliationEpoch")
    probe("reconciliation_epoch_with_dangling_basis_is_rejected",
          [(CO, 'SeedId("SEED-DOC-STATUS")', 'SeedId("SEED-GHOST")')],
          "epoch cleanroom-v1 cites admission basis SEED-GHOST, which no "
          "declared seed row owns")
    probe("reconciliation_epoch_basis_must_name_the_type",
          [("spec/invariants.rs",
            "reconciliation date, and its ReconciliationEpoch corpus membership.",
            "reconciliation date, and its corpus membership.")],
          "SEED-DOC-STATUS does not name ReconciliationEpoch in its statement")
    probe("authoritative_document_missing_epoch_is_rejected",
          [(D16, "reconciliation_epoch: cleanroom-v1\n", "")],
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md: declares no "
          "reconciliation_epoch")
    probe("authoritative_document_with_wrong_epoch_is_rejected",
          [(D16, "reconciliation_epoch: cleanroom-v1",
            "reconciliation_epoch: cleanroom-v0")],
          "claims corpus epoch cleanroom-v0; the current corpus epoch is "
          "cleanroom-v1")
    # Evidence stays in the corpus as evidence; it still claims membership.
    probe("evidence_document_missing_epoch_is_rejected",
          [("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
            "status: AUTHORITATIVE", "status: EVIDENCE-ONLY"),
           ("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
            "reconciliation_epoch: cleanroom-v1\n", "")],
          "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md: declares no "
          "reconciliation_epoch")
    probe("two_documents_cannot_claim_different_current_epochs",
          [(D16, "reconciliation_epoch: cleanroom-v1",
            "reconciliation_epoch: cleanroom-v2")],
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md: claims corpus epoch "
          "cleanroom-v2; the current corpus epoch is cleanroom-v1")
    probe("date_cannot_substitute_for_reconciliation_epoch",
          [(D16, "reconciliation_epoch: cleanroom-v1",
            "reconciliation_epoch: 2026-07-13")],
          "a date (2026-07-13) cannot substitute for a corpus epoch")
    probe("last_reconciled_cannot_be_deleted_when_epoch_exists",
          [(D16, "last_reconciled: 2026-07-13\n", "")],
          "last_reconciled is deleted; the epoch answers WHICH corpus")
    probe("generated_document_missing_source_epoch_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "source_reconciliation_epoch: cleanroom-v1\n", "")],
          "docs/GUARANTEE_GRAPH.generated.md: declares no "
          "source_reconciliation_epoch")
    probe("generated_document_with_wrong_source_epoch_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "source_reconciliation_epoch: cleanroom-v1",
            "source_reconciliation_epoch: cleanroom-v0")],
          "claims corpus epoch cleanroom-v0; the current corpus epoch is "
          "cleanroom-v1")
    probe("corpus_epoch_projection_drift_is_rejected",
          [("docs/00_CONSTITUTION.md",
            f"{'current epoch':<18} cleanroom-v1",
            f"{'current epoch':<18} cleanroom-v9")],
          "the typed owner states")
    probe("corpus_epoch_projection_source_marker_drift_is_rejected",
          [("docs/00_CONSTITUTION.md",
            "<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from "
            "spec/corpus/types.rs by bootstrap/project.py; do not edit -->",
            "<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from "
            "spec/corpse.rs by bootstrap/project.py; do not edit -->")],
          "docs/00 carries no generated CORPUS-RECONCILIATION-EPOCH block "
          "naming spec/corpus/types.rs as source")
    # Moving CURRENT without re-projecting the corpus strands every document
    # on the old epoch — the whole corpus reds, not just the constant.
    probe("changing_current_epoch_requires_whole_corpus_projection",
          _V2_ARMS + [
              (CO, "ReconciliationEpoch = ReconciliationEpoch::CleanroomV1;",
               "ReconciliationEpoch = ReconciliationEpoch::CleanroomV2;")],
          "claims corpus epoch cleanroom-v1; the current corpus epoch is "
          "cleanroom-v2")
    probe("runtime_reconciliation_module_does_not_own_corpus_epoch",
          [("spec/reconciliation.rs", "pub enum ReconciliationRole {",
            "pub struct ReconciliationEpoch;\npub enum ReconciliationRole {")],
          "spec/reconciliation/ speaks ReconciliationEpoch; the runtime "
          "reconciliation module does not own the corpus epoch")
    # GREEN growth: a declared future epoch with a unique spelling, the same
    # live constitutional owner, an authored constitutional description, and
    # ALL membership — CURRENT unchanged, no frozen count anywhere.
    tmp = gate_sandbox(_V2_ARMS + [
        ("docs/00_CONSTITUTION.md",
         "reconciliation that moves the current selection.",
         "reconciliation that moves the current selection. A future "
         "`cleanroom-v2` epoch is declared only by such a reconciliation.")])
    try:
        got = audit.corpus_findings(tmp)
        if got:
            fail(f"future_epoch_may_be_declared_without_moving_current "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_compiler_assumptions(audit) -> list[str]:
    """Named hostile fixtures for the admitted compiler-assumption kinds
    (5.5E3h): two sharp instruments, borders proven by hostiles — no hard
    violation, test allowance, dependency mechanism, or not-yet-earned idea
    can enter the ledger wearing an assumption costume."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.compiler_assumption_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.compiler_assumption_findings(root):
        fail("compiler_assumptions_pass_on_the_real_seed")

    CA = "spec/compiler_assumptions.rs"
    D19 = "docs/19_SECURITY_MODEL.md"

    def synthetic_kind(name, basis):
        """Full structural arms for a smuggled kind: everything present
        except an authoring owner and a basis that names it."""
        return [
            (CA, "    PointerProvenance,\n}",
             f"    PointerProvenance,\n    {name},\n}}"),
            (CA, "        CompilerAssumptionKind::PointerProvenance,\n    ];",
             "        CompilerAssumptionKind::PointerProvenance,\n"
             f"        CompilerAssumptionKind::{name},\n    ];"),
            (CA, 'CompilerAssumptionKind::PointerProvenance => "PointerProvenance",',
             'CompilerAssumptionKind::PointerProvenance => "PointerProvenance",\n'
             f'            CompilerAssumptionKind::{name} => "{name}",'),
            (CA, 'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),',
             'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),\n'
             f'            CompilerAssumptionKind::{name} => ContractId("BP-SECURITY-1"),'),
            (CA, 'CompilerAssumptionKind::PointerProvenance => GuaranteeRef::dec("DEC-067"),',
             'CompilerAssumptionKind::PointerProvenance => GuaranteeRef::dec("DEC-067"),\n'
             f'            CompilerAssumptionKind::{name} => GuaranteeRef::dec("{basis}"),'),
            (CA, "CompilerAssumptionKind::PointerProvenance => false,",
             "CompilerAssumptionKind::PointerProvenance => false,\n"
             f"            CompilerAssumptionKind::{name} => false,"),
            (CA, "CompilerAssumptionKind::PointerProvenance => GateId::G3,",
             "CompilerAssumptionKind::PointerProvenance => GateId::G3,\n"
             f"            CompilerAssumptionKind::{name} => GateId::G3,"),
            (CA, "CompilerAssumptionKind::PointerProvenance => GateId::G9,",
             "CompilerAssumptionKind::PointerProvenance => GateId::G9,\n"
             f"            CompilerAssumptionKind::{name} => GateId::G9,"),
        ]

    probe("compiler_assumption_kind_missing_from_all_is_rejected",
          [(CA, "        CompilerAssumptionKind::PointerProvenance,\n    ];", "    ];")],
          "CompilerAssumptionKind::PointerProvenance is omitted from "
          "CompilerAssumptionKind::ALL")
    probe("assumption_kind_with_unknown_owner_is_rejected",
          [(CA, 'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),',
            'CompilerAssumptionKind::PointerProvenance => ContractId("BP-GHOST-1"),')],
          "assumption kind PointerProvenance cites owner BP-GHOST-1, which no "
          "declared contract owns")
    probe("assumption_kind_owner_must_author_the_spelling",
          [(CA, 'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),',
            'CompilerAssumptionKind::PointerProvenance => ContractId("BP-NETBAT-1"),')],
          "assumption kind PointerProvenance cites owner BP-NETBAT-1, whose "
          "authoritative document does not author the spelling")
    probe("assumption_kind_with_dangling_basis_is_rejected",
          [(CA, 'GuaranteeRef::dec("DEC-067"),', 'GuaranteeRef::dec("DEC-967"),')],
          "assumption kind PointerProvenance cites admission basis DEC-967, "
          "which no declared decision owns")
    probe("assumption_kind_basis_must_name_the_kind",
          [("spec/dispositions.rs",
            "pointer and integer-pointer crossings is PointerProvenance",
            "pointer and integer-pointer crossings is the pointer kind")],
          "assumption kind PointerProvenance cites admission basis DEC-067, "
          "whose forward-policy fields do not name the kind")
    probe("only_unsafe_memory_contract_requires_safety_marker",
          [(CA, "CompilerAssumptionKind::PointerProvenance => false,",
            "CompilerAssumptionKind::PointerProvenance => true,")],
          "only UnsafeMemoryContract intrinsically requires the SAFETY-CONTRACT "
          "marker; PointerProvenance claims it")
    probe("unsafe_memory_contract_must_require_safety_marker",
          [(CA, "CompilerAssumptionKind::UnsafeMemoryContract => true,",
            "CompilerAssumptionKind::UnsafeMemoryContract => false,")],
          "UnsafeMemoryContract must intrinsically require the SAFETY-CONTRACT "
          "marker")
    probe("assumption_kind_wrong_classification_gate_is_rejected",
          [(CA, "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G3,",
            "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G2,")],
          "assumption kind UnsafeMemoryContract classifies at G2; the ledger "
          "boundary is classified at G3")
    probe("assumption_kind_wrong_release_gate_is_rejected",
          [(CA, "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G9,",
            "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G8,")],
          "assumption kind UnsafeMemoryContract qualifies for release at G8; "
          "the release seal consumes the ledger at G9")
    # The border: nothing enters without a basis that names it and an owner
    # that authors it — a decision sentence about a mechanism is an admission
    # direction, not a passport.
    for name, basis, fixture in (
            ("NarrowingNumericCast", "DEC-067",
             "narrowing_numeric_cast_cannot_become_a_ledger_escape"),
            ("LintSuppression", "DEC-068",
             "production_lint_suppression_cannot_become_an_assumption_kind"),
            ("ProductionExpect", "DEC-067",
             "production_expect_cannot_become_an_assumption_kind"),
            ("TestContextualExpect", "DEC-068",
             "test_contextual_expect_is_an_allowance_not_an_assumption"),
            ("SerdeInterop", "DEC-020",
             "compatibility_dependency_is_not_a_compiler_assumption"),
            ("RepresentationLayout", "DEC-067",
             "representation_layout_is_not_currently_admitted"),
            ("AtomicOrdering", "DEC-068",
             "atomic_ordering_is_not_currently_admitted"),
            ("FfiAbi", "DEC-068", "ffi_abi_is_not_currently_admitted"),
            ("TargetFeature", "DEC-068",
             "target_feature_is_not_currently_admitted"),
    ):
        probe(fixture, synthetic_kind(name, basis),
              f"assumption kind {name} cites admission basis {basis}, whose "
              "forward-policy fields do not name the kind")
    probe("docs_compiler_assumption_projection_drift_is_rejected",
          [(D19, "UnsafeMemoryContract   DEC-068   true    G3        G9",
            "UnsafeMemoryContract   DEC-068   false   G3        G9")],
          "docs/19 COMPILER-ASSUMPTION-KINDS row 2 states UnsafeMemoryContract "
          "DEC-068 false G3 G9; the typed catalog states UnsafeMemoryContract "
          "DEC-068 true G3 G9 at that position")
    probe("compiler_assumption_projection_source_marker_drift_is_rejected",
          [(D19, "<!-- COMPILER-ASSUMPTION-KINDS:BEGIN generated from "
                 "spec/compiler_assumptions/types.rs by bootstrap/project.py; do not edit -->",
            "<!-- COMPILER-ASSUMPTION-KINDS:BEGIN generated from "
                 "spec/compiler_presumptions.rs by bootstrap/project.py; do not edit -->")],
          "docs/19 carries no generated COMPILER-ASSUMPTION-KINDS block naming "
          "spec/compiler_assumptions/types.rs as source")
    probe("universal_assumption_identity_is_absent_from_the_public_spec",
          [(CA, "use crate::gates::GateId;",
            "use crate::gates::GateId;\npub struct CompilerAssumptionId;")],
          "declares a universal assumption identity")
    probe("reconciliation_module_does_not_speak_assumption_kinds",
          [("spec/reconciliation.rs", "pub enum ReconciliationRole {",
            "pub struct CompilerAssumptionKindMirror(pub CompilerAssumptionKind);\n"
            "pub enum ReconciliationRole {")],
          "spec/reconciliation/ speaks CompilerAssumptionKind")
    probe("architecture_module_does_not_reexport_assumption_kinds",
          [("spec/architecture/inventory.rs", "pub const REPOSITORY_NAME:",
            "pub use crate::compiler_assumptions::CompilerAssumptionKind;\n"
            "pub const REPOSITORY_NAME:")],
          "spec/architecture/ speaks CompilerAssumptionKind")
    # GREEN structural growth — evidence that the parser and projection are
    # count-free ONLY: every structural surface present (variant, ALL, unique
    # spelling, live authoring owner, live decision naming it, full arms,
    # matching docs/19 row). It does not prove a real future adopter exists.
    tmp = gate_sandbox(synthetic_kind("FutureKindProbe", "DEC-068") + [
        ("spec/dispositions.rs",
         "unsafe blocks functions and impls is UnsafeMemoryContract;",
         "unsafe blocks functions and impls is UnsafeMemoryContract and a "
         "sandbox structural probe kind named FutureKindProbe;"),
        (D19, "A future kind enters",
         "A sandbox structural probe (`FutureKindProbe`) exercises this "
         "boundary as growth evidence only. A future kind enters"),
        (D19, "PointerProvenance      DEC-067   false   G3        G9",
         "PointerProvenance      DEC-067   false   G3        G9\n"
         "FutureKindProbe        DEC-068   false   G3        G9"),
    ])
    try:
        got = audit.compiler_assumption_findings(tmp)
        if got:
            fail(f"structural_future_kind_growth_is_count_free (refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_promotion(audit) -> list[str]:
    """Named hostile fixtures for the typed promotion denominator (5.5E3i):
    four bouncers with badges, all conjunctive, and the receipt cannot claim
    the other three showed up."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.promotion_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.promotion_findings(root):
        fail("promotion_requirements_pass_on_the_real_seed")

    PR = "spec/promotion.rs"
    D12 = "docs/12_TESTPAK.md"
    probe("promotion_requirement_missing_from_all_is_rejected",
          [(PR, "        PromotionRequirement::AuditablePromotionReceipt,\n    ];",
            "    ];")],
          "PromotionRequirement::AuditablePromotionReceipt is omitted from "
          "PromotionRequirement::ALL; the denominator is conjunctive")
    probe("duplicate_promotion_requirement_spelling_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => "NamedProofTarget",',
            'PromotionRequirement::NamedProofTarget => "IndependentEvidenceRoute",')],
          "promotion-requirement spelling IndependentEvidenceRoute is claimed twice")
    probe("empty_promotion_requirement_spelling_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => "NamedProofTarget",',
            'PromotionRequirement::NamedProofTarget => "",')],
          "PromotionRequirement::NamedProofTarget projects an empty spelling")
    probe("promotion_requirement_with_unknown_owner_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => ContractId("BP-TESTPAK-1"),',
            'PromotionRequirement::NamedProofTarget => ContractId("BP-GHOST-1"),')],
          "promotion requirement NamedProofTarget cites owner BP-GHOST-1, "
          "which no declared contract owns")
    probe("promotion_requirement_owner_must_author_the_spelling",
          [(PR, 'PromotionRequirement::NamedProofTarget => ContractId("BP-TESTPAK-1"),',
            'PromotionRequirement::NamedProofTarget => ContractId("BP-NETBAT-1"),')],
          "promotion requirement NamedProofTarget cites owner BP-NETBAT-1, "
          "whose authoritative document does not author the spelling")
    probe("promotion_requirement_with_dangling_basis_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => GuaranteeRef::dec("DEC-015"),',
            'PromotionRequirement::NamedProofTarget => GuaranteeRef::dec("DEC-915"),')],
          "promotion requirement NamedProofTarget cites admission basis DEC-915, "
          "which no declared decision owns")
    probe("promotion_requirement_basis_must_name_the_requirement",
          [("spec/dispositions.rs",
            "is IndependentEvidenceRoute NamedProofTarget QualifiedHostileEvidence",
            "is IndependentEvidenceRoute QualifiedHostileEvidence")],
          "promotion requirement NamedProofTarget cites admission basis DEC-015, "
          "whose forward-policy fields do not name the requirement")
    # The doctrine clauses are law; deleting one reds.
    probe("promotion_requirements_are_conjunctive",
          [(D12, "a missing member\nrefuses promotion,", "each matters,")],
          "docs/12 promotion doctrine no longer states: the requirements are "
          "conjunctive")
    probe("independent_evidence_route_cannot_be_self_grading",
          [(D12, "are not independence.", "are usually enough.")],
          "docs/12 promotion doctrine no longer states: the evidence route "
          "cannot be self-grading")
    probe("named_proof_target_cannot_be_a_generic_issue",
          [(D12, "A generic issue link, chat\ntranscript,", "A chat\ntranscript,")],
          "docs/12 promotion doctrine no longer states: a generic issue is not "
          "a named proof target")
    probe("named_proof_target_cannot_be_a_commit_message",
          [(D12, "chat\ntranscript, commit message, or vague",
            "chat\ntranscript or vague")],
          "docs/12 promotion doctrine no longer states: a commit message is not "
          "a named proof target")
    probe("documented_proof_gap_requires_a_stable_owner",
          [(D12, "a stable name, a ContractId owner, the affected",
            "a name and the affected")],
          "docs/12 promotion doctrine no longer states: a documented proof gap "
          "requires a stable owner")
    probe("qualified_hostile_evidence_requires_rule_qualification",
          [(D12, "satisfying the docs/24 Rule qualification law",
            "satisfying a reviewer")],
          "docs/12 promotion doctrine no longer states: qualified hostile "
          "evidence consumes the docs/24 Rule qualification law")
    for token, fixture in (
            ("NotActivated", "not_activated_cannot_satisfy_qualified_hostile_evidence"),
            ("Unbuildable", "unbuildable_cannot_satisfy_qualified_hostile_evidence"),
            ("InfrastructureFailure",
             "infrastructure_failure_cannot_satisfy_qualified_hostile_evidence")):
        probe(fixture,
              [(D12, f"{token},", "")] if token != "InfrastructureFailure" else
              [(D12, "TimedOut, InfrastructureFailure,", "TimedOut,")],
              f"docs/12 no longer states that {token} cannot satisfy "
              "QualifiedHostileEvidence")
    probe("equivalent_candidate_is_not_a_killed_mutant",
          [(D12, "EquivalentCandidate without its independent equivalence witness,",
            "EquivalentCandidate,")],
          "docs/12 promotion doctrine no longer states: an equivalent candidate "
          "is not a killed mutant")
    probe("baseline_failure_cannot_kill_a_candidate",
          [(D12, "or a failure produced by the unchanged baseline.",
            "or a flaky run.")],
          "docs/12 promotion doctrine no longer states: a baseline failure "
          "cannot kill a candidate")
    probe("commit_message_cannot_substitute_for_promotion_receipt",
          [(D12, "A commit message\nis not this receipt.", "The receipt is above.")],
          "docs/12 promotion doctrine no longer states: a commit message is not "
          "the promotion receipt")
    probe("promotion_receipt_must_bind_resulting_tracked_content",
          [(D12, "promotion decision, and resulting tracked-content commitment.",
            "and promotion decision.")],
          "docs/12 promotion doctrine no longer states: the receipt binds the "
          "resulting tracked-content commitment")
    # The family facts answer four different questions.
    probe("promotion_policy_surface_must_be_candidate_promotion",
          [(PR, "ProofPolicySurface::CandidatePromotion;",
            "ProofPolicySurface::WaiverLogic;")],
          "the promotion policy surface is not ProofPolicySurface::CandidatePromotion")
    probe("promotion_change_basis_must_be_dec074",
          [(PR, 'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec("DEC-074");',
            'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec("DEC-015");')],
          "the promotion policy-change basis is not DEC-074")
    probe("promotion_enforcement_gate_must_be_g3",
          [(PR, "PROMOTION_ENFORCEMENT_GATE: GateId = GateId::G3;",
            "PROMOTION_ENFORCEMENT_GATE: GateId = GateId::G9;")],
          "candidate-promotion policy is not enforced at G3")
    probe("promotion_release_visibility_gate_must_be_g9",
          [(PR, "PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::G9;",
            "PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::G3;")],
          "promotion policy changes are not release-visibly qualified at G9")
    probe("docs_promotion_projection_drift_is_rejected",
          [(D12, "NamedProofTarget           BP-TESTPAK-1   DEC-015",
            "NamedProofTarget           BP-TESTPAK-1   DEC-074")],
          "docs/12 PROMOTION-REQUIREMENTS row 3 states NamedProofTarget "
          "BP-TESTPAK-1 DEC-074; the typed catalog states NamedProofTarget "
          "BP-TESTPAK-1 DEC-015 at that position")
    probe("promotion_projection_source_marker_drift_is_rejected",
          [(D12, "<!-- PROMOTION-REQUIREMENTS:BEGIN generated from "
                 "spec/promotion/types.rs by bootstrap/project.py; do not edit -->",
            "<!-- PROMOTION-REQUIREMENTS:BEGIN generated from "
                 "spec/demotion.rs by bootstrap/project.py; do not edit -->")],
          "docs/12 carries no generated PROMOTION-REQUIREMENTS block naming "
          "spec/promotion/types.rs as source")
    probe("docs24_must_consume_rule_qualification_without_reowning_requirements",
          [("docs/24_GAUNTLET.md",
            "is not restated here.",
            "is not restated here, except IndependentEvidenceRoute.")],
          "docs/24 restates the promotion-requirement inventory")
    probe("mutation_lane_consumer_cannot_point_to_architecture_module",
          [("docs/24_GAUNTLET.md",
            "The typed names are `MutationLane` in `spec/mutation/types.rs`",
            "The typed names are `MutationLane` in `spec/architecture.rs`")],
          "docs/24 points MutationLane at spec/architecture; the typed owner "
          "moved to spec/mutation/types.rs")
    probe("docs31_cannot_restate_a_second_promotion_inventory",
          [("docs/31_FINAL_CONTRADICTION_AUDIT.md",
            "`PromotionRequirement::ALL` is unsatisfied",
            "`PromotionRequirement::ALL` (IndependentEvidenceRoute, "
            "NamedProofTarget, QualifiedHostileEvidence, "
            "AuditablePromotionReceipt) is unsatisfied")],
          "docs/31 restates IndependentEvidenceRoute; the audit consumer owns "
          "no second promotion inventory")
    # GREEN structural growth — count-freedom evidence only.
    tmp = gate_sandbox([
        (PR, "    AuditablePromotionReceipt,\n}",
         "    AuditablePromotionReceipt,\n    ReproducibleCandidateOrigin,\n}"),
        (PR, "        PromotionRequirement::AuditablePromotionReceipt,\n    ];",
         "        PromotionRequirement::AuditablePromotionReceipt,\n"
         "        PromotionRequirement::ReproducibleCandidateOrigin,\n    ];"),
        (PR, 'PromotionRequirement::AuditablePromotionReceipt => "AuditablePromotionReceipt",',
         'PromotionRequirement::AuditablePromotionReceipt => "AuditablePromotionReceipt",\n'
         '            PromotionRequirement::ReproducibleCandidateOrigin => "ReproducibleCandidateOrigin",'),
        (PR, 'PromotionRequirement::AuditablePromotionReceipt => ContractId("BP-TESTPAK-1"),',
         'PromotionRequirement::AuditablePromotionReceipt => ContractId("BP-TESTPAK-1"),\n'
         '            PromotionRequirement::ReproducibleCandidateOrigin => ContractId("BP-TESTPAK-1"),'),
        (PR, 'PromotionRequirement::AuditablePromotionReceipt => GuaranteeRef::dec("DEC-015"),',
         'PromotionRequirement::AuditablePromotionReceipt => GuaranteeRef::dec("DEC-015"),\n'
         '            PromotionRequirement::ReproducibleCandidateOrigin => GuaranteeRef::dec("DEC-015"),'),
        ("spec/dispositions.rs",
         "QualifiedHostileEvidence and AuditablePromotionReceipt",
         "QualifiedHostileEvidence AuditablePromotionReceipt and "
         "ReproducibleCandidateOrigin"),
        (D12, "`AuditablePromotionReceipt`: the complete receipt below.",
         "`ReproducibleCandidateOrigin`: a sandbox structural probe as growth "
         "evidence only. `AuditablePromotionReceipt`: the complete receipt below."),
        (D12, "AuditablePromotionReceipt  BP-TESTPAK-1   DEC-015",
         "AuditablePromotionReceipt  BP-TESTPAK-1   DEC-015\n"
         "ReproducibleCandidateOrigin BP-TESTPAK-1   DEC-015"),
    ])
    try:
        got = audit.promotion_findings(tmp)
        if got:
            fail(f"structural_future_requirement_growth_is_count_free "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_seedcheck_executes_its_law(_audit) -> list[str]:
    """Tier 0 rules, proven by RUNNING seedcheck against mutated typed sources.

    Two rules were repaired in 5.5D4c2 after seedcheck was executed for the first
    time. A repair with no adversary is indistinguishable from a nerf, so each one
    is planted against here. These build and run the real binary: asserting on the
    source text would prove the file contains a string, not that the gate bites.
    """
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        receipt("tier0-law-fixtures", available=False, reason="rustc absent")
        return findings

    used_triples: set[str] = set()

    def run_seedcheck(tree: Path) -> tuple[int, str]:
        # seedcheck links the typed tables at COMPILE time through the tree's
        # spec rlib (5.5E2), so a mutated spec/ is only tested by building the
        # library from the mutated tree. The real seedcheck.rs is copied in
        # rather than reproduced: a hand-written stand-in would drift from the
        # gate it claims to test. A mutation the COMPILER refuses is a refusal,
        # not linker unavailability: its rustc output is returned so a probe
        # can assert the exact reason instead of silently skipping.
        (tree / "bootstrap").mkdir(exist_ok=True)
        if not (tree / "bootstrap/seedcheck.rs").is_file():
            shutil.copy2(root / "bootstrap/seedcheck.rs", tree / "bootstrap/seedcheck.rs")
        # seedcheck is a module tree now (Wave-2 SW4): the root shim mounts
        # bootstrap/seedcheck/*.rs via #[path], so the package must ride along or
        # the build cannot resolve the mounted modules.
        if (root / "bootstrap/seedcheck").is_dir() and not (tree / "bootstrap/seedcheck").is_dir():
            shutil.copytree(root / "bootstrap/seedcheck", tree / "bootstrap/seedcheck")
        tmp = Path(tempfile.mkdtemp(prefix="batpak-tier0-"))
        try:
            exe = tmp / ("sc" + (".exe" if os.name == "nt" else ""))
            built = None
            for target in (None, "x86_64-pc-windows-gnu"):
                rlib = tmp / f"libspec-{target or 'host'}.rlib"
                lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                           "--crate-name", "spec", "-o", str(rlib),
                           str(tree / "spec/lib.rs")]
                if target:
                    lib_cmd[1:1] = ["--target", target]
                lib_built = subprocess.run(lib_cmd, capture_output=True, text=True)
                if lib_built.returncode != 0:
                    if "linking with" not in lib_built.stderr:
                        return (lib_built.returncode, lib_built.stderr)
                    continue
                cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", "sc",
                       "--extern", f"spec={rlib}",
                       "-o", str(exe), str(tree / "bootstrap/seedcheck.rs")]
                if target:
                    cmd[1:1] = ["--target", target]
                built = subprocess.run(cmd, capture_output=True, text=True)
                if built.returncode == 0:
                    # The receipt names the real triple, never "host default"
                    # (5.5E2c): on the hosted MSVC runner the default-target
                    # build IS the authoritative qualification, and a
                    # law-fixture receipt that hides its triple can never
                    # enter the required-receipt denominator.
                    if target is None:
                        m = re.search(r"host: (\S+)",
                                      subprocess.run([rustc, "-vV"], capture_output=True,
                                                     text=True).stdout)
                        target = m.group(1) if m else "host default"
                    used_triples.add(target)
                    break
                if "linking with" not in built.stderr:
                    return (built.returncode, built.stderr)
            else:
                return (-1, "unavailable")
            if not exe.is_file():
                return (-1, "unavailable")
            proc = subprocess.run([str(exe), str(tree)], capture_output=True, text=True)
            return (proc.returncode, proc.stdout + proc.stderr)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def probe(name: str, rel: str, old: str, new: str, needle: str) -> None:
        rel = _resolve_edit_carrier(root, rel, old)
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            code, out = run_seedcheck(tmp)
            if code == -1:
                print(f"selftest: {name} unavailable (no working linker)")
                return
            if code == 0:
                findings.append(f"{name} FAILED (seedcheck admitted the mutation)")
            elif needle not in out:
                findings.append(f"{name} FAILED (wanted {needle!r}, got {out.strip()[:200]!r})")

    # The premise: Tier 0 passes before any mutation. Without this every probe
    # below could be red for free.
    code, out = run_seedcheck(root)
    if code == -1:
        receipt("tier0-law-fixtures", available=False, compiled=True, executed=False,
                reason="no working linker for any attempted target")
        return findings
    if code != 0:
        receipt("tier0-law-fixtures", available=True, compiled=True, executed=True,
                passed=False, reason="seedcheck already fails before any mutation")
        findings.append(f"seedcheck_passes_before_mutation FAILED ({out.strip()[:200]!r})")
        return findings

    DI = "spec/dispositions.rs"
    IN = "spec/invariants.rs"

    # DEC-072: the CLASS decides. A class that requires a gate cannot be empty.
    probe("gate_requiring_decision_class_with_no_gate_is_rejected", DI,
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8],',
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, gates: &[],',
          "DEC-063 is implementation-bearing and names no gate")

    # ...and the exception is DEC-specific. SEED declares no gate-independent
    # class, so it must not inherit DEC's lawful emptiness. This is the probe that
    # separates the repair from a nerf.
    probe("seed_does_not_inherit_the_decision_gate_exception", IN,
          "gates: &[GateId::G0, GateId::G5]",
          "gates: &[]",
          "names no gate")

    # A typed relation carries its family in the TYPE; existence is this
    # executed law. A reference to an undeclared decision must redden the
    # running seedcheck, not merely fail to render an edge.
    probe("dangling_typed_relation_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-068")]',
          'derives_from: &[GuaranteeRef::dec("DEC-999")]',
          "which no declared row owns")

    # The same existence law for witness citations (5.5E2): a witness naming
    # an undeclared obligation reddens the running binary.
    probe("dangling_witness_citation_is_rejected", IN,
          'witnesses: &[WitnessRef::leg("LEG-066"), '
          'WitnessRef::contract("BP-GAUNTLET-1")]',
          'witnesses: &[WitnessRef::leg("LEG-999"), '
          'WitnessRef::contract("BP-GAUNTLET-1")]',
          "which no declared row owns")

    # A contract kind's admitting law resolves or the running binary reddens:
    # a kind whose law was deleted survives as nothing (5.5E3c).
    probe("contract_kind_with_dangling_admitting_law_is_rejected",
          "spec/contracts.rs",
          'ContractKind::Error => GuaranteeRef::leg("LEG-047"),',
          'ContractKind::Error => GuaranteeRef::leg("LEG-999"),',
          "resolves to no declared row")

    # Retirement is supersession with a forwarding address that must point at
    # the LIVING CENSUS (5.5E2j): a successor resolves inside the typed
    # catalog or the running binary reddens.
    probe("dangling_proof_row_successor_is_rejected", "spec/proof.rs",
          'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
          'successors: &[ProofRowId("row_that_was_never_authored_anywhere")]',
          "resolves to no typed catalog identity")

    # ...and a name that occurs in docs/24 PROSE is not a catalog identity:
    # under the deleted substring search, "committed" (which opens a substrate
    # prose line) satisfied existence.
    probe("retired_successor_mentioned_only_in_prose_is_rejected", "spec/proof.rs",
          'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
          'successors: &[ProofRowId("committed")]',
          "resolves to no typed catalog identity")

    # The census defect (an active row deleted while a migration note kept
    # its name) retired with the docs/24 membership parity in 5.5E4d: the
    # typed catalog in spec/proof.rs is the one membership authority, and
    # deleting a typed row takes its guarantee binding and its docs/21
    # projection with it -- witness_reference_findings and the ledger-parity
    # laws own that refusal now.

    # The RUNNING binary refuses a hand-edited toolchain projection and a
    # physical triple sitting in a semantic qualification target (5.5E3a).
    probe("hand_edited_toolchain_projection_reddens_seedcheck",
          "rust-toolchain.toml",
          'channel = "1.97.0"',
          'channel = "1.96.0"',
          "toolchain tracked rust-toolchain.toml violated")
    # 5.5E3a1: the environment is a closed enum, so a triple in the field is
    # a COMPILE refusal — the strongest form. run_seedcheck returns the rustc
    # output for a compile-refused mutation, and the probe asserts the exact
    # type error rather than a runtime finding.
    probe("physical_triple_cannot_substitute_for_semantic_profile",
          "spec/architecture/inventory.rs",
          'environment: QualificationEnvironment::WasmHost,',
          'environment: "wasm32-unknown-unknown",',
          "expected `QualificationEnvironment`")
    # 5.5E3d residue lifecycle, executed by the RUNNING gate: a decision
    # retained only as historical coverage cannot own a future admission
    # barrier, and an existing typed owner's spelling is never re-admitted.
    probe("seedcheck_historical_decision_cannot_own_not_yet_admitted_term",
          "spec/identities.rs",
          'NotYetAdmittedBy(DecisionId("DEC-063"))',
          'NotYetAdmittedBy(DecisionId("DEC-005"))',
          "CompressionId is not yet admitted by DEC-005, which is retained "
          "only as historical coverage and cannot own a future admission barrier")
    probe("seedcheck_wrapper_passport_is_rejected",
          "spec/identities.rs",
          'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
          'entry!("PackageId", "BP-CRYPTO-SECRET-1")',
          "duplicates PackageId, which already has a typed spec owner")
    # 5.5E3d1: the typed predicate, not the Permanent lifetime, decides who
    # owns a pending application — Kill is a dead passport.
    probe("seedcheck_killed_decision_cannot_own_not_yet_admitted_term",
          "spec/dispositions.rs",
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Kill,',
          "CompressionId is not yet admitted by DEC-063, whose Kill disposition is "
          "not a standing future-entry policy; only Lock and Defer own pending "
          "applications")

    # SEED-AUDITED-DENOMINATOR's fence is executed law: reclassifying Expired
    # as green must redden the running seedcheck through
    # is_positive_semantic_terminal().
    probe("expired_proof_counting_green_is_refused",
          "spec/proof.rs",
          "            ProofUnitTerminal::Passed => true,\n"
          "            ProofUnitTerminal::Failed\n"
          "            | ProofUnitTerminal::Refused\n"
          "            | ProofUnitTerminal::Unsupported\n"
          "            | ProofUnitTerminal::SkippedWithAuthority\n"
          "            | ProofUnitTerminal::Expired\n"
          "            | ProofUnitTerminal::Superseded => false,",
          "            ProofUnitTerminal::Passed | ProofUnitTerminal::Expired => true,\n"
          "            ProofUnitTerminal::Failed\n"
          "            | ProofUnitTerminal::Refused\n"
          "            | ProofUnitTerminal::Unsupported\n"
          "            | ProofUnitTerminal::SkippedWithAuthority\n"
          "            | ProofUnitTerminal::Superseded => false,",
          "is positive; only Passed may be")

    # DEC-075's retry fence is executed law, not a comment: reclassifying
    # elapsed wall time as an admissible retry signal must redden the running
    # seedcheck through the real admissible() function.
    probe("weakened_retry_classification_is_refused",
          "spec/reconciliation.rs",
          "            | RetrySignal::OverallMonotonicDeadline\n"
          "            | RetrySignal::RuntimeRestartAuthorization => true,\n"
          "            RetrySignal::ElapsedWallTime\n"
          "            | RetrySignal::ProcessDeath",
          "            | RetrySignal::OverallMonotonicDeadline\n"
          "            | RetrySignal::ElapsedWallTime\n"
          "            | RetrySignal::RuntimeRestartAuthorization => true,\n"
          "            RetrySignal::ProcessDeath",
          "elapsed wall time may not authorize retry")

    # Candidate containment is one law across two constants: an output root
    # relocated under a forbidden write surface must redden seedcheck, not wait
    # for Phase 6 to write a candidate into tracked source.
    probe("candidate_output_root_under_a_forbidden_root_is_rejected",
          "spec/sprouting/inventory.rs",
          'pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";',
          'pub const CANDIDATE_OUTPUT_ROOT: &str = "spec/candidates/";',
          "sits under forbidden write root")

    # A generated projection is classified by its MARKER, not its filename. An
    # authored document cannot shed its frontmatter duties by looking generated,
    # and a projection that drops its marker takes the authored duties back.
    probe("authored_document_cannot_evade_frontmatter_by_looking_generated",
          "docs/GUARANTEE_GRAPH.generated.md",
          "status: GENERATED", "status: AUTHORITATIVE",
          "missing contract_id: in docs/GUARANTEE_GRAPH.generated.md")

    # A generated projection must still name what produced it. Provenance is the
    # price of not claiming authored authority.
    probe("generated_projection_without_source_provenance_is_rejected",
          "docs/GUARANTEE_GRAPH.generated.md",
          "generated_from:", "produced_from:",
          "missing generated_from: in docs/GUARANTEE_GRAPH.generated.md")

    receipt("tier0-law-fixtures", available=True, compiled=True, executed=True,
            passed=not findings,
            target=", ".join(sorted(used_triples)) or "no triple resolved")
    return findings


def test_rust_specification_compiles(_audit) -> list[str]:
    """The typed specification surface must actually compile (5.5D4b).

    Until this session no rustc was available and every Rust contract shipped
    compiler-unverified. When a toolchain IS present this runs; when it is absent
    it reports that plainly rather than passing silently. A skipped check that
    prints PASS is exactly the fake proof this project refuses.
    """
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: rustc unavailable; Rust specification surface remains "
              "compiler-unverified until Gate 0")
        return findings
    tmp = Path(tempfile.mkdtemp(prefix="batpak-rustc-"))
    try:
        # The spec is a real LIBRARY (spec/lib.rs, 5.5E2): one rlib boundary,
        # edition 2024, zero suppressions. The binaries LINK it; nothing mounts
        # spec modules textually anymore, so the visibility the probes below
        # attack is the exact boundary production uses.
        spec_meta = tmp / "libspec.rmeta"
        proc = subprocess.run(
            [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib", "--emit=metadata",
             "--crate-name", "spec", "-o", str(spec_meta), str(root / "spec/lib.rs")],
            capture_output=True, text=True)
        if proc.returncode != 0:
            head = "\n".join(proc.stderr.splitlines()[:6])
            findings.append(f"rust specification surface does not compile (spec):\n{head}")
            return findings
        for name, src in (
            ("seedcheck", root / "bootstrap/seedcheck.rs"),
            ("materialize", root / "bootstrap/materialize.rs"),
        ):
            proc = subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--emit=metadata", "--crate-name", name,
                 "--extern", f"spec={spec_meta}",
                 "-o", str(tmp / (name + ".rmeta")), str(src)],
                capture_output=True, text=True)
            if proc.returncode != 0:
                head = "\n".join(proc.stderr.splitlines()[:6])
                findings.append(
                    f"rust specification surface does not compile ({name}):\n{head}")
        # seedcheck is Tier 0. Compiling it is not running it: until this check
        # existed it was only ever type-checked, and it had been FAILING since
        # 5.5C1 -- three DEC rows red against a gate rule its own file already
        # said the class decides, and the generated Guarantee Graph red against a
        # frontmatter rule that predated generated documents. Nobody saw it
        # because nobody ran it. A gate that is compiled and never executed is a
        # gate in the same sense that a locked door in a field is a gate.
        # Local execution is SUPPLEMENTAL evidence. The authoritative Windows
        # receipt is a hosted MSVC runner; this machine's linker is whatever
        # happens to be installed, and no toolchain here is a declared BatPak
        # prerequisite. Report exactly one of executed-and-passed,
        # executed-and-failed, or unavailable. Never let unavailable wear the
        # green of a check that passed.
        # BOTH bootstrap binaries execute. materialize.rs was the same species as
        # seedcheck: it existed, it type-checked, and it had never once run.
        findings.extend(qualify_binary(rustc, root, tmp, "tier0-seedcheck",
                                       "bootstrap/seedcheck.rs", "seedcheck: PASS"))
        # tier0-materialize rides its own isolated qualification (5.5E5): two
        # temporary output roots outside the seed, a full seed snapshot, an
        # independent byte oracle, an Unchanged rerun, and Cargo on a
        # disposable copy of the candidate. The generic one-root route is the
        # exact defect that scaffolded the inspector's kitchen.
        findings.extend(qualify_materializer(rustc, root, tmp))
        # The #[cfg(test)] refusal tests inside seedcheck are claims too, and a
        # plain compile STRIPS them: until D4d they had never executed or even
        # type-checked -- the same species as materialize, wearing test syntax.
        # The expected count derives from the authored source at run time, so a
        # deleted test shrinks the expectation and an emptied harness is a
        # finding, never a quiet "ok. 0 passed".
        n_tests = (root / "bootstrap/seedcheck.rs").read_text(
            encoding="utf-8").count("#[test]")
        pkg_sc = root / "bootstrap" / "seedcheck"
        if pkg_sc.is_dir():
            n_tests += sum(p.read_text(encoding="utf-8").count("#[test]")
                           for p in sorted(pkg_sc.glob("*.rs")))
        if n_tests == 0:
            findings.append(
                "seedcheck declares no #[test] refusal tests; the harness was emptied")
        findings.extend(qualify_binary(
            rustc, root, tmp, "tier0-seedcheck-tests", "bootstrap/seedcheck.rs",
            f"test result: ok. {n_tests} passed", extra=("--test",), run_args=()))
        # The spec's OWN tests (5.5E2): the ISA admission refusals moved into
        # spec/pakvm_isa/ where the seam they exercise lives — a downstream
        # crate's test cfg no longer reaches across the boundary, which is the
        # boundary working. Qualified with the same artifact binding.
        n_spec_tests = sum(p.read_text(encoding="utf-8").count("#[test]")
                           for p in sorted((root / "spec").rglob("*.rs")))
        if n_spec_tests == 0:
            findings.append(
                "the spec declares no #[test] refusal tests; the harness was emptied")
        findings.extend(qualify_binary(
            rustc, root, tmp, "tier0-spec-tests", "spec/lib.rs",
            f"test result: ok. {n_spec_tests} passed", extra=("--test",),
            run_args=(), link_spec=False))
        # The admission desks have no public bypass and the sealed witnesses
        # cannot be forged or fabricated — proven by COMPILING a hostile
        # consumer against the REAL crate boundary (--extern spec), the exact
        # visibility configuration production uses since the 5.5E2 bake. Each
        # probe must fail for ITS reason: a probe red for an unrelated compile
        # error would certify a boundary nobody tested.
        for name, uses, body, want in (
            ("isa_seam_is_absent_from_production",
             "use spec::pakvm_isa::*;",
             "let ap = algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();\n"
             "    let cp = class_policy(PakVmNodeClass::RowTransform).unwrap();\n"
             "    let _ = admit_candidate_policy(PakVmNodeId::Filter, ap, cp);",
             "cannot find function `admit_candidate_policy`"),
            ("isa_admission_witness_cannot_be_forged",
             "use spec::pakvm_isa::*;",
             "let _ = AdmittedAsPublicSemanticNode(());",
             "cannot initialize a tuple struct which contains private fields"),
            ("firewall_admitted_crossing_cannot_be_forged",
             "use spec::architecture::SyncBatPlane;\n"
             "use spec::pakvm_isa::PakVmNodeId;\nuse spec::syncbat_firewall::*;",
             "let _ = AdmittedCrossing {\n"
             "        from: SyncBatPlane::Bvisor, to: SyncBatPlane::Runtime,\n"
             "        carries: SyncBatAuthority::RetryRestartAuthority,\n"
             "        posture: CrossingPosture::Required,\n"
             '        origin: None, law: "minted by hand", seal: (),\n'
             "    };",
             "field `seal` of struct"),
            ("firewall_origin_cannot_be_fabricated",
             "use spec::architecture::SyncBatPlane;\n"
             "use spec::pakvm_isa::PakVmNodeId;\nuse spec::syncbat_firewall::*;",
             "let _ = admit_crossing(SyncBatPlane::PakVm, SyncBatPlane::Bvisor,\n"
             "        SyncBatAuthority::TypedEffectRequest, Some(PakVmNodeId::ReachTheHost));",
             "named `ReachTheHost` found for enum `PakVmNodeId`"),
            # 5.5E2: per-family guarantee identity is sealed at the crate
            # boundary. A consumer can RECEIVE a GuaranteeRef and read it, but
            # cannot mint one claiming a family it did not come from.
            ("guarantee_identity_cannot_be_minted",
             "use spec::guarantees::{GuaranteeRef, SeedId};",
             'let _ = GuaranteeRef::Seed(SeedId("SEED-FORGED"));',
             "cannot initialize a tuple struct which contains private fields"),
            # 5.5E3b: the package identity is the variant; strings cannot
            # mint the guarantee identities and typed relationships cannot
            # be read as raw names.
            ("architecture_guarantee_id_cannot_be_minted_from_a_string",
             "use spec::guarantees::ArchitectureGuaranteeId;",
             'let _ = ArchitectureGuaranteeId("batpak");',
             "cannot initialize a tuple struct which contains private fields"),
            ("qualification_id_cannot_carry_a_string_package",
             "use spec::guarantees::QualificationId;",
             'let _ = QualificationId { package: "batpak", profile: "semantic" };',
             "expected `PackageId`, found `&str`"),
            ("dependency_endpoint_is_package_typed",
             "use spec::architecture::EDGES;",
             "let _: &str = EDGES[0].importer;",
             "expected `&str`, found `PackageId`"),
            ("qualification_package_is_package_typed",
             "use spec::architecture::QUALIFICATION_PROFILES;",
             "let _: &str = QUALIFICATION_PROFILES[0].package;",
             "expected `&str`, found `PackageId`"),
            ("syncbat_plane_package_is_package_typed",
             "use spec::architecture::SyncBatPlane;",
             "let _: &str = SyncBatPlane::Runtime.package();",
             "expected `&str`, found `PackageId`"),
            # 5.5E3d: "not currently admitted" and "forbidden by decision"
            # are DIFFERENT laws, and no term is currently decision-forbidden,
            # so a forbidden disposition has no spelling at all. Mislabeling
            # the pending passport application as a dead passport is a
            # compile error, not a reclassification.
            ("future_required_identity_cannot_be_mislabeled_rejected",
             "use spec::identities::IdentityTermDisposition;\n"
             "use spec::guarantees::DecisionId;",
             'let _ = IdentityTermDisposition::RejectedBy(DecisionId("DEC-063"));',
             "no variant, associated function, or constant named `RejectedBy`"),
            # 5.5E3e: the three command namespaces do not substitute for one
            # another — the same surface token, different passports.
            ("product_inspect_cannot_substitute_for_testpak_inspect",
             "use spec::commands::{ProductCommand, TestPakCommand};",
             "let _: TestPakCommand = ProductCommand::Inspect;",
             "expected `TestPakCommand`, found `ProductCommand`"),
            ("batql_mode_cannot_substitute_for_product_command",
             "use spec::commands::{BatQlSourceMode, ProductCommand};",
             "let _: ProductCommand = BatQlSourceMode::Ask;",
             "expected `ProductCommand`, found `BatQlSourceMode`"),
            # 5.5E3f: the mutation vocabulary is typed — no raw string mints
            # a lane or result, and architecture.rs no longer re-exports the
            # identity it surrendered.
            ("raw_string_cannot_substitute_for_mutation_lane",
             "use spec::mutation::MutationLane;",
             'let _: MutationLane = "SemanticIr";',
             "expected `MutationLane`, found `&str`"),
            ("raw_string_cannot_substitute_for_mutation_result",
             "use spec::mutation::MutationResult;",
             'let _: MutationResult = "Killed";',
             "expected `MutationResult`, found `&str`"),
            ("architecture_module_does_not_reexport_mutation_identity",
             "use spec::architecture::MutationLane;",
             "let _ = ();",
             "no `MutationLane` in `architecture`"),
            # 5.5E3g: the corpus epoch is typed — no raw string and no
            # runtime reconciliation role substitutes for it.
            ("raw_string_cannot_substitute_for_reconciliation_epoch",
             "use spec::corpus::ReconciliationEpoch;",
             'let _: ReconciliationEpoch = "cleanroom-v1";',
             "expected `ReconciliationEpoch`, found `&str`"),
            ("reconciliation_role_cannot_substitute_for_reconciliation_epoch",
             "use spec::corpus::ReconciliationEpoch;\n"
             "use spec::reconciliation::ReconciliationRole;",
             "let _: ReconciliationEpoch = ReconciliationRole::DurableOrderWitness;",
             "expected `ReconciliationEpoch`, found `ReconciliationRole`"),
            # 5.5E3i: the promotion denominator is typed — no raw string, no
            # policy-surface substitution, and no alternate owner module.
            ("raw_string_cannot_substitute_for_promotion_requirement",
             "use spec::promotion::PromotionRequirement;",
             'let _: PromotionRequirement = "NamedProofTarget";',
             "expected `PromotionRequirement`, found `&str`"),
            ("architecture_module_does_not_reexport_promotion_requirement",
             "use spec::architecture::PromotionRequirement;",
             "let _ = ();",
             "no `PromotionRequirement` in `architecture`"),
            ("mutation_module_does_not_reexport_promotion_requirement",
             "use spec::mutation::PromotionRequirement;",
             "let _ = ();",
             "no `PromotionRequirement` in `mutation`"),
            ("proof_policy_surface_cannot_substitute_for_promotion_requirement",
             "use spec::proof::ProofPolicySurface;\n"
             "use spec::promotion::PromotionRequirement;",
             "let _: PromotionRequirement = ProofPolicySurface::CandidatePromotion;",
             "expected `PromotionRequirement`, found `ProofPolicySurface`"),
            # 5.5E3j: the operator identity and both source surfaces are
            # typed — no raw string mints them, the word and symbol
            # inventories never substitute for one another, and a spoken
            # narration string is not a source-syntax shape.
            ("raw_string_cannot_substitute_for_operator_id",
             "use spec::operators::OperatorId;",
             'let _: OperatorId = "OP-MUL";',
             "expected `OperatorId`, found `&str`"),
            ("raw_string_cannot_substitute_for_operator_word_surface",
             "use spec::operators::OperatorWordSurface;",
             'let _: OperatorWordSurface = "IS";',
             "expected `OperatorWordSurface`, found `&str`"),
            ("raw_string_cannot_substitute_for_operator_symbol_surface",
             "use spec::operators::OperatorSymbolSurface;",
             'let _: OperatorSymbolSurface = "=";',
             "expected `OperatorSymbolSurface`, found `&str`"),
            ("operator_word_surface_cannot_substitute_for_operator_symbol_surface",
             "use spec::operators::{OperatorSymbolSurface, OperatorWordSurface};",
             "let _: OperatorSymbolSurface = OperatorWordSurface::Is;",
             "expected `OperatorSymbolSurface`, found `OperatorWordSurface`"),
            ("spoken_string_cannot_substitute_for_operator_syntax",
             "use spec::operators::OperatorSyntax;",
             'let _: OperatorSyntax = "times";',
             "expected `OperatorSyntax`, found `&str`"),
            # 5.5E4a: the generated-view registry is typed — no raw string
            # or surface value mints a view, no other module re-exports the
            # identity, and no universal GeneratedViewId exists.
            ("raw_string_cannot_substitute_for_generated_view",
             "use spec::generated_views::GeneratedView;",
             'let _: GeneratedView = "OperatorsCatalog";',
             "expected `GeneratedView`, found `&str`"),
            ("generated_view_surface_cannot_substitute_for_generated_view",
             "use spec::generated_views::{GeneratedView, GeneratedViewSurface};",
             "let _: GeneratedView = GeneratedViewSurface::EmbeddedBlock;",
             "expected `GeneratedView`, found `GeneratedViewSurface`"),
            ("architecture_module_does_not_reexport_generated_view",
             "use spec::architecture::GeneratedView;",
             "let _ = ();",
             "no `GeneratedView` in `architecture`"),
            ("universal_generated_view_id_is_absent",
             "use spec::generated_views::GeneratedViewId;",
             "let _ = ();",
             "no `GeneratedViewId` in `generated_views`"),
        ):
            src = tmp / f"{name}.rs"
            src.write_text(
                uses + f"\npub fn probe() {{\n    {body}\n}}\n", encoding="utf-8")
            proc = subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib", "--emit=metadata",
                 "--extern", f"spec={spec_meta}",
                 "--crate-name", name, "-o", str(tmp / (name + ".rmeta")), str(src)],
                capture_output=True, text=True)
            if proc.returncode == 0:
                findings.append(
                    f"{name}: production code reached a sealed, test-only, or "
                    f"fabricated spec item across the crate boundary")
            elif want not in proc.stderr:
                head = "\n".join(proc.stderr.splitlines()[:4])
                findings.append(
                    f"{name}: compile failed, but not with {want!r}:\n{head}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_toolchain(audit) -> list[str]:
    """Named hostile fixtures for the typed toolchain owner (5.5E3a).

    spec/toolchain/types.rs owns every value; the tracked root projection, the
    materializer, and the harness derive. Each fixture plants a hidden owner
    or a substituted dimension and must be refused for ITS law."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, rel, old, new, needle, subdirs=("spec", "docs", "companion", "bootstrap")):
        with isolated_tree(subdirs=subdirs) as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            got = audit.toolchain_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")

    if audit.toolchain_findings(root):
        fail("toolchain_authority_passes_on_the_real_seed")

    # Hidden owners: a literal in the materializer or the harness is a second
    # authority the moment it exists.
    probe("materialized_resolver_mismatch_is_rejected", "bootstrap/materialize/render.rs",
          'resolver = \\"{}\\"',
          'resolver = \\"2\\"',
          "hardcodes a resolver literal")
    probe("workspace_msrv_mismatch_is_rejected", "bootstrap/materialize/render.rs",
          'rust-version = \\"{}\\"',
          'rust-version = \\"1.96\\"',
          "hardcodes a rust-version literal")
    # The planted literal is assembled at run time: written contiguously it
    # would sit in THIS file's source and the hidden-owner scan would fire on
    # its own fixture — the phantom ban catching its author again.
    probe("rustc_edition_mismatch_is_rejected", "bootstrap/selftest/tier0.py",
          '"--edition", TOOLCHAIN_EDITION,',
          '"--edition", ' + '"2021",',
          "hardcodes a rustc edition")
    # exact >= floor, never "same minor" (5.5E3a1). A NEWER qualifying
    # compiler that preserves the MSRV floor is lawful — the tracked
    # projection moves with it and NO finding fires; one BELOW the floor is
    # the contradiction. The tracked file is mutated in lockstep so only the
    # law under test can fire.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain/types.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "RustRelease { major: 1, minor: 97, patch: 0 }",
            "RustRelease { major: 1, minor: 98, patch: 0 }",
            "newer_qualifying_compiler"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'channel = "1.97.0"', 'channel = "1.98.0"',
                      "newer channel"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if got:
            fail(f"newer_qualifying_compiler_may_preserve_msrv (got {got!r})")
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain/types.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "RustRelease { major: 1, minor: 97, patch: 0 }",
            "RustRelease { major: 1, minor: 96, patch: 0 }",
            "older_qualifying_compiler"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'channel = "1.97.0"', 'channel = "1.96.0"',
                      "older channel"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if not any("below the declared MSRV floor" in f for f in got):
            fail(f"qualifying_compiler_below_msrv_is_rejected (got {got!r})")
    # Dimension substitution, semantic side: renaming an environment spelling
    # into a triple is refused at the enum's own spelling arm.
    probe("semantic_target_shaped_like_a_triple_is_rejected", "spec/architecture/types.rs",
          'QualificationEnvironment::WasmHost => "wasm32 host",',
          'QualificationEnvironment::WasmHost => "wasm32-unknown-unknown",',
          "is shaped like a physical target triple")
    # RustupComponent::ALL is the one component denominator: dropping a
    # declared component from it (with the projection moving in lockstep)
    # must be refused, or Rustfmt becomes a declared-but-unconsumed variant.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain/types.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "&[RustupComponent::Clippy, RustupComponent::Rustfmt]",
            "&[RustupComponent::Clippy]",
            "component inventory"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'components = ["clippy", "rustfmt"]',
                      'components = ["clippy"]',
                      "component projection"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if not any("omitted from RustupComponent::ALL" in f for f in got):
            fail("declared_toolchain_component_cannot_be_omitted_from_"
                 f"required_inventory (got {got!r})")

    # Hand-editing or losing the tracked projection is refused.
    probe("root_toolchain_channel_mismatch_is_rejected", "rust-toolchain.toml",
          'channel = "1.97.0"', 'channel = "1.96.0"',
          "does not equal the deterministic projection")
    probe("root_toolchain_component_omission_is_rejected", "rust-toolchain.toml",
          'components = ["clippy", "rustfmt"]', 'components = ["clippy"]',
          "does not equal the deterministic projection")
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        (tmp / "rust-toolchain.toml").unlink()
        got = audit.toolchain_findings(tmp)
        if not any("absent" in f for f in got):
            fail(f"tracked_toolchain_projection_cannot_be_missing (got {got!r})")
    return findings


def test_package_identity(audit) -> list[str]:
    """Named hostile fixtures for the typed package identity (5.5E3b). The
    variant is the identity; cargo names and workspace paths are projections;
    every catalog defect must be refused for ITS law."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, old, new, needle):
        with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
            path = tmp / spec_carrier(root, "architecture", old)
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            got: list[str] = []
            audit.check_architecture(tmp, got)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")

    clean: list[str] = []
    audit.check_architecture(root, clean)
    if clean:
        fail(f"package_catalog_passes_on_the_real_seed (got {clean!r})")

    probe("package_variant_missing_from_all_is_rejected",
          "        PackageId::NetBat,\n        PackageId::TestPak,",
          "        PackageId::TestPak,",
          "PackageId::NetBat is omitted from PackageId::ALL")
    probe("package_id_missing_from_package_specs_is_rejected",
          "    PackageSpec {\n        id: PackageId::NetBat,",
          "    PackageSpec {\n        id: PackageId::TestPak,",
          "PACKAGES does not declare exactly PackageId::ALL in canonical order")
    probe("package_spec_duplicate_id_is_rejected",
          "    PackageSpec {\n        id: PackageId::MacBat,",
          "    PackageSpec {\n        id: PackageId::MacBatCompiler,",
          "PACKAGES does not declare exactly PackageId::ALL in canonical order")
    probe("two_package_ids_cannot_project_the_same_cargo_name",
          'PackageId::BatQl => "batql",',
          'PackageId::BatQl => "batpak",',
          "two package identities project the same cargo name")
    probe("two_package_ids_cannot_project_the_same_workspace_path",
          'PackageId::BatQl => "crates/batql",',
          'PackageId::BatQl => "crates/batpak",',
          "two package identities project the same workspace path")

    # The materializer may not select behavior by raw package name.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        path = tmp / "bootstrap/materialize/render.rs"
        path.write_text(must_replace(
            path.read_text(encoding="utf-8"),
            "package.id == architecture::PackageId::BatPak",
            'package.id.cargo_name() == "batpak"',
            "raw-name selection"), encoding="utf-8")
        got: list[str] = []
        audit.check_architecture(tmp, got)
        if not any("selects behavior by raw package name" in f for f in got):
            fail(f"materializer_cannot_select_package_behavior_by_raw_name (got {got!r})")
    return findings


def test_contract_kinds(audit) -> list[str]:
    """Named hostile fixtures for the admitted contract kinds (5.5E3c). The
    enum is the admitted border crossing: a kind without an admitting law is
    a brochure entry, and the doc fence and the enum may not disagree."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.contract_kind_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.contract_kind_findings(root):
        fail("contract_kinds_pass_on_the_real_seed")

    CK = "spec/contracts.rs"
    # A kind with no admitting law is speculation wearing a variant: adding
    # StateMachine to the enum, the inventory, and the spelling map without a
    # citation is refused for exactly that absence.
    probe("speculative_contract_kind_is_rejected",
          [(CK, "    Composition,\n}", "    Composition,\n    StateMachine,\n}"),
           (CK, "        ContractKind::Composition,\n    ];",
                "        ContractKind::Composition,\n        ContractKind::StateMachine,\n    ];"),
           (CK, 'ContractKind::Composition => "Composition",',
                'ContractKind::Composition => "Composition",\n            '
                'ContractKind::StateMachine => "StateMachine",')],
          "ContractKind::StateMachine cites no admission basis")
    probe("contract_kind_missing_from_all_is_rejected",
          [(CK, "        ContractKind::Composition,\n    ];", "    ];")],
          "ContractKind::Composition is omitted from ContractKind::ALL")
    probe("docs_contract_kind_missing_row_is_rejected",
          [("docs/06_MACBAT.md", "| Composition | LEG-041 |\n", "")],
          "docs/06 omits admitted contract kind row Composition LEG-041")
    probe("docs_contract_kind_extra_row_is_rejected",
          [("docs/06_MACBAT.md", "| Composition | LEG-041 |",
            "| Composition | LEG-041 |\n| StateMachine | LEG-999 |")],
          "docs/06 admits StateMachine, which ContractKind does not declare")
    # The BASIS is semantic identity, not decoration: a wrong law, swapped
    # laws, and reordered rows are each refused positionally.
    probe("docs_contract_kind_wrong_basis_is_rejected",
          [("docs/06_MACBAT.md", "| Error | LEG-047 |", "| Error | LEG-999 |")],
          "docs/06 CONTRACT-KINDS row 1 states Error LEG-999; the typed "
          "catalog states Error LEG-047 at that position")
    probe("docs_contract_kind_swapped_bases_are_rejected",
          [("docs/06_MACBAT.md", "| Error | LEG-047 |", "| Error | LEG-002 |"),
           ("docs/06_MACBAT.md", "| Event | LEG-002 |", "| Event | LEG-047 |")],
          "docs/06 CONTRACT-KINDS row 1 states Error LEG-002; the typed "
          "catalog states Error LEG-047 at that position")
    probe("docs_contract_kind_order_drift_is_rejected",
          [("docs/06_MACBAT.md",
            "| Error | LEG-047 |\n| Event | LEG-002 |",
            "| Event | LEG-002 |\n| Error | LEG-047 |")],
          "docs/06 CONTRACT-KINDS row 1 states Event LEG-002; the typed "
          "catalog states Error LEG-047 at that position")
    probe("mistagged_admitting_guarantee_is_rejected",
          [(CK, 'ContractKind::Error => GuaranteeRef::leg("LEG-047"),',
                'ContractKind::Error => GuaranteeRef::leg("DEC-047"),')],
          "whose family owns the LEG- prefix")

    # Lifecycle law (5.5E3c1): a DELETED basis refuses the kind; a lawfully
    # CLOSED basis is retained evidence and the kind remains admitted —
    # closing the founding obligation must not erase the vocabulary it
    # earned. Process citing superseded DEC-009 on the real seed is the
    # standing green witness for the historical tier.
    probe("deleted_admission_basis_is_rejected",
          [("spec/legacy_obligations.rs",
            'LegacyObligation { id: "LEG-047",',
            'LegacyObligation { id: "LEG-047-GONE",')],
          "ContractKind::Error cites admission basis LEG-047, which no "
          "declared row owns")
    tmp = gate_sandbox([("spec/legacy_obligations.rs",
        'Planned("variant addition compile failure/mutant"), '
        'gates: &[GateId::G1, GateId::G2], compatibility_disposition: '
        'CompatibilityDisposition::None, deletion_condition: '
        'DeletionCondition::OnSuccessorGateClosure, active_or_closed_status: '
        'ObligationStatus::Active }',
        'Planned("variant addition compile failure/mutant"), '
        'gates: &[GateId::G1, GateId::G2], compatibility_disposition: '
        'CompatibilityDisposition::None, deletion_condition: '
        'DeletionCondition::OnSuccessorGateClosure, active_or_closed_status: '
        'ObligationStatus::Closed }')])
    try:
        got = audit.contract_kind_findings(tmp)
        if got:
            fail("closed_admission_basis_remains_valid_retained_evidence "
                 f"(got {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings
