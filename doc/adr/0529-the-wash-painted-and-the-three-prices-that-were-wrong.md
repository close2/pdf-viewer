# 0529 — The wash painted, and the three prices that were wrong

Status: accepted.
Session: 688. Takes ADR 0452's finding — §8.7.4.3 Table 77's `/Background` reported and unpainted —
and corrects the pricing that came with it, together with the claim about its headline witness that
three documents carried.

## The clause, and the sentence nobody had cited

Table 77's cell is a `shall` and ADR 0452 quotes it. What that ADR did *not* find is that the
standard states the **construction** as well as the requirement, in a clause five hundred pages
away. §11.6.7, *Patterns and transparency*, last bullet of the list that says how a pattern's
definition is evaluated:

> If the shading dictionary has a Background entry, the pattern's imp licit transparency group
> shall be filled with the specified background colour before the sh operator is invoked.

Two sentences above it, the same clause says what that group is:

> In both cases, the pattern definition shall be treated as if it were implicitly enclosed in a
> non-isolated transparency group: a non-knockout group for tiling patterns, a knockout group for
> shading patterns.

and, below the list:

> When the pattern is later used to paint a graphics object, the colour, shape, and opacity values
> resulting from the evaluation of the pattern d efinition shall be used as the object's source
> colour ( 𝐶𝑠 ), object shape ( f j ), and object opacity ( qi ) in the transparency compositing
> formulas.

Put together, those three decide the whole shape of the implementation and leave nothing to be
argued from first principles:

- the wash goes **inside** the group, so it is not a second painting operation on the page;
- the shading is painted into the same group afterwards, and the group is a **knockout** one, so
  the shading overwrites the wash inside the shading's bounds rather than compositing over it;
- the group's colour and shape then become the object's, so the path's antialiased edge, §11.6.4.4's
  constant alpha and the blend mode apply **once**, to the pair.

`doc/todo/17` reached the same construction by argument — Table 77's NOTE 1 says "the effect is as
if the painting operation were performed twice" and says it *of the opaque imaging model*, and two
coincident marks of coverage `c` leave `(1 − c)²` of the backdrop where the clause leaves `1 − c`.
That argument is right and it was never necessary. **The standard answers it, and the answer is
stronger than the derivation**, which is `CLAUDE.md` principle 5's own note about a silence that
decays: before recording that a clause states only a requirement, read the *titles* around the
subject. §11.6.7's title has the word *patterns* in it.

The third statement of the `sh` exemption is worth recording for the same reason. Table 77 says the
entry applies "only when the shading is used as part of a shading pattern"; §8.7.4.2 says of the
operator "[t]he `Background` entry, if present, is ignored"; and §11.6.4.2 says a `sh`'s shape is
"1.0 inside and 0.0 outside the bounds of the shading's painti ng geometry, **disregarding the
Background entry**". A condition stated three times is not one to get wrong.

## What was built

`pdf_render::Shading` gains `background: Option<Color>`, resolved through the shading's own colour
space by `pdf_model::shading` under the same `Conversion` and the same §10.5 transfer as its ramp,
and set **only** where a shading arrives through a `/PatternType 2` pattern. `with_alpha` scales it;
`is_opaque` accounts for it.

`pdf_render::ShadingRaster` is the paint. It walks the device pixels of a region and answers, per
pixel, the shading's colour where the shading's geometry has one and the background where it has
none — an axial or radial parameter `/Extend` does not admit, a point no blend circle passes, a
pixel no triangle covers, a point outside §8.7.4.5.2's transformed domain rectangle. Every backend
draws that one raster through the shape it was going to fill anyway.

## Why one raster rather than four constructions

**Because all three backends already have exactly this lane**, and the round's job was to notice.
`render-cpu`'s `fill_with_raster`, `render-gpu`'s `fill_radial`, and quorra's `Paint::Mesh` are all
"a straight-alpha RGBA raster at device resolution, placed at whole pixels, confined to the path" —
built for §8.7.4.5.5's mesh and §8.7.4.5.4's cone, where the clause states an algorithm no gradient
primitive implements. A background is the same kind of thing one clause over: a colour outside the
geometry, which no spread mode of any library can express. `SpreadMode::Pad` pads, `Repeat` repeats,
`Reflect` reflects; none of them says *this other colour*.

That is what makes the alternative pricing wrong rather than merely more expensive. A
background-carrying stop at each non-extended end works for an axial and for a nested radial, and it
is four lines per backend — but it works for **two of the four kinds**, it puts the decision in three
places, and it needs a fifth answer for the one lane that is not ours. One raster in `pdf-render`
needs no answer from anybody: the geometry and the colour are decided once, which is trap 2's rule,
and the cross-backend gate then compares an *evaluation* rather than three libraries' shaders.

## The three prices that were wrong

`doc/habits.md` records that a price is a claim that decays. This block had already found three
wrong prices; here are three more, in one table.

| what the file priced | what it costs |
|---|---|
| axial and radial: "the transparent stop `stops()` places at a non-extended end becomes a background-coloured **opaque** stop" | not taken. It is correct and it is two of four kinds, in three places — and it leaves quorra needing an upstream field |
| mesh: "`MeshRaster` leaves every pixel no triangle covers transparent; the background is its clear colour" | **wrong**. `MeshRaster`'s extent is the *mesh's* bounding box, and Table 77's wash covers "the area to be painted", which is the path. A clear colour paints background only where the mesh nearly reaches already, and `SpreadMode::Pad` then smears the raster's edge over the rest |
| sampled: "the one kind where the NOTE's two-operation shape is exact because the two regions are disjoint" | **wrong twice**. They are not disjoint — Table 77 fills the whole area to be painted, not the complement of the domain — and making them disjoint does not help: two abutting antialiased edges compose as `1 − (1−a)(1−b)`, so the seam at the domain's own boundary replaces the fringe at the path's |
| quorra's gradient lane: "the largest single item, and it is not this tree's to change" | **not owed at all**. Nothing in quorra needs a new field; a background-carrying shading of any kind takes the mesh paint quorra already has, which is the same bytes the other two backends draw. No entry was written into `doc/QUORRA_FEEDBACK.md` because there is nothing to ask for |

The cheapest re-derivation really was the one `doc/habits.md` names: asking what the libraries and
the layers already contain. Three of the four rows fall out of one observation about
`fill_with_raster`.

## The witness that witnesses nothing

`issue13372.pdf` was this entry's headline witness in three documents at once — this ADR's
predecessor, `doc/todo/17`, and §8.7.4.3's ledger row — and all three said the same thing: that the
page's corners project outside `[0, 1]` on the shading's axis, so every marked cell of its CCITT
stencil beyond the band is cyan in the document and unpainted here.

It is not. **The area to be painted is not the page.** The page's whole content stream is

```text
q 0.24 0 0 0.24 0 0 cm
/R7 gs /R8 cs /R9 scn
q 1800 0 0 -2400 375 2850 cm /R14 Do Q
Q
```

so the stencil occupies exactly `(90, 108)` to `(522, 684)` in default user space — and the
shading's `/Coords [90 108 522 684]` is that rectangle's **diagonal**. The axial parameter over it is
`t = (432(x − 90) + 576(y − 108)) / 518400`, which is 0 at one corner, 1 at the opposite one and 0.36
and 0.64 at the other two. Every point of the area to be painted is inside the band, `/Extend`
withholds nothing, and the wash has zero area.

Measured rather than reasoned, by rendering each witness page with and without the change in one
sitting:

```text
                    pixels changed   of the page   largest change
  issue13372.pdf            26 690         5.33%        1 level
  issue18816.pdf               149         0.03%       15 levels, all on one raster row
```

`issue13372.pdf`'s 26 690 are **not the wash**: they are the axial leaving `tiny-skia`'s gradient for
`ShadingRaster`'s evaluation at each pixel centre, one level apiece. `issue18816.pdf`'s 149 are the
wash, and they are the single row of pixels where the filled path runs past the mesh's outermost
patch.

So the entry is implemented on the standard's evidence and defended by fixtures, and **the corpus is
not what ranked it and could not have been**. `CLAUDE.md`'s two denominators is exactly this: the
ledger asks which requirements are implemented, the corpus asks what share of real files render
correctly, and a `shall` that two files state and neither exercises is visible to the first
instrument and invisible to the second.

The general lesson is trap 1's inverted, and it belongs to the *reading* rather than to the picture:
**a claim about what a document draws is a measurement.** "The page's corners" survived three
documents and seventy-two sessions because it sounds like the clause and is not the clause's phrase.

## What is still reported, and why the report has a condition rather than a subject

`Unsupported::ShadingBackground` survives on exactly two conditions:

- a **stroking** selection. Table 77's "area to be painted" is any painting operation's, a stroke
  included; the raster lane is a *fill*'s door in all three backends, because a stroke's outline is
  not the shape a backend is handed. `ShadingDefinition::paints_background` is false for a stroking
  `SCN`, the built shading carries no background, and the shortfall is named. No document in either
  population strokes with a background-carrying pattern;
- an array of the **wrong length**. "[A]n array of colour components appropriate to the colour
  space" is a count, and an array that is not one states no colour. Inventing one would put a wash on
  the page the document did not ask for, which is the opposite of what this entry is.

Both are trap 11's rule applied forward: the report fires on the cases that are not drawn, and a
fill of a usable array owes nothing.

## The reading that has no witness, and is therefore stated as a reading

Neither corpus witness states a `/BBox`, so **which of the two the bounding box clips is a reading**.
Table 77 makes `/BBox` "a temporary clipping boundary when the shading is painted" and §11.6.7 puts
the wash in the group *before* the `sh` — which taken alone would leave the wash unclipped. Against
that, §8.7.4.5.2 is the only sentence in the standard that positions the wash relative to the box,
and it puts it inside: "[p]oints wi thin the shading's bounding box ( BBox ) that fall outside this
transformed domain rectangle shall be painted with the shading's background colour". The box clips
both, on the sentence that mentions both. It is also what leaves `Interpreter::paint_clip` unchanged.

What *does* come off is §8.7.4.5.2's **domain** clip, and that is not a departure but the other
branch of one sentence: the same clause says such points "shall be left unpainted" when there is no
`/Background`, so the clip is the entry's absence and the raster is its presence.

## What it is gated by

Five fixtures in `pdf-model/tests/shadings.rs`, one per shading kind plus the `sh` exemption, each
rendering the *same* fixture twice — with the entry and with it removed — because §8.7.4.5.2 states
the two branches together and a single picture proves only one of them. Two more pin the report's
two conditions. `test_scenes::shading_background` carries all four kinds on one page and is compared
across all three backends, at page scale and at 4×; its radial quadrant is **nested circles** on
purpose, because that is the radial geometry a gradient library *can* express and therefore the one
where keeping the gradient is a mistake a backend could make alone.

Every expected value is the clause's. The wash's colour is the file's own `/Background` through
§8.6.4's device space; where it lands is Table 77's "outside the bounds of the shading object", with
the bounds taken from §8.7.4.5.3's projection, §8.7.4.5.4's blend circles, §8.7.4.5.5's triangles and
§8.7.4.5.2's transformed domain in turn.

## Consequences

- §8.7.4.3 leaves `partial` for `implemented`; §8.7.4.5.2's and §11.6.7's rows record the branch each
  of them owns.
- `issue13372.pdf` page 1 returns to the comparison and to `AMBIGUOUS_IMAGE_REDUCTION`, on the
  halftone-reduction diagnosis it always had and on nothing this round did.
- `function_based_shading_cmyk.pdf` page 2 left `CONTRADICTED_DEVICE_CMYK_CONVERSION` in the same
  run, and **it was not us**: our raster for it is byte-identical across this change, the file states
  no `/Background`, and what moved is the consensus — the closest two references now miss each other
  by 29.06 against a page bound of 1.00. It is in `AMBIGUOUS_DEVICE_CMYK_CONVERSION` now. A page
  moving on the round that touched the clause it looks like is precisely the coincidence to measure
  rather than to assume.
- `doc/todo/17` is deleted. Its argument about the NOTE's two operations is above, with the clause
  that made it unnecessary.
