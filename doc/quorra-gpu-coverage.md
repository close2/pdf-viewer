# The GPU coverage lane, and how this viewer chooses it

*Written for `pdf-viewer` about a change in `quorra` (the sibling checkout at
`/home/cl/projects/render-lib`, upstream `https://github.com/close2/quorra`).
The lane is complete, and this viewer now chooses between it and the CPU one **per
frame**, from the magnification. This is what that choice is, and where it came from.*

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

`quorra_gpu::Options::coverage` sets the default and `Device::set_coverage` changes it
between frames; `QuorraPresenter::set_coverage` forwards the second, and `viewer-ui`
calls it before each `present`. The selector is — `coverage: Coverage::{Cpu, Gpu}`, and
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

### 2. Decide *when*, not *whether* — the crossover is a cliff

Dense 5 933-fill page at 1191×1684, RADV, wall clock per frame in milliseconds:

| magnification | 1× | 2× | 4× | 6× | 8× | 12× | 16× | 20× | 100× |
|---|---|---|---|---|---|---|---|---|---|
| CPU lane | 1.05 | 0.67 | 0.50 | 0.46 | **0.44** | 4.4 | 4.5 | 12.1 | 5.9 |
| GPU lane | 11.3 | 4.2 | 2.3 | 2.1 | 1.71 | **1.66** | 2.06 | 1.58 | 1.90 |

**The crossover sits between 8× and 12×, and it is a cliff rather than a curve.** The
CPU lane costs ten times more at 12× than at 8× — for a fraction as much visible page
— because that is where glyphs cross quorra's 128-pixel atlas limit and stop being
cached. Below it the CPU lane answers from a handful of tiles; above it it rasterises
every glyph on every frame, while the GPU lane is flat because nothing in it depends on
the scale.

So the threshold is **derived, not tuned**: `128 ÷ the height of the text`. For body
text of 10 to 12 points that is 13× down to 10.7×, and `GPU_COVERAGE_MAGNIFICATION` in
`viewer-ui/src/bin/pdf-viewer.rs` is ten — the low end of the band, because being early
costs a fraction of a millisecond and being late costs ten.

The magnification comes from the frame's own transform, as the square root of its
determinant rather than from its `a`: §7.7.3.3's page rotation puts the scale into `b`
and `c` instead, and reading `a` would quietly choose the slow lane on every rotated
page in the corpus. `tests::a_rotated_page_reads_the_same_magnification` is that case.

A page whose text is much larger or smaller than a book's crosses the cliff somewhere
else. The honest improvement is to ask the display list how tall its text is, not to
move the constant.

### 3. Nothing changes in `pdf-render`

`TargetSpec`, `DisplayList`, `Rasterizer` and the display list's contents are
untouched. The lane is entirely inside quorra's coverage step, below the level this
project's contract describes.

## What the lane still does not do

**Two of these three stopped being true while this document said them**, and both were
overtaken by quorra rather than by anything here — which is what a document about
somebody else's code does (ADR 0283). They are kept, struck, because the correction is
worth more than the tidy list.

- **Residue clips take the CPU lane.** A non-rectangular clip multiplies into coverage
  on the CPU, and no pass does that on the device yet. Such commands fall back
  silently and correctly — both kinds of tile share one sheet — so a page full of
  non-rectangular clips gets the CPU lane's cost even under `Coverage::Gpu`.
- ~~**No atlas stands in front of it.**~~ One does, since `74c4994d`: a tile the atlas
  admits *and the page places more than once* is answered from the atlas even under
  `Coverage::Gpu`. What is left of the sentence is the reason it was written — a cache
  keyed on the device transform is what a zoom gesture defeats — and that is now the
  criterion rather than a caveat: the device takes the tiles the atlas will not hold or
  will not benefit from, which during a zoom is most of them.
- ~~**Nothing chooses per *command*.**~~ The lane is chosen per command, since
  `c1f6e2f4` by cost and since `74c4994d` by what the atlas will *do* with the tile —
  refuse it, hold one it already has, hold one the page places once, or hold one the
  page places again. `Coverage::Gpu` is now the statement that the device is *available*
  for a command rather than that it draws every one. The magnification still selects it,
  in `viewer-ui`, for the reason the cliff above gives.

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

## What the lane does on the corpus

**Nothing measured it here until ADR 0283**, which is the point worth taking from this
section: the whole document above rests on one dense page and a set of fixtures, and
`viewer-ui` switches a person onto the lane on the strength of it. The gate that puts a
backend beside the CPU oracle over 974 real first pages now runs either lane —
`PDFVIEWER_QUORRA_COVERAGE=cpu|gpu`, `doc/verify.md` — and the ratchets stay on the
default one, because the two lanes are stated not to draw identical pixels.

The numbers are the ADR's and `doc/QUORRA_FEEDBACK.md` §20's rather than this file's,
for `CLAUDE.md`'s reason. What belongs here is the shape: at the page's own scale the
second lane agrees with the oracle on all but a few dozen pages and refuses a handful,
and **at four times that scale — nearer to where the lane is actually selected — its
refusals are what move**, not its pixels.

## Where the numbers came from

`crates/quorra-gpu/examples/zoom.rs` in the quorra checkout, release build, RADV
(Radeon 890M), the dense 5 933-fill page at 1191×1684, best of five per row.
