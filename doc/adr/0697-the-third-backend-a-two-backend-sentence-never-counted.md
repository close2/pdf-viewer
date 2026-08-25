# 0697 — The third backend a two-backend sentence never counted

Status: accepted.
Context: the ledger's `partial` rows read as a family, on the question ADR 0538's method asks and
not on the errata ranking — §8.9.6, masked images, chosen off the blame ordering's head.

## Why this family, and why the blame ordering

The pair ranking ADR 0567 built has no head left: ADR 0620's round spent the last of the three
strongest pairs and fell through to a tie at rank 4, and ADR 0574's measurement had already put the
pairwise score at chance over individual rows. So the ordering used here is the older one, the one
session 442 read thirty-two rows off and nothing has re-derived since: **every `partial` row by when
its own `note` line was last written**, which `git blame` answers and no sweep does.

Its head is the batch 442 itself wrote — which is the ordering's one known flaw, a row it re-offers
because reading it counts as writing it — and that is exactly what makes the head worth taking now
rather than a fault. Those notes have stood unread for three hundred and twenty-five sessions,
through every change the tree has made since, and a claim's age is the one thing about it that no
sweep in `doc/todo/01` measures. §8.9.6 and §8.9.6.2 sit in that batch, `partial`, in a clause
family that decides pixels.

## What the rows said

Three of them carried one sentence about the family's residue, and two of them carried a second
about how §8.9.6.2's last requirement is met. Both were wrong, in opposite directions.

### The refusal, stated one condition too wide

§8.9.6, §8.9.6.1 and §8.9.6.2 each named, among what the family owes, *a stencil under a
graphics-state soft mask, which would be two masks on one command*. In §8.9.6.2's own note that
sentence follows two paragraphs about a stencil whose current colour is a **pattern**, and it is
true of that construction: the pattern case recomposes the stencil *into* the mask slot — as a
§11.5.2 alpha soft mask for a shading, and as `Interpreter::tile`'s group mask for a tiling — so a
state that is already using the slot has nowhere to put the second mask. `content::image.rs`
refuses exactly that, by name, and reports.

The two parent rows restated the sentence without the paragraphs that bounded it, where it reads as
a refusal of every stencil under a soft mask. It is not one. An ordinary stencil needs no mask slot,
so the state's mask reaches it like any other mark — which is what §11.6.4.3 requires rather than
merely permits: the entries it lists as displacing the state's mask are the image dictionary's
`/SMask`, its `/SMaskInData` and its `/Mask`, and "[e]ither form of mask in the image dictionary"
names the first and the third. `/ImageMask` says the samples carry no *colour*, not that they carry
an opacity, and it is not on that list.

Nothing in this tree had ever asked.
`image_masks.rs::a_stencil_under_the_graphics_states_soft_mask_is_drawn_through_both` asks now — a
stencil under a luminosity group white over the left half of the page and black over the right, so
the mask cuts a cell the stencil marks — and it is calibrated per trap 13 against both of the
behaviours the rows described: with the state's mask dropped for a stencil it fails on the cell that
should have been cut, and with the refusal the rows describe planted it fails on the report.

### The `shall` that was met by two backends out of three

§8.9.6.2's last sentence is the one this family's rows have twice been wrong about, and this is the
third time:

> If image interpolation (see 8.9.5.3, "Image interpolation") is requested during stencil masking,
> the effect shall be to smooth the edges of the mask, not to interpolate the painted colour values.

The rows answered it, correctly, by naming the raster rather than a branch: a stencil decodes to the
fill colour where its bits mark and `[0, 0, 0, 0]` where they do not, so what a filter does with
those four components decides which of the clause's two nouns it operates on. Premultiplied, the
cleared samples contribute their zero coverage and nothing else, and a smoothed edge is the painted
colour at partial coverage exactly. Straight, the black those samples are *stored* with is averaged
in and the edge comes out half dark — which the note says outright, in its own last clause, as the
thing the clause forbids.

Then it wrote **"both backends"**, and there are three.

`crates/render-quorra/examples/filtered_edge_colour` draws the one scene that separates the two
arithmetics — a 4 × 4 image alternating between opaque red and cleared samples, `interpolate: true`,
magnified forty-fold over `Medium::NONE` — and prints how far each backend's partly covered pixels
depart from the colour that was painted:

```text
   cpu: 160 partly covered pixels, worst departure from the painted colour 0 at x=0 ([0, 0, 0, 0])
 vello: 160 partly covered pixels, worst departure from the painted colour 0 at x=0 ([0, 0, 0, 0])
quorra: 160 partly covered pixels, worst departure from the painted colour 131 at x=75 ([124, 0, 0, 125])
```

All three filter, and all three produce the same 160 partly covered pixels — this is not a
disagreement about whether to smooth. Two give `[255, 0, 0, α]` and the third gives `[126, 0, 0, α]`.
`crates/quorra-gpu/src/shaders/image.wgsl` samples the texture and premultiplies afterwards, so a
filtered tap is `mean(rgb) * mean(a)` where the clause asks for `mean(rgb * a)`; the two are equal
whenever the neighbourhood's alphas are equal, which is every opaque image.

**And quorra is what the shipped viewer draws with** — `viewer-ui`'s `pdf-viewer` holds a
`QuorraWindowRenderer` — so this is the page a reader sees, not a comparison lane. `Image::is_smoothed`
turns the filter on for every reduced image as well as for every `/Interpolate`, and the reach is
wider than the clause that names it: an image carrying an `/SMask`, one under an explicit mask, one
with a colour-key range cut out of it and a JPEG 2000 image with an opacity channel all arrive as
straight-alpha RGBA whose cleared samples are black.

## Why nothing could see it

Both cross-backend suites draw images, and **every image scene in both of them is opaque**. On an
opaque raster straight and premultiplied filtering are the same arithmetic, so no tolerance in
either suite could have been tightened into finding this. That is trap 12b's shape at the level of a
property rather than a size: a suite whose scenes never vary the quantity a rule is about cannot
test the rule. The corpus cannot see it either — a darkened stencil edge is a fraction of a pixel
wide against every reference, and the oracle's bounds are not a hue at partial coverage.

What *could* have seen it is the sentence itself, and this is the part worth keeping:

> **A cardinal that counts this tree's own parts is a claim that decays the moment a part is added,
> and nothing in `doc/todo/01` counts those.**

The tenth sweep, `--bin counts`, reads a cardinal only where it governs one of the ledger's own
words for a row — it is a claim about a *clause family*, checked against the family's arithmetic.
"Both backends" governs a noun this tree has a countable population of, and there is no instrument
for that population at all. The same two words are in `pdf-render`'s own `Image::is_smoothed` doc
comment, and in `paint.rs`'s comment above it, both written before quorra existed and both still
standing. A sweep whose left-hand side is a numeral governing `backend`, `rasteriser`, `crate`,
`worker`, `host` or `submodule`, and whose right-hand side is the workspace's own membership, would
have printed all three the day the third backend landed.

## What was decided

- **The clause's requirement is recorded as unmet on one backend rather than met by construction.**
  §8.9.6.2 and §8.9.6 keep `partial` and the note names quorra's filter as what is owed. Nothing
  here can close it: premultiplying the bytes we upload would have the shader multiply by alpha a
  second time, so the fix is on quorra's side of the boundary either as an upload-time
  premultiplication or as a hand-weighted sample. `doc/QUORRA_FEEDBACK.md` §39 is the ask, with both
  options and no preference between them.
- **The two backends that meet the clause are gated.**
  `headless_gpu.rs::cpu_and_gpu_smooth_a_stencils_edges_without_darkening_its_colour` asserts the
  painted colour at every partly covered pixel of that scene, for the CPU oracle and for vello, and
  refuses to pass vacuously on a backend that filtered nothing. It is calibrated against a live
  failure rather than a plant, which is the strongest form trap 13 admits: the third backend in this
  tree fails it today.
- **The example stays**, because it is the only thing in the tree that can print this class of
  difference and because a fix arriving from quorra has to be verifiable from this side in one
  command.
- **The parent rows' refusal gains the condition that bounds it**, and
  `a_stencil_under_the_graphics_states_soft_mask_is_drawn_through_both` holds what they had denied.

## The shape, for the next round that reads a family

An **understating parent** — the mirror of the eighteenth sweep's subject. `--bin overstated` reads
a parent asserting that something *is read* against a child denying it; this is a parent restating a
child's *refusal* and losing the condition the child stated it under, which reads as a larger debt
than the tree has and sends a reader looking for a defect that is not there. Both sides are again
this project's own sentences about its own code, so a program could read it: the discriminator is a
parent's sentence that a child's note contains as a substring **inside a longer conditional the
parent did not carry over**.

Neither of this round's two findings is in any existing sweep's population, and they fail in
opposite directions — one row owing less than it says and another owing more. What they share is
that both were settled by opening the clause beside the code, which is `doc/todo/02` §1's own
sentence about where four of six findings came from.
