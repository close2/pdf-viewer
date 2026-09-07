"""Extract the English text of the ISO 19005 parts in `doc/pdfa/` into Markdown.

`doc/pdfa/` holds the copies of ISO 19005 the project owner bought (question A16), as the
Institute for Standardization of Serbia's identical reprints: three pages of Serbian
national front matter, then ISO's own English text page for page, then one Serbian back
cover. The whole directory is ignored by `doc/.gitignore`, because the licence on those
files is for one reader — so neither the PDFs nor what this script writes beside them may
be committed. A16 is what permits the conversion: "Feel free to extract the text and create
txt or md files in the pdfa directory."

# It reads them with this project's own reader

`quorra-retrieve structure` is the whole extraction, and everything below is formatting.
That was the owner's suggestion — "maybe our own viewer is suitable for extracting text" —
and it turned out to be the better tool as well as the closer one: both files are AES
encrypted, both are tagged, and §14.7's structure tree says where their paragraphs, lists,
headings and tables are. So the paragraphs here are the *document's own* paragraphs rather
than lines guessed back into shape, and three things a page-based extraction has to filter
never appear at all — the running head, the folio, and the rotated line naming the licensee
— because §14.8.2.5.1 NOTE 3 keeps an untagged artifact out of the logical content order.

Two things are still ours and are named where they happen: which pages are the Serbian
reprint's rather than ISO's ([`national_pages`]), and how a structure element becomes a
Markdown block ([`render`]).

# What it prints

One line per file, and then a **coverage report**: every word of the pages' own readback
that did not reach the Markdown. That is the check worth having — a structure tree can omit
content, and an extraction that silently dropped a clause would look exactly like one that
did not. What is expected there is page furniture and the national front matter; anything
else is a finding, and the report names it rather than a number in a document claiming it
does not exist.

Run it as `python3 tools/pdfa-text.py`. It builds `quorra-retrieve` if it is not built.

`python3 tools/pdfa-text.py --overlap` answers a different question with the same files: **how
much of one part\'s clause 6 is the other part\'s.** It is here because the answer decides how a
validator is built — one requirement table covering both parts, or two — and because a number in
a document would go stale the moment the extraction improved. `CLAUDE.md`\'s rule is that a fact
which can be counted is not written down; this is the command that counts it.
"""
import collections
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
PDFA = ROOT / 'doc/pdfa'

# Word wrote these documents' list markers in Symbol, whose 0xBE is `arrowhorizex` — the
# horizontal extender, which is why it prints as a dash and stands for ISO's own em dash.
# mupdf-style private use code points reach us the same way: unmapped, so the mapping back
# is ours to state rather than the file's.
SYMBOL = {'': '—', '': '•', '': '-', '': ' '}

# Control characters Word leaves in a heading's numbering, and the invisible spacing
# characters that would otherwise sit inside a sentence someone tries to match.
NOISE = re.compile('[\x00-\x08\x0b\x0c\x0e-\x1f​-‍⁠﻿\xad]')

# The Serbian reprint's own pages are the ones with Cyrillic on them; ISO's text has none.
CYRILLIC = re.compile('[Ѐ-ӿ]')

# Which ISO edition the reprint is, in the reprint's own words: `Идентичан са ISO
# 19005-2:2011` on its cover — "identical with". It is the one statement in either file that
# says so, because ISO's own designation is in the running header, and a running header is an
# artifact that §14.8.2.5.1 NOTE 3 keeps out of the tree this reads. Matching the phrase rather
# than the reference is what makes it right: `ISO 19005-\d:\d{4}` on its own also matches the
# reprint's own `SRPS ISO 19005-2:2020` and the `ISO 19005-1:2005` its national foreword cites.
EDITION = re.compile(r'Идентичан са ISO (19005-\d):(\d{4})')

# Front matter ISO sets as a plain paragraph rather than as a heading, promoted so that the
# file has a table of contents a reader can jump around in. The negative lookahead is the
# document's own contents page, whose annex lines are the same words followed by dot leaders
# and a page number: those are a list of the headings and not the headings.
FRONT_MATTER = re.compile(r'^(Foreword|Introduction|Bibliography|Annex [A-Z]\b(?!.*\.\.\.).*)$')

# Roles whose text belongs to the block around them rather than to a block of their own:
# §14.8.4.4's inline-level elements, plus the two grouping types these files use inside a
# paragraph. A `Link`'s text is the footnote marker or the cross-reference it wraps, and
# leaving it out would lose it.
INLINE = {'Span', 'Link', 'Quote', 'Code', 'Reference', 'Em', 'Strong', 'Sub', 'Annot',
          'BibEntry', 'Ruby', 'Warichu', 'RT', 'RB', 'WT', 'WP'}

# Roles that are a paragraph of prose.
PARAGRAPH = {'P', 'Caption', 'Note', 'Title', 'Formula', 'Figure', 'Index'}

HEADING = re.compile(r'^H([1-6])$')


def clean(text):
    """One element's text as a line of prose.

    The readback carries the *page's* line breaks, because a content stream has lines and
    not sentences. Inside one structure element those breaks are layout, so they collapse:
    what is left is the paragraph its author wrote, which is the whole reason this script
    reads the structure tree rather than the page.
    """
    for symbol, replacement in SYMBOL.items():
        text = text.replace(symbol, replacement)
    return re.sub(r'\s+', ' ', NOISE.sub('', text)).strip()


class Node:
    """One structure element, with the text and the elements below it **in order**.

    The order is the whole reason [`Node.parts`] exists rather than a string and a list of
    children: a sentence whose middle is a `Link` is stored as text, child, text, and an
    element that kept its own text in one piece would read the link's words at the end.
    """

    def __init__(self, item=None):
        self.kind = (item or {}).get('type') or ''
        self.stated = (item or {}).get('stated') or ''
        self.page = (item or {}).get('page')
        self.parts = []

    @property
    def children(self):
        return [part for part in self.parts if isinstance(part, Node)]

    def descendants(self):
        for child in self.children:
            yield child
            yield from child.descendants()

    def pages(self):
        """Every page this element's own content items are on."""
        seen = {self.page} if self.page is not None else set()
        for child in self.children:
            seen |= child.pages()
        return seen

    def raw(self):
        """This element and everything under it, uncleaned, in the document's own order."""
        return ''.join(part.raw() if isinstance(part, Node) else part for part in self.parts)

    def inline(self):
        """This element and everything under it as one line, in the document's own order.

        Cleaned **once, at the end**: the page's line breaks are what separate two content
        items, so cleaning each one first and joining afterwards runs the last word of one
        into the first word of the next — `ISO` and `19005-2` on the cover became
        `ISO19005-2` while this function did that.
        """
        return clean(self.raw())

    def child(self, kind):
        return next((child for child in self.children if child.kind == kind), None)


def tree(items):
    """The flat walk, back into the tree its depths describe."""
    root = Node()
    stack = [root]
    for item in items:
        depth = item['depth']
        del stack[depth + 1:]
        while len(stack) <= depth:
            stack.append(stack[-1])
        if 'text' in item:
            # A content item: the text belongs to the element it hangs under, in this place
            # in that element's sequence.
            stack[depth].parts.append(item['text'])
            continue
        node = Node(item)
        node.page = item.get('page')
        stack[depth].parts.append(node)
        stack.append(node)
    return root


def national_pages(items):
    """Which pages belong to the Serbian reprint rather than to ISO.

    Decided by the alphabet, which is the one property that separates them: the reprint's
    cover, its copyright page, its national foreword and its back cover are Cyrillic, and
    ISO's own text does not carry a Cyrillic character anywhere. Deciding it by page number
    instead would be a constant that only happens to be right for these two files.
    """
    return {item['page'] for item in items
            if 'text' in item and CYRILLIC.search(item['text'])}


def table(node, out):
    """A `Table` element as a Markdown table.

    §14.8.4.8's `TR` and `TD`/`TH` are the grid, read through whatever `THead`/`TBody`
    grouping the document put between them. **`/ColSpan` and `/RowSpan` are not honoured**:
    Markdown has no spanning cell, so a spanned cell occupies one column here and the file
    says so at the top rather than letting a reader assume the grid is the document's.
    """
    rows = [element for element in node.descendants() if element.kind == 'TR']
    if not rows or not node.inline():
        # A table used for page layout rather than for data — the reprint's cover is one —
        # can end up with every cell empty once the content items are filtered. An empty
        # grid says nothing, so nothing is written.
        return
    grid = []
    for row in rows:
        cells = [cell for cell in row.descendants() if cell.kind in ('TD', 'TH')]
        grid.append([cell.inline().replace('|', r'\|') or ' ' for cell in cells])
    width = max(len(row) for row in grid)
    for row in grid:
        row.extend(' ' for _ in range(width - len(row)))
    out.append('| ' + ' | '.join(grid[0]) + ' |')
    out.append('|' + '---|' * width)
    for row in grid[1:]:
        out.append('| ' + ' | '.join(row) + ' |')
    out.append('')


def listing(node, out, indent=0):
    """An `L` element as a Markdown list, and its `Lbl` as the marker.

    The one case that is not a list at all: ISO numbers its clause headings with a list, so
    `L > LI > (Lbl "6.2.11", LBody > H4)` is a *heading* whose number is the label. Rendering
    that as a bullet would lose every clause number in the document.
    """
    for item in (child for child in node.children if child.kind == 'LI'):
        label = item.child('Lbl')
        marker = label.inline() if label is not None else '—'
        body = item.child('LBody') or item
        blocks = [child for child in body.children if child.kind not in ('Lbl',)]
        heading = next((block for block in blocks if HEADING.match(block.kind)), None)
        if heading is not None:
            out.append('')
            level = int(HEADING.match(heading.kind).group(1))
            out.append('#' * min(level + 1, 6) + f' {marker} {heading.inline()}'.rstrip())
            out.append('')
            for block in blocks:
                if block is not heading:
                    render(block, out)
            continue
        first = True
        for block in blocks:
            if block.kind == 'L':
                listing(block, out, indent + 1)
                continue
            text = block.inline()
            if not text:
                continue
            prefix = f'{marker} ' if first and marker not in ('', '—') else '- '
            out.append('  ' * indent + (prefix if first else '  ') + text)
            first = False
        if first:
            text = body.inline()
            if text:
                out.append('  ' * indent + f'{marker} {text}'.strip())
    out.append('')


def render(node, out):
    """One structure element as Markdown, and its children where it is a container."""
    kind = node.kind
    if kind == 'Table':
        table(node, out)
        return
    if kind == 'L':
        listing(node, out)
        return
    heading = HEADING.match(kind)
    if heading:
        text = node.inline()
        if text:
            out.append('#' * min(int(heading.group(1)) + 1, 6) + ' ' + text)
            out.append('')
        return
    if kind in PARAGRAPH or kind in INLINE:
        text = node.inline()
        if text:
            # ISO sets `Foreword`, `Introduction` and each annex's title as an ordinary
            # paragraph. They are headings to a reader, and promoting them is the one place
            # this script overrules the document's own tagging — deliberately, and only for
            # this closed list of titles.
            out.append(f'## {text}' if FRONT_MATTER.match(text) else text)
            out.append('')
        return
    # A container: its children are blocks, and any text of its own between them is a
    # paragraph the document did not put an element around.
    for part in node.parts:
        if isinstance(part, Node):
            render(part, out)
            continue
        text = clean(part)
        if text:
            out.append(text)
            out.append('')


def readback(binary, pdf, pages, national):
    """Every word ISO's own pages draw, in content order, with §14.8.2.2's artifacts dropped.

    The denominator of the coverage report: what the file draws, against what the structure
    tree reached. Three things are taken out of it, and each is taken out because it is not
    ISO's text rather than to make the number look better — the reprint's national pages,
    the running heads and folios `--no-artifacts` names, and the two rotated lines naming
    the licensee, which are not tagged as artifacts because they are not tagged at all and
    are recognised here by their alphabet.
    """
    words = collections.Counter()
    for index in range(pages):
        if index in national:
            continue
        answer = json.loads(subprocess.run(
            [binary, 'page', str(pdf), str(index), '--no-artifacts'],
            capture_output=True, check=True).stdout)
        for line in answer['text'].splitlines():
            if not CYRILLIC.search(line):
                words.update(clean(line).split())
    return words


def retrieve():
    """The path to `quorra-retrieve`, built if it is not built already."""
    target = pathlib.Path(json.loads(subprocess.run(
        ['cargo', 'metadata', '--format-version', '1', '--no-deps'],
        cwd=ROOT, capture_output=True, check=True).stdout)['target_directory'])
    binary = target / 'release/quorra-retrieve'
    if not binary.exists():
        subprocess.run(['cargo', 'build', '--release', '-p', 'pdf-retrieve'],
                       cwd=ROOT, check=True)
    return binary


def convert(binary, pdf):
    """Writes the Markdown beside one PDF and reports what did not reach it."""
    answer = json.loads(subprocess.run(
        [binary, 'structure', str(pdf)], capture_output=True, check=True).stdout)
    if answer is None:
        raise SystemExit(f'{pdf.name}: states no structure tree; this script has no other way in')
    if answer['truncated']:
        raise SystemExit(f'{pdf.name}: the structure walk hit its bound, so this would be a prefix')
    items = answer['items']

    national = national_pages(items)
    # Searched over the whole document's text joined rather than item by item, because the
    # reprint sets the reference as a link inside the sentence: `Идентичан са ISO ` is one
    # content item and `19005-2:2011` is the next.
    edition = EDITION.search(clean(''.join(item.get('text', '') for item in items)))
    if edition is None:
        raise SystemExit(f'{pdf.name}: states no ISO 19005 edition; is this an ISO 19005 part?')
    part, year = edition.group(1), edition.group(2)

    out = [
        f'# ISO {part}:{year}',
        '',
        'Document management — Electronic document file format for long-term preservation —'
        f' Part {part.rsplit("-", maxsplit=1)[-1]}',
        '',
        f'Extracted from `{pdf.name}` by `tools/pdfa-text.py`, which reads it with this'
        " project's own `quorra-retrieve structure` — so the paragraphs, lists, headings and"
        " tables below are §14.7's structure tree as the document states it, and the running"
        ' heads, folios and the rotated line naming the licensee are absent because'
        ' §14.8.2.5.1 NOTE 3 leaves an untagged artifact out of the logical content order.',
        '',
        'Four departures from the document, all deliberate. The Institute for Standardization'
        f' of Serbia\'s national front and back matter (pages'
        f' {", ".join(str(page + 1) for page in sorted(national))} of the PDF) is left out. A'
        " table's spanning cells are flattened, because Markdown has no spanning cell. The em"
        ' dash ISO uses as a list marker is a Markdown bullet. And the line breaks inside a'
        " paragraph are the page's rather than the author's, so they are collapsed — the words"
        ' are the standard\'s, and the lines are not.',
        '',
        '**This file is licensed to one reader. It is not to be committed or redistributed.**',
        '',
        '---',
        '',
    ]
    # A word break this reader *inferred* rather than read. §14.8.2.6.2 requires a tagged
    # producer to state its word breaks — "any white-space characters that would be present to
    # separate words in a pure text representation shall be present in the tagged PDF
    # representation of the text" — and where one does not, `pdf_model::content` infers a break
    # from the gap between two show operations. A line set with loose tracking can open a gap
    # that wide inside a word, and the result is a word split into single letters. Counted
    # rather than repaired: this is the standard's text, and a script that glued letters back
    # together by guesswork would be editing it.
    marker = len(out)
    # The reprint's own pages go out at the content item, which is where the page is stated;
    # the elements above them are grouping elements shared with ISO's text, and one left with
    # nothing under it renders nothing.
    iso = [item for item in items if 'text' not in item or item['page'] not in national]
    for child in tree(iso).children:
        render(child, out)

    body = '\n'.join(out[marker:])
    # `a`, `A` and `I` are words; every other single letter in an English standard is one
    # this reader split.
    loose = sum(1 for word in body.split()
                if len(word) == 1 and word.isalpha() and word not in ('a', 'A', 'I'))
    out[marker - 2:marker - 2] = [
        f'One number to read this file by: **{loose} of its {len(body.split())} words are a'
        ' single letter that is not a word** — the mark of a word this reader split. §14.8.2.6.2'
        ' asks a tagged producer to state its own word breaks so that a processor need not'
        ' "rely on heuristics based on information such as glyph positioning on the page"; where'
        ' this one did not, the break is inferred from the gap between two show operations, and'
        ' a line set with loose tracking can open that gap inside a word. Nothing here glues'
        ' them back together, because guessing where a word ends would be editing the standard.',
        '',
    ]

    text = re.sub(r'\n{3,}', '\n\n', '\n'.join(out)).rstrip() + '\n'
    destination = PDFA / f'ISO_{part}_{year}.md'
    destination.write_text(text, encoding='utf-8')

    pages = json.loads(subprocess.run(
        [binary, 'document', str(pdf)], capture_output=True, check=True).stdout)['pages']
    missing = readback(binary, pdf, pages, national) - collections.Counter(clean(text).split())
    return destination, len(items), len(text.split()), missing


# A requirement sentence, for the overlap count: `shall` outside a NOTE or an EXAMPLE.
SENTENCE = re.compile(r'(?<=\.)\s+(?=[A-Z(])')


def requirements(part):
    """Every `shall` sentence of one part's clause 6, in order."""
    text = part[part.index('## 6 Technical requirements'):]
    end = re.search(r'\n#*\s*Annex A \(normative\)', text)
    text = text[: end.start()] if end else text
    out = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith(('#', '|', 'NOTE', 'EXAMPLE')):
            continue
        out.extend(s for s in SENTENCE.split(line) if ' shall ' in s)
    return [re.sub(r'\s+', ' ', s).strip() for s in out]


def comparable(sentence):
    """One requirement reduced to what it says, not which part is saying it.

    Three substitutions and then every non-letter dropped. The names of the base standard, of
    the part itself and of the agent it addresses differ between the two parts *by definition*
    — part 2 modifies ISO 32000-1 and speaks of a conforming reader, part 4 modifies
    ISO 32000-2 and speaks of a conforming processor — so leaving them in would count every
    shared rule as different. **Dropping the spaces as well** is not tidiness: ISO 19005-4's
    readback splits words where its producer tracked them (ADR 0921), and a space-sensitive
    comparison would score its sentences as unlike their part 2 twins.
    """
    for pattern, stand_in in (
        (r'ISO 32000-[12](:\d{4})?(:—)?', 'BASE'),
        (r'(this part of ISO 19005|this document)', 'THIS'),
        (r'PDF/A-[24]', 'PDFA'),
        (r'(conforming reader|conforming processor|conforming writer)', 'AGENT'),
    ):
        sentence = re.sub(pattern, stand_in, sentence)
    return re.sub(r'[^a-z]', '', sentence.lower())


def overlap(binary):
    """Prints how much of each part's clause 6 the other part also states.

    Matching is greedy and one-to-one: each of part 2's requirements takes the closest of part
    4's that is still free. Two thresholds, because the two answers differ — a rule stated in
    the same words is a shared *predicate*, and a rule stated in different words is still one
    predicate with two citations.
    """
    import difflib

    parts = {}
    for pdf in sorted(PDFA.glob('*.pdf')):
        answer = json.loads(subprocess.run(
            [binary, 'structure', str(pdf)], capture_output=True, check=True).stdout)
        items = answer['items']
        national = national_pages(items)
        edition = EDITION.search(clean(''.join(item.get('text', '') for item in items)))
        iso = [item for item in items if 'text' not in item or item['page'] not in national]
        out = []
        for child in tree(iso).children:
            render(child, out)
        parts[edition.group(1)] = requirements('\n'.join(out))

    (left, first), (right, second) = sorted(parts.items())
    reduced = [comparable(s) for s in second]
    taken, identical, reworded = set(), 0, 0
    for sentence in first:
        best, at = 0.0, None
        for index, other in enumerate(reduced):
            if index in taken:
                continue
            ratio = difflib.SequenceMatcher(None, comparable(sentence), other).ratio()
            if ratio > best:
                best, at = ratio, index
        if best >= 0.95:
            identical += 1
            taken.add(at)
        elif best >= 0.75:
            reworded += 1
            taken.add(at)
    print(f'ISO {left} clause 6: {len(first)} shall-sentences')
    print(f'ISO {right} clause 6: {len(second)} shall-sentences')
    print(f'  the same rule in the same words: {identical}')
    print(f'  the same rule, reworded:         {reworded}')
    print(f'  only in {left}: {len(first) - identical - reworded}')
    print(f'  only in {right}: {len(second) - len(taken)}')
    print(f'  union, which is what one table would hold: '
          f'{len(first) + len(second) - len(taken)}')


def main():
    binary = retrieve()
    if '--overlap' in sys.argv[1:]:
        return overlap(binary)
    pdfs = sorted(PDFA.glob('*.pdf'))
    if not pdfs:
        raise SystemExit(f'no PDFs in {PDFA}; see doc/questions/A16-buying-iso-19005.md')
    for pdf in pdfs:
        destination, elements, words, missing = convert(binary, pdf)
        print(f'{pdf.name} -> {destination.relative_to(ROOT)}: '
              f'{elements} elements, {words} words')
        print(f'  words the pages draw and the structure tree did not reach: '
              f'{sum(missing.values())}')
        for word, count in missing.most_common(12):
            print(f'    {count:4}  {word!r}')


if __name__ == '__main__':
    sys.exit(main())
