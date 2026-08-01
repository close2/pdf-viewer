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

## Why not simply make the buffer bigger

That is the first question to ask, and the answer took a search rather than an assumption.

**There is no way to ask for a bigger one.** The sizes are computed inside
`vello_encoding::BufferSizes::new`, called by `RenderConfig::new` for every render, from constants
in the function body. `RendererOptions` — the only configuration vello 0.9 takes — has four fields:
`use_cpu`, `antialiasing_support`, `num_init_threads`, `pipeline_cache`. Nothing reaches the sizes,
and the constants are unchanged on vello's `main` at the time of writing.

**Enlarging them would mean forking a dependency**, through `[patch.crates-io]` onto a vendored or
git copy of `vello_encoding`, and then carrying that fork across every upgrade — for a project
whose deny policy is deliberately narrow about sources. The change itself would be one line.

**And it would not fix it, it would move it.** Page 6 needs 2 183 025 tile records where the buffer
holds 2 097 152: four per cent over. Page 5 at the same size needs 2 002 237 — ninety-five per
cent of the limit, drawn today by luck. The requirement scales with area × path density, so a
window twice as tall, a 4K display, or a person pressing `+` walks straight back into it, at 16 MB
of constant device memory per doubling of the tile buffer alone. A fixed number cannot bound an
unbounded quantity; that is the whole shape of the problem, and it is why vello's constants have
the apologetic comment on them in the first place.

**Vello agrees.** Its README says the renderer "can currently be considered in an alpha state", and
lists GPU memory allocation among the work outstanding. Issue 366, *Strategy for robust dynamic
memory, readback, and async*, is open since September 2023 and sets out seven options with the
maintainer's own summary that there is "no good solution to this problem, only a set of tradeoffs";
the strategy it leans on is for a stage that runs out of memory to **subdivide the viewport and
resubmit**. Issue 40, on dynamic GPU memory management, is closed with no implementation. So the
buffers are fixed, the failure is by design detectable and by design not handled, and the handling
is left to the application.

## What this tree does about it

**Ask whether the render happened.** `render_gpu::render_checked` calls
`render_to_texture_async`, which returns the bump allocators, and turns a set `failed` bit into
`GpuRasterError::SceneTooLarge`, naming the stage from the flag's own bits — `binning`, `tile`,
`line`, `segment-count`, `per-tile command list`, as `vello_shaders`' `bump.wgsl` defines them.
Two costs, both deliberate:

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

**Then band the target, which is the actual fix.** A set flag halves the target into horizontal
bands; each band is the same scene translated to the band's top, rendered at the band's height into
a strip texture, and copied into place. Paths outside a band fall away in binning and take their
tile records with them, so halving roughly halves the requirement, and the halving repeats until
the device draws it or a band would be shorter than 32 pixels. This is issue 366's own remedy,
implemented on the caller's side of the API because vello 0.9 does not implement it and offers no
way to avoid needing it.

**Measured**, at 1132×1601 on the 890M, ten renders each through the tier-1 path:

| | bands | GPU | CPU |
|---|---|---|---|
| page 5 (fits) | 1 | 24.6 ms | 96.8 ms |
| page 6 (does not) | 2 | **38.1 ms** | 98.0 ms |

So a banded page costs about 55% more than an unbanded one of the same size and remains **two and
a half times faster than the fallback it replaces** — and it is drawn by the backend the person
chose. The band count is remembered *against the size it was learnt at*, so scrolling and page
turns pay the discovery once while a resize starts again from one pass; without that key it only
ratchets upward, and a single dense page at a large zoom would band every page after it.

**Then draw it anyway, if even that fails.** `viewer-ui` still falls back to `render-cpu` on a
refused page (ADR 0125), so a scene no band can hold is a page on the screen and a line saying
which backend drew it. What was a black page is a page, in the ordinary case on the GPU.

**Watched happen**, in a real window under `Xvfb` at 1200×1700 — the resolution matters, since the
overflow is a property of the scene rather than of the driver, and it reproduces on all three
adapters this machine can offer. Before banding, the window drew the page on the processor and
said so; after it, the device draws it in two bands and says nothing, which is what a working
renderer looks like.

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
