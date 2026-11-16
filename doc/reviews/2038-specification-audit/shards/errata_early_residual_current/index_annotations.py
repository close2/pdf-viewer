#!/usr/bin/env python3
"""Build a compact, reproducible inventory of EC3 issue annotations through physical p.450.

This records issue IDs, pages, annotation shape, reply state, and citation counts. It does not
copy annotation bodies or the licensed source text. Run from the audit worktree root.
"""
from __future__ import annotations

import collections
import json
import re
from pathlib import Path
import pikepdf

ROOT = Path.cwd()
SOURCE = ROOT / "doc/ISO_32000-2_sponsored_EC3.pdf"
LEDGER = ROOT / "doc/reviews/1983-specification-audit/features.json"
SHARDS = ROOT / "doc/reviews/2038-specification-audit/shards"
DEST = ROOT / "scratchpad/r2038/spec-audit/errata_early_residual_current/issues.json"
ISSUE = re.compile(r"Issue #(\d+)")


def txt(obj: object) -> str:
    return str(obj) if obj is not None else ""


pdf = pikepdf.open(SOURCE)
all_anns: list[tuple[int, object]] = []
by_nm: dict[str, tuple[int, object]] = {}
for page_no, page in enumerate(pdf.pages, 1):
    for annot in page.get("/Annots", []):
        all_anns.append((page_no, annot))
        nm = txt(annot.get("/NM"))
        if nm:
            by_nm[nm] = (page_no, annot)


def issue_markers(annot: object) -> set[int]:
    """Every Issue # marker on the annotation, not only its first /Subj marker."""
    out: set[int] = set()
    for key in ("/Subj", "/Contents", "/T"):
        out.update(int(value) for value in ISSUE.findall(txt(annot.get(key))))
    return out


def issue_for(annot: object) -> int | None:
    seen: set[str] = set()
    cur = annot
    for _ in range(12):
        m = ISSUE.search(txt(cur.get("/Subj")))
        if m:
            return int(m.group(1))
        irt = cur.get("/IRT")
        if irt is None:
            return None
        nm = txt(irt.get("/NM"))
        if not nm or nm in seen:
            return None
        seen.add(nm)
        target = by_nm.get(nm)
        if target is None:
            return None
        cur = target[1]
    return None


def content_class(annot: object) -> str:
    body = txt(annot.get("/Contents")).strip()
    low = body.lower()
    subtype = txt(annot.get("/Subtype"))
    if not body:
        return "no-body/markup-target"
    if low.startswith("editor note"):
        return "editorial-note"
    if low.startswith("insert") or low.startswith("add") or low.startswith("new note"):
        return "proposed-insertion"
    if low.startswith("replace") or low.startswith("correct"):
        return "replacement-or-correction"
    if low.startswith("note"):
        return "clarifying-note"
    if subtype in {"/Caret", "/StrikeOut", "/Highlight", "/Underline"}:
        return "targeted-text-change"
    return "annotation-comment"


marked: dict[int, list[tuple[int, object]]] = collections.defaultdict(list)
for page_no, annot in all_anns:
    if page_no <= 450:
        for issue_id in issue_markers(annot):
            marked[issue_id].append((page_no, annot))

members: dict[int, list[tuple[int, object]]] = collections.defaultdict(list)
for page_no, annot in all_anns:
    issue = issue_for(annot)
    if issue is not None and issue in marked:
        members[issue].append((page_no, annot))

ledger_text = LEDGER.read_text()
shard_texts: dict[str, str] = {}
for path in SHARDS.rglob("*"):
    if path.is_file() and path.suffix in {".txt", ".json", ".md"}:
        try:
            shard_texts[str(path.relative_to(ROOT))] = path.read_text(errors="replace")
        except OSError:
            pass

records = []
for issue_id, issue_annotations in sorted(marked.items()):
    all_members = members[issue_id]
    root_types = collections.Counter(txt(a.get("/Subtype")) for _, a in issue_annotations)
    reply_states = collections.Counter(
        txt(a.get("/State")) or "none"
        for _, a in all_members
        if a.get("/IRT") is not None
    )
    target_categories = collections.Counter(
        "text"
        if txt(a.get("/Contents"))
        else "markup-or-structural"
        for _, a in issue_annotations
    )
    ledger_hits = len(re.findall(r"#" + str(issue_id) + r"\b", ledger_text))
    shard_hits = {
        name: len(re.findall(r"#" + str(issue_id) + r"\b", body))
        for name, body in shard_texts.items()
        if re.search(r"#" + str(issue_id) + r"\b", body)
    }
    root_rows = []
    reply_rows = []
    for pn, annot in issue_annotations:
        root_rows.append(
            {
                "physical_page": pn,
                "subj": txt(annot.get("/Subj")),
                "subtype": txt(annot.get("/Subtype")),
                "nm": txt(annot.get("/NM")),
                "irt_nm": txt(annot.get("/IRT").get("/NM")) if annot.get("/IRT") is not None else None,
                "contents_class": content_class(annot),
                "contents_char_count": len(txt(annot.get("/Contents"))),
                "state": txt(annot.get("/State")) or None,
                "state_model": txt(annot.get("/StateModel")) or None,
            }
        )
    for pn, annot in all_members:
        irt = annot.get("/IRT")
        if irt is None:
            continue
        parent_nm = txt(irt.get("/NM"))
        reply_rows.append(
            {
                "physical_page": pn,
                "subj": txt(annot.get("/Subj")),
                "subtype": txt(annot.get("/Subtype")),
                "nm": txt(annot.get("/NM")),
                "irt_nm": parent_nm,
                "irt_issue_id": issue_for(irt),
                "state": txt(annot.get("/State")) or None,
                "state_model": txt(annot.get("/StateModel")) or None,
                "contents_class": content_class(annot),
                "contents_char_count": len(txt(annot.get("/Contents"))),
            }
        )
    records.append(
        {
            "issue_id": issue_id,
            "physical_pages": sorted({pn for pn, _ in issue_annotations}),
            "issue_marked_annotation_count": len(issue_annotations),
            "issue_marked_annotations_with_irt": sum(
                annot.get("/IRT") is not None for _, annot in issue_annotations
            ),
            "root_types": dict(root_types),
            "root_body_presence": dict(target_categories),
            "thread_annotation_count": len(all_members),
            "reply_states": dict(reply_states),
            "issue_annotations": root_rows,
            "thread_edges": reply_rows,
            "ledger_citation_count": ledger_hits,
            "current_shard_citations": shard_hits,
        }
    )

DEST.parent.mkdir(parents=True, exist_ok=True)
DEST.write_text(
    json.dumps(
        {
            "source": "doc/ISO_32000-2_sponsored_EC3.pdf",
            "physical_page_range": [1, 450],
            "method": "pikepdf 10.18.1; every Issue #N marker in /Subj, /Contents, and /T is recorded; reply issue membership follows /IRT and /NM chains; issue-marked annotations can themselves have /IRT and are not called roots; records compact Subj, contents class/length, NM/IRT links, physical page, type and review state; no full annotation Contents is copied",
            "total_unique_issue_ids": len(records),
            "total_issue_marked_annotations": sum(row["issue_marked_annotation_count"] for row in records),
            "total_issue_marked_annotations_with_irt": sum(
                row["issue_marked_annotations_with_irt"] for row in records
            ),
            "unique_issue_ids_with_ledger_citation": sum(row["ledger_citation_count"] > 0 for row in records),
            "unique_issue_ids_with_current_shard_citation": sum(bool(row["current_shard_citations"]) for row in records),
            "issues": records,
        },
        indent=2,
    )
    + "\n"
)
print(f"wrote {DEST.relative_to(ROOT)}: {len(records)} issue IDs")
