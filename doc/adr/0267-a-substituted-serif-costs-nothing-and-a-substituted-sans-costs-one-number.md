# ADR 0267 — A substituted serif costs nothing, and a substituted sans costs one number

Status: accepted, 2026-08-11. Session 431. Amends nothing; measures ADR 0133's stated trade for
the first time and declines to close it.

## The question

`oracle.rs`'s `CONTRADICTED_SUBSTITUTED_FONT` has held pages since the sixth session on a
membership rule that its own first paragraph calls the weakest in the file: *the page names a font
nobody embedded*. That is a hypothesis about what a page carries, not a diagnosis of what it
differs by, and six of the seventeen pages had been opened one at a time over four hundred
sessions while five had never been opened at all.

The five were admitted together — `bad-PageLabels.pdf` page 1, `franz_2.pdf` page 1 and
`issue8088.pdf` pages 1 to 3 — and the question this session asked is the one session 405 asked of
`issue4304.pdf` and got a different answer to: **is the substitution what these pages differ by?**

## What was measured

Three instruments, none of which needs a reference to be trusted:

1. the page's **ink**, `magick <png> -alpha off -channel R -colorspace Gray` at the page's own
   scale and again at 8×;
2. the **bounding box of that ink** at 8×, which is the advances and the cap height together;
3. the two font programs' own **capital `I`**, drawn by `magick` straight from the files at a
   stated pixels-per-em.

## What it found, and it is a clean split by family

**The five unexamined pages all name `/Times-Roman`, and the substitution is invisible on them.**
At 8× the ink's bounding box is 1233 × 143 at (84, 133) in ours, `poppler`'s and `mupdf`'s alike on
`issue8088.pdf` — identical over a 1600-column raster — and one column in 1440 apart on the other
two. Equal width is §9.2.4's advances, which Table 109 does state; equal *height* is the cap
height, which it does not. The ink agrees with the references to at most 0.18 of 255 where the
references span 0.13 among themselves, and each page fails exactly one of the four bounds, the
differing fraction, at 5.01% to 5.68% against 5.00% with structural similarity at worst 0.9906 of
0.9000. That is `CONTRADICTED_GLYPH_EDGES`' diagnosis in all three of its instruments, so the five
moved there and the group is 12 rather than 17.

**The pages that name a Helvetica or Arial face differ by one number.** The compiled-in sans is
Liberation Sans (ADR 0133) and the references resolve one through this machine's `fontconfig`,
which is `NimbusSans`:

| face | `I` at 144 px/em | cap height |
|---|---|---|
| `LiberationSans-Regular.ttf` | 99 rows | **0.687500 em** |
| `NimbusSans-Regular.otf` | 105 rows | **0.729167 em** |
| `LiberationSans-Bold.ttf` at 288 px/em | 198 rows | 0.687500 em |
| `NimbusSans-Bold.otf` at 288 px/em | 210 rows | 0.729167 em |

Those two numbers are what the corpus rasters show, exactly, at two point sizes: `issue6108.pdf` at
12 pt draws a 66-row capital against the references' 70, which is 0.687500 and 0.729167 of 96;
`issue7580.pdf` at 18 pt draws 99 against 105, which is the same two fractions of 144. The
resulting ink deficit runs 1.0% on `issue11403_reduced.pdf` to 7.7% on `issue9243.pdf`, which is a
page of nothing but capitals and is therefore the pure case. The advances are untouched by it and
were checked on the same rasters — `issue7580.pdf` spans 1463 device columns against 1461 and 1462.

`FoxitSerif.pfb` has no such gap against `NimbusRoman`: on `bad-PageLabels.pdf` at 12 pt the drawn
cap is 63 device rows in all three. **So the substitution's cost is not a property of substituting;
it is a property of which two files were compared, and it is one metric.**

## Decision

**The sans face is not changed and the six pages stay listed.** Three reasons, in the order they
bind:

- **§9.5 NOTE 5 puts the choice beyond the standard and says so.** "If a PDF file refers to font
  programs that are not embedded, the results depend on the availability of fonts in the PDF
  processor's environment", and "some details of font naming, font substitution, and glyph
  selection are implementation-dependent and can vary among different PDF processors and operating
  system environments." That is `doc/todo/00`'s shape 3, and it is why every page here is listed
  rather than chased.
- **§9.8.1 describes rather than commands.** "These font metrics provide information that enables
  a PDF processor to synthesise a substitute font or select a similar font when the font program is
  unavailable" — no `shall`, and `/CapHeight` is a Table 120 entry that describes a font rather
  than instructing a renderer. Scaling a substitute to it would be a defensible choice; it would
  not be a clause obeyed.
- **The target would be somebody else's file.** Moving 0.687500 to 0.729167 closes the gap because
  `NimbusSans` sits there, and `CLAUDE.md`'s principle 5 forbids exactly that: "if we have the same
  results as the other libraries, we can assume that we understood the spec correctly — but if not,
  we don't try to match what the others do." There is no third face to choose either: pdf.js's
  bundle, which is where these fourteen files came from, contains no Foxit sans at all — ten `.pfb`
  faces for Courier, Times, Symbol and ZapfDingbats and four Liberation Sans, which is why ADR 0133
  used Liberation Sans for Helvetica.

## What it costs, written down

Six contradicted oracle pages that would agree with the consensus if the cap height matched:
`bug847420.pdf`, `bug850854.pdf`, `issue6069.pdf`, `issue6108.pdf`, `issue7580.pdf` and
`issue9243.pdf`, at 1.0% to 7.7% of the page's ink. Nothing else in the tree is affected, because
the advances — the half Table 109 does state — come from Adobe's published metrics and from the
programs themselves and are exact.

**What would change this decision is a document, not a renderer.** A file that states a
`/FontDescriptor` with a usable `/CapHeight` for a non-embedded face is asking, in the standard's
own vocabulary, for capitals of a stated height; reading it would be §9.8.1's sentence acted on
rather than another program's arithmetic copied. `issue7580.pdf` is not that file — its descriptor
states `/CapHeight 0`, `/Ascent 0` and `/Descent 0` — and no page of the corpus has yet been shown
to be. §9.8.1's ledger row carries `/CapHeight` on its list of Table 120 entries this tree does not
read, and now carries the number that list was missing.

## The instrument this round added, and its result was a negative

`doc/todo/00` step 7 sweeps *our ink minus the lightest live reference's* over the ambiguous
bucket. Run over the **contradicted** list instead — all 68 pages, from artefacts already on disk —
the head is:

```text
−5.115  issue5751.pdf p1              (incomplete: a Type 1 program this reader refuses)
−2.203  issue4436r.pdf p1             CONTRADICTED_SUBPIXEL_IMAGE
−1.549  issue9243.pdf p1              this ADR, and the largest of the six
−0.779  smask_luminosity_oob_transfer.pdf p1   CONTRADICTED_MASK_QUANTISATION
−0.482  issue7580.pdf p1              this ADR
```

and nothing else past −0.4. The head is one of the two pages this tree already reports as
incomplete — a page it draws *nothing* on, which is what a report says made visible — and every
remaining negative entry is one of this ADR's six or a group that already argues its own number.
(The other reported page, `knockout_blend_multiply.pdf`, sits at exactly +0.000 because `hayro`
declines the same construction we do and sets the minimum.) The positive side is likewise all
explained: `+13.704 issue11740_reduced.pdf` and `+9.982 issue14802.pdf` are
`CONTRADICTED_REFERENCES_DREW_NOTHING` and `CONTRADICTED_LINK_BORDER` — references that drew
nothing — which is what that half of the sweep is good at.

**Nothing unexplained anywhere on the contradicted list**, which is the same statement
`doc/todo/00` records for the ambiguous bucket and had never been made about this one.
