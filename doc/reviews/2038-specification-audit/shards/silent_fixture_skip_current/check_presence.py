"""Read-only reproduction of this bounded fixture-presence check; run at repo root."""
import hashlib
import json
import tomllib
from pathlib import Path

root = Path.cwd()
fixture_paths = [
    'doc/pdf.js/test/pdfs/issue1155.pdf',
    'doc/pdf.js/test/pdfs/issue11555.pdf',
    'doc/pdf.js/test/pdfs/issue1155r.pdf',
    'doc/corpora/pdfbox/pdfbox/src/test/resources/input/PDFBOX-3110-poems-beads.pdf',
    'doc/corpora/format-corpus/pdfCabinetOfHorrors/digitally_signed_3D_Portfolio.pdf',
    'doc/corpora/pdfbox/pdfbox/src/test/resources/input/PDFBOX-5838-0024320-reduced.pdf',
    'doc/corpora/pdfbox/pdfbox/src/test/resources/input/PDFBOX-4322-Empty-ToUnicode-reduced.pdf',
]
tests = [
    ('crates/pdf-model/tests/text_layer.rs', 'invisible_text_is_placed_too'),
    ('crates/viewer-core/tests/headless.rs', 'a_page_that_could_not_be_drawn_whole_says_so'),
]
rows = tomllib.loads((root / 'doc/conformance/ledger.toml').read_text())['clause']
result = {
    'workspace': str(root),
    'scope': 'Selected named fixtures only; presence is separate from test execution.',
    'pdfjs_directory_exists': (root / 'doc/pdf.js/test/pdfs').is_dir(),
    'pdfjs_top_level_pdf_count': len(list((root / 'doc/pdf.js/test/pdfs').glob('*.pdf'))),
    'fixtures': [{'path': name, 'is_file': (root / name).is_file()} for name in fixture_paths],
    'tests': [{
        'path': path, 'name': name,
        'source_sha256': hashlib.sha256((root / path).read_bytes()).hexdigest(),
        'literal_ledger_references': [row['clause'] for row in rows
            if any(name in ref for ref in row.get('test', []))],
    } for path, name in tests],
}
print(json.dumps(result, indent=2))
