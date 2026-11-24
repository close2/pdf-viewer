# 3584 — The other three departures re-read against the errata, and the PDF/A errata listed

Session 2220. Status: **accepted**. Context: ADR 3581, ADR 3447, `doc/upstream/verapdf-1.30.3-
departures.md`, `doc/errata/`.

## #1669, #1670, #1671: still veraPDF's

Step 5 of the owner's plan asks for the errata and pdf-issues before a disagreement is called
another implementation's. Run after the fact on the three still open (threads read on 2026-11-24:
no comment on any of the three):

- **#1669** (`re%` read as one operator; ISO 32000-2 §7.2.3, §7.2.4). The site's §7.2 errata do not
  touch the delimiter table or the comment rule. pdf-issues #273 (open, "Comments in stream data")
  asks how §7.2.4's "inside a content stream … introduces a comment" sits with §7.2.3; nothing
  there makes `%` part of an operator. #327 and #826 settle how adjacent numbers are delimited, not
  `%`. **Theirs.**
- **#1670** (a rectangle of indirect numbers read as zero-sized; §7.3.10, §7.9.5). The §7.3.10
  erratum (#379) defines the grammar of an object number and leaves "as an element of an array" in
  place; nothing touches §7.9.5. **Theirs.**
- **#1671** (a CFF encoding supplement read as a glyph index). The reading rests on Adobe Technical
  Note #5176, which has no errata page and no issue; the ISO 19005 clauses it fails under are the
  width and glyph rows, which no PDF/A issue changes. **Theirs.**

## The PDF/A errata

`doc/errata/pdfa.md` lists the thirty-four issues labelled for a part of ISO 19005 or naming it,
each with its resolution as the comments record it and the `pdf-archive` rows it touches. Besides
#77, twelve are listed as recording a working group's agreement to change normative text (#104, #123, #192, #197,
#228, #232, #244, #253, #314, #626, #631, #633 — #104 and #123 published on the errata page,
the rest agreed for the dated revision of part 4). The list is a first reading of the threads; their verdicts, each re-reading its thread, are step 4, the next round's,
with the gate that fails on one without.
