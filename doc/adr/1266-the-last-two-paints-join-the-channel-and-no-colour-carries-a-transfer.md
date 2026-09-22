# 1266 — The last two paints join the channel, and no colour carries a transfer

Status: accepted. Session 1214.
Amends: ADR 0479, which put §10.5's function inside a shading's ramp where its colours are made;
and ADR 1255 §4, which designed this and did not build it.
Depends on: ADR 1125 (the function at an antialiased edge), ADR 1148 (the channel), ADR 0430 (a
cell drawn once and a tile that is a copy), ADR 1255 (the channel's shape is the clause's).
Context: `crates/pdf-model/src/content/pattern.rs`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content/marked.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/tests/transfer_functions.rs`.
Clauses: ISO 32000-2 §11.7.5.2, §11.7.5.3, §10.5, §11.6.7, §8.7.3.1.

`§N` is ISO 32000-2 and nothing else.

## 1. Where the clause puts the map, and where the two paints still put it

§11.7.5.3's NOTE, about the halftone and transfer function:

> This differs from the current halftone and transfer function, whose values are used only when
> all colour compositing has been completed and rasterization is being performed.

ADR 1148 built the channel that does exactly that and left two paints out, each for its own
reason. A **shading pattern's ramp** was sampled under the function (ADR 0479), because applying
the function to a ramp the §10.7.3 simplifier has already reduced to two stops draws the chord
between the transferred ends. A **tiling cell** was interpreted once and copied to every site
(ADR 0430), so one shape in the channel could not stand for the marks of every tile.

Both reasons are about applying the function to a *ramp* or to a *mark*. The channel applies it to
neither: it applies it to the finished device pixel. So the ramp is sampled raw and the curve is
evaluated once per pixel instead of once per stop, which is strictly the clause's own order and
strictly more of the curve than ADR 0479's sampling kept.

## 2. What that changes, beyond the ordering

- `MarkColouring` loses its transfer, so a shading pattern's colours depend on §11.7.2's target
  alone — and `shading::Cache` keys them again, where a stated function had disabled the cache.
- A type 1 shading keeps its **device program** (§7.10.5 lowered for the GPU). It was withdrawn
  under a stated function because there was nowhere on that path to apply one; the map is applied
  to the read-back now, so the program computes the colours it always computed.
- `GraphicsState::solid_fill` and `solid_stroke` no longer take a function, and
  `Interpreter::mark_transfer` answers once instead of splitting into an inside half and an
  on-mark half. **No colour built anywhere in this tree carries §10.5's function.**

## 3. The tiling, and which object the clause chooses a function for

§11.7.5.2's sixth condition names a tiling pattern's cell without making the cell's marks the
topmost objects at a point:

> If the current colour is a tiling pattern, all objects in the definition of its pattern cell
> also satisfy the foregoing conditions.

It is a condition on the object *painted with the pattern* being fully opaque. So that object is
the elementary object the clause chooses a function for, the function is the one in force at the
painting operation, and the cell's objects decide only whether it is withheld.
`Interpreter::tile` reads the function before the cell runs — `run_cell` starts from
`GraphicsState::initial` for §11.6.7's reason, so nothing inside can see it — accumulates the sixth
condition over the cell's marks in `tiling_cell_opaque`, and `record_tiling` puts the function on
every tile of the finished tiling as one run. The shapes are the tiles themselves, so a pixel in
the gap between two cells is enclosed by no object here and keeps whatever is under it.

A consequence worth stating because it was the other way round before: **a `/TR` in a pattern
cell's own `/ExtGState` decides nothing.** §11.7.5.3's NOTE takes the value out of colour-making
altogether, so it is not among the parameters §11.6.7's bullets evaluate a pattern definition
under, and §11.7.5.2 puts the painting mark's function at the point instead. §11.6.7's ledger row
has said so since the six-hundred-and-fifty-fifth session; the code said the opposite.

## 4. The fixtures

`tests/transfer_functions.rs` now asks every shading question at the **pixel**, because that is
where the clause's answer is. `a_ramps_pixel_is_the_composition_and_not_the_chord` keeps ADR 0479's
own discriminator — `/N 2` gives 64 of 255 at the ramp's midpoint, not the chord's 128 — and it
passes with the ramp raw, which is the whole argument of §1 at one pixel.
`a_translucent_mark_over_a_transferred_shading_takes_the_pages_default` is §11.7.5.2's two-mark
case: a translucent blue over a fully opaque shading pattern draws the same pixel as the solid
colour the shading paints there, because the topmost object is not fully opaque and the clause maps
nothing; outside the blue the shading is topmost and opaque and its own function maps the
composite. `a_function_based_shadings_pixels_are_mapped_and_its_device_program_stands` holds §2's
second bullet.

## 5. What is left, and what the report names now

`Unsupported::TransferFunction` named the two paints. It names the one thing the channel still
cannot state: a stencil under an `/SMask` of its own
(`pdf_render::SampleAlpha::Both`), whose single alpha channel is §11.6.4.2's shape multiplied by
§11.6.4.3's opacity and cannot be separated again, so such an image occludes only where its mask is
non-zero where the clause gives it the whole rectangle. That is the residue ADR 1218 and ADR 1255
already named from §11.3.7.2's side. `Interpreter::note_unstatable_shape` raises it only while the
channel is live, which is the geometric over-approximation of what such an image can draw wrong:
with no mark on the page carrying a function, which run owns a pixel changes no colour.

## 6. Where the report is asked, and what asking it anywhere else cost

`draw_mark` is the obvious place and it is the wrong one. Fifty interpretations of ISO 32000-2's
own page 101 under callgrind, one sitting, `examples/callgrind_interpret`:

| | I refs |
|---|---|
| no check at all | 1 250 655 524 |
| the check in `draw_mark`, discriminant test only, branch never taken | 1 252 185 325 (+0.12%) |
| the check at `draw_image`, where the mark is known to be an image | 1 250 681 793 (+0.002%) |

Page 101 is text and vector graphics and carries no image, so the middle row is the cost of
*asking* rather than of answering — a borrow of the `Command` before it is moved, in a function
called once per mark. Two shapes of the question in `draw_mark` measured the same (a call gated on
`TransferBuilder::is_live`, and the discriminant written out inline), which is what said the cost
was the position rather than the test. The check now sits in `Interpreter::draw_image`, which has
the raster in hand already.
