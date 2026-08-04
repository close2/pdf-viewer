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

## What has to be settled

1. **Which font shows the eight codes.** The trace prints the readback and not the font; adding
   the name to it is one line and is the first thing the next session should do.
2. **What §9.6.5 determines for an `/Encoding` that is neither a name nor a dictionary.** The
   clause says what the entry *is*, not what to do when it is something else, so this is the
   robustness question rather than the coverage one — and the answer has to be argued from what
   the other entries still determine, not from what another reader does.
3. **Whether the page should report.** Eight codes out of a page is exactly the case ADR 0152
   decided *not* to report, so if the reading in 2 leaves them undrawable, the honest outcome is
   a diagnosis here and a page that stays quiet — not a report bolted on for one file.

## Why it is not fixed in the session that found it

Item 1 is a measurement and items 2 and 3 are a reading; doing 3 before 2 would put a fallback in
the code with no clause behind it. What is *done* is the finding: a page that is complete by
every gate and is missing eight characters, the malformation that explains it, and the three
routes §9.6.5 leaves open.
