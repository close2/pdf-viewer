"""Extract a specification PDF under `doc/` into Markdown beside `doc/md/`.

The tree's specifications arrive as PDFs and are read as Markdown: `doc/md/` is where
`tools/conformance` looks for ISO 32000-2's own words, and where a person greps. Two more
specifications arrived in the nine-hundred-and-fortieth session — ISO 32000-1:2008, which
ISO 19005-2 makes its base document, and ISO/IEC 15444-1:2000, which ISO 19005-2 §6.2.8.3's
JPEG 2000 rules rest on — and both need the same treatment, so it is a command rather than
two one-off transcriptions.

# It reads them with this project's own reader

`quorra-retrieve` is the whole extraction. That matters for more than tidiness: both documents
are encrypted, one of them forbids extraction through its `/P` flags, and `CLAUDE.md`
principle 3's second half is why this program reads it anyway — a document's restrictions are
the reader's to set, and this program is the reader's.

Where a document carries §14.7's structure tree, the tree's own order is taken; where it does
not, the page's content order is, and `--no-artifacts` drops the running heads and folios that
would otherwise land in the middle of a sentence. Which of the two was used is written into the
file's header, because they are not equally trustworthy and a reader quoting from the result
should know which they have.

# What this does not do

It does not put the output anywhere a gate reads. `conformance::STANDARD` names exactly one
file — ISO 32000-2's — so nothing here can be mistaken for the standard this project is
written against. And it copies nothing into the repository: `doc/md` and `doc/*.pdf` are both
ignored, for ADR 0187's reason. Free to obtain is not free to redistribute.

    python3 tools/spec-md.py "doc/ISO_IEC 15444.pdf"
    python3 tools/spec-md.py doc/PDF32000_2008.pdf --out doc/md/ISO_32000-1_2008.md
"""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

# Control characters and the invisible spacing a producer leaves in a heading's numbering,
# which would otherwise sit inside a sentence somebody tries to match.
NOISE = re.compile('[\x00-\x08\x0b\x0c\x0e-\x1f​-‍⁠﻿\xad]')


def retrieve():
    """The path to `quorra-retrieve`, built if it is not built already."""
    target = pathlib.Path(json.loads(subprocess.run(
        ['cargo', 'metadata', '--format-version', '1', '--no-deps'],
        cwd=ROOT, capture_output=True, check=True).stdout)['target_directory'])
    binary = target / 'release/quorra-retrieve'
    if not binary.exists():
        subprocess.run(['cargo', 'build', '--release', '-p', 'pdf-retrieve'], cwd=ROOT, check=True)
    return binary


def ask(binary, question, pdf, *rest):
    """One question, answered as the JSON the tool prints."""
    out = subprocess.run([str(binary), question, str(pdf), *rest],
                         capture_output=True, check=True).stdout
    return json.loads(out)


def clean(text):
    """One page's readback, with the noise out and the page's own line breaks kept.

    Line breaks are *not* collapsed here, unlike `tools/pdfa-text.py`'s structure-driven
    extraction: without a structure tree there is nothing to say where a paragraph ends, and
    joining lines on a guess would run a clause number into the sentence above it.
    """
    return NOISE.sub('', text).replace('\xa0', ' ')


def convert(binary, pdf, out):
    """Writes one specification's text, and returns (path, pages, words, order)."""
    # One invocation for the whole document, not one per page: `quorra-retrieve` opens and
    # parses the file each time it is asked, and a 756-page specification asked page by page
    # spends its whole cost on re-reading the cross-reference table 756 times.
    read = ask(binary, 'text', pdf, '--no-artifacts', '--logical')
    pages = len(read)
    body, orders = [], set()
    for index, page in enumerate(read):
        if page is None:
            continue
        orders.add(page['order'])
        label = page['label']
        # The page marker is what makes the result citable back to the PDF. It is an HTML
        # comment so that it is invisible where the Markdown is rendered, and on its own line
        # so that it never lands inside a sentence somebody is matching.
        body.append(f'<!-- page {index + 1}{f" ({label})" if label else ""} -->')
        body.append(clean(page['text']))
    order = 'the structure tree' if orders == {'logical'} else (
        "the page's content order" if orders == {'content'} else 'a mixture of both orders')
    header = [
        f'# {pdf.name}',
        '',
        f'Extracted from `{pdf.name}` by `tools/spec-md.py`, which reads it with this project'
        f"'s own `quorra-retrieve`. The text below is in **{order}**, with §14.8.2.2's artifacts"
        ' — running heads, folios, the stamp a sponsored copy carries on every page — dropped.',
        '',
        '**Not redistributable.** `doc/md` is ignored for ADR 0187\'s reason: free to obtain is'
        ' not free to redistribute.',
        '',
        '---',
        '',
    ]
    text = '\n'.join(header + body).rstrip() + '\n'
    out.write_text(text, encoding='utf-8')
    return out, pages, len(text.split()), order


def main():
    arguments = [argument for argument in sys.argv[1:] if not argument.startswith('--')]
    if not arguments:
        raise SystemExit('usage: spec-md.py <doc/something.pdf> [--out doc/md/name.md]')
    pdf = pathlib.Path(arguments[0])
    if not pdf.is_absolute():
        pdf = ROOT / pdf
    if not pdf.is_file():
        raise SystemExit(f'{pdf}: not a file')
    named = next((argument for argument in sys.argv[1:]
                  if argument.startswith('--out=')), None)
    if named:
        out = ROOT / named.removeprefix('--out=')
    elif '--out' in sys.argv[1:]:
        out = ROOT / sys.argv[sys.argv.index('--out') + 1]
    else:
        out = ROOT / 'doc/md' / f'{pdf.stem.replace(" ", "_")}.md'
    out.parent.mkdir(parents=True, exist_ok=True)
    path, pages, words, order = convert(retrieve(), pdf, out)
    print(f'{pdf.name} -> {path.relative_to(ROOT)}: {pages} pages, {words} words, {order}')


if __name__ == '__main__':
    sys.exit(main())
