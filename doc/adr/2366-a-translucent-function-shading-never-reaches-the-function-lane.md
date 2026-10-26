# 2366 — A translucent function shading never reaches the function lane, and the lane keeps its rule

Status: deferred — the shading lane's rule for a translucent paint (ADR 2350) is not given to the
function lane, because no paint this tree builds is translucent there; what would reopen it is in
§3. Session 1766, 2026-10-26, the pixels slot of batch one hundred and sixteen, under the project
owner's switch to code improvements and bug fixes of 2026-10-14. Clauses: ISO 32000-2 §8.7.4.5.2
(the type 1 shading), §11.6.4.4 (the constant alpha), with §10.7, which leaves the conversion of a
colour to the device's bits to the device; no ledger row moves. Context: ADRs 0376, 2311, 2324,
2350, 2351; traps 8, 13, 232, 257. Code read: `raster-gpu/src/shaders/function_lane.wgsl`
(`fs_main`), `raster-gpu/src/function/generate.rs`, `pdf-render/src/shading.rs`
(`Shading::with_alpha`, `with_colours`), `pdf-model/src/content/pattern/colouring.rs`
(`shading_with_alpha`), `render-raster/src/scene/shadings.rs` (`function_paint`). Code changed: the
comment above `function_lane.wgsl`'s `fs_main`, which now says why it rounds at any alpha.

## 1. The construction: the lane is handed an opaque answer or nothing

ADR 2350 left the function lane rounding a program's answer to a level at any alpha (ADR 2311)
where the shading lane now hands a translucent paint to the blend unrounded (trap 257), and asked
whether a translucent function shading reaches a corpus first page. Read in the code first: the
generated function returns alpha 1 inside the domain and the `Background` outside it, which
`render-raster` always passes as `None`, so 0; and a constant alpha below 1 reaches a shading
through `shading_with_alpha`, whose `Shading::with_alpha` calls `with_colours` with
`keep_program` false. The program cannot carry §11.6.4.4's constant, so the shading loses it and
is drawn from the grid through the image lane (`sampled_fill`), never by `Paint::Function`. The
lane's `straight.a` is therefore 0 or 1 for every paint the viewer can send, and on both values the
shading lane's `select(straight, nearest_level(straight), straight.a >= 1.0)` is the function
lane's `nearest_level(straight)` to the byte: the rule change would move no pixel.

## 2. The census, and its control

At the scene builder of a scratch copy of HEAD, a line for every sampled shading and every function
paint the device evaluated, over the first pages of the corpus gate's 974 documents through
`QuorraRasterizer` at 100 pixels a side (958 drawn): 12 sampled shadings on two pages
(`function_based_shading.pdf`, 9 of them with a device program, and
`function_based_shading_cmyk.pdf`, 3 without), every one opaque; 8 function paints, all on the
first of those pages, every one opaque; no translucent type 1 shading on any first page. The control (trap
13): the same two pages with every shading given a constant alpha of 0.5 through `with_alpha`
before the draw — the instrument printed all 12 as translucent and without a program, and the
device evaluated none of them, which is §1's construction observed rather than read.

## 3. What would reopen it

A caller handing raster a `Paint::Function` whose `Background` has an alpha below 1 — raster's own
fixtures do (`function_knockout.rs`'s `a_background_alpha_knocks_out_at_full_shape`), and outside
the domain such a background is rounded to a level before its alpha multiplies it, trap 257's
construction — or this tree keeping a program under a constant alpha, which needs the alpha as a
uniform of its own beside the program. Either is a change with pixels to prove on the arms'
adapter; the shading lane's rule is then one line in `fs_main`, and ADR 2351's half-float remedy is
priced beside it.
