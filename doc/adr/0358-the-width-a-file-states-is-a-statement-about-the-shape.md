# ADR 0358 — The width a file states is a statement about the shape

Status: accepted, 2026-08-14. Session 523. Extends ADR 0267's reading of §9.8.1 to the width half
of `doc/todo/21` item 4, and leaves its cap-height half exactly where it stood.

## Context

`bug1671312_ArialNarrow.pdf` is 1913 bytes, one line of 20 pt text in a non-embedded
`/ArialNarrow`, and session 518 found it at the head of the ambiguous bucket's ratio ranking with
its group's note **backwards** (ADR 0353): ours is the *wide* one. Four measurements said so and
the picture said it in one look — our letters collide where four other renderers have clean gaps:

| | ours, before | poppler | mupdf | ghostscript | hayro |
|---|---|---|---|---|---|
| ink box | x[10, 149] y[15, 34] | x[10, 147] y[15, 34] | x[10, 147] y[15, 34] | | |
| marked pixels in it | **983** | 844 | 825 | 812 | 702 |
| page ink of 255 | **18.45** | 15.52 | 15.32 | 14.97 | 12.71 |
| modal dark run at 576 dpi | **14 px** | 12 px | | | |

The file's own `/StemV 66` is **10.56 device pixels** at that scale. So the advances were already
right — the ink boxes agree to two device columns, which is §9.2.4 and Table 109 honoured — and
what was wrong was the *shape inside* them.

The project owner supplied `mozilla/pdf.js#12725` as evidence. Its own commit message says
"[t]here doesn't seem to be anything definitive about this in the spec, but from experimenting, it
seems acrobat lets PDFs override the widths of the standard fonts", and `CLAUDE.md` principle 5
refuses that as a justification for anything. So this round's first obligation was the clause.

## What the standard says, quoted

**§9.6.2.1, Table 109's `/Widths` row** — the sentence this decision turns on:

> These widths shall be consistent with the actual widths given in the font program.

**§9.6.2.1's closing paragraph**, which is where ISO 32000-2 differs from ISO 32000-1 and where
quoting the older standard would be an error. The heading of §9.6.2.2 now reads *Standard Type 1
fonts (standard 14 fonts) **(PDF 1.0-1.7)***, and Table 109 makes `/FirstChar`, `/LastChar`,
`/Widths` and `/FontDescriptor` "(Required; optional in PDF 1.0-1.7 for the standard 14 fonts)":

> PDF versions 1.0 to 1.7 did not require Type 1 font dictionaries to include FirstChar , LastChar , Widths and FontDescriptor entries as described in 9.6.2.2, "Standard Type 1 fonts (standard 14 fonts) (PDF 1.0-1.7)". For compatibility reasons PDF processors shall provide glyph widths and font descriptor data for those standard fonts for use in processing PDF files when the entries are absent.

**And Table 109's `/FontDescriptor` row answers the pull request's question from the standard
rather than from an experiment:**

> For the standard 14 fonts, the entries FirstChar, LastChar, Widths , and FontDescriptor shall either all be present or all be absent. Ordinarily, these dictionary keys may be absent; specifying them enables a standard font to be overridden

That is the file overriding the built-in metrics, in the standard's own words, and this tree has
done it since `metrics::simple_widths` was written: `/Widths` wins, then §9.6.2.2's published
metrics, then the program's own advances. **The neighbouring question was already answered, and
the answer was derivable.**

**§9.8.1**, on the descriptor as a whole:

> These font metrics provide information that enables a PDF processor to synthesise a substitute
> font or select a similar font when the font program is unavailable.

**Table 120**, on the four entries `doc/todo/21` named. `/StemV`: "The thickness measured
horizontally, of the dominant vertical stems of glyphs in the font. Values shall be positive."
`/AvgWidth`: "The average width of glyphs in the font." `/MaxWidth`: "The maximum width of glyphs
in the font." `/FontBBox`: "a rectangle … that shall specify the font bounding box." Every one of
them describes *the font*, and not one of them instructs a processor.

**§9.8.2**, on the flags, is the nearest the clause comes to an instruction and it is a permission:
the Symbolic/Nonsymbolic distinction "influences the font's default base encoding and **may affect**
a PDF processor's font substitution strategies".

**§9.5's NOTE 5** puts the whole subject outside the standard: "some details of font naming, font
substitution, and glyph selection are implementation-dependent and can vary among different PDF
processors and operating system environments."

**And §9.6.2.2 is where the *substitution* obligation actually is**, which is worth recording
because `doc/todo/21`'s clause list sends a reader to §9.6.4 for it and §9.6.4 is *Type 3 fonts*:
"These fonts, or their font metrics and suitable substitution fonts, shall be available to the PDF
processor." A `shall` about *having* a substitute, and silence about what it looks like.

## The decision, and it is (b) of the three the round was given

**(a) The standard requires nothing of a substituted face's shape.** Not one `shall` in §9.5,
§9.6.2.2, §9.8.1, §9.8.2 or Table 120 is addressed to what a processor draws when it stands a face
in. So there is nothing to implement out of obedience.

**(b) There is a construction derivable from what the file itself states, and it is built.**
Table 109's `shall` binds the `/Widths` array to the *actual widths of the font program the
document meant*. That makes a `/Widths` entry two statements rather than one: where the next glyph
starts, which this tree already honours, and **how wide the absent font drew that glyph**. A
substitute drawn at its own designer's width inside those advances contradicts the second
statement, and on a condensed face the contradiction is what the witness shows.

So `metrics::substitute_stretch` derives one number per substituted simple font — the **median**
over the declared codes of (the width the file states) ÷ (the advance the chosen face states for
the glyph that code reaches) — and `LoadedFont::build_outline` applies it to the outline's **x**
alone. The advances are untouched: they were already the file's.

**This is a documented choice under `CLAUDE.md`'s "where the standard defines nothing" rule**, not
a clause obeyed, and four things about its shape are decided rather than assumed:

- **The axis is the standard's.** Table 120's `/FontStretch` names nine values "ordered from
  narrowest to widest" and then says "[t]he specific interpretation of these values varies from
  font to font". The standard names the axis a face's width lives on and declines to number it;
  `/Widths` numbers it without naming it. Scaling horizontally and leaving the vertical alone is
  that axis, and it is why nothing here touches cap height — ADR 0267's refusal is untouched, and
  this file's `/CapHeight 922` is its `/Ascent 922` against Arial's 716, which is what session 518
  established makes it no witness for that half.
- **It condenses and never expands**, because §9.2.4 makes a width the glyph's *displacement*: it
  bounds the ink from above and says nothing about it from below. Ink wider than the stated width
  contradicts the file visibly; ink narrower than it contradicts nothing, because the file has
  said the absent font set that glyph with room around it, and room is not ink. Widening a shape
  to fill white space would be the same mistake pointing the other way.
- **A median, not a mean**: 0.8201 against 0.8594 on the witness's 218 comparable codes, the mean
  pulled by codes whose glyph names the substitute answers with something unrelated.
- **A ratio below half is refused outright.** A file whose widths disagree with every glyph of the
  face by more than a factor of two is not describing a narrower cut of one design; the likelier
  reading is that the codes are reaching the wrong glyphs, and a glyph shrunk that far is a
  confident wrong mark. There is deliberately no constant at the other end: the other end is 1 and
  the paragraph above is its argument.

**Only a substituted *simple* font is scaled.** An embedded program's outlines are the producer's
own — `loading.rs`'s `an_embedded_program_is_never_reshaped` asserts a stretch of exactly one over
every embedded font on a first page in `doc/` — and repairing a disagreement there would hide the
mapping defect `the_pdf_widths_agree_with_the_font_programs_own_advances` exists to find. A
substituted *composite* font is left alone for a different reason: §9.7.4.2 leaves it reachable
only by character through `/ToUnicode`, so there is no code whose `/W` entry and whose face advance
are two statements about one glyph. A witness there would be a document with a `/W` array, a
`/ToUnicode` and a non-embedded descendant, and `doc/todo/21` now carries it.

## The witness, measured before and after

| | before | after | the four references |
|---|---|---|---|
| ink box | x[10, **149**] y[15, 34] | x[10, **147**] y[15, 34] | poppler and mupdf: x[10, 147] y[15, 34] |
| marked pixels | **983** | **861** | 844, 825, 812, 702 |
| page ink of 255 | **18.45** | **15.28** | 15.52, 15.32, 14.97, 12.71 |
| modal dark run at 576 dpi | **14 px** | **12 px** | poppler 12 px; the file's `/StemV 66` is 10.56 |
| oracle | mean 11.83, worst tile 30.14, differing 9.33%, ssim 0.7968 | mean **8.05**, tile **20.59**, **7.90%**, ssim **0.8566** | |

**The stem is the check that was not fitted.** The scale comes from `/Widths` alone; `/StemV` is a
different entry of a different table, written by the same producer about the same absent font, and
after the change the drawn stem lands on 12 px where the file's own number is 10.56 — from 33% over
its own statement to 9% over. Two of the file's own numbers agreeing is the evidence this
construction rests on, and neither of them is a reference renderer's pixel.

**And the page was looked at** (trap 1): before, the `cc` and `ss` of *Accessory* are welded into
single blots; after, every letter has a gap.

## What it costs, and the estimator's own failure written down

**Over the 974**: 257 first-page fonts are substituted, **30 get a scale below one and 15 of those
by more than half a per cent**, over 22 and 11 documents respectively. The other 227 are
metric-compatible with the face standing in for them and are untouched — which is the shape that
makes this safe to apply to every substituted simple font rather than to a population somebody
picked. `cargo run --release -p pdf-model --example substitute_stretch_census -- <files>` is the
instrument and prints the distribution and every document behind it.

**No oracle verdict moves**: 906 / 67 / 786 before and after, and the 888 per-page lines are
identical but for seven. Five improve — the witness above, `non-embedded-NuptialScript.pdf` (mean
17.02 → 11.47, ssim 0.6524 → 0.7640), `issue13916.pdf` (12.16 → 11.36), `XiaoBiaoSong.pdf` (6.63 →
6.26) and `issue12295.pdf` (5.55 → 5.54) — and two move by a hundredth in the fourth decimal, which
is the 0.03% condensation the census lists for `bug847420.pdf` and `issue7580.pdf`. `doc/todo/00`
step 7's ink sweep over all 786 moves exactly two rows, both of them documents this round condensed
and both downward, which is what removing ink does: `issue13916.pdf` −6.980 → −7.368 and
`issue12295.pdf` −2.823 → −2.829. Every other row is identical, twenty at or past −1 and sixteen of
them documents this tree calls incomplete.

**The asymmetry was measured before it was written down, and that is the honest order.** With
expansion allowed, eleven pages moved instead of seven, and the three extra ones were the only
pages that got *worse*: `issue9291.pdf` at a scale of 1.0665 (mean 7.47 → 9.02), `issue7835.pdf` at
1.1978 and `issue7454.pdf`. The argument in the decision above stands on §9.2.4 without them; what
they add is that the clause's reading and the corpus agree.

**The estimator has one known failure and it is not tuned around.** `issue20489.pdf`'s
`/ArialUnicodeMS` declares codes 30 to 255 and states honest widths for the Latin ones its title
block shows — 71 of them within a fiftieth of 0.93 — while a third of the array sits at 0.3 to 0.4,
which is no cut of any typeface. The median lands at 0.688 and that page's letters are drawn
narrower than its own letters ask for; the page agreed with the reference consensus before and
agrees after, and the cost is visible only beside `poppler`'s title block. **A mode over the ratios
answers 0.928 there and was implemented and measured** — and on `non-embedded-NuptialScript.pdf`,
where a script face stands in for a serif and the ratios are genuinely diffuse rather than a cluster
plus filler, the same mode picks 0.666 against the median's 0.799 and draws the line visibly too
thin. Two estimators, each better on one page and worse on the other, is not a discovery about the
standard, and choosing between them on which page looks nicer is exactly the curve-fitting principle
5 forbids. The simpler statistic stands. **What would settle it is a file, not a preference**: a
document whose array says which of its codes it means — a `/CharSet`, a subset tag, a `/FirstChar`
that bounds the codes it actually shows — which is the same shape of answer `declared_codes` gave
ADR 0270's question one clause over.

**What it costs at load**, measured rather than assumed: `callgrind_interpret` on the witness is
29 495 778 instructions before and 29 692 285 after — **196 507 instructions, 0.67%**, for one
substituted font with 224 declared codes, and that is the whole per-font cost. The corpus gate's
974-document wall clock is inside its own noise across four runs (6.6, 7.4 before; 6.9, 7.1 after).

## Consequences

- §9.8.1's ledger row loses `/StemV`, `/AvgWidth`, `/MaxWidth` and `/FontBBox` from its list of
  Table 120 entries nothing reads only in the sense that they are now *read as evidence*: the
  scale is `/Widths`', and `/StemV` is what checked it. The four remain unread as inputs, and the
  row says so.
- `doc/todo/21` item 4 keeps its cap-height half open with ADR 0267's condition unchanged, and its
  width half is closed by this.
- `LoadedFont::stretch()` is public because a caller measuring a page needs to know a shape was
  scaled, and because a test that asserted it through a raster would be asserting about this
  machine's font collection.
