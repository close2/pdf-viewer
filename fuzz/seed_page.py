#!/usr/bin/env python3
"""Seed a whole-document fuzz corpus with real documents.

    find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
      | python3 fuzz/seed_page.py <target> fuzz/corpus/<target> [--every] -

`find` rather than a glob because the SafeDocs cache is three directories deep and the pdf.js
submodule is a fourth population; the same command with `fuzz/corpus/document` as the destination
seeds that target, which is worth 3010 -> 4351 covered edges on its own (ADR 0264).

**Why a script and not a checked-in directory.** `fuzz/corpus` is gitignored by policy — the
corpora are large and machine-generated — so a seeded target needs a *recipe*, which the `sfnt`,
`xmp`, `confined_wire` and `x509` targets each have. This is that recipe for `page`, and for
`document`, `crypt` and `serialize`, whose input is a whole file in exactly the same sense —
`serialize` most sharply of the four, because it returns at its first `Document::open`
failure and so reaches the writer only on an input that was already a document.

**Why it matters more here than anywhere else.** A from-scratch input reaches
`pdf_model::interpret` only by inventing a header, a page tree, a content stream and a resource
dictionary that agree with each other, which libFuzzer will not do in any number of runs this
machine has time for. Seeded with a real document, every one of those is already true and the
mutations land on the object graph — which is where the four-hundred-and-twenty-fifth session's
crasher lived. This is `sfnt`'s lesson (ADR 0175) applied one layer up.

**The ceiling is the target's own.** `page.rs` refuses an input past 256 KiB, so a seed past it
would be copied, mutated and thrown away every time. Seeds are named by SHA-256 so that a re-run
adds only what is new, and the census below is printed rather than assumed: a corpus of documents
that state no shading would seed nothing about §8.7.4.5.

**One seed per shape** (ADR 1571). Written whole, the documents under the ceiling are some 47 000
files and 4.5 GB per target, and a target loads all of it before its first mutation. `lexer`
tokenises bytes and `object` parses one object at a time, so what their branches turn on is which
lexical forms and which object forms the bytes use; `document`, `serialize` and `linearize` open a
file and walk or rewrite its structure, so what theirs turn on is the file's structure — its
cross-reference form, its object streams, its encryption, its filters, whether it was cut short,
the kinds of object it holds. Each keeps the smallest seed of each shape. `page` interprets the content streams, which are compressed and not read here, so its shape is the
raw bytes' account of which machinery of clauses 8 and 11 a document reaches. `--every` writes the
population whole, which is what the proof in ADR 1571 compares against, and that ADR has what each
shape keeps and what it costs.

`object` takes **objects rather than documents**: its target parses from the first byte and stops
at its first error, and a whole document's first error is its first `obj` keyword, so every byte
after `%PDF-1.x 1 0` was a byte the target never read. Its seeds are the bodies between each
`obj` and `endobj`, the first `MAX_OBJECTS` of each document.
"""

import os
import re
import sys

from seed_shape import Smallest, bucket, mapped, sha256

# `page.rs`'s `MAX_INPUT`, restated so the two cannot disagree silently.
MAX_INPUT = 256 * 1024
# How many of a document's objects `object` reads for its seeds, and the longest body it keeps:
# the shapes come from the first objects of each of tens of thousands of documents, and a body
# past this is a stream whose bytes the target copies rather than parses.
MAX_OBJECTS = 256
MAX_OBJECT = 16 * 1024

# What a seed is worth to a target that interprets a page, asked of the raw bytes. Crude on
# purpose: a construct inside an object stream is missed, so every count here is a lower bound
# and the script says so rather than pretending to be a parser.
CONSTRUCTS = {
    "/Shading": re.compile(rb"/Shading|/ShadingType"),
    "/Pattern": re.compile(rb"/PatternType"),
    "/Function": re.compile(rb"/FunctionType"),
    "form XObject": re.compile(rb"/Subtype\s*/Form"),
    "image XObject": re.compile(rb"/Subtype\s*/Image"),
    "/SMask": re.compile(rb"/SMask"),
    "/Group": re.compile(rb"/Type\s*/Group|/S\s*/Transparency"),
    "/Annots": re.compile(rb"/Annots"),
    "/OCProperties": re.compile(rb"/OCProperties"),
    "an embedded font": re.compile(rb"/FontFile[23]?"),
}

REGULAR = rb"[^\s/\[\]<>(){}%]"

# §7.2's lexical forms, each a branch of `pdf_syntax::Lexer`: the escapes of a literal string
# (§7.3.4.2), a name's `#` (§7.3.5), a hexadecimal string's odd digit and white space (§7.3.4.3),
# the forms of a number (§7.3.3), the three end-of-line markers and the delimiters a token can
# stop on. The shape is the set a document uses.
LEXICAL = {
    name: re.compile(pattern)
    for name, pattern in {
        "comment": rb"%",
        "name escape": rb"/" + REGULAR + rb"*#[0-9A-Fa-f]{2}",
        "bad name escape": rb"/" + REGULAR + rb"*#(?![0-9A-Fa-f]{2})",
        "long name": rb"/" + REGULAR + rb"{128}",
        "escape": rb"\\[nrtbf]",
        "escaped delimiter": rb"\\[()\\]",
        "octal": rb"\\[0-7]{1,3}",
        "unknown escape": rb"\\[^nrtbf()\\0-7\r\n]",
        "continuation": rb"\\\r?\n",
        "nested parenthesis": rb"\([^()\\]*\(",
        "hexadecimal": rb"<[0-9A-Fa-f]",
        "odd hexadecimal": rb"<(?:[0-9A-Fa-f]{2})*[0-9A-Fa-f]>",
        "spaced hexadecimal": rb"<[0-9A-Fa-f]+\s+[0-9A-Fa-f][0-9A-Fa-f\s]*>",
        "real": rb"\d\.\d",
        "leading point": rb"[\s\[(]\.\d",
        "trailing point": rb"\d\.[\s\]/]",
        "sign": rb"[\s\[][+-]\d",
        "double sign": rb"[+-][+-]\d",
        "long number": rb"\d{20}",
        "brace": rb"[{}]",
        "carriage return alone": rb"\r(?!\n)",
        "form feed": rb"\x0c",
        "nul": rb"\x00",
        "high byte": rb"[\x80-\xff]",
        "unbalanced close": rb"\s\)",
        "single angle": rb"[^<>]>[^>]",
    }.items()
}

# §7.3's object forms, each a branch of `pdf_syntax::Parser`, over the part of an object before
# any `stream` keyword: which kinds of object it nests, how a reference and a stream's length are
# written, and the limits `object.rs` sets tighter than the default so a short input reaches them.
GRAMMAR = {
    name: re.compile(pattern)
    for name, pattern in {
        "dictionary": rb"<<",
        "empty dictionary": rb"<<\s*>>",
        "array": rb"\[",
        "empty array": rb"\[\s*\]",
        "nested": rb"<<[^>]*<<|\[[^\]]*\[|\[[^\]]*<<",
        "reference": rb"\d+\s+\d+\s+R\b",
        "integer": rb"(?<![\d.])\d+(?![\d.])",
        "real": rb"\d\.\d|(?<!\d)\.\d",
        "negative": rb"-\d",
        "name": rb"/" + REGULAR,
        "empty name": rb"/[\s/\[\]<>]",
        "name escape": rb"#[0-9A-Fa-f]{2}",
        "literal": rb"\(",
        "string escape": rb"\\",
        "hexadecimal": rb"<[0-9A-Fa-f\s]+>",
        "boolean": rb"\b(?:true|false)\b",
        "null": rb"\bnull\b",
        "stream": rb"\bstream\b",
        "indirect length": rb"/Length\s+\d+\s+\d+\s+R",
        "comment": rb"%",
        "stray keyword": rb"(?<![/\w#])(?!(?:true|false|null|R|stream|endstream)\b)[A-Za-z]{2,}\b",
    }.items()
}

OBJECT = re.compile(rb"\d+\s+\d+\s+obj\b")
# The keys whose presence says what kind of object a body is.
KINDS = re.compile(rb"/(Type|Subtype|Filter|Length|Kids|Count|Parent|Font|Resources)\b")


def filters(data):
    """Every filter name a document's stream dictionaries state in the clear."""
    out = set()
    for one, many in re.findall(rb"/Filter\s*(?:/(\w+)|\[([^\]]*)\])", data):
        out.update([one] if one else re.findall(rb"/(\w+)", many))
    return frozenset(out)


def structure(data):
    """A file's structure, as §7.5 states it: the version its header names, how many
    `startxref`s it carries (§7.5.6's updates), a cross-reference table, a cross-reference
    stream and the hybrid's `/XRefStm` (§7.5.8.4), object streams (§7.5.7), encryption, Annex F's
    parameter dictionary, whether it ends in `%%EOF` (a file cut short is the recovery scanner's),
    and the filters its streams name; and the `/Type` and `/Subtype` names its objects state.

    `Document::open` branches on every one of these, and so does the writer that copies a file's
    objects; the kinds of object a file holds are what the page-tree walk and the copy branch on,
    and the proof in ADR 1571 found files of one structure apart on them. The values inside the
    objects are the walk's data."""
    version = re.search(rb"%PDF-(\d\.\d)", data[:1024])
    return (
        version.group(1) if version else None,
        min(data.count(b"startxref"), 3),
        bool(re.search(rb"(?:^|[\r\n\s])xref\s", data)),
        bool(re.search(rb"/Type\s*/XRef\b", data)),
        data.find(b"/XRefStm") >= 0,
        bool(re.search(rb"/Type\s*/ObjStm\b", data)),
        data.find(b"/Encrypt") >= 0,
        data.find(b"/Linearized") >= 0,
        data.rstrip()[-5:] == b"%%EOF",
        filters(data),
        frozenset(re.findall(rb"/Type\s*/(\w+)", data)),
        frozenset(re.findall(rb"/Subtype\s*/(\w+)", data)),
    )


def pages(data):
    """The largest `/Count` a document states, as its power of two: Annex F's plan has a hint
    table row per page and `linearize.rs` plans at most 64, so the page count's order is a branch."""
    counts = [int(found) for found in re.findall(rb"/Count\s+(\d{1,9})\b", data)]
    return bucket(max(counts, default=0))


def lexical(data):
    """The set of `LEXICAL`'s forms a document uses."""
    return frozenset(name for name, pattern in LEXICAL.items() if pattern.search(data))


def objects(data):
    """The bodies of a document's first `MAX_OBJECTS` objects, each between `obj` and `endobj`."""
    for count, found in enumerate(OBJECT.finditer(data)):
        if count >= MAX_OBJECTS:
            return
        end = data.find(b"endobj", found.end(), found.end() + MAX_OBJECT)
        if end > 0:
            body = data[found.end():end].strip()
            if body:
                yield body


def grammar(body):
    """An object's shape: the set of `GRAMMAR`'s forms the part before `stream` uses, its
    length's order, because `object.rs`'s limits — 1024 elements, 256 entries, depth 32 — are
    reached by length, the lexical forms it uses, and which of the keys that name what an object
    is it states, which the proof in ADR 1571 found objects of one grammar apart on."""
    head = body.split(b"stream", 1)[0]
    return (
        frozenset(name for name, pattern in GRAMMAR.items() if pattern.search(head)),
        bucket(len(head)),
        lexical(body),
        frozenset(re.findall(KINDS, body)),
    )


def page_shape(data):
    """A document's shape for `page`: its `structure`, which of `CONSTRUCTS` it states, and the
    names and numbers that choose a branch of clauses 8 and 11 — its colour spaces (§8.6), blend
    modes (§11.3.5), shading, function and pattern types (§8.7, §7.10) and the `/S` names of its
    soft masks, groups and actions.

    The content streams `pdf_model::interpret` runs are compressed and not read here, so this is
    the raw bytes' account of which machinery a page reaches; the proof in ADR 1571 measured what
    it keeps of the population written whole."""
    names = lambda key: frozenset(re.findall(rb"/" + key + rb"\s*/(\w+)", data))
    numbers = lambda key: frozenset(re.findall(rb"/" + key + rb"\s+(\d)", data))
    return (
        structure(data),
        frozenset(name for name, pattern in CONSTRUCTS.items() if pattern.search(data)),
        names(b"ColorSpace"),
        names(b"BM"),
        numbers(b"ShadingType"),
        numbers(b"FunctionType"),
        numbers(b"PatternType"),
        names(b"S"),
    )


# Each target's seeds out of one document, as `(shape, seed)` pairs.
SEEDS = {
    "page": lambda data: [(page_shape(data), data)],
    "lexer": lambda data: [(lexical(data), data)],
    "object": lambda data: [(grammar(body), body) for body in objects(data)],
    "document": lambda data: [(structure(data), data)],
    "serialize": lambda data: [(structure(data), data)],
    "linearize": lambda data: [(structure(data) + (pages(data),), data)],
}
TARGETS = tuple(SEEDS)


def write(directory, seed):
    """Writes `seed` under its SHA-256; whether it was new."""
    path = os.path.join(directory, sha256(seed))
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(seed)
    return 1


def paths(arguments):
    """The documents named, with `-` standing for a NUL-separated list on standard input, so the
    whole population is one process and one choice of the smallest of each shape."""
    for argument in arguments:
        if argument != "-":
            yield argument
            continue
        for name in sys.stdin.buffer.read().split(b"\0"):
            if name:
                yield os.fsdecode(name)


def main(argv):
    every = "--every" in argv
    argv = [argument for argument in argv if argument != "--every"]
    if len(argv) < 4 or argv[1] not in TARGETS:
        sys.exit(__doc__)
    target, directory = argv[1], argv[2]
    os.makedirs(directory, exist_ok=True)
    whole = every
    chosen = Smallest()
    skipped = written = 0
    census = dict.fromkeys(CONSTRUCTS, 0)
    for path in paths(argv[3:]):
        try:
            if not 0 < os.path.getsize(path) <= MAX_INPUT:
                skipped += 1
                continue
            with mapped(path) as data:
                data = bytes(data)
        except OSError as error:
            print(f"{path}: {error}", file=sys.stderr)
            continue
        if whole:
            # Written as it is read: the population whole is gigabytes, and holding it until the
            # end would be the memory `Smallest` exists to avoid.
            for _, seed in SEEDS[target](data):
                written += write(directory, seed)
        else:
            for shape, seed in SEEDS[target](data):
                chosen.offer(shape, seed)
        for construct, pattern in CONSTRUCTS.items():
            if pattern.search(data):
                census[construct] += 1
    written += chosen.write(directory)
    kept = "every seed" if whole else chosen.summary()
    print(f"seed_page.py {target}: {kept}, {written} new seeds in {directory}, "
          f"{skipped} documents past {MAX_INPUT} bytes")
    print("what the documents read state, counted in the raw bytes (a lower bound — an object")
    print("stream hides its members from a regular expression):")
    for construct, count in sorted(census.items(), key=lambda item: -item[1]):
        print(f"  {count:5}  {construct}")


if __name__ == "__main__":
    main(sys.argv)
