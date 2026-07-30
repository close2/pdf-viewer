# ADR 0052 — The font that said which way round it was

Status: accepted, 2026-07-31.

## Context

After ADR 0050, `issue2537r.pdf` was the only page left in `CONTRADICTED_UNEXPLAINED` where we
differed from **every** reference while they agreed with each other: 10.3 levels from each,
against their own closest agreement of 1.03. It is a two-word page — three references draw
`LINE UP`, we drew three `.notdef` boxes.

The pairwise table found it, as it found ADR 0048's and ADR 0050's, and this time it was the
*ratio* that ranked it: our nearest reference divided by the references' own nearest pair.

## What was wrong

The font is a 60-glyph `Helvetica-Bold` subset in `/FontFile2`, reached by `Identity-H` with no
`/CIDToGIDMap`, so the CID **is** the glyph index — and the CIDs in the content stream are 47,
44, 49, 40, 3, 56, 51, which in the standard Macintosh glyph order are exactly `L I N E ␣ U P`.
Nothing about the PDF is wrong.

The font's `head` table states `indexToLocFormat` as **0x0100**. ISO/IEC 14496-22 defines that
field to be 0 (short offsets) or 1 (long); 0x0100 is 1 written in the wrong byte order and is
neither. `skrifa` reads it strictly, reaches `loca` at the wrong width, and finds no outline
for most glyphs — while still producing outlines for a few, by coincidence, which is why the
font loaded and **nothing reported**.

## Decision

**`repaired_loca_format` rewrites those two bytes, and the font's own table directory is what
decides the value.** This is not a heuristic and not a copy of anybody's leniency:

- `loca` holds `numGlyphs + 1` entries, so its length is `2 × (n + 1)` or `4 × (n + 1)`. Here
  it is **244** bytes for 60 glyphs: the long form exactly, twice the short form.
- The last `loca` entry is the length of `glyf`. Here that is **2056** under the long reading
  and **0** under the short one, against a `glyf` table the directory says is 2056 bytes.

Both tests agree and only one reading satisfies either, so the file states the answer twice.
That is the same shape as the twenty-seventh session's LZW finding and the ninety-six JBIG2
encodings: **a file that states one fact twice can check itself**, and no other implementation
is involved in the argument.

When the field is already 0 or 1, or when *neither* reading satisfies both tests, nothing is
changed and `skrifa`'s own answer stands. The third case has its own test, because a repair
that always succeeds is a guess wearing a derivation's clothes.

## Result

`issue2537r.pdf` agrees with the reference consensus. **815 agreeing and 83 contradicted became
816 and 82**, with the corpus's incomplete count unchanged at 95.

## The spec track: §12.1, §12.2 and §12.4 — nine rows, and `silent` reaches 24

Clause 12's remaining navigation families, read after §12.3's twelve last session. All nine are
`silent`: viewer preferences, page labels, articles, presentations and sub-page navigation, none
implemented and none reported.

Two rows are worth more than the others.

**§12.4.2's page labels** are the only row in either family with no user-interface question in
it — a label is a *string computed from the document*, and `CLAUDE.md` names it in scope. Two
details for whoever builds it: there is no default numbering style, so a range with a prefix and
no `/S` labels every one of its pages identically; and `/A` runs `A`…`Z` then `AA`…`ZZ`, which
is not base-26. It needs a **number tree**, which is also what §12.3.2.4's named destinations
need — so the two rows share one missing piece.

**§12.2's viewer preferences** are mostly about a window this program does not have, and three
of them are not: `/PageLayout`, `/PageMode` and `/Direction` change what a reader sees. The
clause's own fallback is what this program does by accident rather than by decision — "[i]f no
such dictionary is specified, PDF processors should behave in accordance with their own current
user preference settings."

## Consequences

- `CONTRADICTED_UNEXPLAINED` is 38, from 50 five sessions ago; five of the twelve that left
  were fixes.
- The ledger's `silent` count is 24, from 2 three sessions ago, and every one of the
  twenty-two that arrived came from *reading* rather than from any change to the code.
- `pdf-font` now repairs a font before reading it, which it has never done before. The
  precedent is narrow on purpose: the repair is derived from the file, it is confined to a
  field whose legal values are two, and it declines when the file does not decide.
