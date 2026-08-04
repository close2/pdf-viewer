# ADR 0174 — An empty glyph stays empty, and a page-level number cannot clear a mechanism

Status: accepted, two-hundred-and-thirty-ninth session.
Amends ADR 0170.

## Context

`issue7074_reduced.pdf` reads *Our 2015 Graduates* in an embedded `CIDFontType2` subset of Arial
Bold under `Identity-H`. Three reference renderers and `hayro` drew it with spaces; we drew
`Our|2015|Graduates` — a narrow mark where each space belongs. `AMBIGUOUS_SPACE_DRAWN_AS_A_MARK`
had held the page since the two-hundred-and-twenty-fourth session with the finding written out
and the cause not found: "a code that should select a blank glyph and does not".

## What it was

The font is one of the six in the corpus whose `loca` offsets do not ascend. ADR 0170's repair
rebuilds such a `glyf` in glyph order by reading each entry's length **from its own bytes**,
because the file states each glyph's extent twice and only one of the two readings is
self-consistent.

That is true of a *descending* pair. It is not true of an **equal** one. The glyph table's own
standard writes a glyph with no outline by giving it and its successor the same offset, and that
statement is self-consistent whatever the rest of the table does. This font's table begins

```text
0  108  0  108  108  282  0  282  962 …
```

so glyph 3 — the space — has start 108 and successor 108, and the repair read the entry that
*begins* at 108, which is glyph 4. **The space was given a real glyph by the code that exists to
give a glyph back its own bytes.**

## Decision

**Honour a repeated offset; overrule only a descending pair.** One line in
`repaired_loca_order`: where `loca[i] == loca[i + 1]`, append nothing and move on.

This keeps the repair's own argument intact rather than weakening it. The repair overrules the
table exactly where the table contradicts itself, and a repeated offset is the one place it does
not — it is the standard's own spelling of "no outline".

ADR 0170's own witness is unaffected: `issue11131_reduced.pdf`'s table begins `16776 16776 16776
16776 10674 …`, where the repeated offsets are past `glyf`'s end and produced nothing under the
old code either. Its sentence still draws in full.

## Consequences

`issue7074_reduced.pdf` draws its spaces. Ink at 72 dpi **20.83 → 19.59**, against `hayro` 19.52,
`mupdf` 19.15, `poppler` 19.09 and `ghostscript` 18.06; ours at 8× is **19.749** against the
two-ladder limit of 19.751 / 19.755. Before the fix we were the only renderer above that limit,
which is what "extra ink where a space belongs" looks like from the closed form.

No other gate moves: corpus, oracle counts, text extraction and the cross-backend gate are all
unchanged, which is what a defect confined to five glyphs of one document should look like.

### The lesson, and it is about the instrument rather than the font

**A page-level number cannot clear a mechanism of a defect that is five glyphs wide.** The
session that landed ADR 0170 checked its repair against this very document and cleared it —
`AMBIGUOUS_SPACE_DRAWN_AS_A_MARK` records that switching the repair off left the page's ink at
19.576 either way, and concluded "[t]he repair does not reach this document's marks". The
measurement was right and the inference was not: the page's ink is three words of bold nine-point
text, and five narrow bars are under a tenth of a level of it.

The A/B was the right idea pointed at the wrong quantity. What would have answered it is the
quantity the *hypothesis* is about — the glyph the space's code resolves to, with and without the
repair — which is one `assert` rather than one render.
