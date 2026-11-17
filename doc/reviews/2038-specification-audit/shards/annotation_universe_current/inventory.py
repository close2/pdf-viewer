#!/usr/bin/env python3
"""Reconcile EC3 annotation populations without copying annotation text.

Run at the audit repository root with pikepdf. Hash-like markers outside /Subj
are intentionally candidates, not issue IDs: inspect their contexts separately.
"""
import collections
import hashlib
import json
import re
from pathlib import Path
import pikepdf

source = Path('doc/ISO_32000-2_sponsored_EC3.pdf')
marker = re.compile(r'#\s*(\d+)')
rows = []
with pikepdf.Pdf.open(source) as pdf:
    for page, node in enumerate(pdf.pages, 1):
        for annotation in node.get('/Annots', []):
            fields = {key: str(annotation.get(key, '')) for key in ['/Subj', '/Contents', '/T']}
            ids = {key: sorted(set(map(int, marker.findall(value)))) for key, value in fields.items()}
            rows.append(dict(page=page, object=list(annotation.objgen), subtype=str(annotation.get('/Subtype', '')),
                             markers=ids, contents_sha256=hashlib.sha256(fields['/Contents'].encode()).hexdigest(),
                             contents_length=len(fields['/Contents']),
                             reply_to=list(annotation['/IRT'].objgen) if '/IRT' in annotation else None))
by_object = {tuple(row['object']): row for row in rows}
for row in rows:
    current = row
    seen = set()
    inherited = set()
    while current is not None and tuple(current['object']) not in seen:
        seen.add(tuple(current['object']))
        inherited.update(value for values in current['markers'].values() for value in values)
        current = by_object.get(tuple(current['reply_to'])) if current['reply_to'] else None
    row['associated_markers'] = sorted(inherited)
subjects = {value for row in rows for value in row['markers']['/Subj']}
all_fields = {value for row in rows for values in row['markers'].values() for value in values}
unassigned = [row for row in rows if not row['associated_markers'] and row['subtype'] not in ['/Popup','/Link']]
extra = all_fields - subjects
result = dict(source=str(source), source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
              annotation_count=len(rows), subtype_counts=dict(sorted(collections.Counter(row['subtype'] for row in rows).items())),
              subject_marker_count=len(subjects), all_field_marker_count=len(all_fields),
              extra_marker_candidates=sorted(extra),
              extra_marker_occurrences=[row for row in rows if any(extra.intersection(values) for values in row['markers'].values())],
              unassigned_nonlink_nonpopup_counts=dict(sorted(collections.Counter(row['subtype'] for row in unassigned).items())),
              unassigned_nonlink_nonpopup=unassigned,
              limits='Marker counts are not semantic coverage. Links, widget navigation and review-state annotations are not automatically new specification features. Contents-only markers need contextual classification; raw annotation text is not copied.')
Path(__file__).with_name('inventory.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({key:result[key] for key in ['annotation_count','subtype_counts','subject_marker_count','extra_marker_candidates','unassigned_nonlink_nonpopup_counts']},indent=2))
