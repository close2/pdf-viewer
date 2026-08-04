# quorra, measured against the corpus — findings, and what came back

Written 2026-08-03 at the end of this viewer's hundred-and-ninety-fifth session, from the run
described in ADR 0156; **rewritten the same day, after every finding in it was answered.** It is
the counterpart to `RENDER_LIBRARY.md` — that document is the brief this project wrote for a team
building a renderer, and this one is what came back when the renderer met 974 real documents,
and then what the team did about it.

Each finding below keeps its evidence and carries what closed it, because a feedback document
that still reads as a complaint after the complaint was answered is worse than no document.

**Where it stands, at the page's own scale:**

| | first run | now |
|---|---|---|
| agree | 900 | **912** |
| differ | 50 | **44** — 29 of them the antialiasing floor (§4) |
| refused | 7 | **1** |
| median page | 2.64× the CPU backend | **2.05×** |

---

## 0. The instrument, and how to run it

```sh
cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
```

`crates/render-quorra/tests/corpus.rs`. Every one of the 974 pdf.js corpus documents' first
pages, interpreted **once** and handed to `render-cpu` and to quorra as the *same display list*,
compared with `raster-compare`. Nothing in it is about PDF semantics: a difference is two
rasterisers disagreeing and a refusal is a command quorra cannot draw.

- `PDFVIEWER_QUORRA_ONLY=a,b` restricts it to matching file names.
- `PDFVIEWER_QUORRA_SCALE=2` renders at another scale. Both skip the ratchets and say so.
- Every page that differs writes both renders to `target/tmp/quorra/<stem>/{cpu,quorra}.png`.

The glyph-phase quantum is **off** in this gate, so what it measures is the adapter and the
translation rather than a trade `real_pages.rs` gates separately.

The differing and refused pages are held by name in that file. A page arriving in either list
fails the build; so does a page leaving it, because a hole that closes should be noticed — which
is how the numbers in this document were kept honest between the two runs.

---

## 1. §10.7.4's degenerate fill was not asked for — **answered**

**Was: a page of ruling lines came out blank.**

`issue4260_reduced.pdf` rules its grid with zero-height rectangles — `848 1085 10159 0 re f` —
and the CPU backend drew the grid while quorra drew the surrounding box and nothing inside it.
Mean 14.19, structural similarity **0.49**, the worst page in the run.

Not a rasterisation difference. ISO 32000-2 §10.7.4:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears

A subpath with no extent along one axis has zero area, so *any* coverage-based rasteriser
computes nothing for it. This viewer therefore states the answer once, in the crate both backends
consume, so that neither decides it alone: `pdf_render::thinnest_line` for the width and
`pdf_render::split_collapsed_fill` for the split, with the marks filled under the **non-zero**
rule whatever the command's own rule is — a mark added to an even-odd path's winding would punch
a hole in what it was meant to draw.

**Answered**: fills run through `split_collapsed_fill`, as the two sibling backends do.
`issue4260_reduced.pdf` goes from similarity **0.49 to 0.9938** (mean 14.19 → 1.73) and leaves
the shape list for the antialiasing floor.

**It was never a criticism of the library, and the timing is why**: the rule landed in this
viewer in ADR 0154, three sessions before the backend was measured, and nothing announces a new
device decision to a backend. That is the standing argument for keeping such decisions in
`pdf-render`, and the reason this gate exists.

---

## 2. The resource caches never evicted — **answered**

**Was: a long-lived rasteriser stopped drawing, and only a corpus-scale run could see it.**

At four times the page's own scale, **533 of 952 pages were refused**:

```text
resource upload refused: uploading would hold 536871036 resource bytes
(536870896 already resident), over the stated budget of 536870912
```

536 870 896 bytes resident is the 512 MB budget, full. The proof that it was not the pages:
`tiling-pattern-box.pdf` was refused in the full run and **passed on its own**.

`QuorraRasterizer` holds one `Device` and three maps keyed by pinned `Arc` identity, and the
design note beside them is right that this is what lets the cache span `rasterize` calls. What it
had no way to do was stop: the entry pinned the allocation for as long as it lived, and nothing
decided that it should stop living. A per-document suite starts with an empty device every time
and cannot see this; a viewer with a document open all afternoon is exactly the long-lived
instance it describes.

**Answered**, and by the first of the three shapes this document asked for — a policy inside the
device, so the caller need not know: entries carry recency, and after every frame (refused ones
included) the least-recently-used entries the frame did not touch are released until the device
holds no more than **half** its budget. Half rather than all, so eviction is not a cliff and a
hot entry is never evicted.

**The 4× run's 533 resource refusals are zero**, and that scale went from 413 pages agreeing to
**918**.

---

## 3. A refusal message whose arithmetic contradicted it — **answered**

**Was:** six refusals said "frame needs N bytes of instance data, over the stated budget of
33554432" with `N` equal to 21 093, 114 140, 1 170 768, 3 763 825, 20 263 595, 29 621 489 and
29 666 103 — every one of them *under* the budget it was said to exceed.

**Answered on quorra's side.** The refusals that remain add up, and the one that replaced the
mis-stated limit says what it is:

```text
frame needs 1411676992 bytes of instance data, over the stated budget of 268435456
the frame's rasterised coverage outgrew the 16384x16384 scratch image this adapter allows
```

Six of the seven pages the old message refused now draw. **One refusal is left at the page's own
scale** — `bug1721218_reduced.pdf`, this viewer's own worst page by some margin, on the scratch
image's limit — and eighteen at 4×, all with coherent arithmetic.

---

## 4. What is **not** a finding

Twenty-nine of the forty-four differing pages have structural similarity **above 0.99** — the
same shapes in the same places — and twenty of those are one document family (`tracemonkey.pdf`
and its relatives) sitting at mean 1.52 with a worst tile of 5.09. That is a page of dense text
measured against a different glyph rasteriser. `real_pages.rs` measures the specification's own
pages at 1.18 and reports the Vello backend at 1.16 on the same cases: **the two rasterisers'
antialiasing is essentially the whole floor**, and quorra is not adding to it.

The floor also shrinks as the page grows, which is what a floor should do: at 2× only 17 pages
differ at all and at 4× only 16, against 44 here.

Fifteen pages still differ structurally, largest first: `issue16038.pdf` (6.16),
`issue16316.pdf` (5.06), `copy_paste_ligatures.pdf` (3.86), `issue4402_reduced.pdf` (3.86),
`issue12295.pdf` (2.16, similarity 0.879). `issue2177.pdf`, `issue6769.pdf`,
`issue6769_no_matrix.pdf` and `bug946506.pdf` left this list when strokes under an
anisotropic transform started being outlined in path space — a fix nobody asked for, found by
reading the shape list rather than by being told. Their artefact pairs are on disk after a run;
this side has not opened the fifteen, and the list is the offer.

---

## 5. Speed, offscreen

Page one of every document, both backends, same display list, AMD Radeon 890M (RADV), release.
**The GPU figures include the readback to system memory**, which a windowed host does not pay —
`RENDER_LIBRARY.md` section 6.1 measures that at 55% to 92% of a frame — so these are offscreen
numbers and say nothing directly about presenting.

| scale | pages drawn | `render-cpu` | quorra | median page | was |
|---|---|---|---|---|---|
| 1.0 — the page's own | 956 | **2.55 s** | 6.26 s | quorra **2.05×** | 2.64× |
| 2.0 — a window's | 948 | **5.21 s** | 10.16 s | quorra **2.87×** | 3.18× |
| 4.0 | 934 of 952 | **11.34 s** | 24.13 s | quorra **3.24×** | 3.77×, over 419 pages |

**Quote the total against the median and say which.** The totals ratio *improves* with scale
(2.45× → 1.95×) while the median ratio *worsens* (2.05× → 3.24×), and both are true: this
viewer's CPU rasterisation grows with the pixels, so the heavy pages close the gap in the total,
while the median page is small enough to be dominated by a per-frame floor that does not shrink.
The 4× row is comparable with the others for the first time — before the eviction fix it covered
419 pages and the survivors were the ones whose resources fit.

For contrast, the *presented* path — `pdf-viewer` on `Xvfb`, ISO 32000-2 through quorra — puts
page one on screen in **44.6 ms** and turns to page 6 in **9.3 to 17.4 ms** a page. That is the
number the swap was made for, and this gate is deliberately not it.

**The instrument had the same shape of defect the library did, and it is recorded here so that
nobody trusts an earlier draft**: a refused frame is a fast frame, so the first 4× run reported a
median of `0.00×`. Only frames that were produced are timed now.

---

## 6. One thing this side owed back — **settled both ways**

`cargo test -p conformance` used to fail on ten citations in `crates/render-quorra` — `§4.5`,
`§2.2`, `§4.6` — which are sections of `RENDER_LIBRARY.md` rather than of ISO 32000-2. This
tree's rule is that a bare `§` means one document, and the brief never said so.

Settled from both ends: the citations now read `RENDER_LIBRARY.md section 4.5`, and the
conformance checker was taught to say what is wrong when a project document's name precedes a
`§` — so the next person to write one is told rather than left to guess.

---

## 7. A refused frame wedged the surface — **answered**

**The report, from a person using the viewer:** dragging a selection across a page made the window
stop answering, and a resize sometimes recovered it.

**Half of it was ours** and is fixed (ADR 0176): the host drew one `Multiply` fill per selection
quad, quorra gives every non-`Over` blend its own compositor layer, and
`compose::internal_texture_bytes` prices them all before allocating any. The refusal's arithmetic
is exact —

```text
(63 + 1) × 2 × 800 × 1000 × 4 = 409 600 000     the number the refusal printed
```

— so a selection quad cost 6.4 MB of frame budget and 63 of them, one short paragraph of text,
spent the 256 MiB. One fill of one path with one subpath per quad draws the same pixels for two
layers instead of sixty-four. **No complaint about the pricing**: counting before allocating is
§5, the message named both numbers, and this side was asking for something absurd.

**The other half is not ours, and it is the one the report was about.** After the refusal, every
subsequent present blocked for **exactly one second** and returned `SurfaceProblem::Timeout`, for
ever. The process sits at 4% CPU, so it is blocked rather than spinning.

Reading `quorra-gpu` at `3f45555` names a mechanism, and this side offers it as a reading rather
than as a diagnosis it has instrumented:

- `Device::render` calls `bind_target` — which for `Target::Surface` **acquires the swapchain
  texture** — and only then prices the compositor's internal textures and returns
  `RenderError::FrameBudgetExceeded`. So a refused frame drops a `wgpu::SurfaceTexture` that was
  never presented. `wgpu` discards it on drop, but the acquire semaphore was never waited on by
  any submission, which is the shape that exhausts a Vulkan swapchain.
- `Surface::acquire` sets `needs_reconfigure` for `Suboptimal` and `Outdated` and **not for
  `Timeout`**, so nothing ever reconfigures the surface again. A host resize changes the
  configured size and reconfigures, which is exactly why a resize recovers the window.

**Answered on quorra's side at `4aab7e2`, in all three of the shapes this section asked for.**
The budget is priced *before* `bind_target`, with the reason written into the comment there, so a
refused surface frame costs no acquire at all; `Timeout` now sets `needs_reconfigure`, as `Lost`
does; and `Device::invalidate_surface` gives a host somewhere to go when the device says no.

**Measured against the original report, on the same recipe.** The per-quad selection was restored
locally to reproduce it exactly, and the same drag over `issue14821.pdf` now gives:

```text
SELECTION quads 70   present -> failed: frame needs 454400000 scene-derived bytes,
                               over the stated budget of 268435456 … in 6.4 ms
SELECTION quads 56   present -> failed: … in 6.6 ms
SELECTION quads 42   present -> failed: … in 6.3 ms
```

**Every refused present costs 6 ms instead of blocking for 1.008 s, no `Timeout` is reported at
all, and the drag keeps updating throughout** — the quad count falls as the pointer moves, which
is the window answering. A page the device refuses for the *other* reason draws through the CPU
fallback and presents: `bug1721218_reduced.pdf` outgrows the 16384×16384 scratch image, comes back
on the processor in 1.68 s, and zoom, scroll and the sidebar all work afterwards.

**So `doc/todo/13` is closed and deleted**, both halves: the selection's cost by ADR 0176 and the
refused frame's recovery by this. Nothing is owed on this side; the CPU fallback still re-presents
the same overlay lists, so an over-budget *overlay* leaves the page not updating rather than the
window dying — which is a visible consequence with a live window behind it, and no longer a defect
worth a file.
