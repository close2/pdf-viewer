# 0582 — The seam is not the model, and the standard names what produces it

**Status.** Accepted. Amends ADR 0308, which stays the measurement of record.

## Context

`doc/todo/11` item 5 has carried the seam between two abutting marks since the
four-hundred-and-seventy-third session as *witnessed, measured on all three rasterisers, and not a
defect of this program*. ADR 0308's central sentence is why:

> The union of two halves is three quarters. **So the seam is not a deviation from the model — it
> is the model, applied to the fractional shape §11.3.7.2's NOTE 1 says anti-aliasing produces.**

That claim was never checked against the clauses that state where the model's values live, and item
5's own instruction to a later round was to check it. It does not survive.

## What the standard says

Four sentences, in the order that settles it.

**§11.2 says where shape lives, and it is a `shall`.**

> Two scalar quantities called shape and opacity mediate compositing of an object with its
> backdrop. Conceptually, for each object, these quantities shall be defined at every point in the
> plane, just as if they were additional colour components.

§11.3.7.1 repeats it — "keep in mind that conceptually these values are computed for every point on
the page" — and it is the whole of clause 11's arithmetic: every formula in §11.3 and §11.4 is a
function of values at *a point*.

**§11.6.4.2 says what the value is at a point, and it is two-valued.**

> For objects defined by a path or a glyph and painted in a uniform colour with a path-painting or
> text-showing operator … the shape shall always be 1.0 inside and 0.0 outside the path.

§11.3.7.2's own bullet says the same: "whose value shall be 1.0 for points inside the object and
0.0 outside".

**§11.3.7.3's union is therefore a function of those point values.** Two marks that abut share a
boundary and their interiors are disjoint, so at every point one of the two shapes is 1.0 and the
union is 1.0. **The model states no seam anywhere.** Nothing is left of the backdrop between two
abutting opaque marks, at any point of the plane, and that is a derivation from the clause rather
than a preference.

**§11.3.7.2's NOTE 1 is what introduces the fraction, and it is informative and permissive.**

> Mathematically, elementary objects have "hard" edges, with a shape value of either 0.0 or 1.0 at
> every point. However, when such objects are rasterized to device pixels, the shape values along
> the boundaries **can** be anti-aliased, taking on fractional values representing fractional
> coverage of those pixels.

A `can`, in a NOTE, about what happens *when such objects are rasterized to device pixels* — which
is a step the model does not contain. So the seam is what comes out of evaluating a **non-linear**
function on **pixel-averaged** arguments: averaging and union do not commute, and
`avg(f_b ∪ f_s) ≠ avg(f_b) ∪ avg(f_s)` is the whole of the artefact. ADR 0308 read the NOTE as
though it extended the model's domain to pixels. It does not; it permits an approximation of it.

**§11.2's own NOTE 1 then names the loss by its cause**, which is the sentence that makes this more
than a quibble:

> The order in which objects are specified determines the stacking order but not necessarily the
> order in which the objects are actually painted onto the page. In particular, the transparency
> model does not require a PDF processor to rasterize objects immediately or to commit to a raster
> representation at any time before rendering the entire stack onto the page. This is important,
> since rasterization often causes significant loss of information and precision that is best
> avoided during intermediate stages of the transparency computation.

That is the standard describing conflation, and describing the cure — do not commit to a raster
representation before the stack is rendered — as a thing the model *does not require*. A permission
to defer, not an obligation to.

## Decision

**Record the seam as a departure from a value the clause defines, rather than as the clause's own
arithmetic.** Nothing about the artefact, its measurement or its price changes; what changes is the
sentence under it, and one of the three consequences is a rule rather than a wording.

1. **`doc/todo/11` item 5 stops saying "not this program's defect" and says what it is**: this
   tree's anti-aliasing departure — `doc/todo/_scan-conversion.md`'s departure (1), §10.7.1's NOTE —
   meeting a non-linear compositing function. Every rasteriser measured has it because every
   rasteriser measured commits to a raster per object, which is precisely what §11.2 NOTE 1 says
   causes loss. Three renderers agreeing remains evidence about our *reading*, and here the reading
   they share is one the clause does not require (principle 5).
2. **The cure and its price are unchanged and are re-derived, not inherited.** Two constructions,
   both still priced rather than taken: draw at *N*× and box-filter down (`N²` of the rasteriser
   and of the raster, on the frame time-to-first-page is measured from, and not exact — `gs
   -dGraphicsAlphaBits=4` leaks 0.2200 where the geometry leaks nothing), or keep each boundary
   pixel's marks as sub-pixel geometry until the pixel is finished. Nothing found in this round
   collapses either price: the second is the same conflation-free rasteriser item 4 names, it still
   has to answer what a blend mode and a transparency group do to a fragment list, and the artefact
   still halves per doubling of magnification and is gone by 8×.
3. **The gate does not move, and that was designed in.** `abutting_marks.rs` asserts §11.3.7.3 as an
   **upper** bound precisely so that a rasteriser leaving nothing passes unchanged. It now also
   asserts the case this ADR separates out — see ADR 0583 — and asserting the point-wise answer as a
   *lower* bound is deliberately not done, because the departure that produces the shortfall is
   licensed and is not being withdrawn.

## What this does not change

- **ADR 0308's numbers stand.** The three rasterisers, the four references, the owner's page at four
  scales, `poppler`'s zero being trap 9: all measured, all unaffected by which sentence explains the
  arithmetic.
- **One figure in it has aged and is corrected where it is quoted.** ADR 0308's table records the
  processor at 0.2510 and `render-quorra` at 0.2471; run today the two are exchanged — 0.2471 on the
  processor, 0.2510 on the device — because ADR 0476 made an axis-aligned rectangle's coverage exact
  on the processor, so `0.75` of a pixel is now measured rather than supersampled and `0.25` rounds
  the other way. Both are within one level of 255 of the union's 0.2500 and neither says anything
  new; the §11.3.7.3 ledger row quoted the old attribution and has been corrected.
- **The exclusion in `CLAUDE.md` is untouched.** "Where the standard defines nothing, done means a
  documented choice" is not the situation here: the standard defines this and we depart from it with
  a licence, which is a different entry in the same taxonomy and is what
  `doc/todo/_scan-conversion.md` exists to hold.

## Why it matters beyond the wording

Item 5 was blocking item 7's remainder, and the block rested on this sentence. Reading it found the
block was addressed to the wrong clause entirely — §11.3.7.3 governs two *objects*, and item 7's
remaining case is one object's subpaths, which §11.6.2 governs and answers the other way. ADR 0583
takes that. **The general lesson is `CLAUDE.md`'s own**: a claim that the standard says something is
a claim that decays, and "it is the model" is the strongest form such a claim can take — it forecloses
the question instead of ranking it.
