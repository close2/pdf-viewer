# The GPU coverage lane, and what this viewer needs to do to use it

*Written for `pdf-viewer` about a change in `quorra` (the sibling checkout at
`/home/cl/projects/render-lib`, upstream `https://github.com/close2/quorra`).
Status at the time of writing: the lane exists and is proven; it is **not yet wired
into quorra's encoder**, so nothing in this viewer can switch to it today. What
follows is what it will cost when it can.*

## Why there is a second lane at all

quorra rasterises coverage — the anti-aliased shape of a glyph or a path — **on the
CPU**, into an R8 tile that a textured quad then draws (its ADR 0008). Small text is
cheap because the tiles are cached in an atlas keyed by outline, transform and
sub-pixel phase: a dense page's 5 933 glyph fills collapse to 107 distinct tiles.

Zoom breaks that. Past 128 device pixels a glyph never enters the atlas, so every
visible glyph is re-rasterised on the CPU **every frame**, cached by nothing. Measured
on this machine (RADV, dense page, 1191×1684): at 20× magnification a frame spends
6.8 ms in `encode` for **thirty** commands, and a 1×→20× zoom gesture's worst frame
cost 156 ms before command culling and 9.3 ms after it. What remains is all
rasterisation.

The second lane rasterises coverage on the GPU instead, by Evan Wallace's method
([*Easy Scalable Text Rendering on the
GPU*](https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac),
2016):

- one triangle per outline segment, fanned from an anchor point, accumulated with
  additive blending — the sum at a sample **is** the winding number of
  ISO 32000-2 §8.5.3.3;
- one extra triangle per curve, kept only where Loop and Blinn's implicit test says the
  fragment is inside it, which adds or subtracts the bulge between chord and curve
  purely by the triangle's own orientation;
- the fill rule applied per sample, and the fraction of samples that passed is the
  coverage byte.

The consequence that matters here: **nothing in it depends on the magnification.**
Cubics are converted to quadratics once, at `upload_outline`, and a frame at 100×
re-uses exactly what a frame at 1× did. There is no flattening tolerance in device
pixels, so there is no per-frame cost that grows as a person zooms in.

## What it is not

Three things this viewer's authors should know before asking for it:

1. **Anti-aliasing is sampled, not exact.** The CPU lane computes the *exact* area of
   each pixel a shape covers (quorra's ADR 0005/0008) — 256 levels, analytically. The
   GPU lane counts samples on an ordered grid: sixteen samples give seventeen levels.
   On a half-covered pixel both produce 128; on a pixel covered 0.31 the CPU lane gives
   79 and the GPU lane gives 75 (5/16). **Text at reading size is where this is most
   visible**, which is exactly where the CPU lane is already fast. The sample count is
   an option; it costs time, not memory.
2. **It changes nothing about determinism *across* adapters — deliberately.** The
   sample grid is stated in quorra rather than taken from the driver's multisample
   pattern, precisely so that RADV and lavapipe still agree byte for byte. That is the
   promise this project's CI leans on, and it survives. (Wallace's own packing of
   samples into the bits of an 8-bit colour buffer would not have: it is even-odd only,
   and PDF needs non-zero — §8.5.3.3.2 is the default rule. quorra accumulates a
   *signed* winding into a float target instead, and both rules fall out of the same
   number.)
3. **It is not a replacement.** Both lanes produce the same artefact — an R8 tile in
   the same sheet — so the choice is per device, not per crate, and the CPU lane
   remains the one whose bytes are exact and whose output the `render-cpu` oracle
   agrees with.

## What this viewer has to do

Nothing structural. The whole of it is one option and one decision about when to set
it.

### 1. Pass the option through `render-quorra`

`quorra_gpu::Options` will gain a coverage selector. `render-quorra` constructs
`Options` in two places — `QuorraRasterizer::new` and `QuorraPresenter::around` — and
both currently take `Options::default()`. The change is to thread a choice in:

```rust
// crates/render-quorra/src/present.rs
pub fn with_options(
    window: impl Into<quorra_gpu::wgpu::SurfaceTarget<'static>>,
    coverage: quorra_gpu::Coverage,
) -> Result<Self, QuorraRasterError> { … }
```

Keep `new` as it is, defaulting to the CPU lane. A backend knob this project's §8.3
talks a host out of is still a knob; this one earns its place only if the measurement
below says it does.

### 2. Decide *when*, not *whether*

The two lanes have opposite cost curves, and the crossover is a magnification:

| | CPU lane | GPU lane |
|---|---|---|
| page at 1×, dense text | 107 atlas tiles, ~1.0 ms/frame | one triangle per segment, every frame |
| page at 20× | 6.8 ms/frame, cached by nothing | unchanged by the zoom |
| memory | atlas, 8 MiB budget | a winding texture the size of the visible tiles |

So the honest integration is **not** a build-time choice but a per-frame one keyed on
magnification, and the viewer is the only crate that knows the magnification. The
natural home is `viewer-ui`'s presenter call, and the natural signal is the one already
in hand: `RenderRequest::target`'s scale against the page size.

Do not implement that until quorra reports the crossover. Ask for the number; do not
guess it.

### 3. Nothing changes in `pdf-render`

`TargetSpec`, `DisplayList`, `Rasterizer` and the display list's contents are
untouched. The lane is entirely inside quorra's coverage step, below the level this
project's contract describes.

## What is left in quorra before any of this is possible

Recorded here so that this viewer's todo list can point at it rather than re-derive it
(quorra's ADR 0016 has the full version):

- **The encoder integration.** The lane is proven end to end — an aligned square is
  solid inside and empty outside, a half-covered column reads exactly 128, and nested
  same-wound squares fill under §8.5.3.3.2 and hollow under §8.5.3.3.3 — but the
  encoder still routes every fill to the CPU rasteriser. Wiring it means the scratch
  packer reserving tile space without bytes, and the frame budget pricing the winding
  texture before it is allocated.
- **Residue clips.** A non-rectangular clip multiplies into the coverage mask on the
  CPU. Until the same happens on the GPU, a command under one has to take the CPU lane,
  which means a frame can need both sheets at once.
- **The crossover measurement**, per §2 above.

## Where the numbers came from

`crates/quorra-gpu/examples/zoom.rs` in the quorra checkout, release build, RADV
(Radeon 890M), the dense 5 933-fill page at 1191×1684. Run it before believing any of
the figures above: they were taken on a loaded machine and the ratios are what carry.
