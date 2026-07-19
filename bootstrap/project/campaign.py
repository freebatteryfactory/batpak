"""Campaign closure graph schema projection (F5, DEC-079..DEC-082).

spec/campaign/types.rs owns the closure node/edge vocabulary, the campaign
terminals and their denominator-home law, and the frontier/freshness axes.
This projector parses the typed owner directly; no Python field, edge-kind,
terminal, or axis spelling map exists here. The view projects the closure
SCHEMA the campaign's runtime-minted records will instantiate — never runtime
candidate data: no campaign has run at generation time, the same way
GateInventory projects gate rows, not gate executions.
Standard library only; shares no parser with bootstrap/audit.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .guarantees import Unadmitted

CAMPAIGN_TYPES = "spec/campaign/types.rs"


def _campaign_all_variants(src: str, enum_name: str) -> list[str]:
    body = re.search(
        r"pub const ALL: &'static \[" + enum_name + r"\] = &\[(.*?)\];", src, re.S)
    if not body:
        raise Unadmitted(f"{CAMPAIGN_TYPES}: {enum_name}::ALL inventory unreadable")
    variants = re.findall(re.escape(enum_name) + r"::(\w+)", body.group(1))
    if not variants:
        raise Unadmitted(f"{CAMPAIGN_TYPES}: {enum_name}::ALL names no variants")
    return variants


def campaign_closure_schema(root: Path):
    """(node fields, edge kinds, terminals, frontier, freshness) parsed from
    the typed campaign owner. Node fields are (name, type) in authored order;
    edge kinds are (kind, derived-from clause) where the clause is the
    variant's own authored "Read from ..." law naming the required record
    field the edge is read back out of; terminals are (terminal,
    remains-in-denominator verdict) from the exhaustive denominator-home
    match. A missing field set, edge law, or denominator arm refuses."""
    src = (root / CAMPAIGN_TYPES).read_text(encoding="utf-8")
    node = re.search(r"pub struct CampaignClosureNode \{(.*?)\n\}", src, re.S)
    if not node:
        raise Unadmitted(f"{CAMPAIGN_TYPES}: CampaignClosureNode unreadable")
    fields = re.findall(r"pub (\w+): ([^,\n]+),", node.group(1))
    if not fields:
        raise Unadmitted(f"{CAMPAIGN_TYPES}: CampaignClosureNode declares no fields")
    edge_enum = re.search(r"pub enum CampaignClosureEdgeKind \{(.*?)\n\}", src, re.S)
    if not edge_enum:
        raise Unadmitted(f"{CAMPAIGN_TYPES}: CampaignClosureEdgeKind unreadable")
    derived: dict[str, str] = {}
    for doc, variant in re.findall(
            r"((?:[ \t]*///[^\n]*\n)+)[ \t]*(\w+),", edge_enum.group(1)):
        clause = " ".join(
            line.strip().lstrip("/").strip() for line in doc.strip().splitlines())
        derived[variant] = clause.split(":")[0]
    edge_kinds = _campaign_all_variants(src, "CampaignClosureEdgeKind")
    for kind in edge_kinds:
        if not derived.get(kind, "").startswith("Read from"):
            raise Unadmitted(
                f"{CAMPAIGN_TYPES}: CampaignClosureEdgeKind::{kind} states no "
                "derived-from-required-field law")
    terminals = _campaign_all_variants(src, "CampaignTerminal")
    denom_body = re.search(
        r"pub const fn remains_in_denominator\(self\) -> bool \{\s*"
        r"match self \{(.*?)\n        \}", src, re.S)
    if not denom_body:
        raise Unadmitted(
            f"{CAMPAIGN_TYPES}: remains_in_denominator matrix unreadable")
    denom = dict(re.findall(
        r"CampaignTerminal::(\w+) => (true|false),", denom_body.group(1)))
    for terminal in terminals:
        if terminal not in denom:
            raise Unadmitted(
                f"{CAMPAIGN_TYPES}: CampaignTerminal::{terminal} has no "
                "denominator-home arm")
    frontier = _campaign_all_variants(src, "FrontierState")
    freshness = _campaign_all_variants(src, "EvidenceFreshness")
    return (fields, [(k, derived[k]) for k in edge_kinds],
            [(t, denom[t]) for t in terminals], frontier, freshness)


def render_campaign_closure_graph(root: Path) -> str:
    """docs/39 CAMPAIGN-CLOSURE-GRAPH: the closure-graph SCHEMA a campaign's
    records instantiate — the typed node field set, the derived closure edge
    kinds each naming the required record field it is read back out of, the
    terminal denominator-home law, and the frontier/freshness axes, parsed
    from spec/campaign/types.rs. This view renders NO runtime candidate data:
    no campaign has run at generation time, and the projection is the
    schema/law the campaign evidence will instantiate — the same way
    GateInventory projects gate rows, not gate executions."""
    fields, edges, terminals, frontier, freshness = campaign_closure_schema(root)
    fw = max(len(n) for n, _t in fields) + 2
    ew = max(len(k) for k, _c in edges) + 2
    tw = max(len(t) for t, _d in terminals) + 2
    sections = [
        "Closure node fields (one node per lineage record)\n" + "\n".join(
            f"  {n:<{fw}} {t}" for n, t in fields),
        "Closure edge kinds (derived from required record fields)\n" + "\n".join(
            f"  {k:<{ew}} {c}" for k, c in edges),
        "Campaign terminals (denominator home)\n" + "\n".join(
            f"  {t:<{tw}} remains_in_denominator {d}" for t, d in terminals),
        "Frontier states\n" + "\n".join(f"  {s}" for s in frontier),
        "Evidence freshness\n" + "\n".join(f"  {s}" for s in freshness),
    ]
    return "```text\n" + "\n\n".join(sections) + "\n```"
