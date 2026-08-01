# ADR 0127 — A page the device drew nothing of, and said nothing about

Status: accepted, 2026-08-01.

## The report, and what it turned out to be

> I can't view page 6, 7, 1010 or 1011 of `ISO_32000-2_sponsored_EC3.pdf`. … `--cpu` does show
> page 6. Opening page 6 directly without `--cpu` shows a black page.

That last sentence is the whole diagnosis, and it arrived only because the previous session
shipped the flag that could produce it (ADR 0125, ADR 0126). Reproduced headlessly on the same
AMD 890M, rendering page 6's display list at two scales:

| | 708×1001 | 1132×1600 |
|---|---|---|
| CPU ink | 9 757 958 | 24 909 896 |
| GPU ink | 9 670 433 | **0** |

A sweep of the scale finds a cliff, not a slope: 1.8 draws, 1.9 draws **nothing**, and every scale
above it draws nothing. Page 5 at the same scale is fine.

## The cause, in Vello's own words

`vello_encoding::BufferSizes::new`:

> The following buffer sizes have been hand picked to accommodate the vello test scenes as well as
> paris-30k. These should instead get derived from the scene layout using reasonable heuristics.

They are constants — `tiles`, `seg_counts`, `segments` at 2²¹ each, `bin_data` at 2¹⁸. A scene
whose per-tile segment lists need more overflows one *on the device*: the shaders set a `failed`
bit in the bump allocators, stop filling, and the fine rasteriser writes nothing. `Renderer::
render_to_texture` returns `Ok(())`.

Page 6 is 5933 paths of small text over 71 × 100 tiles. It is not a complex page — it ranks 857th
of 1023 in this document by path point — it is an *ordinary* page at an ordinary resolution, and
the resolution is what a 1023-page A4 fitted to a 1280×1600 window at scale factor 1.6 asks for.
Which is to say: a normal laptop screen.

## What this tree does about it

**Ask whether the render happened.** `render_gpu::render_checked` calls
`render_to_texture_async`, which returns the bump allocators, and turns a set `failed` bit into
`GpuRasterError::SceneTooLarge` naming the buffer that wanted the most room. Two costs, both
deliberate:

- **The API is deprecated**, in favour of the synchronous call — which cannot answer the question.
  There is nowhere else in Vello 0.9 that the flag is exposed. `#[expect(deprecated)]` with the
  reason written above it, to be removed when Vello stabilises the statistics API its own note
  promises.
- **One synchronisation per render**, priced below.
- **`debug_layers` had to be enabled on the dependency**, because `render_to_texture_async` only
  downloads the bump buffer when it is (`let robust = cfg!(feature = "debug_layers")`). The name
  is unfortunate; what it buys here is not debugging but the difference between a blank page and
  an error.
- **And that feature panics on a scene with no lines**, which is the second thing this session
  had to pay for. With it on, vello slices its captured line buffer to `bump.lines` entries and
  gives the slice to wgpu, which rejects an empty one: *"buffer slices can not be empty"*. A
  scene with no lines is ordinary — a blank page has none — and under `panic = "abort"` that is
  not a failed render but a dead viewer. Two of this crate's own fixtures, a zero-length stroke
  and a clip that admits nothing, went from passing to aborting the moment the feature went on,
  which is the only reason it was caught before shipping.

  The answer is one rectangle in a fully transparent paint, added to every scene by
  `keep_the_line_soup_non_empty` at the same choke point. It contributes four lines to the soup
  and composites as the identity, and *that* is checked rather than argued: the fourteen
  cross-backend fixtures compare every pixel against the processor's and are unchanged. It is a
  workaround for a defect in a dependency, written down as one, to be removed when vello guards
  the slice.

And one device synchronisation per render, to map the flag. **Measured, because principle 2 asks
for a number rather than an assurance**: twenty renders of page 5 at 596×842 on the 890M, with the
feature and without, twice each — 7.24 and 7.36 ms per render without, 8.00 and 8.28 ms with, so
**about +0.8 ms, near enough 11%**. Renderer construction was 42 and 52 ms without against 48 and
57 ms with, which is inside the run-to-run spread and therefore no measurement at all; it is also
off the startup critical path by construction, since page one draws on the processor while the
device initialises.

Affordable *for this program*, and the reason is worth stating: a document viewer renders a frame
when a person turns a page, not sixty times a second. 0.8 ms against a page that is blank is not a
close call. A game would answer differently.

**Then make it the only route.** `render_checked` is `pub`, and `viewer-ui`'s surface path calls
it rather than `Renderer::render_to_texture`. This matters more than it looks: the check first
landed in `render-gpu`'s `rasterize`, which is the *tier-1* path — the one the tests use — while
the window draws to its own surface through tier 2 and would have kept the defect. The person's
black page was on the path the fix did not yet cover.

**Then draw it anyway.** `viewer-ui` already falls back to `render-cpu` on a refused page (ADR
0125), so the error turns straight into a page on the screen and a line saying which backend drew
it. The error's own sentence is what gets printed, so what reaches the person names the buffer
that ran out. What was a black page is now a page, slightly slower, with the reason named.

**Watched happen**, in a real window under `Xvfb` at 1200×1700 — the resolution matters, since the
overflow is a property of the scene rather than of the driver, and it reproduces on all three
adapters this machine can offer:

```
note: page 6: the graphics device could not draw a 1200x1700 scene: it needs more room than
Vello's tile buffer has, so the device drew nothing, so it was drawn on the processor instead
```

with the window's own pixels back at mean 0.946 and standard deviation 0.204 — text on white. A
black page is mean 0.

## The gate that was missing, and now is not

`headless_gpu.rs` compares the two backends over `test-scenes`: a gradient, a knockout group,
sixteen blend modes — a handful of commands each, at one modest size. **The corpus and the oracle
never touch the GPU backend at all**; they render with `render-cpu`, which is the correctness
oracle. So the one thing nothing checked was a *real page at a real window's resolution* through
the GPU — which is the only thing a person ever looks at.

`render-gpu/tests/real_pages.rs` is that check: the specification's own PDFs at 1.0 and at 1.9008,
GPU against CPU, asserting that a backend may *refuse* a scene but may not silently draw nothing.
It fails against the code that shipped this morning, and against this code it reports pages 6 and 7
at 1.9008 as refused — naming `tile` as the buffer that overflowed — with the other six renders
drawing within a per cent of the processor's ink.

## The lessons

**A test suite made of small scenes tests small scenes.** Fourteen cross-backend fixtures, every
one of them a fixture, and the first real page at a real size was blank. This project already
knows the shape — *a scene set is worth what its scenes can express*, ADR 0046, about blend modes
that had never been compared — and it recurred one axis over: not which *feature* a scene uses,
but how *large* it is.

**A dependency's silence is not a report.** Every layer this project owns reports what it cannot
do. Vello's answer to "I ran out of room" is a bit in a buffer nobody reads and an `Ok(())`, and
the tree took the `Ok` at face value for as long as the GPU backend has existed. **Where a
dependency returns success, ask what it does when it fails** — and if the answer is "nothing
visible", that is a report this project has to construct itself.

**And the report came from a person, again.** Four sessions in a row now: the mirrored click, the
slow page turn, the stale binary, and this. Each lived on the path from a key press to a lit
pixel, and that path had no instrument until two sessions ago. It has three now — `--trace`,
`--cpu`, and a real window under `Xvfb` — and this is the first defect the instruments caught
rather than the first that needed them.
