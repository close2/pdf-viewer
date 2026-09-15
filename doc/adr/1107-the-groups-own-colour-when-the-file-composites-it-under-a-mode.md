# ADR 1107 — The group's own colour, when the file composites it under a mode

Status: accepted, 2026-09-15. Session 1093. Performs ISO 32000-2 §11.4.4's result step for itself
on `render-cpu`, the backend `CLAUDE.md` principle 2 makes the correctness oracle, so that a
non-isolated transparency group the *file* composites under a blend mode is drawn rather than
reported. Reverses the one guarantee ADR 0237 asked the display list for, and the two other
backends refuse the combination by name instead of substituting §11.4.5's backdrop.

## What was reported, and for how long

ADR 0237 gave a non-isolated group its own model on one condition, restated by
`pdf_render::Command::Group`'s `isolated` ever since: the composite at the `Do` must be §11.3.3's
**Normal** blend function, because that is the step whose multiplication by Table 140's group
alpha cancels §11.4.4's division by it. Under any other mode nothing cancels, so `pdf-model` set
`isolated: true` — §11.4.5's transparent initial backdrop, which is a *different picture* — and
reported it. The ledger rows for §11.4.3, §11.4.4 and §11.4.8 all stood on that one sentence.

The obstacle was real and is stated in `run_transparency_group` in the clause's own terms: the
removal

```text
C = Cn + (Cn − C0) × (α0 ÷ αgn − α0)
```

divides by a quantity an eight-bit raster of premultiplied samples does not hold. `αn` is the
union of `α0` and `αgn`, so over a *transparent* backdrop the group alpha can be recovered from
the buffer — and over an opaque one, which is an ordinary page, it cannot: both are 1 whatever
the group did.

## What the clause says to do about that, in the sentence nobody had taken up

§11.4.4's NOTE 4 answers it outright:

> For shape and alpha, backdrop removal can be accomplished by maintaining two sets of variables
> to hold the accumulated values.

and §11.4.8's recurrence says what the second set costs to build: shape and alpha accumulate by
union over the elements' own source values and **read no colour at all**. So the second set is
the same elements run a second time onto transparency, whose accumulated alpha *is* `αgn` — the
same number the first run would have accumulated separately, because no backdrop enters it.

`render-cpu` now does that (`CpuRasterizer::remove_the_backdrop`), rewrites the buffer into Table
139's `C` and `α` (`blend::remove_backdrop`), and then paints it exactly as it paints any other
group, which is the clause's own next sentence:

> The result of applying the group compositing function shall then be treated as if it were a
> single object, which in turn is composited with the group's backdrop according to the formulas
> defined in this subclause.

Under Normal the interpolation stays: it is the same two steps with the cancellation taken, one
rounding instead of three, and it walks the band instead of the elements twice.

## The fixture's numbers, and they are the clause's

An opaque 0.5 grey page; the group's one element an opaque 0.5 grey under `/Multiply`, which is
what §11.4.4's NOTE 2 makes the two kinds of group differ by at all. §11.3.5.2 gives Multiply as
`cb × cs` and Screen as `cb + cs − cb × cs`; §11.3.6 at `αs = αb = 1` leaves the blend function
alone.

| at the `Do` | non-isolated | isolated |
|---|---|---|
| `/Multiply` | **0.125** | 0.25 |
| `/Screen` | **0.625** | 0.75 |

The left column is `Cn = 0.25` with the removal's factor zero at `α0 = αgn = 1`; the right is
§11.4.5's transparent initial backdrop, where §11.3.6 says "[a]n alpha value of αs = 0.0 or αb =
0.0 results in no blend mode effect" and the element's Multiply does nothing. Before this change
the left column *read the right one*, which is the plant the test is calibrated on. The control
is the same page with nothing blending inside the group, where §11.4.4's NOTE 3 makes the two
models one picture: 0.25 and 0.75 for both, and a removal doing anything to a group with nothing
to remove would move it.

## What it cost, and what it did not move

**Nothing in the tracked corpus states it.** `doc/pdf.js`'s 974 first pages and `doc/corpora`'s
503 state the construction **zero** times, counted by a probe on the interpreter's own condition
and calibrated on the planted fixture (trap 13); a crawl of 89 286 documents states it on 440 of
their first pages, which is why the fixture is the gate and not a corpus page. `raster_golden`
moved nothing of this round's — the two pages it names are a sibling's, proved by an A/B of
`display_list_digest` over the two builds — and the oracle's verdicts and `render-raster --test
corpus`'s lists are identical either side.

The second run of the elements is paid **only** by a group that is non-isolated, non-knockout,
outside every knockout group, holds an element that blends, and is composited under a mode of its
own. Every other group takes the path it took before, including the interpolation.

## What is deliberately not done

- **The two other backends are not taught it.** `render-quorra` has no lane for a result step
  taken between the elements and the composite — a scene states the group and the library
  composites it — and Vello's layer begins transparent. Both already refuse a non-isolated group
  they cannot draw, by name, and that refusal now covers this one; substituting §11.4.5's
  backdrop is the silence trap 5 is about.
- **A knockout group keeps its report.** §11.4.6 gives *each element* the group's initial
  backdrop, so what would have to be removed is not one buffer's contribution; §11.4.4's result
  step is about the group as a whole and that is what this builds.
- **`group_blit_mask`'s intersection is not extended to it.** After the removal the buffer does
  hold the group's own alpha, so ADR 0492's `min(f·S, C·S)` becomes expressible — but §8.5.4's
  shape question is `alpha_is_shape`'s and is asked one layer up, and widening it here would be a
  second reading of that clause free to drift from the one that decides the picture.
