"""Sweep this tree for quotations of `CLAUDE.md` that `CLAUDE.md` no longer contains.

`CLAUDE.md` says "**Quotation marks mean verbatim.**" of the *standard*, and the tree has a
gate for that half: `conformance::prose` reads every quotation in this project's prose against
the specification Markdown in `doc/md/`, and `--bin quotations` reports the ones that match a
specification for a few words and then diverge.

Its discriminator is *matches a specification and then diverges*, so a quotation whose source
is not a specification is invisible to it — it matches nothing, and matching nothing is how a
quotation of a **different** document looks. This tree quotes two documents verbatim, not one:
ISO 32000-2, and the file that decides what is in scope. Only the first had a reader.

The second one moves. `CLAUDE.md`'s authoring exclusion was redrawn on 2026-09-03 (RFC 0002
section 11.1, ADR 0816) and the sentence it replaced — the one naming linearisation,
object-stream packing and optimisation together as a generator's and therefore out of scope —
went on being quoted, as `CLAUDE.md`'s, in twenty-three conformance-ledger rows and paraphrased
in one module header, while `pdf_syntax::serialize` implemented §7.5.7 object-stream packing in
the same crate and `pdf_transform::optimize` shipped the other one.

# What it reads

Every tracked text file, for a quoted span of at least `MIN_WORDS` words *attributed* to
`CLAUDE.md` — the mention running into the opening delimiter, within `ATTRIBUTION` characters
of it. The unit is a paragraph, because a Rust doc comment attributes on one line and quotes on
the next; a `.toml` note is one line and is its own paragraph.

Quotation marks are the population because `CLAUDE.md` makes them one: "Quotation marks mean
verbatim." A paraphrase without them is prose and is nobody's finding.

A span is *answered* when it appears, normalised, in `CLAUDE.md` — or in any specification
under `doc/md/`, which is what a paragraph quoting the standard beside a mention of `CLAUDE.md`
looks like and is not this sweep's business. What is left is a span attributed to one of the
two verbatim sources this tree has and present in neither.

# It reports and does not fail

`doc/adr/`, `doc/history/` and `doc/rfc/` are records: a document quoting the sentence it
retired is quoting it correctly, and a round does not edit another round's record. Those are
printed under a heading of their own. `raster/` is quorra's tree with a governing document of
its own, and is not read at all.

Everything else is maintained prose. A span there is still a **question for a person** rather
than a build failure, for the reason `--bin quotations` gives one clause over: attribution is a
proximity rule, so the report carries a residue that is correct prose — a document saying what
`CLAUDE.md` *used to* state, a note quoting its own earlier wording, a mention that happens to
sit sixty characters in front of somebody else's sentence. This program exits non-zero only
where it cannot read what it needs.

    tools/governing-quotations.py            # the report
    tools/state.sh governing                 # the same, beside the other counts
"""

from __future__ import annotations

import os
import re
import subprocess
import sys

#: The shortest quoted span worth reading. Below this a quoted fragment is a term of art —
#: "shall", "may", a key name — rather than a sentence somebody took from a document.
MIN_WORDS = 5

#: How far after a mention of `CLAUDE.md` a quotation is still that mention's. Long enough
#: for the verbs this tree actually uses to attribute with — "principle 5 excludes producing
#: one by name:" is forty-seven characters — and short enough that the next sentence's
#: quotation of something else is not swept in.
ATTRIBUTION = 60

#: Where the records live: quoting a retired sentence is what these are for. `doc/rfc/` is
#: one of them for a reason worth stating — an RFC quotes the rule it proposes to change, and
#: `CLAUDE.md`'s authoring exclusion was in fact redrawn *by* RFC 0002 section 11.1, so that
#: document's quotation of the sentence it replaced is the proposal rather than a stale copy.
RECORDS = ("doc/adr/", "doc/history/", "doc/history.md", "doc/rfc/")

#: quorra's own tree, with a governing document this sweep does not hold.
FOREIGN = ("raster/",)

#: What a quotation can be found in without being this sweep's business.
SPECIFICATIONS = "doc/md"

TEXT = (".rs", ".toml", ".md", ".sh", ".py", ".c", ".h", ".txt")


def normalise(text: str) -> str:
    """Reduce prose to what two copies of one sentence have in common.

    Markdown emphasis, backticks, the bracketed first letter a quotation of a mid-sentence
    `shall` starts with, the three dashes, the four quotation marks and every run of white
    space are all things one copy spells differently from another without either being a
    misquotation. Case is dropped for the same reason: a sentence quoted as the start of
    another one keeps its meaning and loses its capital.
    """
    text = text.replace("’", "'").replace("‘", "'")
    text = text.replace("“", '"').replace("”", '"')
    text = text.replace("—", "-").replace("–", "-")
    text = re.sub(r"\[([A-Za-z])\]", r"\1", text)
    text = re.sub(r"[*`_\\]", "", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip().casefold()


def paragraphs(text: str, one_line: bool) -> list[tuple[int, str]]:
    """The file cut into units that carry an attribution and its quotation together.

    A TOML note is one very long line, so `one_line` makes every line its own paragraph;
    anywhere else a paragraph is a run of lines with no blank line in it, which is what a
    Rust doc comment's `//!` block and a Markdown paragraph both are.
    """
    lines = text.splitlines()
    if one_line:
        return [(i, line) for i, line in enumerate(lines, 1)]
    units: list[tuple[int, str]] = []
    start, buffer = 1, []
    for i, line in enumerate(lines, 1):
        if line.strip():
            if not buffer:
                start = i
            buffer.append(line)
        elif buffer:
            units.append((start, "\n".join(buffer)))
            buffer = []
    if buffer:
        units.append((start, "\n".join(buffer)))
    return units


def spans(unit: str) -> list[str]:
    """Every quoted span in one paragraph that is *attributed* to `CLAUDE.md`.

    Naming the file in the same paragraph is not attribution: a paragraph about scope
    routinely names `CLAUDE.md` and then quotes the standard, another document or itself.
    What attributes a span is the naming running into it — `CLAUDE.md`'s "…",
    `CLAUDE.md` principle 5 excludes producing one by name: "…" — so the mention has to be
    the last thing before the opening delimiter, within `ATTRIBUTION` characters of it and
    outside any earlier quotation.
    """
    # A TOML value's own delimiters are not quotation marks, and leaving them in pairs the
    # opening one with the note's first *real* quote — which silently hid twenty-three
    # findings on this sweep's first calibration run.
    flat = re.sub(r'^\s*[A-Za-z_][A-Za-z_0-9-]* = "(.*)"\s*$', r"\1", unit)
    flat = flat.replace('\\"', '"')
    # A Rust doc comment's own markers, and a TOML note's escaped newlines, are not prose.
    flat = re.sub(r"(?m)^\s*(?://[/!]?|#)\s?", " ", flat)
    flat = flat.replace("\\n", " ")
    found = []
    gap_from = 0
    for match in re.finditer(r'"([^"]+)"', flat):
        quoted, opening = match.group(1), match.start()
        gap, gap_from = flat[gap_from:opening], match.end()
        if "CLAUDE.md" in quoted or len(quoted.split()) < MIN_WORDS:
            continue
        mention = gap.rfind("CLAUDE.md")
        if mention < 0 or len(gap) - (mention + len("CLAUDE.md")) > ATTRIBUTION:
            continue
        found.append(quoted)
    return found


def fragments(span: str) -> list[str]:
    """A span cut at its ellipses, because each side is a separate claim about the source."""
    parts = [p for p in re.split(r"\s*(?:\.\.\.|…)\s*", normalise(span)) if p]
    return [p for p in parts if len(p.split()) >= MIN_WORDS] or [normalise(span)]


def read(path: str) -> str:
    with open(path, encoding="utf-8", errors="replace") as handle:
        return handle.read()


def main() -> int:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(root)

    governing = normalise(read("CLAUDE.md"))
    specifications = [
        normalise(read(os.path.join(SPECIFICATIONS, name)))
        for name in sorted(os.listdir(SPECIFICATIONS))
        if name.endswith(".md")
    ]

    listed = subprocess.run(
        ["git", "ls-files"], capture_output=True, text=True, check=True
    ).stdout.split("\n")

    maintained: list[tuple[str, int, str]] = []
    recorded: list[tuple[str, int, str]] = []
    read_count = 0
    files_read = 0

    for path in listed:
        if not path.endswith(TEXT) or path == "CLAUDE.md":
            continue
        if path.startswith(FOREIGN):
            continue
        text = read(path)
        if "CLAUDE.md" not in text:
            continue
        files_read += 1
        for line, unit in paragraphs(text, one_line=path.endswith(".toml")):
            if "CLAUDE.md" not in unit:
                continue
            for span in spans(unit):
                read_count += 1
                pieces = fragments(span)
                if all(piece in governing for piece in pieces):
                    continue
                if any(
                    all(piece in body for piece in pieces) for body in specifications
                ):
                    continue
                where = recorded if path.startswith(RECORDS) else maintained
                where.append((path, line, span))

    if recorded:
        print("== quoted in a record, which is what a record is for ==\n")
        for path, line, span in recorded:
            print(f"  {path}:{line}\n    {span[:160]}")
        print()

    print("== attributed to CLAUDE.md and in neither CLAUDE.md nor doc/md/ ==\n")
    if not maintained:
        print("  none")
    for path, line, span in maintained:
        print(f"  {path}:{line}\n    {span[:160]}")

    print(
        f"\nread {read_count} quotations attributed to CLAUDE.md in {files_read} files that "
        f"name it: {len(maintained)} in maintained prose, {len(recorded)} in records"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
