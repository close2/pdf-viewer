# ADR 0415 — The flag whose refusal was inverted

Status: accepted, 2026-08-18. Session 580. Honours §11.6.4.3's and §11.6.4.4's `/AIS` inside
§11.4.6's knockout construction, where it had been read and refused since ADR 0234. Amends
`doc/todo/23-transparency-departures.md` and the ledger rows for §11.3.7.2, §11.4.6, §11.6.4.3 and
§11.6.4.4. No display-list vocabulary, no backend arm and no raster was added.

## The reading

Table 57's `/AIS` is one boolean, and two clauses state what it decides. §11.6.4.3's NOTE 1, of the
soft mask:

> This is a boolean flag, set with the AIS ("alpha is shape") entry in a graphics state parameter
> dictionary (8.4.5, "Graphics state parameter dictionaries"): true if the soft mask contains shape
> values, false for opacity.

and §11.6.4.4, of the two alpha constants:

> As described previously for the soft mask, the AIS ('alpha is shape') entry in a graphics state
> parameter dictionary shall determine whether the alpha constants are interpreted as shape values
> ( true ) or opacity values ( false ).

Alpha is their product either way — §11.3.7.1 defines it "as the product of shape and opacity" — so
the flag changes no pixel anywhere the product is all that is used. §11.4.6 is the one place it is
not, and the clause says so itself:

> The existence of the knockout feature is the main reason for maintaining a separate shape value
> rather than only a single alpha that combines shape and opacity.

because its second stage takes "a weighted average of this result with the object's immediate
backdrop, **using the source shape as the weighting factor**".

### The third sentence, which nobody had multiplied by the other two

§11.6.4.2's last paragraph:

> All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere.

and §11.3.7.2's arithmetic:

> The three opacity inputs shall be multiplied together, producing an intermediate value called the
> source opacity.

Put the four sentences together. Under `/AIS true` the mask is `f_m` and the constants are `f_k`,
so `q_m = q_k = 1`; §11.6.4.2 has already made `q_j = 1`; the product is `q_s = 1`; and §11.3.7.1's
alpha is `f_s × q_s = f_s`.

**Under `/AIS true` the number a rasteriser already draws an element with *is* that element's
shape.**

## Why that inverts the refusal

`transparency::stated_shape` builds a knockout element's shape by *removing* §11.6.4.3's mask and
§11.6.4.4's constant from the element — which is the clause under `/AIS false` and, as the ledger
row said, "exactly wrong under true". So the entry was read (ADR 0234) and every knockout group was
refused by name while it was set.

The sentence that survived from there, in three places, was that honouring it "means composing the
mask into the shape instead of into the object, which is a second `stated_shape` rather than a new
vocabulary". That is the right shape of answer and it overstates the work: composing the mask and
the constants *into* the shape yields the element back. The shape command under `/AIS true` is the
element itself, with only its blend mode dropped — §11.4.6 leaves a knockout element nothing to
blend against, its backdrop being transparent and §11.3.6's formula with `α_b = 0` being the source
colour whatever the blend function is.

So `pdf_render::Command::Shaped`, which has stated `(object, shape)` since ADR 0234 and which all
three backends have drawn since ADR 0291, states both halves as the same command, and §11.4.6's two
stages come out as `DestinationOut` with alpha `f` followed by `Plus` with `f × C`:

```text
P' = (1 − f) × P + f × q × C     with q = 1
```

which is the clause. `shape_the_alpha_already_is` is those seven lines.

**And the refusal was inverted, not merely narrowed.** `/AIS true` is the one reading under which a
rasteriser's single number per pixel *cannot* disagree with the shape — every other source of
opacity has been taken away — and it is exactly where this tree refused, for eighty-three sessions.
The condition that admits a *bare* knockout draw runs the other way and had to be narrowed rather
than widened: a bare draw is Porter-Duff Source modulated by coverage, so it computes
`(1 − cov) × P + cov × α × C` and reads the paint's own alpha as opacity — right under `/AIS false`,
where a translucent solid is therefore allowed, and wrong under true. `element_shape_is_coverage`
answers `false` for every element under the shape reading, and each states its pair instead.

## What is refused instead, and it is a scope rather than a construction

`/AIS` is a graphics state parameter, so one group's content may paint some marks under each
reading, and the two build a shape differently. `Interpreter::alpha_sources` is therefore a
three-valued record — `Opacity`, `Shape`, `Mixed` — seeded at a group's `Do` from the value in force
there, folded into by every `gs`, and folded outward into the enclosing scope when the group closes.
A `Mixed` group is refused with a report that says so.

Two precisions were needed and both are the same one:

- **A reading nothing was painted under is replaced rather than mixed in.** The commonest shape in a
  real file is a form whose content *opens* with the `gs` that states `/AIS`, where the value
  inherited from the `Do` reached no mark at all. `Interpreter::alpha_sources_mark` records the
  display list's length when the record last changed, and a statement arriving while the list is
  still that long replaces it. The invariant is that a command leaves the list only to be folded
  into a replacement or because a group painted nothing — and where that ever stopped holding the
  comparison fails and the record says `Mixed`, which costs a report rather than a pixel.
- **The same question is asked when a group closes**, so a nested group whose content is the first
  thing an enclosing group painted gives the enclosing group its reading rather than mixing with a
  seed that reached nothing.

The outward propagation is what makes the shape reading safe one level down: a `Shape` answer for a
group means every mark inside it, at every depth, was painted under `/AIS true`. A soft mask's group
is the one exception and is restored exactly, because its marks become one alpha per pixel rather
than elements of anything.

## The one element the shape reading still refuses, and why

A **non-isolated** group used as a knockout element. §11.3.7.2 makes a group object's opacity "the
result of the opacity computations for all of the objects it contains", which under this reading is
1.0 throughout — so an *isolated* group's accumulated alpha is the union of its elements' alphas,
which is the union of their shapes, which is §11.3.7.2's shape of a group object. (Compositing by
Over takes that union whatever blend function the colours go through, and §11.4.6's
`(1 − f) × F + f` is the same union — the identity `shape_without_the_mask_and_the_constants`
already relies on.) A non-isolated group is drawn on a buffer seeded from its backdrop (ADR 0237),
so what it accumulates carries the backdrop's alpha beside its own and is not a shape at all. That
element keeps the report, and the report now names it.

## The two callers the specification supplies

§11.6.2's one object painted in parts and §9.3.8's text object are knockout groups the standard
makes out of something that is not one, and both asked a **page-wide** flag where the entry's scope
is the graphics state. They ask `alpha_sources` now, through the same
`transparency::knockout_group_elements` that gives the explicit group its elements — so under the
shape reading they state the pair too, instead of being refused.

The reading they ask is the accumulated one rather than the state's own at the operator, and that is
deliberate: a `B` whose fill is a tiling pattern has a *group* for one of its two parts, and
§11.3.7.2 gives a group object the opacity of its contents, so the reading its contents ran under is
what decides whether its alpha is its shape.

## The populations, measured

Trap 11's discipline: derive the condition from the clause, print what it matched, cost it in gated
pages. An `eprintln!` at each of the four sites, over both corpora.

- **The 974-document corpus: nothing at all.** 23 knockout groups on its first pages, not one with
  `/AIS` in force; no `B` operator and no text object refused for it either; and over *every page* of
  the nine documents that state `/AIS true`, only `issue18032.pdf` has a knockout group and its
  reading settles to `Opacity`. So §11.6.4.3's row saying "none of their knockout groups is drawn
  today" had expired with ADR 0327's scoping, and this construction moves no corpus pixel.
- **The 65 944 crawled web documents: one.** 1621 knockout groups on their first pages, 33 with
  `/AIS` in force, of which 3 have a knockout rule that can change a pixel — and **exactly one
  document, `6573550.pdf`, carried the report**. Two text objects were refused page-wide for the
  flag as well.

`6573550.pdf` draws its knockout group now. 13 736 of its 1 128 099 pixels move, by at most 3 of
255, all inside the artwork the group paints; the page keeps its unrelated §11.4.4 report. The
hand-built witness is the discriminator that matters: one fixture, two readings, **127 of 255
apart** — an opaque red square under a blue one masked at a half, which is `(127, 127, 255)` when
the mask is opacity and `(127, 0, 127)` when it is shape.

## What this is worth, and what it is not

It is one report on one document in 66 918. What makes it worth a round is the *shape* of the
finding rather than the count, and it is this project's most repeated shape: ADR 0277 found
§11.3.5.3's K rule was the clause's own functions on a neutral pair, ADR 0307 found two of three
knockout witnesses needed no construction, and this one finds a refusal standing exactly where the
clause guarantees the thing being refused. Each was a sentence of the standard multiplied by another
sentence of the standard that nobody had put beside it.

The general debt §11.3.7.2 names — a shape channel every command carries — is untouched and is what
the non-isolated group element still wants.
