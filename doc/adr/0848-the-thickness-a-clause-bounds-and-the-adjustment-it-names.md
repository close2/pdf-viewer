# 0848 — The thickness a clause bounds, and the adjustment it names

Date: 2026-09-03. Session 903.
Status: accepted.
Clauses: ISO 32000-2 §10.7.5, §10.7.4, §10.7.1, §8.4.3.2, §8.4.1.
Amends the evidence ADRs 0028, 0419 and 0688 rest on; supersedes nothing.

## Context

§10.7.5's ledger row has been `partial` since the nineteenth session for one stated reason: the
clause has two requirements and this tree implements the second. ADR 0688 gave the first one a
number — 31.43 structural-similarity bounds against `poppler` on `bug1743245.pdf`, the one page
where a reference grid-fits and we do not — and ADR 0844 gave the *second* one its first real
witness. Neither asked the question this round was sent to ask: **is the first requirement's own
bound met, and what would meeting it the other way cost?**

The clause, whole, in the order it states things:

> When a stroke is drawn along a path, the scan conversion algorithm may produce lines of
> nonuniform thickness because of rasterization effects. In general, the line width and the
> coordinates of the endpoints, transformed into device space, are arbitrary real numbers not
> quantised to device pixels. A line of a given width can intersect with different numbers of
> device pixels, depending on where it is positioned.

> For best results, it is important to compensate for the rasterization effects to produce
> strokes of uniform thickness. This is especially important in low-resolution display
> applications. To meet this need, PDF 1.2 provides an optional automatic stroke adjustment
> feature. When stroke adjustment is enabled, the line width and the coordinates of a stroke
> shall automatically be adjusted as necessary to produce lines of uniform thickness. The
> thickness shall be as near as possible to the requested line width -no more than half a pixel
> different.

> If stroke adjustment is enabled and the requested line width, transformed into device space, is
> less than half a pixel, the stroke shall be rendered as a single-pixel line.

So the first requirement is a `shall` whose **action** is qualified — "as necessary" — and whose
**outcome** is bounded by a number: half a device pixel of thickness. The second is an
unconditional action. Both are conditioned on the parameter Table 51 initialises to `false`, which
is ADR 0849's subject and not this one's.

## 1. The instrument, because uniformity is a claim across placement

Every measurement this clause has been argued from — ADR 0419's seventeen widths, ADR 0844's four
resolutions — varies the **width** or the **resolution**. The clause's first requirement varies
neither: it is a claim about one width drawn at different **positions**, and no instrument in this
tree had ever moved that variable.

So: a **phase ladder**. Eight rules of one width on one page, each a further eighth of a device
pixel along, everything else held; ink per rule, and the device-pixel columns it lands in. Thickness
is ink divided by the rule's device length, which is the width the device actually laid down. At
72 dpi on a 200 × 100 page, so one point is one device pixel. Ours through
`pdf-model/examples/render_at`, `poppler` through `pdftoppm -r 72 -cropbox`, `mupdf` through
`mutool draw -r 72`, `ghostscript` through `-sDEVICE=png16m -dGraphicsAlphaBits=4`, `hayro` through
`pdfref-hayro`. Each document was built twice, with `/SA true` and with no `/ExtGState` at all.

**Axis-aligned, requested width 0.6 of a device pixel** — thickness achieved, over the eight
placements:

| | min | max | spread | worst departure from 0.6 | columns touched |
|---|---|---|---|---|---|
| **ours** | 0.5961 | 0.6000 | **0.0039** | **0.0039** | 1 or 2 |
| `mupdf` | 0.5882 | 0.6471 | 0.0588 | 0.0471 | 1 or 2 |
| `ghostscript` | 0.7373 | 0.8039 | 0.0667 | 0.2039 | 1 or 2 |
| `poppler` | 1.0000 | 1.0000 | 0.0000 | **0.4000** | 1, always |
| `hayro` | 1.0000 | 1.0039 | 0.0039 | 0.4039 | 1 or 2 |

**Axis-aligned, requested width 1.0:**

| | min | max | spread | columns |
|---|---|---|---|---|
| ours | 1.0000 | 1.0039 | 0.0039 | 1 or 2 |
| `poppler` | 1.0000 | 1.0000 | 0.0000 | 1, always |
| `mupdf` | 1.0000 | 1.0039 | 0.0039 | 1 or 2 |
| `hayro` | 1.0000 | 1.0039 | 0.0039 | 1 or 2 |
| `ghostscript` | 1.2667 | 1.2745 | 0.0078 | 2, always |

**`/SA` changes nothing for any of the four references, at either width, in either direction.**
That is ADR 0688's finding confirmed a third time and ADR 0844's a second, now on a document
written for the purpose: `poppler` grid-fits whether the document asks or not, `hayro` floors at
one device pixel whether it asks or not, and `mupdf` and `ghostscript` never read the entry. Only
this tree's raster moves when `/SA` moves.

**And ghostscript without anti-aliasing is the control that says what the clause's own algorithm
does**: `-sDEVICE=png16m` at its default draws the 1.0 rule as **two solid columns at every
placement** — thickness 2.0, uniform, and a whole device pixel from the requested width. That is
§10.7.4 read literally, and it is the reading under which the first requirement's second sentence
does real work.

## 2. Turned, which is where this device is worst

The same ladder at 45°, eight placements an eighth of a device pixel apart along the rule's
perpendicular, ink against the rule's own area, spread expressed as device pixels of width:

| requested width | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
|---|---|---|---|---|---|
| 3.0 | **0.0028** | 0.0286 | 0.0416 | 0.0501 | 0.0056 |
| 1.0 | 0.1802 | 0.1936 | 0.0416 | 0.1878 | 0.0028 |
| 0.6 | 0.1108 | 0.1859 | 0.0028 | 0.2337 | 0.0028 |

Two things, and the first is the one worth keeping. **Above the substitution band this tree is the
most uniform of the five** — 0.0028 of a device pixel at width 3.0, which is the closed form.
**Inside it we are the second worst**, at 0.1802, and the reason is ours rather than
anti-aliasing's: at or under one device pixel a turned rule takes `pdf_render::substitute_width`
(ADR 0268, boundary ADR 0285), which states the mark one device pixel wide and carries the width it
gave up in the paint's alpha, and that substitution is not exact across placement the way the
closed form is. `mupdf` and `hayro` are both better there.

It is still inside the clause's bound — 0.18 against 0.5 — so it is a margin rather than a debt,
and it is recorded as the worst number this device produces for the quantity §10.7.5 bounds.

## 3. The population, over three corpora

`pdf-model/examples/stroke_adjustment_census` is new and is the instrument the decision needed.
`examples/absence_audit`'s §10.7.5 block counts documents that *state* `/SA true` in a dictionary;
this one counts what reaches the display list, and then how much of it a grid fit could move at
all: a stroke the second requirement already promotes has nothing left to adjust, a curve or a
diagonal has no pair of edges to put on integers, and an axis-aligned run whose edges are already
on the grid is where a fit would put it.

| | files | pages painting a stroke under `/SA` | pages a fit could move | strokes | under `/SA` | promoted | axis-aligned, not promoted | off the grid |
|---|---|---|---|---|---|---|---|---|
| `doc/pdf.js` | 974 | 16 | **6** | 4 054 129 | 778 | 96 | 466 | **460** |
| curated | 1251 | 21 | 7 | 4 081 289 | 1060 | 308 | 478 | 472 |
| `CC-MAIN-2021-31` | 65 944 | 8378 | **4832** | 12 959 746 | 1 836 739 | 1 343 558 | 123 216 | **119 672** |

**The widening changes the answer and is why it was taken** (ADR 0490's rule, `--bin
undenominated`'s subject). Over the corpus every gate in `doc/todo/02` §2 walks it is six documents
of nine hundred and seventy-four, 438 of the 460 strokes on one of them (`issue14297.pdf`); over
the crawl it is **7.4% of the pages that open**. A decision resting on the first number alone would
have been a decision about `doc/pdf.js`.

One figure in that table is worth reading on its own: of the 1 836 739 crawl strokes drawn with the
parameter enabled, **1 343 558 — 73% — are under half a device pixel**, which is the requirement
this tree already implements. The clause's two halves are not equal partners in the world: three
strokes in four that ask for stroke adjustment are asking for the promotion.

## 4. Decision

**The coordinate adjustment is declined, the row stays `partial`, and the reason is the sentence's
own bound rather than the architecture.**

It is not declined for cost. It can be built here, and the reading found how cheaply: the fit is a
**device-space translation**, so it needs one function in `pdf-render` beside `Stroke::device_width`
and three call sites — `render_cpu::convert::stroke`, `render_gpu::scene::stroke`,
`render_quorra::stroke` — each composing the returned offset into a transform it already builds. No
path is rewritten, so the `Arc<Path>` sharing that exists because glyph outlines dominate a text
page is untouched, and quorra's `cache::StrokeKey` (ADR 0402) is unaffected because the offset
travels in the transform rather than in the key. Saying "it cannot be done in this architecture"
would have been false, and the architecture question was asked before the clause question was
answered so that it could not become the answer.

It is declined because **quantising a stroke's coordinates to the device pixel grid breaks the
other half of the same sentence on a device that anti-aliases.** Both edges of a rule land on
integers only if the device width is a whole number, so a 0.6-pixel rule fitted to the grid is
drawn 1.0 pixels thick — which is exactly what `poppler` does, and it is **0.4 of a device pixel
from the requested width where this tree is 0.0039 from it**, a hundredfold on the quantity the
clause's second sentence bounds. On the aliased device §10.7.4 describes the two halves are
compatible, because the achieved thickness is a whole number of pixels either way; on the
anti-aliasing device §10.7.1's NOTE permits, they pull against each other, and the sentence says
which one wins — *the thickness shall be as near as possible to the requested line width*.

So on this device the adjustment is not "as necessary": the outcome it exists to produce is already
produced, to within one and a half levels of an eight-bit raster, and every route to performing the
adjustment as written moves the bounded quantity away from the bound.

**Three things are recorded instead**, because a decision with no artefact is a memory:

1. **`render-cpu/tests/stroke_width.rs::stroke_adjustment_holds_the_thickness_within_half_a_pixel_at_every_placement`**
   — the phase ladder as a test: three widths, eight placements, two scales, held to the clause's
   own half-pixel bound and to a tenth of it. The tight bound is eight times the worst departure
   measured (0.0059 of a device pixel, at width 2.5) and it was checked against something rather
   than against nothing: at 0.005 it fails (trap 13). `poppler`'s construction reads 0.4 here and
   would fail it while staying inside the clause's bound, which is what makes the test about this
   decision rather than about arithmetic.
2. **`pdf-model/examples/stroke_adjustment_census`**, §3's instrument, with the three scopes
   `long_mitre_census` established, so the population can be re-asked when the corpus grows.
3. **§10.7.5's ledger row**, which keeps its status and loses its old argument. The row said the
   non-uniformity a grid fit removes "is an artefact of the binary scan conversion §10.7.4
   describes, which this tree already departs from by anti-aliasing", and that "nothing reports it
   because there is no page on which this device could do better". The first half is right and was
   unmeasured for eight hundred sessions; the second half is now false in the plain sense — 4832
   crawl pages would draw differently — and true in the sense that matters, which the row now
   states as a number rather than as a claim.

## 5. What is honestly left, and it is a reading rather than a debt

**The clause never defines "thickness", and the two readings available give opposite verdicts.**
Read as the ink the device lays down across the rule, this tree meets the bound by a factor of
eighty and no reference meets it at the requested width. Read as *the number of device pixels the
line intersects* — which is what the clause's own first paragraph names as the defect, "[a] line of
a given width can intersect with different numbers of device pixels, depending on where it is
positioned" — this tree does not meet it and `poppler` does, on axis-aligned rules only.

This ADR chooses the first, and the choice is stated as one:

- the second reading measures the output of the **aliased** algorithm. On an area-sampling device
  the number of pixels a rule touches is not its rendered width; the coverage profile is. Reading a
  bound stated in device pixels of *width* as a count of *touched pixels* imports the algorithm
  §10.7.1's NOTE says is not defined by PDF;
- §8.4.3.2's neighbouring sentence — "[t]he actual line width achieved can differ from the
  requested width by as much as 2 device pixels, depending on the positions of lines with respect
  to the pixel grid" — is a `can` describing that same algorithm, not a definition of thickness,
  and the aliased control run in §1 is what two device pixels looks like;
- and the first reading is the only one under which the sentence's two halves can both be honoured
  at once on this device, which is §4.

**The status stays `partial` under the ledger's own vocabulary rather than under this reading.**
`implemented` means every normative requirement in the clause is *executed*; the first requirement's
action is not executed, whatever one concludes about its outcome, and a row that hid an unexecuted
`shall` behind a favourable reading is the shape this ledger exists to prevent.

## 6. What was considered and refused

- **A half-integer fit — snap the rule's centreline so a sub-pixel rule lands wholly inside one
  column.** It is better than `poppler`'s: one column at every placement *and* the exact requested
  thickness, so it satisfies both readings of §1. It is refused because it is this project's
  construction rather than the clause's — the clause says the coordinates are quantised to device
  pixels, not to half-pixels — and because of a hazard it shares with `poppler`'s and the clause is
  silent about: **a fitted stroke moves and an abutting fill does not.** A cell border drawn under
  `/SA` beside a fill that is not opens a gap of up to half a device pixel, and §11.6.2's rule about
  portions of one object does not reach two different objects. Worth building the day the owner
  wants `poppler`'s picture; not worth inventing under a `shall` that is already satisfied.
- **Reporting a stroke the fit would have moved.** Trap 11: the condition would fire on the clause
  being met, on 7.4% of the crawl's pages, up to 438 times on one page of `doc/pdf.js`. ADR 0844
  refused the same shape for the second requirement and the reasoning is unchanged.
- **Moving the row to `implemented`.** §5.
- **Fixing the turned rule's 0.1802.** It is §10.7.4's substitution rather than this clause's, it is
  inside this clause's bound, and it belongs to `doc/todo/11` beside the two marks that are still
  lost. Recorded here because this is the instrument that found it.

## 7. What this cost the cross-backend comparison, which is nothing, and why that was not obvious

The round was sent to price the oracle, on the ground that a coordinate adjustment changes geometry
the oracle compares. Measured rather than assumed, the price splits in two:

- **Between our own three rasterisers there is none, and there would have been none had the fit been
  built.** The offset is computed in `pdf-render` from `to_device` and consumed by three call sites
  that already share `Stroke::device_width`, so all three backends would receive the same geometry —
  the same argument ADR 0285 made for the width, one quantity over. What would *not* survive is a
  fit applied to a **glyph** outline under `/SA`, because quorra quantises glyph placement to reuse
  its atlas (`glyph_quantum`, ADR 0498) and a sub-pixel translation and a sub-pixel quantum are two
  policies for one number; that is a second reason a fit must be confined to axis-aligned runs, and
  a glyph outline is a curve, so §3's census already excludes every one of them.
- **Against the four references it is six pages of `doc/pdf.js`**, named by the census:
  `annotation-underline.pdf`, `bug1743245.pdf`, `issue13325_reduced.pdf`, `issue14297.pdf`,
  `issue15629.pdf` and `issue21570.pdf`. One of those is `AMBIGUOUS_STROKE_ADJUSTMENT`'s only page,
  whose whole derivation is that `/SA` decides a pixel for this tree and for nobody else; a fit
  would move it toward `poppler` and away from `mupdf`, `ghostscript` and `hayro` at once, and the
  note would have to be rewritten around a different mechanism. Nothing in this round moves a pixel,
  so none of that is spent.
