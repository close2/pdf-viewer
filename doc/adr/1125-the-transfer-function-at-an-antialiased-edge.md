# ADR 1125 — §11.7.5.2's transfer function is chosen by shape, and applied after compositing

Status: accepted, 2026-09-16. Session 1118; **built in session 1148**, whose section is last. Settles the design question `doc/todo/13` says a round
taking §11.7.5.2's last shape inherits — how the clause's "nonzero object shape value" decides the
transfer function at a partly covered pixel — and records the construction that follows from it,
which is deferred rather than started for the reasons below. No pixel moves in this session: the
decision is what a later round builds, and `render-cpu/tests/transfer_edge.rs` is the fixture it
inherits.

## The question

§11.7.5.2 chooses the transfer function at a point by the topmost object covering it, and defines
that object by shape:

> The topmost object at any point shall be defined to be the topmost elementary object in the
> entire page stack that has a nonzero object shape value ( f j) at that point (that is, for which
> the point is inside the object).

*Nonzero*, not one. A pixel an antialiased edge covers partially has a nonzero shape there, so it
is inside that object and takes that object's function — although its colour is a blend of that
object's colour with the backdrop's. The question was whether the edge pixel is inside the object
for the purpose of choosing the function; the clause's own words answer it, and the answer is yes.

## What follows, and where it runs

§11.7.5.3's NOTE says when the chosen function runs:

> This differs from the current halftone and transfer function, whose values are used only when
> all colour compositing has been completed and rasterization is being performed.

So the clause composites the raw colours first and maps the finished pixel once. At an edge pixel
the object encloses, that is `transfer(blend(object, backdrop))`. This tree applies §10.5's
transfer to an object's colour *before* compositing (`pdf-model`'s `fill_paint`, `stroke_paint`
and the image sample map, ADR from the 358th session; a shading's own colours in `shading` and
`mesh`, ADR 0479), so at the same pixel it draws `blend(transfer(object), backdrop)`. The two
agree wherever the object fully covers the pixel — the interior, where the composite *is* the
object's colour — and diverge at the edge by as much as the transfer bends the colour.

The construction that closes the gap is the one `doc/todo/13`'s third piece derives, and this ADR
adopts it unchanged: a per-pixel *transfer identity* rasterised beside the colour — each fully
opaque mark writing its own function's index where its shape is nonzero, each mark that is not
fully opaque writing the page default's — and one pass over the finished raster mapping each pixel
through the function its index names. Trap 2 binds: the channel's meaning is `pdf-render`'s to
state, or two backends answer it apart.

## Why it is deferred rather than built

Three measurements, each taken this session:

- **The population is one document.** `examples/transfer_function_census` over the corpus: 13 state
  a Table 57 `/TR` or `/TR2`, exactly one states anything but `/Identity` or `/Default`
  (`issue6931_reduced.pdf`), and it draws its one image fully opaque with no translucent mark over
  it and no shading. So the change has no oracle witness, which is what makes it a `doc/todo` entry
  and not a defect — the deferral's honesty is a measurement, and it decays the moment a document
  turns up painting a translucent mark over a transferred opaque one.
- **Both backends composite through `tiny-skia`.** `render-cpu` and `render-raster` both hand marks
  to `tiny-skia`'s pipeline, which exposes no per-pixel "which object last wrote me with nonzero
  shape" hook. The index channel is therefore a *second* rasterisation pass over every mark —
  re-running clip, group and mask geometry to write indices — not a field added to an existing
  pass. That is the "matching pass in all three backends" `doc/todo/13` prices, and it is large.
- **It must cost nothing on the 973 documents that state no transfer.** The channel is gated on a
  display list holding any transfer at all; where none does, no second pass and no final map run.
  The gate is cheap but the machinery it guards is not, and building the machinery is the work.

Starting that half-way — the channel in `render-cpu` alone — would make the oracle and
`render-raster` disagree at every transferred edge, regressing their agreement on the one corpus
witness. `CLAUDE.md` principle 1 governs: a change this size is not started until it can be
finished across both backends, so this session records the decision and plants the fixture instead.

## The fixture

`render-cpu/tests/transfer_edge.rs` builds an opaque grey fill with an antialiased edge over white,
under an inverting transfer, and measures three pixels: the interior, where the pipeline and the
clause agree because coverage is one; the edge, where the pipeline draws `blend(transfer(object),
backdrop)` and the clause asks for `transfer(blend(object, backdrop))`, a measured gap of half a
unit at a half-covered edge; and a control with no transfer, unchanged. It is the regression
witness the build owes (trap 8, trap 13): when the channel lands, the edge's asserted value becomes
the clause's, in one place.

## Correction and sharpened pricing, session 1137

The round that inherited the build looked at the two backends and found the deferral's second
measurement wrong in its premise and its price. The design above is unchanged and correct; what
follows corrects *why it is deferred* and plants the raster half of the fixture.

- **`render-raster` is not a `tiny-skia` compositor.** It is the `raster-gpu` **wgpu compute**
  rasteriser (`render-raster` depends on `raster-gpu`, `raster-scene`, `wgpu`), and only `render-cpu`
  uses `tiny-skia`. So "both backends composite through `tiny-skia`" was false. `QuorraRasterizer::rasterize`
  returns the composited pixels to the CPU and already runs several CPU-side passes over that
  read-back — `resolve_grey`, `crop_to_page`, `impose_within`, `resolve_blending`. The transfer map
  belongs beside them: a per-pixel final map on the read-back, needing **no change to the GPU
  pipeline at all**.
- **Agreement is achievable, so it is not the blocker.** §11.7.5.2's per-pixel index — the topmost
  fully-opaque object's function at a point — is a pure function of geometry and opacity, independent
  of colour. Both backends applying the identical final map therefore agree by construction, to
  within the colour-composite tolerance they already share. The "whole second rasterisation pass in
  each backend" was priced for a channel *threaded through compositing*; the colour-independence
  makes it a coverage pass that decides the index plus a final map each backend applies to its own
  read-back — materially smaller than this ADR and `doc/todo/13` first said, and it does not need
  the GPU rasteriser reopened.
- **The true blocker is the per-mark carrier.** §11.7.5.2's function is a property of an *elementary
  object*, so it must attach to the leaf `Command::Fill` / `Command::Image` marks — groups do not
  carry it. That is **203 `Fill` + 69 `Image` = 272 construction sites across eleven crates**,
  including `pdf-model` (a sibling's colour work this batch), `render-gpu`, the viewer crates and
  `test-scenes`, none of which this round owns; the field cannot be added backend-by-backend without
  breaking compilation, and cannot land on a five-sibling shared branch. The one carrier that touches
  almost no sites — an `Option` side-table on `DisplayList`, keyed by top-level command position — is
  flat-list only: it cannot index a mark nested inside a transparency group, so it would silently
  drop the grouped case (trap 5), which principle 1 forbids as much as it forbids the one-backend
  change. So the build stays deferred — not for disagreement, which is solved, but for a carrier that
  cannot be landed correctly this round.
- **The population is unchanged**: still `issue6931_reduced.pdf` alone, a fully opaque image with no
  translucent overlap (session 1118's census; no document was added to the corpus this batch). No
  oracle witness for the antialiased-edge overlap.

`render-raster/tests/transfer_edge.rs` is the raster half of the fixture, the parallel of the CPU
one 1118 planted: it measures that the raster backend draws the same pre-composite ordering at the
transferred edge (`blend(transfer(object), backdrop)`, gap of half a unit to the clause's
`transfer(blend(object, backdrop))`), and that the two backends *agree* on that value within the bar
`headless_quorra` holds them to. That agreement is the reason a channel added to one backend alone is
forbidden, and the reason the witness must flip both backends together when the carrier lands.


## Built, session 1148: the carrier, and what it turned out to be

The blocker 1137 named was the carrier, and the answer is that it is not a field on the mark at
all. A field on `Command::Fill`/`Command::Image` is the 272 sites; a side-table keyed by position
cannot reach a mark nested in a group. What reaches both is a **parallel channel on the
`DisplayList`**, the construction `set_blending`'s companion list already uses one clause over.

- **`pdf_render::TransferBuilder`** collects every elementary mark the interpreter draws, in
  painting order, as an opaque *shape* — the same path, transform, clip and fill rule, painted
  white under Normal with no mask — together with the function in force when it was drawn. It
  reaches nested marks because it is filled at leaf creation (`Interpreter::draw_mark`), which is
  the one route every mark takes, groups included; ordering survives because a group's leaves are
  drawn before the `Command::Group` that collects them and therefore in the page's own order.
- **Runs, not indices.** Consecutive marks sharing a function are one `TransferRun`, and run
  numbers rise with painting order — so the topmost object covering a pixel is the one in the
  *highest* run covering it, and nothing per-pixel has to carry an ordering. `resolve_transfers`
  walks the runs top down and takes the first whose coverage at a pixel is nonzero, which is the
  clause's own "nonzero object shape value" and means every mark is rasterised at most once.
- **Zero where nothing states a function.** The builder is inert until a mark carries one, so a
  page that states none records no shape and `DisplayList::transfers()` is `None`; on ISO 32000-2
  page 101 under callgrind no instruction of the channel executes at all.
- **Both backends, the same pass.** `resolve_transfers` takes a closure that rasterises one shape
  list; `render-cpu` supplies `encode_in_strips` onto transparency and `render-raster`
  `QuorraRasterizer::render`. Each maps its own read-back, so the rule is stated once (trap 2) and
  the two agree by construction — `render-raster/tests/transfer_edge.rs` measures that they do.

**What the fixtures now say.** Both halves assert §11.7.5.2's own value at the half-covered edge —
0.375 under an inverting transfer over white, where the pre-composite ordering drew 0.875 — and the
no-transfer control is unchanged.

**What is left, and it is two paints.** A *shading*'s colours are sampled under the function where
they are made, because mapping a simplified ramp's two stops draws the chord between the transferred
ends (ADR 0479); a *tiling* cell is interpreted once and copied to every site (ADR 0430), so one
recorded shape cannot stand for its marks. Both keep §10.5's pre-composite application,
`Interpreter::tile` records the finished tiling as occluders so that nothing beneath it is mapped
twice, and `Unsupported::TransferFunction` is narrowed to exactly those two. **ADR 0479's reason
does not survive the channel** — a raw ramp simplified and then mapped per device pixel has no chord
— so a later round can put a shading on the channel too; that is an amendment to 0479 and was not
taken here.

`render-gpu` has no pass of its own over a Vello scene's result and refuses a list carrying the
channel by name, which sends the frame to the CPU backend. `viewer-confined`'s wire format has no
shape for a second sequence of marks per run, so such a page crosses as pixels its own CPU backend
drew (`Uncodable::TransferChannel`) — which is the same picture, by the route two of `doc/pdf.js`'s
pages already take.
