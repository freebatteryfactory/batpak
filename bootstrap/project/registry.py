"""The generated-view registry mechanics (5.5E4a).

Parses the typed spec/generated_views.rs registry and provides the marker block
pattern and canonical block serializer that the plan builder dispatches through.
Standard library only.
"""
from __future__ import annotations

import re


def block_pattern(name: str) -> re.Pattern[str]:
    return re.compile(
        r"(<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n)(.*?)(\n<!-- " + re.escape(name) + r":END -->)",
        re.S,
    )


# --- The generated-view registry (5.5E4a) ------------------------------------
# spec/generated_views.rs owns which generated views exist. This projector
# parses the typed registry independently, dispatches through VIEW_RENDERERS,
# and builds EVERY projection plan by iterating the registry — the scattered
# hand-built plan denominator is dead, and a manual literal plan cannot
# bypass the registry.
GENERATED_VIEWS_SRC = "spec/generated_views.rs"
_VIEW_SPEC_ARM = re.compile(
    r"GeneratedView::(\w+) => GeneratedViewSpec \{\s*"
    r"authority_sources: &\[([^\]]*)\],\s*"
    r"target: GeneratedViewTarget::(?:Static\(&\[([^\]]*)\]\)|(EligibleMarkdownCorpus)),\s*"
    r"surface: GeneratedViewSurface::(\w+),\s*"
    r'marker: (?:Some\("([^"]+)"\)|None),\s*'
    r"generator: BootstrapToolId::(\w+),", re.S)


def parse_generated_views(root: Path) -> list[dict]:
    """The typed registry in GeneratedView::ALL order. Unknown or incomplete
    arms fail generation; there is no raw fallback."""
    src = (root / GENERATED_VIEWS_SRC).read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[GeneratedView\] = &\[(.*?)\];", src, re.S)
    if not all_body:
        raise SystemExit("project: spec/generated_views.rs declares no GeneratedView::ALL")
    order = re.findall(r"GeneratedView::(\w+)", all_body.group(1))
    arms: dict[str, dict] = {}
    for m in _VIEW_SPEC_ARM.finditer(src):
        arms[m.group(1)] = {
            "name": m.group(1),
            "sources": re.findall(r'"([^"]+)"', m.group(2)),
            "targets": re.findall(r'"([^"]+)"', m.group(3) or ""),
            "corpus": bool(m.group(4)),
            "surface": m.group(5),
            "marker": m.group(6),
            "generator": m.group(7),
        }
    views = []
    for name in order:
        if name not in arms:
            raise SystemExit(f"project: GeneratedView::{name} has no parseable spec() arm")
        views.append(arms[name])
    return views


def canonical_block(marker: str, source: str, body: str) -> str:
    """One embedded view, marker AND body: the BEGIN provenance is generated
    mechanically from the registry, never preserved from the target. Multiple
    authority sources are joined in registry order by "; " — source order is
    provenance, and reordering without changing the set is drift."""
    return (f"<!-- {marker}:BEGIN generated from {source} by bootstrap/project.py; "
            f"do not edit -->\n{body}\n<!-- {marker}:END -->")
