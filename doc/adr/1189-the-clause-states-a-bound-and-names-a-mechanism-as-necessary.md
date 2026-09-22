# 1189 — §10.7.5 states a bound and names its mechanism "as necessary", so the departure is re-grounded on the clause

Session 1176. Status: **accepted**.
Context: `crates/pdf-render/src/paint.rs` (`Stroke::adjust`, `Stroke::device_width`,
`thinnest_line`), `crates/pdf-model/src/content/ext_gstate.rs` (`/SA` read from Table 57),
`crates/render-cpu/src/convert.rs`, `crates/render-gpu/src/scene.rs`,
`crates/render-raster/src/stroke.rs`, `crates/render-cpu/tests/stroke_width.rs`,
`doc/conformance/ledger.toml` §10.7.5.
Amends: ADR 0028 and ADR 0848 — the ground of the departure, not its verdict.
Answers: ADR 0419 section 6 (stroke adjustment as a host preference), which is declined here on
the clause. Completes ADR 1165, which re-derived eleven of the twelve `departed` rows and left
this one to another round.
Builds: ADR 0208 and ADR 0226 (the boundary against §10.7.4), ADR 0268, ADR 0285, ADR 0476,
ADR 1082 (the scan converter this re-measurement runs on), ADR 1119 (the word `departed`).
Clauses: ISO 32000-2 §10.7.5, §10.7.4, §10.7.1, §8.4.5 (Table 57), §8.4.3.2.

## 1. The clause, read whole, is one bound and two `shall`s of different kinds

§10.7.5 is four paragraphs and a NOTE, and the grammar of each matters.

Its first paragraph is descriptive and its verb is `may`: "the scan conversion algorithm may
produce lines of nonuniform thickness because of rasterization effects", because "the line width and
the coordinates of the endpoints, transformed into device space, are arbitrary real numbers not
quantised to device pixels". That is the hazard, stated as a possibility of *an* algorithm.

The second paragraph carries the first `shall`, and it is conditional in its own words:

> "When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
> automatically be adjusted as necessary to produce lines of uniform thickness. The thickness shall
> be as near as possible to the requested line width -no more than half a pixel different."

Three things are in that sentence. *Adjusted* is the mechanism. *As necessary* conditions it.
*To produce lines of uniform thickness* states what the necessity is measured against, and the
sentence that follows puts a number on it: no more than half a device pixel from the requested
width. The clause does not say "grid-fit the coordinates"; it says reach a bound, and names moving
the width and the coordinates as how a device that needs to would get there.

That reading is not charity toward this implementation — it is what §10.7.1 says the clause may do:
"The specifics of the scan conversion algorithm are not defined as part of PDF. Different
implementations can perform scan conversion in different ways; techniques that are appropriate for
one device could be inappropriate for another." A clause inside §10.7 that bounded an *outcome* and
conditioned its *mechanism* is exactly what a clause inside §10.7 is able to write.

The third paragraph carries the second `shall`, and it has none of that structure — a stated
antecedent, a stated consequent, no "as necessary" and no purpose clause:

> "If stroke adjustment is enabled and the requested line width, transformed into device space, is
> less than half a pixel, the stroke shall be rendered as a single-pixel line."

So: **`/SA true` obliges a processor to one measurable bound and one unconditional substitution.**
The substitution is performed, in `Stroke::device_width`, which all three rasterisers call — the
same function that answers §8.4.3.2's zero width, because this clause's own NOTE says they are the
same width. The bound is met, and section 3 re-measures by how much.

## 2. When the document says nothing, a processor may not adjust

Table 57's `/SA` sets the parameter and Table 58 gives it the initial value `false`. The clause's
last paragraph says whose decision it is: "Because automatic stroke adjustment can have a
substantial effect on the appearance of lines, PDF provides means to control whether the adjustment
shall be performed."

A processor that adjusts when the parameter is `false` has not exercised a device-dependent
permission — it has overridden a graphics-state parameter the producer set, by writing it or by
taking the initial value the standard supplies. That is a departure from the file, and `CLAUDE.md`'s
target is that "[e]very PDF that exists renders as its producer specified".

This decides ADR 0419 section 6, which had deferred "stroke adjustment on when the document has not
asked for it" as a host preference for which "[n]othing is built for it now". Three answers, in
order of strength:

1. **The clause forbids it.** The parameter exists to control the adjustment and the document has
   set it. A preference that overrides it is a preference to render contrary to the file, which is
   not the same species as the values `viewer_host::policy` carries — those answer questions about
   *this machine and this reader* that no document states.
2. **`CLAUDE.md`'s four levels are not this channel.** Principle 3's levels are for "the permissions
   a *document* asserts over the person reading it — Table 22's `/P` flags, §12.8.2.2's `/DocMDP`,
   §12.8.6's usage rights". `/SA` asserts nothing over a reader; it is a producer saying how their
   own marks should look.
3. **It would make this device worse on the one quantity the clause bounds.** The preference is for
   the references' picture, and the references reach it by drawing a 0.6-device-pixel rule one
   device pixel thick: 0.4000 from the requested width, where this device measures 0.0039. A
   preference whose only effect is a hundredfold regression against the clause's own number is not
   one to add a channel value for.

So the preference is declined, and it is declined on the clause rather than on cost. Nothing about
the host channel having grown reopens it.

## 3. The measurement re-taken, and the one figure that had gone stale

ADR 0848 measured a **phase ladder**: eight rules of one width, each an eighth of a device pixel
further along, ink over the rule's device length. Two things had happened to it since.

- It measured `render-cpu` only, through `pdf-model/examples/render_at`. Its cross-backend claim was
  architectural — all three rasterisers call `Stroke::device_width` — not measured.
- **ADR 1082 replaced the scan converter underneath half of it.** Its axis-aligned rungs go to
  ADR 0476's exact rectangle closed form and ADR 0226's band route, which ADR 1082 explicitly
  declined to touch. Its 45° rungs are diagonals, which is precisely the population ADR 1082 moved
  off `tiny-skia`'s sixteenth-lattice supersampler onto `render-cpu/src/area.rs`'s analytic area.

The ladder was re-run for this ADR on today's `render-cpu`: 128 rungs — two orientations, four
widths (0.6, 1.0, 2.5, 3.0 user units), two scales (1.0 and 2.0), eight placements each, with and
without `/SA true`. The harness validates itself on the half ADR 1082 did not touch, reproducing
that ADR's figures to the digit.

| ladder | ADR 0848 | today | clause's bound |
|---|---|---|---|
| axis-aligned, requested 0.6 device px | 0.0039 | **0.0039** | 0.5 |
| axis-aligned, worst over all widths | 0.0059 | **0.0078** (at scale 2.0) | 0.5 |
| 45°, requested 0.6 device px | 0.1108 spread | **0.0055** spread, 0.0038 worst | 0.5 |
| 45°, requested 1.0 device px | 0.1802 spread | **0.0029** spread, 0.0018 worst | 0.5 |
| 45°, requested 3.0 device px | 0.0028 spread | **0.0028** spread | 0.5 |

So the axis-aligned figure still describes this device, and **the turned one does not**: ADR 0848's
0.1802 — the number its own ledger row calls "[t]he worst thickness this device produces for the
quantity the clause bounds", and the one rung where that row says we are worse than `mupdf` and
`hayro` — was taken on a scan converter that no longer exists. ADR 1082 took it to 0.0038 without
anybody noticing, because the gate that covers this ladder
(`render-cpu/tests/stroke_width.rs::stroke_adjustment_holds_the_thickness_within_half_a_pixel_at_every_placement`)
holds only axis-aligned rules, and at a tolerance eight times the recorded worst.

The worst departure this device now produces for the quantity §10.7.5 bounds is **0.0078 of a device
pixel**, over all 128 rungs. The clause allows 0.5. `/SA true` and no `/ExtGState` at all agree
exactly at every rung above half a pixel, which is the promotion's antecedent being honoured rather
than assumed.

## 4. What the revisit note claimed, and which claims held

`doc/adr_revisit/0419-…` is an argument, and it was tested rather than followed.

- **Held.** The channel ADR 0419 section 6 named as the preference's home is built, many times over:
  `viewer_host::policy` now answers about import data, extraction, restrictions, machine fonts,
  submission, links, trust anchors, reference files, the audience and the clock. "Nothing is built
  for it" has indeed expired as a *reason*. It is not the reason the preference is declined here.
- **Held.** ADR 0493's population is what the note says: 19 211 of 65 703 crawled documents state
  `/SA` as a name. ADR 0848's census refines which half of the clause they are asking for —
  1 343 558 of 1 836 739 crawl strokes drawn with the parameter enabled are under half a device
  pixel, so **three in four are asking for the requirement this tree performs**.
- **Did not hold.** The note says the decision "rests on a device-floors argument written before
  ADR 0262 made the renderer composite in document-stated blending spaces". ADR 0028 decides two
  separate things, and that is the premise of the *other* one. The colourant premise — "this device
  is a screen with three additive process colourants and no separations" — belongs to the
  overprinting decision. The stroke decision's premise is "a device that quantises coverage to whole
  pixels, and this is not such a device", which is a property of the rasteriser and not of the
  display. The row was never grounded on a screen premise, so there was no screen premise to expire.

It is re-grounded on the clause anyway, because the clause is a better ground than a claim about our
own rasteriser: a premise about this device expires when the device changes, and section 1's reading
does not.

## 5. The row stays `departed`, and the residue is one undefined word

Two requirements, one of them executed, would make the status `partial`. It is not that, and it is
not `implemented` either, for a reason worth stating precisely so a later round does not move it on
a cheerful reading of section 1.

The clause never defines **thickness**. Under the coverage-weighted reading — ink over device length,
which is what an anti-aliasing device deposits and what section 3 measures — the bound is met by a
factor of sixty-four. Under a touched-pixel-count reading, a 0.6-pixel rule that lands across two
pixel columns is "two pixels thick" at one placement and "one" at another, and no anti-aliasing
device meets the uniformity sentence at all while `poppler`'s grid fit meets it exactly.

`departed` is the honest word for that: ADR 1119 defines it as a requirement decided against with
its cost recorded, and the cost recorded here is that **on one reading of an undefined word this
device does not produce uniform thickness, and the mechanism the clause names for reaching
uniformity is not performed**. What is no longer part of the record is that the departure is
expensive: it costs 0.0078 of a device pixel against a permitted 0.5, and performing the mechanism
would cost 0.4000.

## 6. What this does not close

The ladder is still `render-cpu`'s. `render-raster` expands on device-space geometry with one scalar
width — its own source says that is "exact for a similarity transform and exactly wrong for any
other" — so a phase ladder on `render-raster` and `render-gpu` remains unrun work, and the
cross-backend claim remains architectural. The gate that covers the ladder holds axis-aligned rules
at a tolerance of 0.05; a turned rung and a tighter tolerance would put section 3's numbers under a
gate instead of under an ADR. Neither is built here: `crates/render-cpu/**` is another round's this
batch.
