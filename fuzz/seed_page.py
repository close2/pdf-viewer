#!/usr/bin/env python3
"""Seed a whole-document fuzz corpus with real documents.

    find corpus-cache/safedocs doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \
      | xargs -0 python3 fuzz/seed_page.py fuzz/corpus/page

`find` rather than a glob because the SafeDocs cache is three directories deep and the pdf.js
submodule is a fourth population; the same command with `fuzz/corpus/document` as the destination
seeds that target, which is worth 3010 -> 4351 covered edges on its own (ADR 0264).

**Why a script and not a checked-in directory.** `fuzz/corpus` is gitignored by policy — the
corpora are large and machine-generated — so a seeded target needs a *recipe*, which the `sfnt`,
`xmp`, `confined_wire` and `x509` targets each have. This is that recipe for `page`, and for
`document` and `crypt`, whose input is a whole file in exactly the same sense.

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
"""

import hashlib
import os
import re
import sys

# `page.rs`'s `MAX_INPUT`, restated so the two cannot disagree silently.
MAX_INPUT = 256 * 1024

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


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    written = 0
    skipped = 0
    census = dict.fromkeys(CONSTRUCTS, 0)
    for path in argv[2:]:
        try:
            with open(path, "rb") as handle:
                data = handle.read(MAX_INPUT + 1)
        except OSError as error:
            print(f"{path}: {error}", file=sys.stderr)
            continue
        if len(data) > MAX_INPUT or not data:
            skipped += 1
            continue
        name = hashlib.sha256(data).hexdigest()
        with open(os.path.join(directory, name), "wb") as handle:
            handle.write(data)
        written += 1
        for construct, pattern in CONSTRUCTS.items():
            if pattern.search(data):
                census[construct] += 1
    print(f"{written} document(s) written to {directory}, {skipped} past {MAX_INPUT} bytes")
    print("what the seeds state, counted in the raw bytes (a lower bound — an object")
    print("stream hides its members from a regular expression):")
    for construct, count in sorted(census.items(), key=lambda item: -item[1]):
        print(f"  {count:5}  {construct}")


if __name__ == "__main__":
    main(sys.argv)
