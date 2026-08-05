# The GPU coverage lane, and what this viewer needs to do to use it

*Written for `pdf-viewer` about a change in `quorra` (the sibling checkout at
`/home/cl/projects/render-lib`, upstream `https://github.com/close2/quorra`).
The lane is complete and selectable: `quorra_gpu::Options::coverage`. Nothing in this
viewer uses it yet, and this is what using it would cost.*

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

`quorra_gpu::Options` has the selector — `coverage: Coverage::{Cpu, Gpu}`, and
`coverage_samples` for its quality. `render-quorra` constructs `Options` in two places — `QuorraRasterizer::new` and `QuorraPresenter::around` — and
both currently take `Options::default()`. The change is to thread a choice in:

```rust
// crates/render-quorra/src/present.rs
pub fn with_options(
    window: impl Into<quorra_gpu::wgpu::SurfaceTarget<'static>>,
    coverage: quorra_gpu::Coverage,
) -> Result<Self, QuorraRasterError> { … }
```

Keep `new` as it is, defaulting to the CPU lane. A backend knob this project's §8.3
talks a host out of is still a knob, and this one earns its place only where the
measurement below says it does.

### 2. Decide *when*, not *whether* — the crossover is measured

Dense 5 933-fill page at 1191×1684, RADV, wall clock per frame:

| magnification | CPU lane | GPU lane |
|---|---|---|
| 1× | **1.1 ms** | 15.2 ms |
| 4× | **0.54 ms** | 2.5 ms |
| 20× | 12.5 ms | **1.9 ms** |
| 100× | 7.4 ms | **2.2 ms** |

**The crossover is between 4× and 20×.** The shape is what the two designs predict: the
CPU lane has a glyph atlas and pays per pixel rasterised, so it wins while glyphs are
small and repeated; the GPU lane has no atlas and pays per triangle, so it wins once a
glyph costs more to fill than to describe.

So the choice is **per frame, keyed on magnification**, and this viewer is the only
crate that knows the magnification. The natural home is the presenter call in
`viewer-ui/src/bin/pdf-viewer.rs`, and the natural signal is already in hand:
`RenderRequest::target`'s scale against the page size.

Two cautions before wiring it up. The number above is this laptop's — take it again on
the target machine, with `cargo run --release -p quorra-gpu --example zoom -- gpu`.
And switching lanes mid-session means switching *devices* today, because the lane is an
`Options` field read at construction: either hold two presenters, or ask quorra for a
per-frame selector. The second is a smaller change and is worth asking for before
building the first.

### 3. Nothing changes in `pdf-render`

`TargetSpec`, `DisplayList`, `Rasterizer` and the display list's contents are
untouched. The lane is entirely inside quorra's coverage step, below the level this
project's contract describes.

## What the lane still does not do

- **Residue clips take the CPU lane.** A non-rectangular clip multiplies into coverage
  on the CPU, and no pass does that on the device yet. Such commands fall back
  silently and correctly — both kinds of tile share one sheet — so a page full of
  non-rectangular clips gets the CPU lane's cost even under `Coverage::Gpu`.
- **No atlas stands in front of it.** That is most of why 1× costs what it does, and it
  is deliberate: a cache keyed on the device transform is exactly what a zoom gesture
  defeats.
- **Selecting the lane per frame** is not possible yet (see §2).

## One thing worth knowing about correctness

The two lanes do not produce identical pixels, and the difference is not all in the
direction you would expect. They agree **exactly** where no edge crosses a pixel. On a
straight edge they differ by at most an eighth of a pixel of coverage — the sample
grid. On a **curved** edge they differ by up to a quarter pixel more, and there **the
GPU lane is the more accurate of the two**: quorra's CPU rasteriser flattens curves to
a quarter-pixel tolerance and a chord cuts inside a convex curve, while the GPU lane
draws the quadratics themselves. If this viewer's comparison harness starts reporting
differences on curved artwork after a switch, that is the reason, and the CPU backend
is not the reference to trust on it.

## Where the numbers came from

`crates/quorra-gpu/examples/zoom.rs` in the quorra checkout, release build, RADV
(Radeon 890M), the dense 5 933-fill page at 1191×1684, best of five per row.
