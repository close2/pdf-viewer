# 0396 — A standard set of fourteen may not disagree with itself

Status: accepted
Date: 2026-08-17
Session: 561

Takes `doc/todo/21` §6, written by session 558 out of `doc/corpora/pdf-differences`' ink ranking:
`OverlappingGlyphClipping.pdf` sat at **−8.989** of 255 from the lightest of three references that
agree with each other to 1.2, against a next-worst of −1.237. Amends the ledger rows for §9.3.6,
§9.5, §9.6.2.2 and §8.5.3.3.2.

## Context

The page is 2794 bytes and hand-commented. It shows `(B 8)` in `/Times-Bold` at 500 pt and `(8B)`
in `/Helvetica` at 660 pt, both in text rendering mode 5 — stroke and add to the clipping path —
and then fills a blue rectangle through the clip that `ET` leaves. Neither font is embedded.

Session 558 diagnosed it by construction rather than by inference: Helvetica-Bold over Helvetica
unions, Times-Bold over Times-Roman unions, Times-Bold over Helvetica **cancels**. This tree
answers Helvetica with Liberation Sans, an `sfnt`, and Times with one of Foxit's bare CFF programs,
and the two carry opposite contour conventions.

Session 561 measured that claim rather than inheriting it. The signed area of a capital `B` in the
em square, over all fourteen compiled-in faces:

| faces | format | signed area of `B` |
|---|---|---|
| the four Liberation Sans | `sfnt` (`glyf`) | **−0.186** to −0.275 |
| the ten Foxit faces | bare CFF | **+0.114** to +0.230 |

The split is exactly the format split, and it is total: no face straddles it.

## The clause, first

Three sentences decide this, and the order they are read in is what makes the answer an argument
rather than a preference.

**§9.3.6 makes direction visible.** At `ET`:

> the accumulated glyph outlines, if any, shall be combined into a single path, treating the
> individual outlines as subpaths of that path and applying the non-zero winding number rule

and its NOTE 2 states the consequence outright:

> Due to the use of non-zero winding number rule, the direction of the paths comprising each glyph
> can cause different output for overlapping glyphs.

So the standard has considered this case and describes it as a difference, not as an error. One
clause family over, §8.5.3.3.2's rule is the mechanism: two contours of opposite sign sum to zero
where they overlap.

**§9.5's NOTE 5 makes the choice of face ours.** "[S]ome details of font naming, font
substitution, and glyph selection are implementation-dependent and can vary among different PDF
processors and operating system environments." Nothing in §9.6.2.2, §9.8.1 or Table 120 states a
requirement on a substituted glyph's *shape*; session 523 established that while reading the same
neighbourhood for ADR 0358. So the direction is not something the file said and not something the
standard says: it is this program's, entirely.

**§9.6.2.2 is what makes ours bad.** Its first sentence:

> The PostScript language names of 14 Type 1 fonts, known as the standard 14 fonts, are as
> follows: Times-Roman, Helvetica, Courier, Symbol, Times-Bold, Helvetica-Bold, Courier-Bold,
> ZapfDingbats, Times-Italic, Helvetica-Oblique, Courier-Oblique, Times-BoldItalic,
> Helvetica-BoldOblique, CourierBoldOblique.

**One set, of one kind of program.** A document may draw two of them into a single path — this one
does — and fourteen Type 1 programs do not disagree with each other about which way a contour
runs. A stand-in set that answers one of the fourteen with an `sfnt` and another with a CFF
manufactures a disagreement the thing it stands in for does not have, in the one place §9.3.6 makes
direction visible.

**So the defect is neither the clause's nor the page's nor the rasteriser's. It is a property of
`data/standard-fonts/`**, and it is checkable in-tree with no reference at all.

## Decision

### 1. Every outline of a face *this program chose* is wound one way; an embedded program is never touched

`pdf_font::substituted::wound_counter_clockwise`, called from `LoadedFont::build_outline` under
`self.substituted`. The condition is the whole of the boundary: for an embedded program the
direction is the producer's statement about their own font and §9.3.6 NOTE 2 protects it, so a
document that arranges two embedded faces to cancel gets the cancellation it asked for.

### 2. Counter-clockwise, and the choice is documented as a choice

The standard states no direction anywhere. Two things point the same way and neither is a
derivation:

- **Ten of the fourteen already carry it.** Normalising to the other convention would change ten
  faces to spare four.
- **§9.6.2.2 calls them Type 1 fonts**, and the ten that are Type 1-derived charstrings are the ten
  that are counter-clockwise. The set stands in for one kind of program, so it takes that
  program's convention.

Adopting a *de facto* convention as though it were derived is exactly what principle 5 forbids, so
this is recorded as a choice with a reason and not as a reading.

### 3. Measured, not inferred from the format

`Path::signed_area` over the whole glyph decides each outline's direction; the format does not. An
OpenType face on this machine is an `sfnt` wrapper around CFF charstrings and is wound the CFF way,
and `substitute::installed_wider` may return any face the machine has — so a rule keyed on
`Format::Sfnt` would reverse those and break them. An outer contour always encloses more than the
counters inside it, so the *sum* over a glyph carries the outer contour's sign, which is why one
number per glyph is enough. `pdf-render`'s cubic weights are the exact Green's-theorem integral
rather than a flattening, so a glyph whose contour bulges away from its handles is not misjudged.

### 4. The three ways session 558 listed, and why this is the third of them

| §6's option | why not |
|---|---|
| normalise when §9.3.6's clip accumulates | fixes the *page* and leaves the *set* inconsistent, so the property could not be asserted without building a page to assert it on; and it would have to be undone again the next time two glyph outlines meet in one path |
| replace the four Liberation faces with CFF ones | a data and licence question, and it would have to reproduce Liberation's Helvetica metrics and keep ADR 0133's machine-independence — a large change for a property a function states in three lines |
| **reverse at load** | taken, with one amendment: *measure* the direction rather than keying it on the format, which costs nothing and makes the rule true of machine-installed substitutes as well as of the compiled-in fourteen |

### 5. It changes no glyph drawn on its own, and that is a proof rather than a hope

Reversing every subpath of a path negates every winding number. Both of §8.5.3.3's rules test a
winding number's *magnitude* — non-zero, and odd — so a fill, a clip and a stroke of a reversed
glyph paint exactly what they painted before. Only a **combination** with a path that was not
reversed can move, and in this tree the only place two glyph outlines enter one path is §9.3.6's
clip.

The corpus says the same thing from the other end. `display_list_digest` over all 974 first pages
moves **162 lines**, every one of them with the same command count and the same `Debug` byte length
and a different hash — the same commands with the points in a different order — while the oracle's
1794 pages are byte-identical in every per-page metric line and `doc/todo/00` step 7's ink sweep is
byte-identical over all 786 ambiguous pages. Geometry changed on a sixth of the corpus and not one
pixel of it did.

## What it costs

**Nothing at startup**, which is `CLAUDE.md`'s rule for compiled-in data: the faces are still
`include_bytes!`d `static` data with no parse at launch, and the normalisation happens inside
`LoadedFont`'s existing outline cache — once per distinct glyph a page actually shows, and only for
a font the document did not embed.

Measured under callgrind, interpretation end to end on two substituted-text pages:

| page | before | after | |
|---|---|---|---|
| `issue20489.pdf` p1 | 37 075 144 | 37 144 162 | +0.19% |
| `pr12564.pdf` p1 | 182 243 769 | 182 901 300 | +0.36% |

The reversal allocates one `Vec` per reversed glyph, which the cache then holds in place of the
original — no steady-state growth.

## The witness

At 72 dpi on the crop box, ink as `(1 − mean) × 255` over a luma greyscale, which is
`doc/todo/00` step 7's own instrument:

| | ink |
|---|---|
| `poppler` | 79.962 |
| `mupdf` | 79.801 |
| `ghostscript` | 80.995 |
| ours, before | 70.812 — **−8.989** from the lightest |
| ours, after | 78.685 — **−1.116** |

**And the page was looked at, before and after** (trap 1), beside the corpus's own
`overlapping-glyph-clip-correct.png`. Before, the left pair shows white where the `8` crosses the
`B` and the right pair loses the whole of the `8`'s upper bowl; after, the union is solid and the
only white left is where **both** glyphs have a counter — which is what the non-zero rule says and
what the reference picture shows. The residue of −1.116 is the faces' own shapes, which §9.5's NOTE
5 leaves ours, and it is now smaller than that corpus's previous next-worst gap of −1.237.

## The three constructions, asserted

`crates/pdf-model/tests/glyph_clip_direction.rs` builds each pair rather than taking the corpus
page, because what went wrong was a property of the set. Each glyph's own clip is rasterised alone,
so the union and the overlap are measured rather than assumed, and the combined clip is then held
against the union pixel by pixel. Deleting `wound_counter_clockwise` was run, and it is what makes
the numbers below evidence:

| pair | overlap | union pixels lost, before | after |
|---|---|---|---|
| Helvetica + Helvetica-Bold | 1851 | **0** | 0 |
| Times-Bold + Times-BoldItalic | 1816 | **0** | 0 |
| Helvetica + Times-Bold | 1642 | **1760** | 0 |

The two same-family rows pass on both sides of this ADR and are kept for that reason: three cases
that all failed would not have said that each family was internally consistent while the *set* was
not. `crates/pdf-font/src/standard.rs::every_compiled_in_face_winds_its_contours_the_same_way`
states the property of the set directly, over all fourteen, and fails without the fix.

## Consequences

- `doc/todo/21` §6 is closed. The population it asked for first was never counted, and this ADR
  records why that ordering was wrong rather than skipped: the cost of counting was a corpus sweep,
  the cost of fixing was three lines and a measurement, and the property is checkable without a
  population at all. A count of pages that accumulate two substituted faces into one clip would
  have priced the *symptom*; the defect was in the set.
- `pdf-render` gains `Path::signed_area` and `Path::reversed`, both public and both general
  geometry. They are used from `pdf-font` today; the alternative was a private reversal inside
  `pdf-font` that no test in `pdf-render` could reach, and the subtle cases — a cubic's handles
  swapping ends, an unclosed subpath, several subpaths — belong beside the type that has them.
- **A face installed on this machine is normalised too**, which is broader than `doc/todo/21` §6's
  rule and free. Two machine-found substitutes of different formats would have had the same defect
  and no compiled-in face need be involved.
