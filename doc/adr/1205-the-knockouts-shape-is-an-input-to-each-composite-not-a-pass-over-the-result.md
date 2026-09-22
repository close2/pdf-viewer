# ADR 1205 — §11.4.6's shape is an input to each element's composite, so it does not take §11.7.5.2's channel

Status: accepted. Session 1184.
Amends: ADR 1113, whose declination of a per-pixel shape channel rested on a premise the tree has
since removed, and which is upheld here on a different reason. Corrects ADR 1022 §5's price for
`SampleAlpha::Both`, and the reading three ledger rows carried about a non-isolated group used as
an element.
Depends on: ADR 1125 (`pdf_render::transfer_channel`), ADR 0234 (`Command::Shaped`), ADR 0327 and
ADR 1009 (the own-backdrop construction), ADR 1170 (the synthesised non-isolated group).
Context: `crates/pdf-model/src/content/transparency.rs`, `crates/pdf-render/src/transfer_channel.rs`,
`crates/pdf-model/src/image.rs`, `crates/render-cpu/src/lib.rs`, `crates/render-raster/src/scene.rs`.
Clauses: ISO 32000-2 §11.3.7.2, §11.3.7.3, §11.4.4, §11.4.6, §11.4.8, §11.6.4.2, §11.6.4.3,
§11.7.5.2, §11.7.5.3.

## 1. The premise that expired, and the question it reopened

ADR 1113 declined a per-pixel shape channel on a cost — "one raster per element that is open
rather than a second channel every command … carries for the life of every display list" — and
noted that §11.7.5.2's transfer identity "is a different quantity with the same shape of cost, and
it stays named rather than built". Session 1148 built it (ADR 1125): `pdf_render::transfer_channel`
is a run-numbered per-pixel channel on the `DisplayList`, resolved in both backends. The stated
premise is gone, so the conclusion had to be re-taken rather than re-quoted.

**It is upheld, and not on the cost.** The two quantities have the same *shape* of cost and
opposite *positions* in the pipeline, and the position is what decides it.

## 2. The clause's own sentence about when each is applied

§11.7.5.3's NOTE says when §11.7.5.2's function runs:

> This differs from the current halftone and transfer function, whose values are used only when
> all colour compositing has been completed and rasterization is being performed.

So the transfer channel can be **a pass over a finished raster**: every mark is rasterised once as
a shape, the topmost run covering a pixel decides, and the page is mapped at the end
(`resolve_transfers`). Nothing in the compositing needs it.

§11.4.6 puts its shape in the other place. The clause's general case proceeds in two stages per
element — composite the object with the group's initial backdrop at a source shape of 1.0, then
"[c]ompute a weighted average of this result with the object's immediate backdrop, using the source
shape as the weighting factor" — which is, with `B` the initial backdrop, `Pᵢ` the accumulation and
`fᵢ` the element's shape,

```text
Pᵢ = (1 − fᵢ) × Pᵢ₋₁ + fᵢ × Eᵢ(B)
```

The factor multiplies `Pᵢ₋₁`, the accumulation *as it stood under that element*. A pass over the
finished page has only `Pₙ`, and `Pᵢ₋₁` is not recoverable from it: where two elements overlap,
`Pₙ` holds one number per pixel and the recurrence consumed `n` of them. §11.4.6 says as much in
its own words, which is why the separation exists at all:

> The existence of the knockout feature is the main reason for maintaining a separate shape value
> rather than only a single alpha that combines shape and opacity.

**So what could move is the carrier, not the resolution.** A channel could hold the shapes
somewhere other than on the commands; the backend would still have to rasterise each one at the
moment it composites that element. `Command::Shaped` already does exactly that, rasterises each
element's shape at most once, and exists only inside a knockout group. A channel would buy a
different place to keep the same rasters and would cost every command, both censuses and
`viewer-confined`'s wire — which is ADR 1022 §7's price and ADR 1113's, still correct, now for a
reason that does not depend on no channel existing.

## 3. The residues, re-tested against the tree rather than against the ADRs

Four things were `partial` under §11.4.6. Each was read against the code it names (ADR 1201's
method), and two of the four were mis-named.

- **A non-isolated group used as an element — never a shape gap, and the refusal was
  unreachable.** §11.3.7.2 gives a group's shape with no mention of a backdrop: "The shape of a
  group object shall be the union" of the shapes of the objects it contains. So the shape of a
  non-isolated group is that union, and accumulating the union on transparency is the way to hold
  it alone; isolation decides what the group's *colour* and *alpha* carry, not its shape.
  `shape_the_alpha_already_is` had the condition on the pattern and the argument in the sibling
  function. And the refusal string that named it could not fire: `run_transparency_group` emits a
  group nested inside a knockout group with `isolated: true` (the `enclosing_knockout` term), and
  §11.7.4.3's synthesised group is `isolated: self.inside_knockout`, so no non-isolated group
  reaches `stated_shape` by either route. The condition is removed and the string with it.
- **The one position that does refuse a non-isolated group is a *backdrop*.**
  `implicit_knockout_group` declines a part that is one, under **both** `/AIS` readings, because
  §11.4.6's NOTE 6 gives a nested group "the same as that of the outer group; it is not the
  immediate backdrop of the inner group" and none of the three constructions can hand that over.
  §11.7.4.4's row named it "under the shape reading", which is wrong twice: the guard is
  unconditional, and the quantity is the backdrop.
- **`/AIS` both ways, narrowed by one element kind.** `AlphaSourcesSeen::settled_over` borrowed
  `group_alpha_is_shape`'s predicate whole, and that predicate declines a `Command::Shaped` for its
  own reason — proving that the alpha two composites leave is §8.5.4's shape is a separate
  argument, which ADR 1113 §3 priced as costing exactness and never correctness. This clause's
  question is different: a stated pair arrives with its shape **already stated**, under the reading
  its own group's content ran under, and §11.6.4.3 and §11.6.4.4 give the flag only a soft mask and
  two alpha constants to reinterpret — none of which is still an input to a pair. So a `Mixed`
  group holding an inner knockout group's element is described by both readings and is drawn.
  `flag_reinterprets_nothing` asks this clause's question, beside the other one rather than
  through it.
- **`SampleAlpha::Both`, and ADR 1022 §5's price is corrected.** That price was for a field on
  `pdf_render::Image` — "one `Image` would have to carry two rasters" — paid by every stencil under
  an `/SMask` whether or not a knockout group ever asks. A shape stated as a **second command** is
  free outside a knockout group, and the two rasters do exist apart: `pdf_model::image::Picture` is
  `Masked { base, opacity }` on the deferred route, where `base` is the stencil's own samples. What
  blocks it is neither a carrier nor a channel: `Picture::source` multiplies the pair into one
  `ImageSource` before a `Command::Image` exists, and the eager route has already combined them, so
  by the time `stated_shape` is asked there is nothing to separate. **The build is one `Picture`
  arm's work in `pdf-model::image`** — hand the base back beside the command for an image drawn
  while `Interpreter::inside_knockout`, on both routes or neither (trap 5: one route would make the
  refusal depend on the grids rather than on the clause). It is not this round's, because
  `crates/pdf-model/src/image.rs` is not this round's file.
- **The own-backdrop construction on the two other backends** is unchanged and is a backdrop
  again, not a shape: a scene states no per-element backdrop, so `refuse_untranslatable_group`
  refuses it by name and the frame goes to the oracle. `doc/QUORRA_FEEDBACK.md` section 50 states
  what a scene would need.

## 4. What was built, and what it cost

`flag_reinterprets_nothing`, and `shape_the_alpha_already_is`'s arm without the isolation
condition. Two fixtures in `content::transparency::tests`, each calibrated by planting its own
construction away (trap 13):
`a_stated_pair_leaves_mixed_content_a_reading` draws the overlap of an opaque lower element and a
stated pair at `(128, 128, 0)` — §11.4.6's stage b) at a shape of 1.0, where stage a) composites
the object with the transparent initial backdrop and §11.3.6 leaves the blend mode nothing to do,
so the group's result is the object's own alpha of ½ over §11.4.7's red page — and its control, the
same parts with the pair replaced by its object, stays refused because the flag then does
reinterpret §11.6.4.4's constant.
`a_non_isolated_groups_shape_is_the_union_accumulated_on_transparency` holds the arm's three
answers.

**Cost, `callgrind_interpret`, 50 interpretations of ISO 32000-2 page 101, one sitting, two arms
from one tree**: 1 252 899 002 → 1 252 899 128, **+126 instructions, +0.00001%**. Neither change is
on the default path by construction — `Option::or_else` is not evaluated where the `/AIS` record is
already settled, and the other arm is inside a knockout group — and the number is what bounds the
claim rather than restating it.

## 5. What a later round should not re-litigate

That §11.4.6's shape can be carried the way §11.7.5.2's transfer function is. It cannot be
*resolved* that way, and the carrier is not where the remaining residues are. A round that wants
`SampleAlpha::Both` drawn should open `pdf-model::image`'s `Picture`, not `pdf-render`.
