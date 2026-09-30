#!/usr/bin/env python3
"""Every superlative the test notes and the ledger's notes scope to a population that grows.

A superlative — the tightest limit this bucket has measured, the largest fall this list has had,
the first document to reach a path — is a claim about every member of a population, and the
population grows after the sentence is written. It decays the way a ledger note's claim does: a
later row overtakes it and nothing points back at the sentence (trap 5's family; the habit in
`doc/habits/the-ledger-and-claims-about-this-tree.md` that an unread claim is edited, never
corrected by a sentence appended after it). ADR 1427.

The population read is the prose a round reads before deciding what a page is:

  oracle.rs, corpus.rs    the doc comments (`///`, `//!`) of `crates/pdf-model/tests/`'s two
                          corpus gates, where every page list carries its diagnosis
  raster_golden           `tests/raster_golden.rs`'s doc comments and the `#` header of its table
                          (the table's columns are hashes; it has no prose column)
  ledger                  every `note = "…"` of `doc/conformance/ledger.toml`

A hit is a sentence holding a superlative word *and* a scope that names the growing population —
"this file", "the list", "has measured", "yet", "so far", "on record", "of any" — because that
pairing is what makes it a claim about members not yet written. A superlative scoped to one
measurement ("the worst tile", "the largest of its three ratios") is a name for a quantity and is
not listed. The numbers each sentence asserts are printed beside it, because the check is to read
each against the table it claims: the same file's later rows, the ledger, `tools/state.sh`.

A sentence that names a *command* instead of a number ("`tools/state.sh` prints the tightest")
defers to that command and stays; it is listed under `defers` so that the count says so.

    tools/superlatives.py            # the counts per source, then every hit
    tools/superlatives.py --count    # the counts alone

It reads and prints; it rewrites nothing. Run with `PYTHONDONTWRITEBYTECODE=1` (trap 69).
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

SOURCES = [
    ("oracle.rs", "crates/pdf-model/tests/oracle.rs", "rust"),
    ("corpus.rs", "crates/pdf-model/tests/corpus.rs", "rust"),
    ("raster_golden", "crates/pdf-model/tests/raster_golden.rs", "rust"),
    ("raster_golden", "crates/pdf-model/tests/raster_golden.tsv", "hash"),
    ("ledger", "doc/conformance/ledger.toml", "ledger"),
]

# A rank: the word that places one member above all the others.
RANK = re.compile(
    r"\b(tightest|loosest|largest|smallest|widest|narrowest|highest|lowest|biggest|greatest"
    r"|deepest|longest|shortest|slowest|fastest|heaviest|lightest|closest|furthest|farthest"
    r"|worst|best|first)\b",
    re.IGNORECASE,
)

# A quantifier: the word that says no member is otherwise. Far commoner than a rank and mostly a
# statement about one object ("the only operand"), so it is listed only beside a scope in time.
QUANTIFIER = re.compile(r"\b(only|never|every|no other|none of)\b", re.IGNORECASE)

POPULATION = (
    r"(?:file|bucket|list|group|gate|corpus|ladder|tree|project|population|ledger|campaign"
    r"|census|oracle|device|walk|sweep)"
)

# The scope that turns a rank into a claim about a population that grows: the population named as
# the thing ranked over ("on this list", "this bucket has measured"), or a scope in time.
IN_POPULATION = re.compile(
    rf"\b(?:in|on|of|across|over) (?:this|the whole|this whole|the) {POPULATION}\b"
    rf"|\b(?:this|the|that|its) {POPULATION} (?:has|have|had) (?:yet |ever )?"
    r"(?:measured|had|seen|found|recorded|reached|held|produced|met)\b"
    rf"|\bthis {POPULATION} produces\b",
    re.IGNORECASE,
)
IN_TIME = re.compile(
    r"\bso far\b|\bto date\b|\bon record\b|\bever\b|\byet\b(?! (?:another|again))"
    r"|\b(?:has|have) (?:yet )?(?:measured|had|seen|recorded)\b",
    re.IGNORECASE,
)

COMMAND = re.compile(r"`(?:tools/[\w./-]+|cargo [^`]+|[\w-]+ --[\w-]+[^`]*)`")
NUMBER = re.compile(r"(?<![\w.§])\d+(?:\.\d+)?%?(?![\w])")


def sentences(text):
    """Splits prose on sentence ends, keeping each sentence's offset into `text`."""
    start = 0
    for match in re.finditer(r"(?<=[.!?])\s+(?=[A-Z*`(\"])|\n\s*\n", text):
        yield start, text[start : match.start()]
        start = match.end()
    if start < len(text):
        yield start, text[start:]


def rust_blocks(lines):
    """Each run of doc-comment lines, as (first line number, joined text, line offsets)."""
    block, first, offsets = [], 0, []
    for number, line in enumerate(lines, start=1):
        stripped = line.strip()
        body = None
        for prefix in ("///", "//!"):
            if stripped.startswith(prefix):
                body = stripped[len(prefix) :].strip()
        if body is None:
            if block:
                yield first, block, offsets
            block, offsets = [], []
            continue
        if not block:
            first = number
        offsets.append((sum(len(b) + 1 for b in block), number))
        block.append(body)
    if block:
        yield first, block, offsets


def hash_blocks(lines):
    block, offsets, first = [], [], 0
    for number, line in enumerate(lines, start=1):
        if not line.startswith("#"):
            break
        if not block:
            first = number
        offsets.append((sum(len(b) + 1 for b in block), number))
        block.append(line.lstrip("#").strip())
    if block:
        yield first, block, offsets


def ledger_blocks(lines):
    for number, line in enumerate(lines, start=1):
        if line.startswith('note = "'):
            body = line[len('note = "') :].rstrip()
            body = body[:-1] if body.endswith('"') else body
            yield number, [body.replace('\\"', '"')], [(0, number)]


def line_of(offsets, at):
    found = offsets[0][1]
    for offset, number in offsets:
        if offset <= at:
            found = number
    return found


def hits():
    found = []
    for label, path, kind in SOURCES:
        full = os.path.join(ROOT, path)
        with open(full, encoding="utf-8") as handle:
            lines = handle.read().split("\n")
        reader = {"rust": rust_blocks, "hash": hash_blocks, "ledger": ledger_blocks}[kind]
        for _, block, offsets in reader(lines):
            text = "\n".join(block)
            for at, sentence in sentences(text):
                flat = " ".join(sentence.split())
                word = RANK.search(flat)
                scope = IN_POPULATION.search(flat) or IN_TIME.search(flat)
                if not word:
                    word = QUANTIFIER.search(flat)
                    scope = IN_TIME.search(flat)
                if not word or not scope:
                    continue
                # The word and its scope have to be near one another: a sentence of three
                # clauses that says "first" in one and "this list" in another is two statements,
                # and the pair is what the claim is.
                if abs(word.start() - scope.start()) > 90:
                    continue
                numbers = NUMBER.findall(flat)
                shape = "defers" if COMMAND.search(flat) and not numbers else "asserts"
                found.append((label, path, line_of(offsets, at), shape, numbers, flat))
    return found


def main(argv):
    found = hits()
    counts = {}
    for label, _, _, shape, _, _ in found:
        counts.setdefault(label, {"asserts": 0, "defers": 0})[shape] += 1
    total = sum(c["asserts"] for c in counts.values())
    deferring = sum(c["defers"] for c in counts.values())
    print(
        f"{len(found)} superlative sentence(s) scoped to a growing population: {total} assert a "
        f"number or a rank, {deferring} defer to a command"
    )
    for label in dict.fromkeys(s[0] for s in SOURCES):
        c = counts.get(label, {"asserts": 0, "defers": 0})
        print(f"  {label:14} {c['asserts']:4} asserting  {c['defers']:4} deferring")
    if "--count" in argv:
        return 0
    for label, path, line, shape, numbers, flat in found:
        shown = flat if len(flat) <= 240 else flat[:237] + "..."
        print(f"\n{path}:{line} [{shape}] numbers: {', '.join(numbers) or '-'}\n    {shown}")
    print(
        "\nA hit is a reading list, not a verdict: read each against the table it claims (the "
        "same file's later rows, the ledger, tools/state.sh) and rewrite an overtaken one as what "
        "is. A superlative about one measurement, or one the scope does not reach, is noise."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
