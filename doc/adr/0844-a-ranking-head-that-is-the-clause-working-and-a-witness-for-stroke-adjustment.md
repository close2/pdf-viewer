# 0844 — A ranking head that is the clause working, and the first real page to witness §10.7.5

Date: 2026-09-03. Session 901.
Status: accepted.
Clauses: ISO 32000-2 §10.7.5, §9.3.6, §8.4.3.2, §10.7.4.
Supersedes nothing; amends the evidence ADR 0688 rests on and adds a third rule to the ink
ranking `doc/oracle-and-corpus.md` §3d states.

## Context

`batch5/ocrmypdf`, 205 documents, is the largest of the issue trackers nobody had walked. The
survey is in `doc/todo/03` §46: 4 unopenable (none of them a PDF at all), 4 incomplete — **1.95%**,
the second-lowest rate of any tracker — and every one of the four incomplete reports names a
population this project has already argued and priced.

So the round did what §46's shape asks and ranked the **whole** directory by ink rather than the
incomplete list alone, ours flattened on white against `pdftoppm -cropbox` and `mutool draw` at
72 dpi. That ranking has exactly one head, and it is **darker** rather than lighter:
`ocrmypdf-99-0.zip-0.pdf`, ours **9.9448** against `poppler` 8.6045 and `mupdf` 7.4452 — 1.34 above
the darkest reference, where the next row in either direction is 0.28.

The page is a Chinese hospital laboratory report, one embedded TrueType subset (`SRPUEP+SimSun`),
one 110 × 49 signature JPEG, no shortfall reported: `open_one` says `unsupported []`.

## The measurement

Three instruments, in the order they were run, and the third is the one that settles it.

**The resolution ladder** (`doc/todo/00` step 6), our ink against both references:

| | ours | poppler | mupdf |
|---|---|---|---|
| 72 dpi | 9.9448 | 8.6045 | 7.4452 |
| 144 dpi | 7.8971 | 7.5527 | 7.3490 |
| 288 dpi | 7.2626 | 7.4733 | 7.2681 |
| **576 dpi** | **7.2727** | **7.2921** | **7.2762** |

A spread of 2.50 levels at 72 dpi becomes 0.02 at 576. Three renderers that converge on one number
are drawing the same shapes, so nothing here is geometry, a missing mark or an extra one.

**The mode**, taken off the content stream. `mutool show -b` on the page's one form XObject: 108
text runs at `2 Tr`, three at `0 Tr`, under `0.240226 w` for the 9.00847 pt body, `0.3203 w` for
12.0112 pt and `0.426 w` for the 15.975 pt title, with an outer `0.24 0 0 0.24 0 0 cm` and a
`4.16667 0 0 4.16667 0 0 cm` inside every `q`, so the CTM at each stroke is unity and those widths
*are* the device widths. Replacing `2 Tr` with `0 Tr` in a `qpdf --qdf` copy puts all three
renderers within 0.04 of each other (6.0587 / 6.0620 / 6.0953 at 144 dpi): the whole disagreement
is the stroke half of the mode.

**The entry.** `/R8` is `<</SA true/Type/ExtGState>>`, invoked once before the text. Renaming `/SA`
to `/S1` in place, the experiment ADR 0688 used on `bug1743245.pdf`:

| | ours | poppler | mupdf |
|---|---|---|---|
| `/SA`, 72 dpi | 9.9448 | 8.6045 | 7.4452 |
| `/S1`, 72 dpi | **7.2840** | 8.6045 | 7.4452 |

Ours falls to the converged value. **Both references are byte-identical**, which is the second
independent confirmation — after ADR 0688's, on a different document and a different measure — that
neither poppler nor mupdf reads Table 57's `/SA` at all.

## Decision

**The head is this tree obeying §10.7.5 and it is held, unchanged.** The clause is not ambiguous
here:

> If stroke adjustment is enabled and the requested line width, transformed into device space, is
> less than half a pixel, the stroke shall be rendered as a single-pixel line.

0.240226 and 0.426 of a device pixel are both under a half at 72 dpi, and 0.480226 is still under
one at 144 — which is why the ladder's disagreement survives to 144 dpi and dies at 288, where
0.961 crosses the threshold. §9.3.6 is what carries the clause to a glyph: "Stroking, filling, and
clipping shall have the same effects for a text object as they do for a path object", so a glyph
outline in mode 2 is stroked under exactly the graphics state a path would be. `Stroke::device_width`
is the one function all three backends call, and it is right.

**Nothing is changed in the code**, and the reason is worth stating rather than assumed: the only
change available would be to stop promoting, which is a `shall` this tree already meets and which
two references miss. Principle 5's direction is the whole of it — agreement with poppler and mupdf
would be evidence we had read the clause right, and their agreeing with *each other* here is
agreement about an entry neither of them reads.

**What is recorded instead is three things**, because a decision with no artefact is a memory:

1. **A `doc/checks/fixed-documents.toml` row** for the page, `reports = []` and `ink = 8.945 ..
   10.945`. Until this round §10.7.5's second requirement was pinned by
   `render-cpu/tests/stroke_width.rs::stroke_adjustment_promotes_only_a_sub_half_pixel_line` — a
   fixture — and by `oracle.rs`'s `AMBIGUOUS_STROKE_ADJUSTMENT`, whose one page is measured on
   *structural similarity* because the promotion there costs no ink at all. This is the first real
   document in the tree where the requirement is worth **2.66 levels of a 7.27-level page**, which
   is 37%, and the band is a twentieth of that.
2. **The §10.7.5 ledger row**, which now carries this witness beside `bug1743245.pdf`'s. The row's
   status does not move: the *first* requirement — grid-fitting the coordinates — is still not
   implemented, and this round found nothing that bears on it.
3. **A third rule for the ink ranking**, below, which is the transferable half.

## The instrument rule, and why it is worth a numbered decision

`doc/oracle-and-corpus.md` §3d's ranking is what every corpus chunk since `doc/todo/03` §8 has used
to choose the document it opens. Round 876 recorded two ways to be wrong with it — `pdftoppm`
without `-cropbox`, and reading our alpha channel as ink. This is a third and it is not an
operating error:

**At 72 dpi the ranking can put this tree at the head of a directory for obeying a clause neither
reference reads.** The ranking's own arithmetic cannot tell that from a defect — it is one
subtraction over three means — and the page *looks* wrong beside the references, boldly and over
every glyph, so trap 1's "look at the page" confirms the false reading rather than breaking it.
Two instruments break it and both are cheap: the ladder, which separates scan conversion from
geometry in four renders, and `/SA`, which is one `grep` over `qpdf --qdf` output.

So the rule is: **before opening a page the ranking calls dark, climb the ladder; and where the
head is dark rather than light, ask whether the document states `/SA true`.** A ranking head that
converges at 288 dpi is the rasteriser's and not the interpreter's, and a dark head on an `/SA
true` document is this clause until something else is shown.

The population it reaches was measured rather than guessed. Over the tracker's 201 openable
documents, **13 state `/SA true`** (`qpdf --qdf --stream-data=uncompress` and a grep), and
**exactly one** of the thirteen is displaced by it: the other twelve either stroke nothing under
half a pixel or stroke too little of the page to move a mean, and they are spread from +0.17 to
−33.59 of the darkest reference. So the condition is not `/SA true` on its own but `/SA true` **and**
a stroke the CTM puts under half a device pixel, which is why the rule above names the ladder first
and the entry second: the ladder fires on the phenomenon, the entry names the cause.

## What was considered and refused

- **Widening the ranking to a higher resolution as a matter of course.** Rejected: the ranking's
  whole value is that it is three commands over a directory, and 288 dpi is sixteen times the
  pixels. The ladder is a *second* instrument for a head, not a replacement for the first.
- **Turning the promotion off below some fraction of a pixel**, so that a 0.24-pixel line stays
  0.24. That is the clause read backwards: the sentence is unconditional once `/SA` is true, and
  §8.4.3.2's own "The actual line width achieved can differ from the requested width by as much as
  2 device pixels" is the standard saying that a device is expected to depart from the arithmetic
  here. It would also make this tree agree with two renderers by matching their output, which
  `CLAUDE.md` forbids outright.
- **Reporting the promotion**, so that a page says it happened. Refused on trap 11's test: the
  condition a report would fire on is *the clause being obeyed*, which is not a shortfall, and a
  page of 108 text runs would say it 108 times. What the tree owes here is a pin, and the
  fixed-documents row is it.
