"""Admitted-vocabulary catalog audits (5.5E3c-i).

Owns the contract-kind, identity, promotion-requirement, compiler-assumption,
corpus-reconciliation-epoch, mutation, and command-namespace catalogs. Each
reparses its typed owner, reconstructs the generated docs projections, and
proves owner/admission-basis coherence. Shares no parser with
bootstrap/project.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .corpus import (
    A_GENERATED_BLOCK,
    EXCLUDE_DIRS,
    G_DEC_ROW,
    G_LEG_ROW,
    _spec_module_source,
    _uncomment,
    batql_extract_block,
    declared_contract_ids,
    frontmatter,
)

# --- Admitted contract kinds (5.5E3c, block form 5.5E4a) ---------------------
# spec/contracts.rs owns the admitted set; docs/06 projects it through the
# ordinary CONTRACT-KINDS generated block. A kind whose admitting citation
# dangles, and a block that admits more or fewer kinds than the enum, are both
# refused — and the retired HYBRID fence (a generator editing selected columns
# inside an authored fence) may not return.
A_CONTRACT_KIND_HYBRID_FENCE = re.compile(
    r"The admitted kinds, each with its admitting law:\s*\n+```text\n(.*?)\n```", re.S)


def contract_kind_findings(root: Path) -> list[str]:
    out: list[str] = []
    src_path = root / "spec/contracts.rs"
    if not src_path.is_file():
        return ["missing spec/contracts.rs"]
    src = _uncomment(src_path.read_text(encoding="utf-8"))
    enum_body = re.search(r"pub enum ContractKind \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(r"pub const ALL: &'static \[ContractKind\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"ContractKind::(\w+)", all_body.group(1)) if all_body else []
    if not variants:
        out.append("spec/contracts.rs declares no ContractKind variants")
    for missing in sorted(set(variants) - set(inventory)):
        out.append(f"ContractKind::{missing} is omitted from ContractKind::ALL")
    for phantom in sorted(set(inventory) - set(variants)):
        out.append(f"ContractKind::ALL names {phantom}, which the enum does not declare")
    # tag/prefix agreement and resolution against the declared tables
    tagged = re.findall(r'ContractKind::(\w+) => GuaranteeRef::(leg|dec)\("([^"]+)"\)', src)
    leg_ids = {m[0] for m in G_LEG_ROW.findall(
        (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8"))}
    dec_ids = {m[0] for m in G_DEC_ROW.findall(
        (root / "spec/dispositions.rs").read_text(encoding="utf-8"))}
    for kind, tag, ident in tagged:
        if tag == "leg" and not ident.startswith("LEG-"):
            out.append(f"ContractKind::{kind} tags {ident!r} as leg, whose family "
                       "owns the LEG- prefix")
        elif tag == "dec" and not ident.startswith("DEC-"):
            out.append(f"ContractKind::{kind} tags {ident!r} as dec, whose family "
                       "owns the DEC- prefix")
        elif ident not in (leg_ids | dec_ids):
            out.append(f"ContractKind::{kind} cites admission basis {ident}, "
                       "which no declared row owns")
    for kind in set(variants) - {k for k, _, _ in tagged}:
        out.append(f"ContractKind::{kind} cites no admission basis")
    # docs/06 projects EXACTLY the ordered (kind, admission-basis) pairs, in
    # ContractKind::ALL order, through the ordinary CONTRACT-KINDS generated
    # block. Comparing sets of kind names alone would let the doc confidently
    # explain the wrong provenance.
    spelling_of = dict(re.findall(r'ContractKind::(\w+) => "(\w+)",', src))
    basis_of = {k: ident for k, _tag, ident in tagged}
    expected = [(spelling_of.get(v, ""), basis_of.get(v, "")) for v in inventory]
    doc = (root / "docs/06_MACBAT.md").read_text(encoding="utf-8")
    if A_CONTRACT_KIND_HYBRID_FENCE.search(doc):
        out.append("docs/06 hybrid admitted-kinds fence may not return; "
                   "CONTRACT-KINDS is an ordinary generated block and no "
                   "generator edits selected columns inside an authored fence")
    body = batql_extract_block(doc, "CONTRACT-KINDS")
    if body is None:
        out.append("docs/06 carries no generated CONTRACT-KINDS block")
        return out
    listed = [tuple(cell.strip() for cell in line.strip().strip("|").split("|"))
              for line in body.splitlines()[2:] if line.strip()]
    for i, want in enumerate(expected):
        if i >= len(listed):
            out.append(f"docs/06 omits admitted contract kind row {want[0]} {want[1]}")
        elif listed[i] != want:
            out.append(
                f"docs/06 CONTRACT-KINDS row {i + 1} states {' '.join(listed[i])}; "
                f"the typed catalog states {want[0]} {want[1]} at that position")
    for extra in listed[len(expected):]:
        out.append(f"docs/06 admits {extra[0]}, which ContractKind does not declare")
    return out


# --- Identity catalogs (5.5E3d) ----------------------------------------------
# spec/identities.rs owns four catalog axes plus the non-cataloged residue.
# This auditor independently parses the typed owner, reconstructs the five
# generated docs/16 blocks (ordered entries AND owners), and runs the standing
# corpus-sweep law: every identity-shaped term discovered in the AUTHORED
# corpus resolves through exactly one of five classification paths. Discovery
# deliberately excludes generated projection blocks and spec/identities.rs
# itself — the denominator must never prove itself by rereading its own
# answer sheet.
A_IDENTITY_AXES = (
    ("IDENTITY-CATALOG", "IdentityKind"),
    ("GENERATION-CATALOG", "GenerationKind"),
    ("BINDING-CATALOG", "BindingKind"),
    ("VERSION-CATALOG", "VersionIdentityKind"),
)
A_IDENT_DISPOSITION = {
    "OwnedElsewhere": "owned elsewhere",
    "NotYetAdmittedBy": "not yet admitted by",
}
# Exact case-sensitive identifier tokens ending in one of the six admitted
# suffixes, with token boundaries: "Valid", "provide", and prose containing
# lowercase "id" mint no surprise passports.
A_IDENT_TERM = re.compile(
    r"\b[A-Z][A-Za-z0-9]*(?:Id|Hash|Digest|Commitment|Generation|Version)\b")


def _identity_block_body(name: str, doc: str):
    # The BEGIN marker is provenance, verified independently of the renderer
    # (5.5E3d1): it must name spec/identities.rs as the source authority
    # exactly, or the block is not a generated identity block at all.
    m = re.search(
        r"<!-- " + re.escape(name)
        + r":BEGIN generated from spec/identities\.rs by bootstrap/project\.py; "
        + r"do not edit -->\n```text\n(.*?)\n```\n<!-- "
        + re.escape(name) + r":END -->", doc, re.S)
    return m.group(1) if m else None


def identity_catalog_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/identities.rs"
    if not path.is_file():
        return ["missing spec/identities.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    # Owner coherence (5.5E3d2): the owner must AUTHOR the term. Map each
    # declared contract to its authoritative document's authored text —
    # generated blocks removed, so a projection can never notarize its own
    # owner claim. An occurrence in some other live contract's document does
    # not satisfy the claim: a live building cannot sign a passport merely
    # because the applicant exists somewhere else in town.
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)

    def owner_authors(cid: str, term: str) -> bool:
        return bool(re.search(r"\b" + re.escape(term) + r"\b", owner_text.get(cid, "")))

    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_disposition = {m[0]: m[3] for m in G_DEC_ROW.findall(dsrc)}
    # Spellings with existing typed spec owners are referenced, never
    # re-admitted: a duplicate variant is a wrapper passport.
    owned_body = re.search(
        r"pub const EXISTING_TYPED_OWNER_SPELLINGS: &\[&str\] = &\[(.*?)\];", src, re.S)
    existing_owned = set(re.findall(r'"(\w+)"', owned_body.group(1))) if owned_body else set()
    if not existing_owned:
        out.append("spec/identities.rs declares no EXISTING_TYPED_OWNER_SPELLINGS list")
    # Independent parse of the four axes, in Kind::ALL order. The \b anchor
    # keeps "VersionIdentityKind::..." from satisfying "IdentityKind::...".
    catalogs: dict[str, list[tuple[str, str]]] = {}
    axis_of: dict[str, str] = {}
    for marker, kind in A_IDENTITY_AXES:
        all_body = re.search(
            r"pub const ALL: &'static \[" + kind + r"\] = &\[(.*?)\];", src, re.S)
        inventory = re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        arms = {v: (s, o) for v, s, o in re.findall(
            r"\b" + kind + r"::(\w+) => \{?\s*entry!\(\s*\"([^\"]+)\",\s*\"([^\"]+)\"\s*\)",
            src)}
        if not inventory:
            out.append(f"spec/identities.rs declares no {kind} inventory")
            continue
        entries: list[tuple[str, str]] = []
        for v in inventory:
            if v not in arms:
                out.append(f"{kind}::{v} is missing from ALL or declares no catalog entry")
                continue
            spelling, owner = arms[v]
            entries.append((spelling, owner))
            if spelling in axis_of:
                out.append(
                    f"identity spelling {spelling} answers two questions: it appears "
                    f"in both {axis_of[spelling]} and {kind}")
            else:
                axis_of[spelling] = kind
            if owner not in contract_ids:
                out.append(f"{kind} entry {spelling} names owner {owner}, "
                           "which no declared contract owns")
            elif not owner_authors(owner, spelling):
                out.append(f"{kind} entry {spelling} names owner {owner}, "
                           "whose authoritative document does not author the term")
            if spelling in existing_owned:
                out.append(f"{kind} entry {spelling} duplicates a spelling that "
                           "already has a typed spec owner; the catalog may "
                           "reference it but never re-admit it")
        for phantom in sorted(set(arms) - set(inventory)):
            out.append(f"{kind}::{phantom} declares an entry but is omitted from {kind}::ALL")
        catalogs[marker] = entries
    # Chronology/order/topology/coordinate/cursor/navigation vocabulary is
    # excluded by executed law, not convention.
    excluded_body = re.search(
        r"pub const EXCLUDED_CHRONOLOGY_AND_NAVIGATION: &\[&str\] = &\[(.*?)\];",
        src, re.S)
    excluded = set(re.findall(r'"(\w+)"', excluded_body.group(1))) if excluded_body else set()
    if not excluded:
        out.append("spec/identities.rs declares no EXCLUDED_CHRONOLOGY_AND_NAVIGATION list")
    for term in sorted(excluded & set(axis_of)):
        out.append(f"{term} is chronology/navigation vocabulary and may not enter "
                   f"the {axis_of[term]} catalog")
    # The residue table: the denominator's non-admitted half.
    residue_body = re.search(
        r"pub const NON_CATALOGED_IDENTITY_TERMS: &\[NonCatalogedIdentityTerm\] = &\[(.*?)\n\];",
        src, re.S)
    residue = re.findall(
        r'term: "(\w+)",\s*disposition: IdentityTermDisposition::(\w+)\('
        r'(?:ContractId|DecisionId)\("([^"]+)"\)\)',
        residue_body.group(1)) if residue_body else []
    if not residue:
        out.append("spec/identities.rs declares no NON_CATALOGED_IDENTITY_TERMS rows")
    residue_seen: set[str] = set()
    for term, variant, ref in residue:
        if term in residue_seen:
            out.append(f"residue term {term} is declared twice; one term, one path "
                       "also means one row on that path")
        residue_seen.add(term)
        if term in axis_of:
            out.append(f"{term} resolves through two paths: the residue table and "
                       f"the {axis_of[term]} catalog")
        if variant == "OwnedElsewhere":
            if ref not in contract_ids:
                out.append(f"residue term {term} is owned elsewhere by {ref}, "
                           "which no declared contract owns")
            elif not owner_authors(ref, term):
                out.append(f"residue term {term} is owned elsewhere by {ref}, "
                           "which does not author the term")
        elif variant == "NotYetAdmittedBy":
            disp = dec_disposition.get(ref)
            if disp is None:
                out.append(f"residue term {term} is not yet admitted by {ref}, "
                           "which no declared decision owns")
            elif disp in ("Supersede", "RetainAsEvidence"):
                # Mirrors Disposition::guarantee_lifetime: these two derive
                # HistoricalCoverageOnly, and a decision retained only as
                # historical coverage has no forward policy authority to own
                # a future admission barrier.
                out.append(f"residue term {term} is not yet admitted by {ref}, "
                           "which is retained only as historical coverage and "
                           "cannot own a future admission barrier")
            elif disp not in ("Lock", "Defer"):
                # Mirrors Disposition::may_own_not_yet_admitted_identity:
                # Permanent is not enough — Kill is a permanent PROHIBITION
                # (a dead passport), Keep is admitted retained policy, and
                # neither expresses "excluded now, with a standing future
                # entry path". Only Lock and Defer do.
                out.append(f"residue term {term} is not yet admitted by {ref}, "
                           f"whose {disp} disposition is not a standing "
                           "future-entry policy; only Lock and Defer own "
                           "pending applications")
            else:
                # Decision coherence (5.5E3d2): the cited decision must NAME
                # the pending term in its forward-policy fields — subject,
                # successor, or replacement contract. stale_aliases retire
                # vocabulary and carry no future admission authority.
                row = re.search(
                    r'DecisionSpec \{ id: "' + re.escape(ref)
                    + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                    + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
                fields = " ".join(g or "" for g in row.groups()) if row else ""
                if not re.search(r"\b" + re.escape(term) + r"\b", fields):
                    out.append(f"residue term {term} is not yet admitted by {ref}, "
                               "whose forward-policy fields do not name the term")
        else:
            out.append(f"residue term {term} carries unknown disposition {variant}")
    # docs/16 carries five generated blocks projecting ordered entries AND
    # owners; this auditor reconstructs each body independently.
    doc = (root / "docs/16_IDENTITY_TIME_AND_NAVIGATION.md").read_text(encoding="utf-8")
    for marker, _kind in A_IDENTITY_AXES:
        want = [f"{s:<30} {o}" for s, o in catalogs.get(marker, [])]
        _identity_block_parity(marker, want, doc, out)
    residue_want = [f"{t:<30} {A_IDENT_DISPOSITION.get(v, '?'):<22} {r}"
                    for t, v, r in residue]
    _identity_block_parity("IDENTITY-RESIDUE", residue_want, doc, out)
    # The standing corpus-sweep law, TRUE set equality (5.5E3d1). Authored
    # corpus: the authoritative docs and companion, minus every generated
    # projection block. Three refusal shapes: discovered but unclassified;
    # classified residue with no authored occurrence; classified catalog
    # entry with no authored adopter. The adopter direction uses exact
    # token presence rather than the sweep grammar because a spelling like
    # bare "Commitment" is lawful catalog vocabulary the prefix+suffix
    # grammar deliberately does not mint from prose.
    authored: list[str] = []
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        text = rel.read_text(encoding="utf-8")
        if frontmatter(text).get("status") == "GENERATED":
            continue
        authored.append(A_GENERATED_BLOCK.sub("", text))
    blob = "\n".join(authored)
    discovered = set(A_IDENT_TERM.findall(blob))
    classified = set(axis_of) | {t for t, _v, _r in residue}
    for term in sorted(discovered - classified):
        out.append(f"identity-shaped term {term} appears in the authoritative corpus "
                   "and resolves through none of the five classification paths")
    for term in sorted(set(axis_of)):
        if not re.search(r"\b" + re.escape(term) + r"\b", blob):
            out.append(f"catalog entry {term} appears in no authored corpus document; "
                       "an entry without an authored adopter is a brochure passport")
    for term in sorted({t for t, _v, _r in residue}):
        if not re.search(r"\b" + re.escape(term) + r"\b", blob):
            out.append(f"residue term {term} no longer appears in the authored corpus; "
                       "a disposition for a term nobody authors is an expired application")
    # Self-neutering guards (5.5E3d1): the exclusion list must equal the
    # vocabulary the authored docs/16 navigation and time-and-order fences
    # declare, and the wrapper guard must equal the identity types the spec
    # actually declares elsewhere. Deleting a guard row before smuggling the
    # term in is refused by parity, not by the guard it deleted.
    doc16_authored = A_GENERATED_BLOCK.sub(
        "", (root / "docs/16_IDENTITY_TIME_AND_NAVIGATION.md").read_text(encoding="utf-8"))
    required_excluded: set[str] = set()
    for heading in ("Navigation types", "Time and order"):
        fence = re.search(r"## " + heading + r"\n+```text\n(.*?)\n```", doc16_authored, re.S)
        if not fence:
            out.append(f"docs/16 carries no authored {heading} fence to derive "
                       "the chronology/navigation exclusion from")
            continue
        required_excluded.update(
            line.split()[0] for line in fence.group(1).splitlines() if line.strip())
    for term in sorted(required_excluded - excluded):
        out.append(f"EXCLUDED_CHRONOLOGY_AND_NAVIGATION omits {term}, which docs/16's "
                   "navigation and time-and-order fences declare")
    for term in sorted(excluded - required_excluded):
        out.append(f"EXCLUDED_CHRONOLOGY_AND_NAVIGATION lists {term}, which docs/16's "
                   "navigation and time-and-order fences do not declare")
    required_owned: set[str] = set()
    for rel in ("spec/architecture.rs", "spec/proof.rs", "spec/gates.rs",
                "spec/operators.rs"):
        text = (_spec_module_source(root, "architecture") if rel == "spec/architecture.rs"
                else (root / rel).read_text(encoding="utf-8"))
        required_owned.update(re.findall(
            r"pub (?:struct|enum) (\w+Id)\b",
            _uncomment(text)))
    for term in sorted(required_owned - existing_owned):
        out.append(f"EXISTING_TYPED_OWNER_SPELLINGS omits {term}, which the spec "
                   "declares as an existing typed owner")
    for term in sorted(existing_owned - required_owned):
        out.append(f"EXISTING_TYPED_OWNER_SPELLINGS lists {term}, which no spec "
                   "module declares as an existing typed owner")
    return out


def _identity_block_parity(marker: str, want: list[str], doc: str,
                           out: list[str]) -> None:
    body = _identity_block_body(marker, doc)
    if body is None:
        out.append(f"docs/16 carries no generated {marker} block naming "
                   "spec/identities.rs as source")
        return
    listed = [line.rstrip() for line in body.splitlines() if line.strip()]
    want = [line.rstrip() for line in want]
    for i, expect in enumerate(want):
        if i >= len(listed):
            out.append(f"docs/16 {marker} omits row {expect.split()[0]}")
        elif listed[i] != expect:
            out.append(f"docs/16 {marker} row {i + 1} states {' '.join(listed[i].split())}; "
                       f"the typed catalog states {' '.join(expect.split())} at that position")
    for extra in listed[len(want):]:
        out.append(f"docs/16 {marker} lists {extra.split()[0]}, "
                   "which the typed catalog does not project")


# --- Promotion requirements (5.5E3i) -----------------------------------------
# spec/promotion.rs owns the conjunctive denominator: four required members,
# no optional flag, no waiver variant, no second list anywhere. docs/12 owns
# what each requirement means; docs/24 consumes Rule qualification; docs/31
# consumes the typed law. The authored doctrine clauses are themselves law:
# each fragment below has a hostile proving its deletion reds.
A_PROMOTION_DOCTRINE = (
    ("a missing member refuses promotion",
     "the requirements are conjunctive"),
    ("are not independence",
     "the evidence route cannot be self-grading"),
    ("A generic issue link, chat",
     "a generic issue is not a named proof target"),
    ("chat transcript, commit message, or vague",
     "a commit message is not a named proof target"),
    ("a stable name, a ContractId owner, the affected",
     "a documented proof gap requires a stable owner"),
    ("Rule qualification law",
     "qualified hostile evidence consumes the docs/24 Rule qualification law"),
    ("EquivalentCandidate without its independent equivalence witness",
     "an equivalent candidate is not a killed mutant"),
    ("a failure produced by the unchanged baseline",
     "a baseline failure cannot kill a candidate"),
    ("A commit message\nis not this receipt.",
     "a commit message is not the promotion receipt"),
    ("resulting tracked-content commitment",
     "the receipt binds the resulting tracked-content commitment"),
)
A_PROMOTION_DISQUALIFIED = (
    "Survived", "NotActivated", "Refused", "Unbuildable", "TimedOut",
    "InfrastructureFailure",
)


def promotion_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/promotion.rs"
    if not path.is_file():
        return ["missing spec/promotion.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    enum_body = re.search(r"pub enum PromotionRequirement \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[PromotionRequirement\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bPromotionRequirement::(\w+)", all_body.group(1)) \
        if all_body else []
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"PromotionRequirement::{missing} is omitted from "
                   "PromotionRequirement::ALL; the denominator is conjunctive "
                   "and complete")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"PromotionRequirement::ALL names {phantom}, which the enum "
                   "does not declare")
    if re.search(r"\boptional\b|\bwaiver\b", src, re.I):
        out.append("spec/promotion.rs speaks of optional or waiver; the "
                   "denominator is conjunctive with no escape hatch")

    def fn_arms(name):
        body = re.search(
            r"pub const fn " + name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
            src, re.S)
        return dict(re.findall(r"PromotionRequirement::(\w+)\s*=>\s*([^,]+),",
                               body.group(1), re.S)) if body else {}

    spellings = fn_arms("spelling")
    owners = fn_arms("semantic_owner")
    bases = fn_arms("admission_basis")
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)
    seen: set[str] = set()
    rows: list[str] = []
    for v in inventory:
        s = spellings.get(v, "").strip().strip('"')
        if not s.strip():
            out.append(f"PromotionRequirement::{v} projects an empty spelling")
            continue
        if s in seen:
            out.append(f"promotion-requirement spelling {s} is claimed twice")
        seen.add(s)
        owner = re.search(r'ContractId\("([^"]+)"\)', owners.get(v, ""))
        oid = owner.group(1) if owner else ""
        if oid not in contract_ids:
            out.append(f"promotion requirement {s} cites owner {oid}, which no "
                       "declared contract owns")
        elif not re.search(r"\b" + re.escape(s) + r"\b", owner_text.get(oid, "")):
            out.append(f"promotion requirement {s} cites owner {oid}, whose "
                       "authoritative document does not author the spelling")
        basis = re.search(r'GuaranteeRef::dec\("([^"]+)"\)', bases.get(v, ""))
        bid = basis.group(1) if basis else ""
        if bid not in dec_ids:
            out.append(f"promotion requirement {s} cites admission basis {bid}, "
                       "which no declared decision owns")
        else:
            row = re.search(
                r'DecisionSpec \{ id: "' + re.escape(bid)
                + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
            fields = " ".join(g or "" for g in row.groups()) if row else ""
            if not re.search(r"\b" + re.escape(s) + r"\b", fields):
                out.append(f"promotion requirement {s} cites admission basis {bid}, "
                           "whose forward-policy fields do not name the requirement")
        rows.append(f"{s:<26} {oid:<14} {bid}")
    # The family facts answer four different questions.
    surface = re.search(
        r"PROMOTION_POLICY_SURFACE: ProofPolicySurface = ProofPolicySurface::(\w+);", src)
    if not surface or surface.group(1) != "CandidatePromotion":
        out.append("the promotion policy surface is not "
                   "ProofPolicySurface::CandidatePromotion")
    change = re.search(
        r'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec\("([^"]+)"\);', src)
    if not change or change.group(1) != "DEC-074":
        out.append("the promotion policy-change basis is not DEC-074; requirement "
                   "admission and change classification are different laws")
    egate = re.search(r"PROMOTION_ENFORCEMENT_GATE: GateId = GateId::(\w+);", src)
    if not egate or egate.group(1) != "G3":
        out.append("candidate-promotion policy is not enforced at G3")
    rgate = re.search(r"PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::(\w+);", src)
    if not rgate or rgate.group(1) != "G9":
        out.append("promotion policy changes are not release-visibly qualified at G9")
    # DEC-074 binds the typed change law without restating the inventory.
    row74 = re.search(
        r'DecisionSpec \{ id: "DEC-074",.*?successor: "([^"]*)"', dsrc)
    f74 = row74.group(1) if row74 else ""
    for token in ("PromotionRequirement", "ProofPolicySurface::CandidatePromotion",
                  "spec/promotion.rs"):
        if token not in f74:
            out.append(f"DEC-074 does not name {token}; the change law must bind "
                       "the typed boundary")
    # docs/12: authored doctrine clauses plus the generated projection.
    doc12 = (root / "docs/12_TESTPAK.md").read_text(encoding="utf-8")
    authored12 = A_GENERATED_BLOCK.sub("", doc12)
    flat12 = " ".join(authored12.split())
    for fragment, label in A_PROMOTION_DOCTRINE:
        if " ".join(fragment.split()) not in flat12:
            out.append(f"docs/12 promotion doctrine no longer states: {label}")
    disq = re.search(r"Not satisfied by:(.*?)(?:\n\n|`Auditable)", authored12, re.S)
    disq_text = disq.group(1) if disq else ""
    for token in A_PROMOTION_DISQUALIFIED:
        if not re.search(r"\b" + token + r"\b", disq_text):
            out.append(f"docs/12 no longer states that {token} cannot satisfy "
                       "QualifiedHostileEvidence")
    m = re.search(
        r"<!-- PROMOTION-REQUIREMENTS:BEGIN generated from spec/promotion\.rs by "
        r"bootstrap/project\.py; do not edit -->\n```text\n(.*?)\n```\n<!-- "
        r"PROMOTION-REQUIREMENTS:END -->", doc12, re.S)
    if not m:
        out.append("docs/12 carries no generated PROMOTION-REQUIREMENTS block "
                   "naming spec/promotion.rs as source")
    else:
        header = f"{'requirement':<26} {'owner':<14} admission basis"
        family = [f"{'policy surface':<26} {surface.group(1) if surface else '?'}",
                  f"{'policy-change basis':<26} {change.group(1) if change else '?'}",
                  f"{'enforcement gate':<26} {egate.group(1) if egate else '?'}",
                  f"{'release-visibility gate':<26} {rgate.group(1) if rgate else '?'}"]
        expected = [header] + rows + family
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        for i, want in enumerate(expected):
            if i >= len(listed):
                out.append(f"docs/12 PROMOTION-REQUIREMENTS omits row "
                           f"{want.split()[0]}")
            elif listed[i] != want.rstrip():
                out.append(f"docs/12 PROMOTION-REQUIREMENTS row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(want.split())} at that position")
        for extra in listed[len(expected):]:
            out.append(f"docs/12 PROMOTION-REQUIREMENTS lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    # docs/24 consumes without re-owning; the retired MutationLane owner path
    # cannot return.
    doc24 = A_GENERATED_BLOCK.sub(
        "", (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8"))
    if re.search(r"MutationLane`? in `?spec/architecture\.rs", doc24):
        out.append("docs/24 points MutationLane at spec/architecture.rs; the "
                   "typed owner moved to spec/mutation.rs and left no re-export")
    if "QualifiedHostileEvidence" not in doc24 or "Rule qualification" not in doc24:
        out.append("docs/24 no longer states that QualifiedHostileEvidence "
                   "consumes its Rule qualification law")
    if re.search(r"\bIndependentEvidenceRoute\b", doc24):
        out.append("docs/24 restates the promotion-requirement inventory; it "
                   "consumes Rule qualification without re-owning the denominator")
    # docs/31 consumes the typed law and owns no second list.
    doc31 = A_GENERATED_BLOCK.sub(
        "", (root / "docs/31_FINAL_CONTRADICTION_AUDIT.md").read_text(encoding="utf-8"))
    if "PromotionRequirement::ALL" not in doc31:
        out.append("docs/31 no longer consumes spec/promotion.rs "
                   "PromotionRequirement::ALL as the promotion condition")
    for s in ("IndependentEvidenceRoute", "NamedProofTarget",
              "QualifiedHostileEvidence", "AuditablePromotionReceipt"):
        if re.search(r"\b" + s + r"\b", doc31):
            out.append(f"docs/31 restates {s}; the audit consumer owns no second "
                       "promotion inventory")
    # The raw array stays dead, and the forbidden alternate owners neither
    # declare nor re-export the type. dispositions.rs lawfully NAMES it in
    # DEC-074's amended text; naming is not ownership.
    for rel in sorted((root / "spec").rglob("*.rs")):
        rsrc = _uncomment(rel.read_text(encoding="utf-8"))
        rel_spec = rel.relative_to(root / "spec").as_posix()
        # The owning top-level spec module: a concept-door "<name>.rs" or the
        # same-name decomposition directory "<name>/**" both belong to <name>.
        owner = rel_spec.split("/", 1)[0]
        owner = owner[:-3] if owner.endswith(".rs") else owner
        if owner in ("architecture", "mutation", "proof", "reconciliation") \
                and re.search(r"\bPromotionRequirement\b", rsrc):
            out.append(f"spec/{rel_spec} speaks PromotionRequirement; the type has "
                       "one owner module")
        if re.search(r"pub const PROMOTION_REQUIREMENTS\b", rsrc) \
                and owner != "promotion":
            out.append(f"spec/{rel_spec} resurrects the raw PROMOTION_REQUIREMENTS "
                       "array; the napkin is retired")
    return out


# --- Compiler-assumption kinds (5.5E3h) --------------------------------------
# spec/compiler_assumptions.rs owns the admitted ledgerable kinds — two
# sharp instruments, never a junk drawer. Hard violations, test allowances,
# dependency mechanisms, and not-yet-earned ideas keep their existing
# owners; the borders are proven by hostiles, not by a second census table.
def compiler_assumption_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/compiler_assumptions.rs"
    if not path.is_file():
        return ["missing spec/compiler_assumptions.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    enum_body = re.search(r"pub enum CompilerAssumptionKind \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[CompilerAssumptionKind\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bCompilerAssumptionKind::(\w+)", all_body.group(1)) \
        if all_body else []
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"CompilerAssumptionKind::{missing} is omitted from "
                   "CompilerAssumptionKind::ALL")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"CompilerAssumptionKind::ALL names {phantom}, which the enum "
                   "does not declare")

    def fn_arms(name):
        body = re.search(
            r"pub const fn " + name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
            src, re.S)
        arms: dict[str, str] = {}
        if not body:
            return arms
        for arm, value in re.findall(
                r"(CompilerAssumptionKind::\w+(?:\s*\|\s*CompilerAssumptionKind::\w+)*)"
                r"\s*=>\s*([^,]+),", body.group(1), re.S):
            for v in re.findall(r"\bCompilerAssumptionKind::(\w+)", arm):
                arms[v] = value.strip()
        return arms

    spellings = fn_arms("spelling")
    owners = fn_arms("semantic_owner")
    bases = fn_arms("admission_basis")
    markers = fn_arms("requires_safety_contract_marker")
    cgates = fn_arms("classification_gate")
    rgates = fn_arms("release_qualification_gate")
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)
    seen: set[str] = set()
    rows: list[str] = []
    for v in inventory:
        s = spellings.get(v, "").strip('"')
        if not s.strip():
            out.append(f"CompilerAssumptionKind::{v} projects an empty spelling")
            continue
        if s in seen:
            out.append(f"assumption-kind spelling {s} is claimed twice")
        seen.add(s)
        owner = re.search(r'ContractId\("([^"]+)"\)', owners.get(v, ""))
        oid = owner.group(1) if owner else ""
        if oid not in contract_ids:
            out.append(f"assumption kind {s} cites owner {oid}, which no declared "
                       "contract owns")
        elif not re.search(r"\b" + re.escape(s) + r"\b", owner_text.get(oid, "")):
            out.append(f"assumption kind {s} cites owner {oid}, whose authoritative "
                       "document does not author the spelling")
        basis = re.search(r'GuaranteeRef::dec\("([^"]+)"\)', bases.get(v, ""))
        bid = basis.group(1) if basis else ""
        if bid not in dec_ids:
            out.append(f"assumption kind {s} cites admission basis {bid}, which no "
                       "declared decision owns")
        else:
            row = re.search(
                r'DecisionSpec \{ id: "' + re.escape(bid)
                + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
            fields = " ".join(g or "" for g in row.groups()) if row else ""
            if not re.search(r"\b" + re.escape(s) + r"\b", fields):
                out.append(f"assumption kind {s} cites admission basis {bid}, whose "
                           "forward-policy fields do not name the kind")
        marker = markers.get(v, "")
        if v == "UnsafeMemoryContract" and marker != "true":
            out.append("UnsafeMemoryContract must intrinsically require the "
                       "SAFETY-CONTRACT marker")
        if v != "UnsafeMemoryContract" and marker == "true":
            out.append(f"only UnsafeMemoryContract intrinsically requires the "
                       f"SAFETY-CONTRACT marker; {s} claims it")
        cg = re.search(r"GateId::(\w+)", cgates.get(v, ""))
        if not cg or cg.group(1) != "G3":
            out.append(f"assumption kind {s} classifies at "
                       f"{cg.group(1) if cg else '?'}; the ledger boundary is "
                       "classified at G3")
        rg = re.search(r"GateId::(\w+)", rgates.get(v, ""))
        if not rg or rg.group(1) != "G9":
            out.append(f"assumption kind {s} qualifies for release at "
                       f"{rg.group(1) if rg else '?'}; the release seal consumes "
                       "the ledger at G9")
        rows.append(f"{s:<22} {bid:<9} {marker:<7} "
                    f"{cg.group(1) if cg else '?':<9} {rg.group(1) if rg else '?'}")
    # docs/19 carries the fact rows; the semantic owner projects, docs/12
    # only consumes.
    doc19 = (root / "docs/19_SECURITY_MODEL.md").read_text(encoding="utf-8")
    m = re.search(
        r"<!-- COMPILER-ASSUMPTION-KINDS:BEGIN generated from "
        r"spec/compiler_assumptions\.rs by bootstrap/project\.py; do not edit -->"
        r"\n```text\n(.*?)\n```\n<!-- COMPILER-ASSUMPTION-KINDS:END -->", doc19, re.S)
    if not m:
        out.append("docs/19 carries no generated COMPILER-ASSUMPTION-KINDS block "
                   "naming spec/compiler_assumptions.rs as source")
    else:
        header = f"{'kind':<22} {'basis':<9} {'marker':<7} {'classify':<9} release"
        expected = [header] + rows
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        for i, want in enumerate(expected):
            if i >= len(listed):
                out.append(f"docs/19 COMPILER-ASSUMPTION-KINDS omits row "
                           f"{want.split()[0]}")
            elif listed[i] != want.rstrip():
                out.append(f"docs/19 COMPILER-ASSUMPTION-KINDS row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(want.split())} at that position")
        for extra in listed[len(expected):]:
            out.append(f"docs/19 COMPILER-ASSUMPTION-KINDS lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    # No universal assumption identity, and no other module speaks the type.
    for rel in sorted((root / "spec").rglob("*.rs")):
        rsrc = _uncomment(rel.read_text(encoding="utf-8"))
        rel_spec = rel.relative_to(root / "spec").as_posix()
        owner = rel_spec.split("/", 1)[0]
        owner = owner[:-3] if owner.endswith(".rs") else owner
        if re.search(r"pub (?:struct|enum|type) (?:CompilerAssumptionId|AssumptionKind)\b",
                     rsrc):
            out.append(f"spec/{rel_spec} declares a universal assumption identity; "
                       "only earned ledgerable kinds enter CompilerAssumptionKind")
        if owner in ("architecture", "reconciliation") and re.search(
                r"\bCompilerAssumptionKind\b", rsrc):
            out.append(f"spec/{owner}.rs speaks CompilerAssumptionKind; the type has "
                       "one owner module")
    return out


# --- Corpus reconciliation epoch (5.5E3g) ------------------------------------
# spec/corpus.rs owns which corpus epochs exist and which is CURRENT. The
# epoch answers "which coherent semantic corpus does this document belong
# to" — never a date, release, digest, proof freshness, or the runtime
# reconciliation posture that spec/reconciliation.rs owns under DEC-075.
def corpus_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/corpus.rs"
    if not path.is_file():
        return ["missing spec/corpus.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    enum_body = re.search(r"pub enum ReconciliationEpoch \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[ReconciliationEpoch\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bReconciliationEpoch::(\w+)", all_body.group(1)) \
        if all_body else []
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"ReconciliationEpoch::{missing} is omitted from "
                   "ReconciliationEpoch::ALL")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"ReconciliationEpoch::ALL names {phantom}, which the enum "
                   "does not declare")
    cur = re.search(
        r"pub const CURRENT_RECONCILIATION_EPOCH:\s*ReconciliationEpoch = "
        r"ReconciliationEpoch::(\w+);", src)
    if not cur:
        out.append("spec/corpus.rs declares no CURRENT_RECONCILIATION_EPOCH")
        return out
    current = cur.group(1)
    if current not in inventory:
        out.append(f"CURRENT_RECONCILIATION_EPOCH names {current}, which "
                   "ReconciliationEpoch::ALL does not declare")
    spellings = dict(re.findall(r'ReconciliationEpoch::(\w+) => "([^"]*)",', src))
    owners = dict(re.findall(
        r'ReconciliationEpoch::(\w+) => ContractId\("([^"]+)"\)', src))
    bases = dict(re.findall(
        r'ReconciliationEpoch::(\w+) => \{?\s*GuaranteeRef::Seed\(SeedId\("([^"]+)"\)\)',
        src))
    inv_src = _uncomment((root / "spec/invariants.rs").read_text(encoding="utf-8"))
    seen: set[str] = set()
    for v in inventory:
        s = spellings.get(v, "")
        if not s.strip():
            out.append(f"ReconciliationEpoch::{v} projects an empty spelling")
            continue
        if s in seen:
            out.append(f"epoch spelling {s} is claimed twice")
        seen.add(s)
        oid = owners.get(v, "")
        if oid not in contract_ids:
            out.append(f"epoch {s} cites owner {oid}, which no declared contract owns")
        else:
            otext = None
            for p in sorted(root.rglob("*.md")):
                rel = p.relative_to(root)
                if any(part in EXCLUDE_DIRS for part in rel.parts):
                    continue
                text = p.read_text(encoding="utf-8")
                if frontmatter(text).get("contract_id") == oid:
                    # Authored BODY only: every document's frontmatter now
                    # carries the projected epoch spelling, and metadata may
                    # not notarize its own authorship claim.
                    body = re.sub(r"\A---\n.*?\n---\n", "", text, flags=re.S)
                    otext = A_GENERATED_BLOCK.sub("", body)
                    break
            for token in ("ReconciliationEpoch", s):
                if otext is None or not re.search(
                        r"(?<![A-Za-z0-9_-])" + re.escape(token) + r"(?![A-Za-z0-9_-])",
                        otext):
                    out.append(f"epoch {s} cites owner {oid}, whose authoritative "
                               f"document does not author {token}")
        basis = bases.get(v, "")
        row = re.search(r'id: "' + re.escape(basis) + r'", statement: "([^"]*)"', inv_src)
        if not row:
            out.append(f"epoch {s} cites admission basis {basis}, which no declared "
                       "seed row owns")
        elif "ReconciliationEpoch" not in row.group(1):
            out.append(f"{basis} does not name ReconciliationEpoch in its statement; "
                       "the admission basis must name what it admits")
    # Every eligible document claims the CURRENT corpus epoch; standalone
    # generated documents bind their SOURCE epoch. A date never substitutes.
    current_spelling = spellings.get(current, "")
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        meta = frontmatter(p.read_text(encoding="utf-8"))
        key = ("source_reconciliation_epoch" if meta.get("status") == "GENERATED"
               else "reconciliation_epoch")
        got = meta.get(key)
        if got is None:
            out.append(f"{rel.as_posix()}: declares no {key}; every eligible "
                       "document claims the current corpus epoch")
            continue
        if key == "reconciliation_epoch" and "last_reconciled" not in meta:
            out.append(f"{rel.as_posix()}: last_reconciled is deleted; the epoch "
                       "answers WHICH corpus, the date answers WHEN reviewed, and "
                       "both stand")
        if re.fullmatch(r"\d{4}-\d{2}-\d{2}", got):
            out.append(f"{rel.as_posix()}: a date ({got}) cannot substitute for a "
                       "corpus epoch")
        elif got != current_spelling:
            out.append(f"{rel.as_posix()}: claims corpus epoch {got}; the current "
                       f"corpus epoch is {current_spelling}")
    # spec/reconciliation.rs owns runtime execution reconciliation, never the
    # corpus epoch.
    recon = _uncomment((root / "spec/reconciliation.rs").read_text(encoding="utf-8"))
    if re.search(r"\bReconciliationEpoch\b", recon):
        out.append("spec/reconciliation.rs speaks ReconciliationEpoch; the runtime "
                   "reconciliation module does not own the corpus epoch")
    # The docs/00 fact block: exact provenance and ordered content.
    doc00 = (root / "docs/00_CONSTITUTION.md").read_text(encoding="utf-8")
    m = re.search(
        r"<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from spec/corpus\.rs by "
        r"bootstrap/project\.py; do not edit -->\n```text\n(.*?)\n```\n<!-- "
        r"CORPUS-RECONCILIATION-EPOCH:END -->", doc00, re.S)
    if not m:
        out.append("docs/00 carries no generated CORPUS-RECONCILIATION-EPOCH block "
                   "naming spec/corpus.rs as source")
    else:
        want = [f"{'current epoch':<18} {current_spelling}",
                f"{'semantic owner':<18} {owners.get(current, '')}",
                f"{'admission basis':<18} {bases.get(current, '')}"]
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        if listed != want:
            out.append(f"docs/00 CORPUS-RECONCILIATION-EPOCH states {listed}; the "
                       f"typed owner states {want}")
    return out


# --- Mutation vocabulary (5.5E3f) --------------------------------------------
# spec/mutation.rs owns the lanes and result classifications as total const
# functions. This auditor independently parses the fn arms, executes the
# semantic fences, reconstructs the two docs/12 fact tables, and refuses the
# lettered lane nicknames anywhere in authoritative prose.
A_LANE_ALIAS = re.compile(r"\bLane\s+[ABC]\b")


def _mutation_arms(src: str, enum: str, name: str) -> dict[str, str]:
    impl = re.search(r"impl " + enum + r" \{(.*?)\n\}", src, re.S)
    body = re.search(
        r"pub const fn " + name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
        impl.group(1) if impl else "", re.S)
    arms: dict[str, str] = {}
    if not body:
        return arms
    for arm, value in re.findall(
            r"(" + enum + r"::\w+(?:\s*\|\s*" + enum + r"::\w+)*)\s*=>\s*([^,]+),",
            body.group(1), re.S):
        for v in re.findall(r"\b" + enum + r"::(\w+)", arm):
            arms[v] = value.strip()
    return arms


def mutation_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/mutation.rs"
    if not path.is_file():
        return ["missing spec/mutation.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    inventories: dict[str, list[str]] = {}
    for enum in ("MutationLane", "MutationResult"):
        enum_body = re.search(r"pub enum " + enum + r" \{(.*?)\n\}", src, re.S)
        variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) \
            if enum_body else []
        all_body = re.search(
            r"pub const ALL: &'static \[" + enum + r"\] = &\[(.*?)\];", src, re.S)
        inventory = re.findall(r"\b" + enum + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        for missing in [v for v in variants if v not in inventory]:
            out.append(f"{enum}::{missing} is omitted from {enum}::ALL")
        for phantom in [v for v in inventory if v not in variants]:
            out.append(f"{enum}::ALL names {phantom}, which the enum does not declare")
        inventories[enum] = inventory
        spellings = _mutation_arms(src, enum, "spelling")
        owners = _mutation_arms(src, enum, "semantic_owner")
        seen: set[str] = set()
        for v in inventory:
            s = spellings.get(v, "").strip('"')
            if not s.strip():
                out.append(f"{enum}::{v} projects an empty spelling")
                continue
            if s in seen:
                out.append(f"{enum} spelling {s} is claimed twice")
            seen.add(s)
            owner = re.search(r'ContractId\("([^"]+)"\)', owners.get(v, ""))
            oid = owner.group(1) if owner else ""
            if oid not in contract_ids:
                out.append(f"{enum} {s} cites owner {oid}, which no declared "
                           "contract owns")
            elif not re.search(r"\b" + re.escape(s) + r"\b", owner_text.get(oid, "")):
                out.append(f"{enum} {s} cites owner {oid}, whose authoritative "
                           "document does not author the spelling")
    # Lane facts: the semantic fences, executed against the parsed arms.
    lane_fields = {name: _mutation_arms(src, "MutationLane", name) for name in (
        "spelling", "admission_basis", "requires_per_candidate_rust_compile",
        "requires_real_rustc_semantics", "requires_activation_evidence",
        "permits_production_profile_slots", "requires_independent_evidence_route",
        "gates")}
    lane_rows: list[str] = []
    for v in inventories.get("MutationLane", []):
        s = lane_fields["spelling"].get(v, "").strip('"')
        basis = re.search(r'DecisionId\("([^"]+)"\)',
                          lane_fields["admission_basis"].get(v, ""))
        bid = basis.group(1) if basis else ""
        if bid not in dec_ids:
            out.append(f"lane {s} cites admission basis {bid}, which no declared "
                       "decision owns")
        else:
            row = re.search(
                r'DecisionSpec \{ id: "' + re.escape(bid)
                + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
            fields = " ".join(g or "" for g in row.groups()) if row else ""
            if not re.search(r"\b" + re.escape(s) + r"\b", fields):
                out.append(f"{bid} forward-policy fields do not name lane {s}; "
                           "the admission boundary must name what it admits")
        pc = lane_fields["requires_per_candidate_rust_compile"].get(v, "")
        rr = lane_fields["requires_real_rustc_semantics"].get(v, "")
        act = lane_fields["requires_activation_evidence"].get(v, "")
        slots = lane_fields["permits_production_profile_slots"].get(v, "")
        ind = lane_fields["requires_independent_evidence_route"].get(v, "")
        gates = re.findall(r"GateId::(\w+)", lane_fields["gates"].get(v, ""))
        if act != "true":
            out.append(f"lane {s} does not require activation evidence; a green "
                       "test with a dormant mutant is not evidence")
        if slots != "false":
            out.append(f"lane {s} permits production-profile mutation slots")
        if ind != "true":
            out.append(f"lane {s} does not require an independent evidence route")
        if gates != ["G3"]:
            out.append(f"lane {s} gates {gates} are not exactly G3")
        if v == "SemanticIr":
            if rr != "false":
                out.append("SemanticIr claims real rustc semantics; the reference "
                           "interpreter lane must not")
            if pc != "false":
                out.append("SemanticIr requires a per-candidate Rust compile; the "
                           "semantic lane compiles nothing")
        if v == "SelectableCompiled":
            if rr != "true":
                out.append("SelectableCompiled must run under real rustc semantics")
            if pc != "false":
                out.append("SelectableCompiled must not require a per-candidate "
                           "Rust compile; one shard compiles once")
        if v == "CompilerBacked":
            if pc != "true":
                out.append("CompilerBacked must require a per-candidate Rust compile")
            if rr != "true":
                out.append("CompilerBacked must run under real rustc semantics")
        lane_rows.append(f"{s:<20} {pc:<7} {rr:<7} {act:<7} {slots:<7} {ind:<7} "
                         f"{' '.join(gates)}".rstrip())
    # Result classification: only Killed kills, only Survived survives, every
    # result stays in the denominator.
    result_fields = {name: _mutation_arms(src, "MutationResult", name) for name in (
        "spelling", "counts_as_kill", "counts_as_survival", "appears_in_denominator")}
    result_rows: list[str] = []
    for v in inventories.get("MutationResult", []):
        s = result_fields["spelling"].get(v, "").strip('"')
        kill = result_fields["counts_as_kill"].get(v, "")
        surv = result_fields["counts_as_survival"].get(v, "")
        denom = result_fields["appears_in_denominator"].get(v, "")
        if v == "Killed" and kill != "true":
            out.append("Killed must count as a kill")
        if v != "Killed" and kill == "true":
            out.append(f"only Killed counts as a kill; {s} claims one")
        if v == "Survived" and surv != "true":
            out.append("Survived must count as survival")
        if v != "Survived" and surv == "true":
            out.append(f"only Survived counts as survival; {s} claims one")
        if denom != "true":
            out.append(f"{s} leaves the denominator; nothing exits silently to "
                       "improve a score")
        result_rows.append(f"{s:<22} {kill:<7} {surv:<7} {denom}")
    # docs/12 fact tables: exact ordered content plus provenance.
    doc12 = (root / "docs/12_TESTPAK.md").read_text(encoding="utf-8")
    lane_header = (f"{'lane':<20} {'compile':<7} {'rustc':<7} {'activ':<7} "
                   f"{'slots':<7} {'indep':<7} gates")
    result_header = f"{'result':<22} {'kill':<7} {'surv':<7} denominator"
    for marker, expected in (("MUTATION-LANES", [lane_header] + lane_rows),
                             ("MUTATION-RESULTS", [result_header] + result_rows)):
        m = re.search(
            r"<!-- " + marker + r":BEGIN generated from spec/mutation\.rs by "
            r"bootstrap/project\.py; do not edit -->\n```text\n(.*?)\n```\n<!-- "
            + marker + r":END -->", doc12, re.S)
        if not m:
            out.append(f"docs/12 carries no generated {marker} block naming "
                       "spec/mutation.rs as source")
            continue
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        expected = [line.rstrip() for line in expected]
        for i, want in enumerate(expected):
            if i >= len(listed):
                out.append(f"docs/12 {marker} omits row {want.split()[0]}")
            elif listed[i] != want:
                out.append(f"docs/12 {marker} row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(want.split())} at that position")
        for extra in listed[len(expected):]:
            out.append(f"docs/12 {marker} lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    # The lettered nicknames are dead in authoritative prose.
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        text = rel.read_text(encoding="utf-8")
        if frontmatter(text).get("status") == "GENERATED":
            continue
        for m in A_LANE_ALIAS.finditer(A_GENERATED_BLOCK.sub("", text)):
            out.append(f"{rel.name} cites the letter alias {m.group(0)!r} in "
                       "authoritative prose; the lanes are SemanticIr, "
                       "SelectableCompiled, and CompilerBacked")
    return out


# --- Command namespaces (5.5E3e) ---------------------------------------------
# spec/commands.rs owns three separate closed namespaces and each entry's
# typed authority relation. A command's namespace identity and its invoked
# semantic authority are separate axes: Direct owners own the command-level
# operation; a Composite's composition owner owns ONLY orchestration and
# routing, and composition never transfers ownership.
A_COMMAND_NAMESPACES = (
    ("PRODUCT-COMMANDS", "ProductCommand", "docs/26_COMMAND_PLANE.md"),
    ("TESTPAK-COMMANDS", "TestPakCommand", "docs/26_COMMAND_PLANE.md"),
    ("BATQL-SOURCE-MODES", "BatQlSourceMode", "companion/BATQL_LANGUAGE.md"),
)


def _command_block_body(name: str, doc: str):
    m = re.search(
        r"<!-- " + re.escape(name)
        + r":BEGIN generated from spec/commands\.rs by bootstrap/project\.py; "
        + r"do not edit -->\n```text\n(.*?)\n```\n<!-- "
        + re.escape(name) + r":END -->", doc, re.S)
    return m.group(1) if m else None


def command_catalog_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/commands.rs"
    if not path.is_file():
        return ["missing spec/commands.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)

    def owner_authors(cid: str, term: str) -> bool:
        return bool(re.search(r"\b" + re.escape(term) + r"\b", owner_text.get(cid, "")))

    # No universal CommandId anywhere in the public spec: command identity
    # lives in three separate namespaces or nowhere.
    for rel in sorted((root / "spec").rglob("*.rs")):
        if re.search(r"pub (?:struct|enum|type) CommandId\b",
                     _uncomment(rel.read_text(encoding="utf-8"))):
            out.append(f"spec/{rel.name} declares a universal CommandId; command "
                       "identity lives in three separate namespaces or nowhere")
    # The composition boundary is DECLARED in authored docs/26 prose, and the
    # typed authority must agree in both directions — a command cannot
    # silently collapse to a single sovereign or silently gain a composite.
    doc26 = (root / "docs/26_COMMAND_PLANE.md").read_text(encoding="utf-8")
    comp_sec = re.search(r"## Composite commands\n(.*?)(?:\n## |\Z)",
                         A_GENERATED_BLOCK.sub("", doc26), re.S)
    declared_composites: set[str] = set()
    if not comp_sec:
        out.append("docs/26 carries no authored Composite commands section "
                   "describing the composition boundary")
    else:
        declared_composites = set(re.findall(r"`(\w+)`", comp_sec.group(1)))
    companion = (root / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")
    docs_of = {"docs/26_COMMAND_PLANE.md": doc26,
               "companion/BATQL_LANGUAGE.md": companion}
    for marker, kind, docrel in A_COMMAND_NAMESPACES:
        all_body = re.search(
            r"pub const ALL: &'static \[" + kind + r"\] = &\[(.*?)\];", src, re.S)
        inventory = re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        if not inventory:
            out.append(f"spec/commands.rs declares no {kind} inventory")
            continue
        enum_body = re.search(r"pub enum " + kind + r" \{(.*?)\n\}", src, re.S)
        variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
        for missing in [v for v in variants if v not in inventory]:
            out.append(f"{kind}::{missing} is omitted from {kind}::ALL")
        for phantom in [v for v in inventory if v not in variants]:
            out.append(f"{kind}::ALL names {phantom}, which the enum does not declare")
        tokens: dict[str, str] = {}
        for arm, tok in re.findall(
                r"\b" + kind + r"::(\w+(?:\s*\|\s*" + kind + r"::\w+)*) => \"([^\"]+)\",",
                src):
            for v in re.split(r"\s*\|\s*", arm):
                tokens[v.split("::")[-1]] = tok
        authority: dict[str, tuple[str, str, list[str]]] = {}
        for arm_body, direct, comp in re.findall(
                r"(" + kind + r"::\w+(?:\s*\|\s*" + kind + r"::\w+)*)\s*=>\s*\{?\s*"
                r"(?:CommandAuthority::Direct\(ContractId\(\"([^\"]+)\"\)\)"
                r"|CommandAuthority::(Composite \{.*?\}))", src, re.S):
            if direct:
                entry = ("direct", direct, [])
            else:
                owner = re.search(r'composition_owner: ContractId\("([^"]+)"\)', comp)
                delegates = re.findall(r'ContractId\("([^"]+)"\)',
                                       comp.split("delegates:", 1)[-1])
                entry = ("composite", owner.group(1) if owner else "", delegates)
            for v in re.findall(r"\b" + kind + r"::(\w+)", arm_body):
                authority[v] = entry
        seen_tokens: set[str] = set()
        rows: list[str] = []
        for v in inventory:
            if v not in tokens or v not in authority:
                out.append(f"{kind}::{v} declares no token or no authority relation")
                continue
            token = tokens[v]
            shape, owner, delegates = authority[v]
            rows.append(f"{token:<10} {shape:<10} {owner:<28} "
                        f"{' '.join(delegates)}".rstrip())
            if not token.strip():
                out.append(f"{kind}::{v} projects an empty token")
            if token in seen_tokens:
                out.append(f"{kind} token {token} is claimed twice; tokens are "
                           "unique within their namespace")
            seen_tokens.add(token)
            if kind != "BatQlSourceMode" and token.lower() in ("ask", "do"):
                out.append(f"{kind} admits {token}: ASK and DO are language modes, "
                           "never CLI verbs, and DO is never a top-level command")
            if shape == "direct":
                if owner == "BP-COMMAND-PLANE-1":
                    out.append(f"{kind} {token} is Direct-owned by BP-COMMAND-PLANE-1, "
                               "which may own command composition but never the "
                               "command-level meaning")
                if owner not in contract_ids:
                    out.append(f"{kind} {token} cites owner {owner}, which no "
                               "declared contract owns")
                elif not owner_authors(owner, token):
                    out.append(f"{kind} {token} cites owner {owner}, whose "
                               "authoritative document does not author the token")
            else:
                if owner not in contract_ids:
                    out.append(f"{kind} {token} cites composition owner {owner}, "
                               "which no declared contract owns")
                elif not owner_authors(owner, token):
                    out.append(f"{kind} {token} cites composition owner {owner}, "
                               "whose authoritative document does not author the token")
                if not delegates:
                    out.append(f"{kind} {token} is a composite with no delegates; "
                               "an orchestration boundary with nothing to route "
                               "to is refused")
                if len(set(delegates)) != len(delegates):
                    out.append(f"{kind} {token} repeats a delegate; each delegate "
                               "retains its own law exactly once")
                if owner in delegates:
                    out.append(f"{kind} {token} lists its composition owner as its "
                               "own delegate; composition never transfers ownership")
                for d in delegates:
                    if d not in contract_ids:
                        out.append(f"{kind} {token} delegates to {d}, which no "
                                   "declared contract owns")
            if kind == "ProductCommand":
                if shape == "composite" and token not in declared_composites:
                    out.append(f"docs/26 does not declare {token} a composition "
                               "boundary; the typed authority may not silently "
                               "gain a composite")
                if shape == "direct" and token in declared_composites:
                    out.append(f"docs/26 declares {token} a composition boundary; "
                               f"a Direct owner would collapse it to a single "
                               "sovereign")
        body = _command_block_body(marker, docs_of[docrel])
        if body is None:
            out.append(f"{docrel} carries no generated {marker} block naming "
                       "spec/commands.rs as source")
            continue
        listed = [line.rstrip() for line in body.splitlines() if line.strip()]
        for i, expect in enumerate(rows):
            if i >= len(listed):
                out.append(f"{docrel} {marker} omits row {expect.split()[0]}")
            elif listed[i] != expect:
                out.append(f"{docrel} {marker} row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(expect.split())} at that position")
        for extra in listed[len(rows):]:
            out.append(f"{docrel} {marker} lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    return out

