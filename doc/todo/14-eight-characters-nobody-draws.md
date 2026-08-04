# Eight characters `issue14821.pdf` states and nobody draws

Status: **found by measurement, diagnosed to the file's malformation, not settled.**
Priority: 14 — the last silent member of a population the two-hundred-and-forty-fifth session
counted
Corpus: 1 document (8 codes); the population it heads is 24 codes over 8 documents
Clauses: §9.6.5, §9.6.6, §9.6.3
Code: `crates/pdf-font/src/lib.rs`, `crates/pdf-model/src/content.rs`

## How it was found, which is the part worth keeping

Not from a ranking and not from a picture. `Interpretation::codes_without_a_glyph` counts the
codes a page shows that reach no glyph, `tests/corpus.rs` sums it over the 974 documents **on
pages that report nothing**, and `PDFVIEWER_TRACE_MISSING_GLYPH=1` names each one's readback. The
whole silent population is 24 codes over 8 documents, and this file is 8 of them — a third,
and the only one where the readbacks are ordinary text: `1`, `2`, `3`, `7`, `e` and three `x`s.

Everything else on that list is ones and twos, two of them a replacement character or a CJK
ideograph, which is a `/ToUnicode` question rather than a glyph one.

## What the page does

A form of `textNN` labels, 596 × 843. Ours draws **3.690** of ink against `poppler`'s **3.869**,
and side by side the difference is legible: ours reads `te t` where `poppler` reads `text`, and a
filled block at the top left of `poppler`'s render is absent from ours.

So eight characters of the page's own text are not drawn, and nothing says so — the page is
`complete` by every gate, which is exactly the silence ADR 0152 chose to accept in the general
case and the reason for counting how much of it there is.

## What the file says, checked

Four of its fonts — `/Helvetica`, `/Helvetica-Bold` and `/ABCDEE+Segoe#20UI,Bold` among them —
state **`/Encoding 26 0 R`**, and object 26 is a **content stream**: `/Filter /FlateDecode
/Length 681`, whose data begins `<0043> Tj`. There is exactly one object 26 in the file, so this
is not an incremental update shadowing a real encoding — the producer wrote the wrong object
number.

§9.6.5 makes `/Encoding` "a name or a dictionary". A stream is neither, so the entry is
**unusable**, and what a reader does next is the question:

- fall back to the font program's own `cmap` (§9.6.6.4's route for a symbolic TrueType), or
- fall back to the *standard* encoding the font's `/BaseFont` implies (§9.6.6.2), or
- report the entry and draw what it can.

`/ABCDEE+Segoe#20UI,Bold` is a **subset** with an embedded `FontFile2`, so the first two routes
can disagree about eight codes very easily: a subset keeps the glyphs its document uses and a
`cmap` that may or may not address them by the codes the content stream shows.

## Which fonts, and it is **two** failures rather than one

The trace names the font since the two-hundred-and-forty-seventh session:

```text
MISSING font=/F5 code=91 read="x"   code=26 read="7"   code=21 read="2"   code=22 read="3"
MISSING font=/F7 code=101 read="e"  code=120 read="x"  code=49 read="1"
```

- **`/F5` is object 40**: `/ABCDEE+Arial`, `/Subtype /Type0`, `/Encoding /Identity-H`, with a
  `/ToUnicode`. Under `Identity-H` with `/CIDToGIDMap /Identity` the code **is** the glyph index,
  so codes 21, 22, 26 and 91 name glyphs 21, 22, 26 and 91 of an embedded **subset**. The
  `/Encoding 26 0 R` malformation does not touch this font at all.
- **`/F7` is object 41**: `/ABCDEE+Segoe#20UI,Bold`, a simple `TrueType` with `/Flags 32`
  (nonsymbolic) and an embedded `FontFile2`, whose codes are plain ASCII — `e`, `x`, `1`. This is
  the font that states `/Encoding 26 0 R`.

**So the `/Encoding` pointing at a content stream explains at most three of the eight**, and the
other five are a composite font resolving a CID to a glyph its subset should contain. Two
mechanisms, one page — which is why the first thing to settle is no longer "which font".

## What has to be settled

1. **`/F5`: does glyph 91 exist in the subset?** `maxp`'s `numGlyphs` against the four codes is
   one reading of the font program and settles whether this is a producer error every reader
   shares — in which case `poppler` drawing something means it draws a `.notdef` — or a lookup of
   ours that fails where §9.7.4.2's identity mapping says it should not.
2. **`/F7`: what §9.6.5 determines for an `/Encoding` that is neither a name nor a dictionary.**
   The clause says what the entry *is*, not what to do when it is something else, so this is the
   robustness question rather than the coverage one. Note that a stream object answers `as_dict`
   with its *stream dictionary* in this tree, so the current behaviour is "an encoding dictionary
   with no `/BaseEncoding` and no `/Differences`" rather than "no encoding at all" — and §9.6.6.2
   makes those two different things. **Check which one is being read before choosing between
   them.**
3. **Whether the page should report.** Eight codes out of a page is exactly the case ADR 0152
   decided *not* to report, so if 1 and 2 leave them undrawable, the honest outcome is a diagnosis
   here and a page that stays quiet — not a report bolted on for one file.

## Why it is not fixed in the sessions that found it

Items 1 and 2 are two readings of two different clauses, and item 3 is a decision that depends on
both. What is *done* is the finding — a page complete by every gate and missing eight characters
— and the narrowing: **the malformation everyone would blame explains three of the eight, and the
other five are somewhere else entirely.**
