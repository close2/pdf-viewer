# ADR 0555 — The offset was ours, and the lane that looked exact was not

Status: accepted, 2026-08-23. Session 699. Corrects `doc/QUORRA_FEEDBACK.md` §31 in place, adds
`doc/QUORRA_FEEDBACK.md` §38.3, and amends §10.7.4's ledger row. No code changes: the finding is
that the code was right and the report about it was not.

## The claim this retracts

§31 of `doc/QUORRA_FEEDBACK.md` — this tree's five-hundred-and-seventy-eighth session — reported
that quorra's two coverage lanes "disagree about **where a mark goes**, by up to an eighth of a
device pixel", that "the default lane carries a sub-pixel offset that the gpu lane does not", and
that on `bug1743245.pdf` "the oracle and the gpu lane both draw what the file states, to three
decimals, and the default lane does not". It offered a hypothesis — "that is what a position
quantised once per command to a ¼-pixel grid looks like from the outside" — and asked quorra two
questions on the strength of it.

**Every sentence of that reading is inverted.** The default lane draws what the file states; our
own oracle carries the offset; and the sampled lane looked exact on that page by coincidence.

## How it was settled

quorra's reply of 2026-08-23 (`doc/QUORRA_CLIP_LANE_AND_UPLOAD.md` §4) stated a falsifiable
prediction in the honest form — that the `quorra_scene::Affine` handed to `render-quorra` and the
`tiny_skia::Transform` handed to `render-cpu` differ by a scale of 0.998899 and an offset of
+0.1571 device pixels, and that **if the two are equal to the bit their conclusion is wrong**. Two
prints, one in each backend's stroke encoder, over `bug1743245.pdf`'s 536 stroked rules:

```
CPU  stroke at = Transform { a: 0.31695467, b: 0.0, c: 0.0, d: -0.31735003, e: 313.62668, f: 0.31732178 }
QUO  stroke to_device = Transform { a: 0.31695467, b: 0.0, c: 0.0, d: -0.31735003, e: 313.62668, f: 0.31732178 }
```

Equal to the bit, on every rule, in both axes. They are the same expression by construction —
`Encoder::placed` is `affine(t.then(self.target.transform))` and `ToDevice::of` is
`transform.then(self.page).then(translate(0, −rows))` with `rows` zero for a whole-page surface —
so the prediction could not have held, and the transform was never where to look.

## Where it actually was, and the A/B that says so

`render-cpu` does not hand `tiny-skia` a sub-pixel axis-aligned rule at all. `draw_sub_pixel_rule`
sends it to `pdf_render::sub_pixel_bands`, which is **this tree's own §10.7.4 substitution** (ADR
0226): a rectangle thinner than a device pixel is restated as the whole pixel line it lies in — or
as the two it straddles, each at its own share — painted at the coverage its area implies.
`spans()` is the snap, `at.floor()` is the pixel line, and `render-quorra` has no such step because
quorra's rasterisers measure the mark's own area.

`bug1743245.pdf` is 0.5-unit strokes under a 0.317 CTM: **0.1586 device pixels**, so every rule on
the page goes through the substitution. Disabling it and re-running `examples/lane_diff` on the
four pages §31 named:

| page | oracle vs default lane | oracle vs sampled lane |
|---|---|---|
| `bug1743245.pdf`, substitution on | mean **3.0896** | mean 0.7384 |
| `bug1743245.pdf`, substitution off | mean **0.8849** | mean 3.0193 |
| `issue21068.pdf` on / off | 2.5799 / **1.4677** | 1.0941 / 1.1957 |
| `bug1863910.pdf` on / off | 1.1668 / **0.4499** | 1.3679 / 0.7585 |
| `issue16500.pdf` on / off | 0.3773 / **0.2889** | 0.4003 / 0.4439 |

The two columns swap. With our own substitution taken away, our oracle agrees with quorra's
default lane and disagrees with the sampled one — which is the whole of §31 read backwards.

## And their affine was our snap, measured rather than fitted

The probe's own numbers close it. The vertical rules' device x on that page are

```
313.62668  330.10830  346.58997  363.07160  379.55325  396.03488  412.51654
```

— a pitch of **16.48164** device pixels. Snapping each rule into the pixel line it lies in
replaces that pitch with a whole number of pixels, 16.5, so the snapped column is the stated
column scaled by `16.48164 / 16.5 = 0.998887`. quorra fitted **0.998899** to six commands with two
free values and called it a warning sign. It was not a coincidence and not a fit: it is the ratio
of a stated pitch to a whole pixel, and the offset beside it is the mean phase of the same snap.

§31's own observation — that the offset is "constant within one drawing command and different
between commands" — says the same thing once it is read the right way round: `sub_pixel_bands`
decides per subpath, and one `q … cm … S … Q` is one subpath.

## Why nobody caught it for a hundred and twenty rounds

The instrument that produced §31's table is `examples/lane_diff`, and it renders **the oracle**
beside the two lanes. `doc/traps/oracle-and-references.md`'s trap 3 is about invoking a reference
wrongly; this is one shape further out — a comparison in which one of the three columns is this
tree's own deliberate departure from the geometry, and the report treated it as the geometry. The
sentence "only one of them can be the exact one" was right and the ranking under it was not.

**The lesson is a trap and it is new**: *a comparison against our own renderer is not a comparison
against the specification, and a departure this tree took on purpose reads exactly like somebody
else's defect.* §10.7.4's ledger row has recorded that substitution, with its arithmetic, since the
sixteenth session; the round that wrote §31 did not open it.

## What the sampled lane's exactness on that page was

The sampled lane quantises coverage to a lattice of period 0.25 device pixels (quorra's ADR 0076,
`doc/QUORRA_CLIP_LANE_AND_UPLOAD.md` §3). On a page whose rules are 0.1586 pixels thick, that
lattice puts each rule's ink into whole pixels — the same pixels our snap chooses, most of the
time. So the sampled lane matched the oracle to three decimals *because both quantise*, by two
different constructions that happen to agree on this page's placements. §31 read the agreement as
evidence of exactness in a lane that has a stated quarter-pixel bound, which is
`doc/habits.md`'s *an agreement is evidence about a reading, not a definition of one* met from the
inside.
