# ADR 0492 — The shape a group has when its opacity is one

Status: accepted, 2026-08-22. Session 666. Adds `alpha_is_shape` to `pdf_render::Command::Group`,
`group_alpha_is_shape` to `pdf-model`, `scan::intersect_group` to `render-cpu`, and
`crates/render-cpu/tests/group_clip_intersection.rs`. Amends §11.4.4's, §11.3.6's, §11.3.7.2's and
§10.7.4's ledger rows, `doc/todo/11` item 4, and adds `doc/QUORRA_FEEDBACK.md` section 36.
**Pixels move**, on four of the corpus's 786 ambiguous pages and on the two documents named below.

## The debt, and how long it had no witness

`doc/todo/11` item 4 is §10.7.4's clipping paragraph: a clipping region is a *set of pixels* and
this tree composed it with a mark's coverage as a product. Three of its compositions were paid —
the clip chain (ADR 0280), a fill's own coverage (ADR 0355), a clip standing beside a soft mask
(ADR 0363) — and the last one with a corpus witness was **a group's raster**, which had had no
small witness for nineteen sessions. Session 662 found out why and built one (ADR 0489):

> §11.4.4's NOTE 5 flattens a group away unless a soft mask is in force, so `draw_group`'s blit is
> only *reached* with one beside it.

`crates/pdf-model/examples/coincident_edge_probe` states one rectangle twice, four ways, each with
and without a luminosity mask worth 1.0 at every pixel — a mask that cannot change what any pixel
should be and does change which composition the mark goes through. Seven rungs gave the edge its own
coverage; the eighth squared it.

662 declined the fix and priced it: **a shape channel beside a group's raster, which nothing in this
tree carries today and which would cost a band's bytes per live group.** This round was told to
re-derive that price before believing it, on `doc/habits.md`'s rule that a price is a claim that
decays. It decayed.

## What §11.4.4 requires of a group's shape

Table 139 returns **two** numbers from the group compositing function and names them separately: a
computed *shape* `f`, "used as the object shape when the group is treated as an object", and a
computed *alpha* `α`, used as its object alpha. Table 140 says what accumulates into them —
`fgi`, "the accumulated source shapes of group elements E 1 to E i , excluding the initial
backdrop", and `αgi` beside it — and the clause states outright that both are results:

> The group shape and alpha, 𝑓 g i and αg i , shall accumulate only the shape and alpha of the group
> elements, excluding the group backdrop. Their final values shall become the group results returned
> by the group compositing function.

§8.5.4 then says what the clip at the blit does to the first of the two, in a sentence written for
groups specifically:

> Similarly, the shape of a transparency group (defined as the union of the shapes of its
> constituent objects) shall be influenced both by the clipping path in effect when each of the
> objects is painted and by the one in effect at the time the group's results are painted onto its
> backdrop.

and §10.7.4 says what "influenced by" means on a raster: "the intersection of the set of pixels
defined by the clipping region with the set of pixels for the region to be painted". §11.4.8's
summary prints the arithmetic the intersection feeds — `𝑓𝑠𝑖 = 𝑓𝑗𝑖 × 𝑓𝑚𝑖 × 𝑓𝑘𝑖` and
`𝛼𝑠𝑖 = 𝛼𝑗𝑖 × (𝑓𝑚𝑖 × 𝑞𝑚𝑖) × (𝑓𝑘𝑖 × 𝑞𝑘𝑖)` — so the clip belongs inside `𝑓𝑗`, and the mask and the
constants outside it. A backend that multiplies the clip into the finished alpha has put it in the
wrong bracket, and where the group's `/BBox` is exactly its content's rectangle the two brackets
hold the same edge and the pixel is painted at the square of its coverage.

## The re-derivation: the price is a boolean, not a channel

A raster of premultiplied samples holds one number per pixel. §11.3.7.1 gives the relation between
the two the clause wants: alpha is "the product of shape and opacity". So the two coincide exactly
where the group's **opacity is 1.0 at every point**, and §11.6.4.2 says where that is:

> All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere. Any desired opacity
> less than 1.0 shall be applied by means of an opacity mask or constant

— §11.6.4.3's soft mask and §11.6.4.4's `CA`/`ca`, both of which §11.6.4.3's NOTE 1 hands to *shape*
instead when `/AIS` is true. That is a question about the file, decidable while the content stream is
being run, and **the answer is one bit**. Where it is `true` the group's accumulated alpha *is* its
shape and `min(α, C)` is the clause's intersection rather than an estimate of it; no second channel,
no second buffer, no second render.

**And the layers already held more than the item assumed**, which is the other half of the
re-derivation:

- `pdf-model`'s `shape_without_the_mask_and_the_constants` and `shape_the_alpha_already_is` already
  construct a *command that draws a group's shape*, under either `/AIS` reading, including the
  `Command::Group` arm whose comment is "[a] group's shape is the union of its elements'". So the
  shape channel is not something this tree has never had — it has had it since ADR 0234, for
  knockout elements.
- `Command::Shaped` already carries a shape half through the display list, and `render-cpu`'s
  `encode_shaped` and `knockout_on_backdrop` already draw one.
- `render-cpu`'s `scan::intersected` already composes `min(M·S, P)` for a *fill*, with the identity
  `min(f, C)·S = min(f·S, C·S)` and the argument that rounding is monotone (ADR 0363).

So the general case — a group whose opacity genuinely varies — costs a second full render of the
group's content into a band-sized mask, and the exact case costs **one boolean and one linear pass
over the band**. Those are two different prices and item 4 quoted the first for both.

## What was built

- **`pdf_render::Command::Group::alpha_is_shape`**, documented with the clause reading above. A
  backend may treat it as `false` at any time: it enables an exact composition rather than licensing
  anything.
- **`pdf_model::content::transparency::group_alpha_is_shape`**, asked of a group's elements under the
  `/AIS` reading **its own content** ran under, with a nested group answering by the flag it was
  already given. Three cases are declined rather than disproved, each because the argument would be a
  different one: a knockout group (its accumulation is §11.4.6's `(1 − f) × P + f × E`, not §11.4.4's
  union, and its elements arrive as `Command::Shaped` pairs drawn by two composites), a
  `Command::Shaped` element, and a command whose kind is unknown.
- **`render_cpu::scan::intersect_group`**, which walks the group's band and replaces each pixel's
  alpha with `min(scaled(α, S), P)` — the same closed form `intersected` uses, for the same reason —
  rescaling all four premultiplied channels by `α′ / α` so that the group's *unpremultiplied* colour
  is unchanged. §11.3.6 is why that is the right correction: the source alpha "control[s] the
  influence of the backdrop and source colours", so what is being corrected is the weight, not the
  colour it weights. `α′ ≤ α` always, so nothing overflows; where `α` is zero so is `α′`.
  It declines where the clip is already a set of pixels, which is `intersected`'s own first decline
  and what keeps the pass off the pages that do not need it.
- **`render-cpu`'s `group_blit_mask`**, which takes the composition for an **isolated** group only
  and hands the blit no mask when it does. A non-isolated group's buffer starts as a copy of the page
  (`initial_backdrop`), so its alpha is the backdrop's unioned with the group's and is not the shape
  at all; the interpolation buries the group's shape inside `E(B)` besides, where no factor can reach
  it. A group compositing in a four-component space is *not* excluded, because
  `pdf_render::blending::resolve` writes three channels of each pixel and leaves the fourth.

## The instrument, and the eighth rung

662's probe is the discriminator, and the requirement was that a fix move it and leave the other
seven where they are:

```text
  restated as     no soft mask     soft mask         (was, in 662)
  fill alone            0.5059        0.5059          0.5059  0.5059
  W n clip              0.5059        0.5059          0.5059  0.5059
  form /BBox            0.5059        0.5059          0.5059  0.5059
  group /BBox           0.5059        0.5059          0.5059  0.2549
```

All eight rungs agree. `crates/render-cpu/tests/group_clip_intersection.rs` is the gate, at three
scales and under a rotation, and it asserts four things: that a group of one mark costs one level of
255 and no more (the buffer round trip, which is the control the others are judged against); that a
group clipped by its own content's rectangle equals that control; the same with a unit soft mask
beside the clip, which changes the route and not the picture; and — the one that keeps the flag
honest — that a group whose `alpha_is_shape` is `false` still comes out at the **product**, its
boundary ink the identity's squared.

**The identity is asserted within one level of 255 rather than exactly**, and the reason is worth the
sentence: this tree rasterises a clipping region and a filled mark through two different entry
points, and a rectangle's edge can land a level apart in them. `min` is exact only while both sides
round the same way — ADR 0363's own qualification, one layer up.

## The corpus witnesses

- **`issue7891_bc1.pdf` page 1**, which 662 named. Its two boundary rows are covered 0.504 and 0.456
  and were painted at **0.2549 and 0.2079** — those numbers squared. They now read **0.5059 and
  0.4549**, their own coverage. The page is `CONTRADICTED_TIGHT_CONSENSUS` before and after, which is
  what 662 predicted: the two rows are 0.0197 of a distance of 0.1721, so this moves the page toward
  the bound and past neither.
- **`issue21346.pdf` page 1**, which item 4 asked for a ladder on. Device column 14 of row 89 goes
  `(232, 240, 246)` → `(221, 233, 241)` against an interior of `(206, 223, 235)`, so the edge is
  **0.469 → 0.694** of the mark where departure (1) of §10.7.4's row gives 0.827 and the clause gives
  1.000. In the ratio item 4 uses, `0.827^4.0` became `0.827^1.9`: about two of the page's remaining
  factors are paid and about two are left.

## What it cost, on every gate

Measured before and after by disabling the composition in `group_blit_mask` and re-running, so both
halves are this tree on this machine:

- **Oracle**: identical, verdict for verdict — 908 agree, 65 contradicted, 786 ambiguous, 2 our
  geometry, 2 reference geometry, 13 not comparable, 18 no render, over 1794 pages.
- **Cross-backend gate**: identical — 957 pages, 933 agree, 22 differ, 2 refused, 17 not comparable,
  the same 22 names with the same means. **Unlike ADR 0355, which cost four pages**, this one costs
  none, and the reason is that quorra's own edge treatment already differs on the pages where a group
  blit could show.
- **`doc/todo/00` step 7's ink sweep**, all 786 ambiguous pages: **four rows move and all four move
  up**, by 0.003 to 0.013 of 255 — `issue13520.pdf` +0.588 → +0.601, `bug1721218_reduced.pdf`
  +0.241 → +0.252, `highlights.pdf` +0.249 → +0.255, `transparency_group.pdf` +0.030 → +0.033. The
  negative tail is byte-identical, head `issue12418_reduced.pdf` −19.447 as in session 598, and the
  alarm holds again.

**So no gate in this tree can see this change**, and that is a fact about the gates rather than about
the fix. The instrument that can is the probe, which is why it exists.

## The other two backends cannot take it, and that is where the composite is

`render-quorra` hands a group to `quorra_scene::GroupSpec` and the library folds the group's alpha,
the soft mask, the clip rectangle's analytic overlap and the clip residue into one scalar in
`composite.wgsl`. `render-gpu` hands it to Vello's `push_layer`, with the clip already open around it
as a stack of `push_clip_layer` calls. Neither backend owns the arithmetic, so neither can take the
intersection without the library taking it.

This is trap 2's shape — "a decision either could make alone is a decision neither has made" — and
the answer is the one ADR 0355 already used: **the decision is stated in the shared crate**, on the
command, in the layer that knows the `/AIS` reading, so all three backends read the same statement
and two of them decline it for a reason that is written down beside them. `doc/QUORRA_FEEDBACK.md`
section 36 is the ask, and it is deliberately an ask for a *flag plus a `min`* rather than for a
shape channel: the whole point of the flag is that it says when nobody needs one.

## What item 4 still owes

A stroke's coverage and an image's edge, both unchanged and both without a witness on this
construction. A **non-isolated** group's raster, with the reason above. A group whose opacity is not
1.0 everywhere, which is where the shape channel is genuinely the price — a second render of the
group's content into a band-sized mask, plus the `Command::Shaped` wrapper that would carry it — and
which no corpus document has yet been shown to need. And the two backends that still multiply, now
including the group blit.

## The errata, read for clause 11 before any of the above

`cargo run -p spec-errata -- emit doc/*.pdf` files four annotations under clause 11 and none of them
touches the arithmetic here. Two on p. 428 (Issue #688) replace the glyph `a` with `α` in §11.4.8's
summary — the page is genuinely §11.4.8, checked against `pdftotext`, and the printed formulas do
read `𝑓𝑔0 = 𝑎𝑔0 = 0` and `𝑎0 = 0` where they mean the alphas. Two on p. 436 (Issue #619) mark entries
of §11.6.6 "Deprecated in PDF 2.0". `spec-errata check` lists no struck passage anywhere in §11.3 or
§11.4 that `doc/md/` still carries as current text.

One thing the reading found that is not an erratum: **`doc/md/`'s extraction of §8.5.4 breaks the
word "backdrop" across a space**, so the group sentence quoted at the top of this ADR cannot be
quoted verbatim in code past "when each of the objects is painted" — the conformance gate rejects it,
correctly. Both places that quote it stop there and say why.
