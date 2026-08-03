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
