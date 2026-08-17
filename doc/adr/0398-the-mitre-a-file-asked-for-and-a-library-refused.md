# 0398 — The mitre a file asked for and a library refused

Status: accepted
Date: 2026-08-17
Session: 563

Takes `doc/todo/11` §6, opened by the five-hundred-and-fifty-eighth session: `pdf-differences`'
`LargeMitreLimit.pdf` states `333 M` on a 10-unit line, ISO 32000-2 §8.4.3.5 admits a mitre 166.676
line widths long, and this tree drew a bevel. Amends the ledger's §8.4.3.5 and §8.4.3.4 rows and
`doc/todo/_scan-conversion.md`.

## Context

### What the clause requires, and the closed form it states

§8.4.3.4 says what a mitre join is — "[t]he outer edges of the strokes for the two segments shall be
extended until they meet at an angle, as in a picture frame" — and §8.4.3.5 bounds it:

> The miter limit shall impose a maximum on the ratio of the miter length to the line width (see
> "Figure 15 -Miter length"). When the limit is exceeded, the join is converted from a miter to a
> bevel.

The clause then states the ratio as a formula, which `doc/md/` cannot carry (its line reads
`formula-not-decoded`; `pdftotext -layout` over `doc/ISO_32000-2_sponsored_EC3.pdf` prints it):

```text
miterLength / lineWidth = 1 / sin(φ / 2)
```

for the angle φ between the segments in user space. Two more sentences finish it: "[w]hen the line
width is zero, the miter length is zero", and a NOTE of one line — "Very large miter lengths are
allowed."

**The clause checks the reading itself**, which is worth doing before writing code against it. Its
EXAMPLE says a limit of 1.414 converts miters to bevels for φ under 90°, 2.0 for φ under 60°, and
10.0 for φ under approximately 11.5°; `1/sin 45° = 1.41421`, `1/sin 30° = 2`, `1/sin 5.75° = 9.98`.
So the ratio is the *whole* join's length over the width, and since the two inner offset lines cross
as far below the vertex as the two outer ones cross above it, **the tip sits `(w/2) / sin(φ/2)` from
the vertex** — half the mitre length. Every number in this round is that expression.

Three consequences, each of which decides code:

- **A limit is a maximum, not a length.** Over it the join is a bevel, not a mitre truncated at the
  limit. PDF has no spelling for the truncated form; SVG's `miter-clip` and `tiny-skia`'s
  `LineJoin::MiterClip` are that other thing, and neither is what `M` selects.
- **The result may be enormous, by the standard's own NOTE.** A processor that imposes a ceiling of
  its own has substituted its judgement for the file's.
- **The ratio is a function of the geometry and the width alone.** No device quantity enters it, so
  it is decidable in the crate both backends share.

### The witness, and what the corpus article adds

`doc/corpora/pdf-differences/LargeMitreLimit/` is four hand-written joins on one page, `10 w`,
`333 M` and `333.3276 M`, at φ = 0.687516° — reduced by session 558 to four lines. Its README is a
PDF Association article about this exact defect: "some implementations imposed additional
unspecified maximum limits on the mitre limit when an explicitly specified mitre join would be
silently ignored and converted to the much less visible bevel join, against the PDF files explicit
request". It tabulates mitre *lengths* — 1666.8 units at this angle — and rules the page with a grid
every 100 units so that a reader can measure what was drawn. It is evidence that the reading above
is the one the clause's authors meant; it is not where the arithmetic came from.

### What was actually wrong, which is not what the todo file predicted

`doc/todo/11` §6 said a fix "is three strokers rather than one", because Vello and quorra "have
their own strokers with their own thresholds". **Measured, that is wrong, and the measurement is the
round's first finding.** `render-quorra/examples/mitre_ladder` walks one join from 45° to 0.2° at
`333 M` on all three backends:

The tip's page-space height, in a page whose join sits at 200 and whose ink is measured to the
last row carrying any:

| ratio | processor (before) | vello | quorra | clause |
|---:|---:|---:|---:|---:|
| 88.15 | tip at 635 | 641 | 641 | 640.75 |
| 90.23 | 646 | 651 | 651 | 651.16 |
| 95.50 | **the join itself** | 678 | 677 | 677.47 |
| 166.68 | **the join itself** | 1033 | 1033 | 1033.38 |
| 333.36 | the join itself | the join itself | the join itself | bevel: over `333 M` |

So exactly one of the three was wrong, the cutoff is between 90.23 and 95.50, and the last row is
the clause's own conversion working everywhere. The cause is `tiny-skia`'s `dot_to_angle_type`,
which classifies a join by the dot product of the two segments' normals *before* the limit is read
and calls anything within `SCALAR_NEARLY_ZERO` — `1/4096` — of −1 `Nearly180`. Since the library's
own comment gives `sin(φ/2) = sqrt((1 + dot) / 2)`, that angle test is the **ratio** `1 /
sqrt(1/8192)` = 90.51 in disguise: a sharper join is bevelled with the file's limit unread.

### The population

`pdf-model/examples/long_mitre_census` counts joins whose ratio the file's own limit admits and the
stroker refuses, over first pages: **2 of 1441** — `LargeMitreLimit.pdf` and
`LargeMitreLimit-Beziers.pdf`, 8 strokes, sharpest ratios 166.677 and 111.125, longest mitres 1666.8
and 1111.3 device pixels. None of the 974 pdf.js documents, none of the fourteen specifications,
none of `format-corpus`, `pdf20examples` or `pdfbox`. One page of those corpora states such a limit
without having such a join, which is the pre-filter firing and finding nothing.

That number cuts both ways and the round is written around both edges. It says the change may not
cost the ordinary stroking path anything measurable, and it says the corpus cannot be the
instrument: **no gate in `doc/todo/02` §2 renders either witness**, so the whole verification is the
clause's arithmetic in a cross-backend scene.

## Decision

### 1. The geometry is stated in `pdf-render`, and the library's own limit is stated in the backend

Trap 2's first instance is the rule: *where two backends are the oracle, a decision either can make
alone is a decision neither has made* — which is why the device decisions live in `pdf-render`. A
per-library workaround inside `render-cpu`'s stroking code would have been exactly the shape that
rule forbids, and so would `content.rs` predicting a backend's join code from the layer above it
(`doc/todo/45`).

The split that satisfies it is the one `crate::degenerate` and `crate::sub_pixel` already use:

- **`pdf_render::mitre_wedges` decides and constructs.** Given a path, a stroke and a half-width it
  answers, from §8.4.3.5's formula alone, which joins the limit admits and where each tip goes. It
  is the only place in the tree that turns `M` into geometry, and its unit tests are written against
  the closed form.
- **`render-cpu` owns the one number that is a fact about `tiny-skia`.** `BEVELLED_BY_THE_STROKER =
  90.51` is derived from that library's `SCALAR_NEARLY_ZERO` in a comment that shows the algebra,
  and passed *into* the shared crate as the ratio the caller cannot draw. `pdf-render` therefore
  contains no library's threshold and the backend contains no clause arithmetic.
- **The other two backends call nothing.** They draw the clause already, measured, at every rung of
  the ladder; adding a construction they do not need would be the more invasive change, and ADR
  0226 set the precedent — "the graphics device never had either fault, so this is the *oracle*
  being brought up to it".

What holds that arrangement honest is that the *gate* is on all three. Each backend is held to
§8.4.3.5's own arithmetic rather than to the others' pixels, so the day a library changes its mind
in either direction, `render-quorra/tests/mitre_limit.rs` fails and says which one.

### 2. A mitre is a bevel plus a triangle, and the two are drawn as one path

§8.4.3.4's bevel "finish[es] the segments with butt caps" and fills "the resulting notch beyond the
ends of the segments … with a triangle"; the mitre extends the same two outer edges until they
cross. So the mitre's outline is the bevelled outline plus one triangle per join, whose base **is**
the bevel's outer edge and whose apex is the tip. That decomposition is why nothing needs to be
approximated: the two shapes share an edge, overlap nowhere, and their union is the mitred outline.

`render-cpu` therefore asks `tiny-skia` for the outline of the same stroke with **bevel** joins,
appends the wedges to it in one `tiny_skia::Path`, and fills it once under the non-zero rule — which
is what `stroke_path` does with an outline anyway, so nothing about the ordinary geometry changes.

**Two draws would have been wrong, and the witness proves it rather than illustrating it.** Two
marks sharing an edge composite by §11.3.7.3's union function and leave a seam along it — the
measured departure `doc/todo/_scan-conversion.md` item 5 records — where coverage accumulated inside
one scan conversion adds. `LargeMitreLimit.pdf` sets `/CA 0.6` on the stroke, so the seam would have
been visible rather than theoretical, and the gate asserts the ink at alpha 0.6 as well as at 1.0
for that reason. The non-zero rule matters for the other half: a triangle outside the outline is
filled whichever way it winds, where the even-odd rule would have punched it out.

**On a path that needs one wedge, every admitted mitre on that path becomes ours**, because a
`tiny_skia::Stroke` carries one join style for the whole path. That is the deliberate choice rather
than the incidental one: it means no join on such a path is drawn by a threshold nobody stated, and
the moderate joins get the same triangle the library would have drawn.

### 3. What is declined, and why each decline is not a silence

- **A dash pattern.** A dash decides where a stroke still has a join, and the walk is over the
  undashed path, so a wedge could be added at a vertex the dasher has cut away. Declined in
  `pdf-render`, where the reason is stated. The census says no document reaches it.
- **A stroke at or under one device pixel.** There `tiny-skia` draws a hairline, which has no joins
  at all, and §10.7.4's own substitutions (ADRs 0226, 0268, 0290) own the geometry — a wedge under
  the coverage quantum is their question, not this one. Declined in `render-cpu`, which is where the
  hairline lives. The census says no document reaches it either.
- **A join that doubles back exactly, and one that does not turn.** The clause answers both: the
  ratio is unbounded for the first, so every finite limit is exceeded, and §8.4.3.4 makes join
  styles "significant only at points where consecutive segments of a path connect at an angle".

Neither decline is reported at runtime, and the argument is the one `doc/todo/11` §6 asked for. A
report's condition would have to be "this backend's stroker would refuse a mitre this file admits",
which is a prediction about a library made in the layer above it; and the two declines are
*unwitnessed* — a report with no members costs gated pages and buys nothing (trap 11). What does
watch them is `render-quorra/tests/corpus.rs`: the processor and the device now agree about a long
mitre, so a page where a decline mattered would show as a differing page there rather than as
silence.

### 4. The entry test is one comparison, and that is what makes the change free

A join this construction is owed for has a ratio **over** the caller's threshold and **at or under**
the file's own limit, so a stroke whose `M` is at or under that threshold cannot have one — whatever
its geometry. `mitre_wedges` tests that first and returns without walking the path. Table 51's
initial limit is 10 and the threshold is 90.51, so every stroke in every corpus document takes the
early exit.

Measured with callgrind at `RAYON_NUM_THREADS=1`, on `issue12295.pdf` page 1 — 65 859 strokes, the
corpus's stroke-heaviest page — and on page 101 of ISO 32000-2: see this session's history file for
the two figures. The cost of the change on a page with no long mitre is that comparison per stroke.

## Consequences

- `pdf-differences`' two `LargeMitreLimit` documents draw their four spikes, on all three backends,
  to the length §8.4.3.5's ratio states. `mutool` and `ghostscript` put the tip within a few pixels
  of the same place, which is evidence that the reading is right; `poppler` still puts its highest
  ink **at the join** on the first file and reaches 199 units of the clause's 555.6 on two of the
  second's four cases, which is the other kind of evidence.
- **No corpus or oracle verdict moves**, because no gated document has such a join — the census is
  what says so in advance, and the run confirms it.
- `pdf-render` gains one module and one public pair, `mitre_wedges` and `sharpest_admitted_mitre`;
  the second is the census's instrument and shares the first's walk, so a count of what documents
  state and the construction that draws it cannot disagree about where a path's joins are.
- The wedge walker and `crate::outline`'s bound walker are held together by a property rather than
  by shared code: `the_bound_contains_every_tip` fails if either sees a vertex the other does not,
  on a curve's tangent or at the point a `Close` turns into a join.
- One thing this round did **not** find, and it is recorded because the arithmetic suggested it:
  `scan::stroke` asks its anti-aliasing guard with an outset of `width × miter_limit`, which for
  `333 M` exceeds the scan converter's ±8191 coordinate range at four times scale, so the guard
  looked like it must be turning anti-aliasing off on the witness. It is not: the same page's 45°
  arm carries eleven partial coverages at 4× before the change and sixteen after. The claim is
  dropped rather than written down.
