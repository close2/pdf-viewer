#!/usr/bin/env python3
"""Build a discovery crosswalk from EC3 issue IDs to current audit artifacts.

This deliberately separates exact mentions and structured record shape from semantic
coverage. It does not infer that a cited issue was understood or that a test proves it.
Run from the r2038 audit worktree with pikepdf installed.
"""
from __future__ import annotations

import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

import pikepdf

ROOT = Path('.')
SOURCE = Path('doc/ISO_32000-2_sponsored_EC3.pdf')
SHARDS = Path('doc/reviews/2038-specification-audit/shards')
OUT = Path(__file__).resolve().parent / 'crosswalk.json'
ISSUE_MARKER = re.compile(r'#\s*(\d+)(?!\d)')
ISSUE_KEYS = {'issue', 'issues', 'issue_id', 'issue_ids', 'errata_issue', 'ec3_issue'}
DIRECT_ISSUE_TEXT_KEYS = {'errata', 'effective_errata', 'ec3', 'annotation', 'annotation_evidence', 'source_location', 'primary_locator', 'source_and_errata', 'source/effective_ec3'}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_inventory() -> tuple[dict[int, dict[str, Any]], str]:
    inventory: dict[int, dict[str, Any]] = {}
    with pikepdf.Pdf.open(SOURCE) as pdf:
        for page_no, page in enumerate(pdf.pages, 1):
            for annotation in page.get('/Annots', []):
                subject = str(annotation.get('/Subj', ''))
                # The historical 360-ID census is specifically the annotation /Subj scan.
                for issue in map(int, ISSUE_MARKER.findall(subject)):
                    row = inventory.setdefault(issue, {
                        'physical_pages': set(),
                        'annotation_objects': set(),
                        'annotation_types': Counter(),
                    })
                    row['physical_pages'].add(page_no)
                    objgen = tuple(annotation.objgen)
                    row['annotation_objects'].add(objgen)
                    subtype = str(annotation.get('/Subtype', 'unknown')).lstrip('/')
                    row['annotation_types'][subtype] += 1
    frozen = {
        issue: {
            'physical_pages': sorted(row['physical_pages']),
            'annotation_objects': [list(x) for x in sorted(row['annotation_objects'])],
            'annotation_count': sum(row['annotation_types'].values()),
            'annotation_types': dict(sorted(row['annotation_types'].items())),
        }
        for issue, row in sorted(inventory.items())
    }
    return frozen, sha(SOURCE)


def scalar_issue_values(value: Any) -> set[int]:
    if isinstance(value, bool):
        return set()
    if isinstance(value, int):
        return {value}
    if isinstance(value, str):
        vals = set(map(int, ISSUE_MARKER.findall(value)))
        if value.strip().isdigit():
            vals.add(int(value.strip()))
        return vals
    if isinstance(value, list):
        vals: set[int] = set()
        for item in value:
            vals |= scalar_issue_values(item)
        return vals
    return set()


def record_shape(value: dict[str, Any]) -> dict[str, Any]:
    keys = {str(key).lower() for key in value}
    source = bool(keys & {'source', 'source_location', 'primary_locator', 'effective_location', 'source_pages', 'source_page', 'clause', 'clause_title'})
    disposition = bool(keys & {'classification', 'verdict', 'assessment', 'disposition', 'decision', 'effective_change', 'change', 'result', 'conclusion'})
    assertion = bool(keys & {'code', 'tests', 'test_assertions', 'assertions', 'consumer', 'implementation', 'implementation_and_assertions', 'ledger_review_code'})
    exposed = {}
    for key, val in value.items():
        if str(key).lower() in {'classification', 'verdict', 'assessment', 'disposition', 'decision', 'effective_change', 'change', 'result', 'conclusion'} and isinstance(val, str):
            exposed[str(key)] = val[:240]
    return {'source_locator_field_present': source, 'disposition_field_present': disposition, 'assertion_or_consumer_field_present': assertion, 'disposition_text': exposed}


def iter_structured_records(value: Any, pointer: str = '$'):
    if isinstance(value, dict):
        present = []
        for key, item in value.items():
            if str(key).lower() in ISSUE_KEYS:
                present.append((str(key), item))
        direct_text_refs = []
        for key, item in value.items():
            normalized = str(key).lower()
            if normalized in DIRECT_ISSUE_TEXT_KEYS and isinstance(item, str):
                direct_text_refs.append((str(key), item))
        direct_issue_ids = set()
        for _, item in direct_text_refs:
            direct_issue_ids |= set(map(int, ISSUE_MARKER.findall(item)))
        if direct_issue_ids:
            present.append(('direct_issue_text', sorted(direct_issue_ids)))
        # Several reviewed errata batches use a list of result rows with a numeric `id`
        # rather than an `issue` key. Admit that as an issue key only in per-annotation
        # result/selection lists and only when the row also carries a source locator.
        path_parts = pointer.lower().split('/')
        if (
            not present
            and any(part in {'results', 'selected_annotations', 'reviewed_candidates'} for part in path_parts)
            and isinstance(value.get('id'), int)
            and not isinstance(value.get('id'), bool)
            and any(k in value for k in ('source', 'source_location', 'effective_location', 'primary_locator'))
        ):
            present.append(('id', value['id']))
        if present:
            issues: set[int] = set()
            for _, item in present:
                issues |= scalar_issue_values(item)
            if issues:
                # Preserve field names and a compact evidence-shape summary, not arbitrary
                # neighboring text. The human reviewer can open the cited artifact.
                keys = sorted(map(str, value.keys()))
                yield pointer, issues, keys, record_shape(value), [key for key, _ in present]
        for key, item in value.items():
            yield from iter_structured_records(item, pointer + '/' + str(key).replace('~', '~0').replace('/', '~1'))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from iter_structured_records(item, pointer + '/' + str(index))


def text_mentions(value: Any, pointer: str = '$'):
    if isinstance(value, str):
        yield pointer, set(map(int, ISSUE_MARKER.findall(value)))
    elif isinstance(value, dict):
        for key, item in value.items():
            yield from text_mentions(item, pointer + '/' + str(key).replace('~', '~0').replace('/', '~1'))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from text_mentions(item, pointer + '/' + str(index))


def evidence_files() -> list[tuple[str, Path, str]]:
    files = []
    for path in sorted(SHARDS.rglob('*')):
        if path.is_file() and 'errata_disposition_crosswalk_current' not in path.parts and path.name in {'features.json', 'decisions.json', 'findings.txt', 'parent-review.json'}:
            files.append((str(path), path, 'committed_current_shard'))
    # Global audit outputs can contain reviewed findings and gate/test evidence. They are
    # separate references; their EC3-number mentions are not treated as issue disposition.
    for name in ('findings.json', 'expected-failures.json', 'gate-results.json', 'decisions.json'):
        path = Path('doc/reviews/2038-specification-audit') / name
        if path.exists():
            files.append((str(path), path, 'committed_global_audit'))
    return files


def main() -> None:
    inventory, source_hash = source_inventory()
    early_index = json.loads(Path('doc/reviews/2038-specification-audit/shards/errata_early_residual_current/issues.json').read_text())
    late_index = json.loads(Path('doc/reviews/2038-specification-audit/shards/errata_late_residual_current/issue_coverage.json').read_text())
    early_rows = {row['issue_id']: row for row in early_index['issues']}
    late_rows = {row['issue']: row for row in late_index['issues']}
    early_ids, late_ids, primary_ids = set(early_rows), set(late_rows), set(inventory)
    index_union = early_ids | late_ids
    evidence: dict[int, dict[str, list[dict[str, Any]]]] = {
        issue: {'structured_records': [], 'literal_mentions': []} for issue in inventory
    }
    files = evidence_files()
    file_hashes = []
    file_counts = Counter()
    artifact_ids = {display: index for index, (display, _, _) in enumerate(files)}
    for display, path, provenance in files:
        raw = path.read_text(errors='replace')
        file_hashes.append({'path': display, 'sha256': hashlib.sha256(raw.encode()).hexdigest(), 'provenance': provenance})
        file_counts[provenance] += 1
        try:
            parsed = json.loads(raw) if path.suffix == '.json' else raw
        except json.JSONDecodeError:
            parsed = raw
        for pointer, ids, keys, shape, ref_fields in iter_structured_records(parsed):
            for issue in sorted(ids & inventory.keys()):
                evidence[issue]['structured_records'].append({
                    'artifact': display,
                    'provenance': provenance,
                    'pointer': pointer,
                    'record_keys': keys,
                    'issue_reference_fields': ref_fields,
                    **shape,
                })
        # Exact #ID matches are discovery only. Do not count numeric JSON values as prose.
        for pointer, ids in text_mentions(parsed):
            for issue in sorted(ids & inventory.keys()):
                evidence[issue]['literal_mentions'].append({
                    'artifact': display,
                    'provenance': provenance,
                    'pointer': pointer,
                })

    rows = []
    for issue, source in inventory.items():
        ev = evidence[issue]
        structured = ev['structured_records']
        mentions = ev['literal_mentions']
        mention_groups: dict[tuple[str, str], list[str]] = {}
        for mention in mentions:
            mention_groups.setdefault((mention['artifact'], mention['provenance']), []).append(mention['pointer'])
        compact_mentions = [
            {'artifact_id': artifact_ids[artifact], 'count': len(pointers), 'sample_pointers': pointers[:1]}
            for (artifact, provenance), pointers in sorted(mention_groups.items())
        ]
        candidate_shapes = [x for x in structured if x['source_locator_field_present'] and x['disposition_field_present']]
        if any(x['assertion_or_consumer_field_present'] for x in candidate_shapes):
            automated_class = 'source_disposition_assertion_candidate'
        elif candidate_shapes:
            automated_class = 'source_disposition_candidate'
        elif structured:
            automated_class = 'structured_issue_reference_only'
        elif mentions:
            automated_class = 'literal_mentions_only'
        else:
            automated_class = 'no_exact_issue_reference_found'
        provenance_counts = Counter(x['provenance'] for x in structured + mentions)
        early = early_rows.get(issue)
        late = late_rows.get(issue)
        rows.append({
            'issue': issue,
            'primary_source': source,
            'automated_class': automated_class,
            'evidence': {'structured_records': [
                {
                    'artifact_id': artifact_ids[x['artifact']],
                    'pointer': x['pointer'],
                    'issue_reference_fields': x['issue_reference_fields'],
                    'source_locator_field_present': x['source_locator_field_present'],
                    'disposition_field_present': x['disposition_field_present'],
                    'assertion_or_consumer_field_present': x['assertion_or_consumer_field_present'],
                    'disposition_text': x['disposition_text'],
                }
                for x in structured
            ], 'literal_mentions_by_artifact': compact_mentions},
            'provenance_counts': dict(sorted(provenance_counts.items())),
            'prior_issue_index_links': {
                'early_issue_inventory': None if early is None else {
                    'physical_pages': early.get('physical_pages', []),
                    'ledger_citation_count': early.get('ledger_citation_count', 0),
                    'current_shard_citation_count': len(early.get('current_shard_citations', [])),
                    'note': 'Citation/index metadata only; no semantic coverage implied.'
                },
                'late_issue_coverage_index': None if late is None else {
                    'physical_pages': late.get('physical_pages', []),
                    'review_issue_reference_count': len(late.get('review_issue_references', [])),
                    'implementation_issue_reference_count': len(late.get('implementation_issue_references', [])),
                    'note': 'Citation/index metadata only; no semantic coverage implied.'
                },
            },
            'semantic_disposition': 'not_assessed_by_script',
        })

    counts = Counter(row['automated_class'] for row in rows)
    result = {
        'title': 'EC3 issue-ID to current audit evidence discovery crosswalk',
        'baseline': 'b901326c',
        'source': {'path': str(SOURCE), 'sha256': source_hash, 'issue_universe_method': 'Unique #N markers in primary PDF annotation /Subj fields, enumerated directly with pikepdf.'},
        'prior_inventory_reconciliation': {
            'early_inventory_path': 'doc/reviews/2038-specification-audit/shards/errata_early_residual_current/issues.json',
            'early_inventory_id_count': len(early_ids),
            'late_inventory_path': 'doc/reviews/2038-specification-audit/shards/errata_late_residual_current/issue_coverage.json',
            'late_inventory_id_count': len(late_ids),
            'intersection_count': len(early_ids & late_ids),
            'union_count': len(index_union),
            'primary_subj_scan_count': len(primary_ids),
            'ids_in_primary_scan_but_not_prior_union': [
                {'issue': i, 'physical_pages': inventory[i]['physical_pages']}
                for i in sorted(primary_ids - index_union)
            ],
            'qualification': 'This reconciles two prior page-range issue inventories against the primary /Subj scan. It does not contradict the committed literal_issue_candidates.json total; that artifact also reports the 360-ID /Subj universe. The four differences are multi-ID subject markers omitted from the prior per-issue indexes.',
        },
        'evidence_scope': {
            'committed_current_shards': str(SHARDS),
            'committed_global_audit_files': ['findings.json', 'expected-failures.json', 'gate-results.json', 'decisions.json'],
            'files_scanned': len(files),
            'files_by_provenance': dict(sorted(file_counts.items())),
            'manifest_sha256': hashlib.sha256(json.dumps(file_hashes, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
            'artifact_map': [{'artifact_id': artifact_ids[path], 'path': path, 'provenance': provenance} for path, _, provenance in files],
        },
        'summary': {
            'primary_issue_ids': len(rows),
            'automated_classes': dict(sorted(counts.items())),
            'structured_records': sum(len(x['evidence']['structured_records']) for x in rows),
            'structured_record_shape_candidates': dict(sorted(Counter(
                row['automated_class'] for row in rows
            ).items())),
            'literal_mentions': sum(sum(y['count'] for y in x['evidence']['literal_mentions_by_artifact']) for x in rows),
            'semantic_coverage_claimed': False,
            'interpretation_limits': [
                'Issue ID mention is not proof of source/effective-rule review.',
                'Structured issue fields are leads; field presence and test citations still require human source/assertion review.',
                'No match does not establish missing behavior coverage or missing feature implementation.',
                'The issue universe is defined here by unique #N markers in primary-PDF annotation /Subj fields; /Contents-only and /T-only issue markers are outside this census method.',
            ],
        },
        'issues': rows,
    }
    OUT.write_text(json.dumps(result, ensure_ascii=False, separators=(',', ':')) + '\n')
    print(json.dumps(result['summary'], indent=2))


if __name__ == '__main__':
    main()
