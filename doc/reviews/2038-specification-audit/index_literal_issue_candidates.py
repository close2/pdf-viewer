"""Find EC3 issue labels without literal #ID citations in selected current audit prose.

Run from the audit worktree. The output is discovery metadata, not semantic coverage.
This script deliberately does not store annotation contents in the output.
"""
import json
import hashlib
import re
from pathlib import Path
import pikepdf

root = Path('doc/reviews/2038-specification-audit/shards')
paths = sorted(p for p in root.rglob('*') if p.name in {
    'features.json', 'decisions.json', 'findings.txt', 'parent-review.json'
})
text = '\n'.join(p.read_text() for p in paths)
source_hash = hashlib.sha256(Path('doc/ISO_32000-2_sponsored_EC3.pdf').read_bytes()).hexdigest()
assert source_hash == 'c7d4ec236794558d48205394d499a9791575b5f5794e39377382e633846a6f80'
issues = {}
with pikepdf.Pdf.open('doc/ISO_32000-2_sponsored_EC3.pdf') as pdf:
    for physical_page, page in enumerate(pdf.pages, 1):
        for annotation in page.get('/Annots', []):
            for issue in re.findall(r'#(\d+)', str(annotation.get('/Subj', ''))):
                item = issues.setdefault(int(issue), {'pages': set(), 'objects': set()})
                item['pages'].add(physical_page)
                item['objects'].add(annotation.objgen)
candidates = []
for issue, item in sorted(issues.items()):
    if not re.search(r'#' + str(issue) + r'(?!\d)', text):
        candidates.append({
            'issue': issue,
            'physical_pages': sorted(item['pages']),
            'annotation_objects': [list(obj) for obj in sorted(item['objects'])],
        })
result = {
    'source': 'doc/ISO_32000-2_sponsored_EC3.pdf',
    'source_sha256': 'c7d4ec236794558d48205394d499a9791575b5f5794e39377382e633846a6f80',
    'method': 'Issue IDs in annotation Subj; exact literal #ID absent from selected current shard prose filenames. This can miss semantic ownership, numeric JSON issue fields, alternative citation styles and prior-round evidence. It is not a count of unimplemented or unreviewed features.',
    'selected_prose_files': len(paths),
    'selected_prose_sha256': hashlib.sha256(text.encode()).hexdigest(),
    'unique_issue_ids': len(issues),
    'candidate_count': len(candidates),
    'candidates': candidates,
}
output = Path('scratchpad/r2038/spec-audit/literal_issue_candidates.json')
output.write_text(json.dumps(result, indent=2) + '\n')
print(f'{len(issues)} distinct issue IDs; {len(candidates)} literal-citation candidates')
