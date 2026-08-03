# quorra, measured against the corpus — findings for the library's team

Written 2026-08-03, at the end of this viewer's hundred-and-ninety-fifth session, from the run
described in ADR 0156. It is the counterpart to `doc/RENDER_LIBRARY.md`: that document is the
brief this project wrote for a team building a renderer, and this one is what came back when the
renderer met 974 real documents.

**Everything here is a measurement with a command beside it.** Where something is a matter of
taste it says so, and where the difference is the two rasterisers' antialiasing rather than
anybody's defect it says that too — §4 exists so that the rest can be trusted.

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

Today's result, on an AMD Radeon 890M (RADV, Vulkan), release build:

```text
957 pages compared: 900 agree, 50 differ, 7 refused, 17 not comparable
```

The 50 and the 7 are held by name in that file. A page arriving in either list fails the build;
so does a page leaving it, because a hole that closes should be noticed.

---

## 1. §10.7.4's degenerate fill is not asked for

**Severity: a page of ruling lines comes out blank.**

`issue4260_reduced.pdf` rules its grid with zero-height rectangles — `848 1085 10159 0 re f` —
and the CPU backend draws the grid while quorra draws the surrounding box and nothing inside it.
Mean 14.19, structural similarity **0.49**, the worst page in the run. The two renders are in
`target/tmp/quorra/issue4260_reduced/`.

This is not a rasterisation difference. ISO 32000-2 §10.7.4:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears

A subpath with no extent along one axis has zero area, so *any* coverage-based rasteriser
computes nothing for it. This viewer therefore states the answer once, in the crate both
backends consume, so that neither decides it alone:

- `pdf_render::thinnest_line(to_device) -> Option<f32>` — one device pixel in the path's own
  space, which is also §8.4.3.2's zero-width line and §10.7.5's sub-half-pixel one.
- `pdf_render::split_collapsed_fill(path, thinnest) -> Option<CollapsedFill>` — the subpaths
  that enclose an area, and the rectangles the rest should mark. `None` for an ordinary path,
  and it allocates nothing then; the predicate behind it is memoised on the `Path`.

`render-cpu`'s `draw_fill` and `render-gpu`'s `encode_fill_command` are the two call sites to
copy. The marks are filled under the **non-zero** rule whatever the command's own rule is — a
mark is a shape in its own right, and adding it to an even-odd path's winding would punch a hole
in what it was meant to draw.

`crates/render-quorra/src/stroke.rs` already asks `pdf-render` for §8.5.3.2's degenerate
*strokes*; what is missing is the same question for fills, in `scene.rs`'s `fill`.

**This is not a criticism of the library, and the timing says why**: the rule landed in this
viewer in ADR 0154, three sessions before the backend was measured, and nothing announces a new
device decision to a backend. It is the standing argument for keeping such decisions in
`pdf-render`, and the reason this gate exists.

---

## 2. The resource caches never evict

**Severity: a long-lived rasteriser stops drawing. Only a corpus-scale run can see it.**

At four times the page's own scale, **533 of 952 pages are refused**:

```text
resource upload refused: uploading would hold 536871036 resource bytes
(536870896 already resident), over the stated budget of 536870912
```

536 870 896 bytes resident is the 512 MB budget, full. The proof that it is not the pages:

```sh
PDFVIEWER_QUORRA_SCALE=4 PDFVIEWER_QUORRA_ONLY=tiling-pattern-box \
  cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
```

`tiling-pattern-box.pdf` is refused in the full run and **passes on its own**. The refusals begin
partway through the alphabet and never stop.

`QuorraRasterizer` holds one `Device` and three maps — outlines, images and ramps — keyed by
pinned `Arc` identity, and the design note beside them is right that this is what lets the cache
span `rasterize` calls (a zoom re-uploads nothing). What it does not have is a way *out*: the
entry pins the allocation for as long as it lives, and nothing decides that it should stop
living. A per-document test suite starts with an empty device every time and cannot see this; a
viewer with a document open all afternoon is exactly the long-lived instance it describes.

What this side would find easy to work with, in rough order of preference:

1. An eviction policy inside the device with a stated budget — least-recently-used is the
   obvious one, and the caller does not have to know about it.
2. Failing that, a `Device::release_unused()` or a generation the caller can retire, so the
   host can drop everything a closed document uploaded.
3. Failing both, a documented "resources are resident until the device is dropped", so a host
   can build a device per document and pay the pipeline warm-up.

The refusal itself is *correct behaviour* — it refuses rather than drawing something plausible,
which is what `doc/RENDER_LIBRARY.md`'s failure contract asked for and is more than Vello does.
The gap is that nothing can make room.

---

## 3. A refusal message whose arithmetic contradicts it

**Severity: cosmetic, but it costs the reader the diagnosis.**

Six of the seven refusals at the page's own scale say:

```text
frame refused: frame needs N bytes of instance data, over the stated budget of 33554432
```

with `N` equal to **21 093** (`issue14497.pdf`), **114 140** (`zerowidthline.pdf` at 4×),
**1 170 768** (`tiling-pattern-large-steps.pdf`), **3 763 825** (`issue9418.pdf`), **20 263 595**
(`issue1905.pdf`), **29 621 489** and **29 666 103** (`bug1703683_page2_reduced.pdf`,
`bug1721218_reduced.pdf`). Every one of them is *under* the 33 554 432 it is said to exceed.

Either the message names the wrong budget or the comparison is against a different quantity. The
number that is actually being exceeded is the one worth printing, and the largest of these is a
page this viewer already knows is pathological (`bug1721218_reduced.pdf` is its worst page by
some margin), so the limit may well be the right one — it is only the sentence that does not add
up.

The seventh refusal is `issue17848.pdf`: "this backend cannot draw a mesh shading with no
visible raster", which is a stated limit rather than a defect.

---

## 4. What is **not** a finding

Twenty-eight of the fifty differing pages have structural similarity **above 0.99** — the same
shapes in the same places — and twenty of those are one document family (`tracemonkey.pdf` and
its relatives) sitting at mean 1.52 with a worst tile of 5.09. That is a page of dense text
measured against a different glyph rasteriser. `real_pages.rs` measures the specification's own
pages at 1.18 and reports that the Vello backend measures 1.16 on the same cases: **the two
rasterisers' antialiasing is essentially the whole floor**, and quorra is not adding to it.

The remaining twenty-two differ structurally. One is §1 above; the rest are unexamined and are
listed by name in `DIFFERS_IN_SHAPE` with their numbers, largest first: `issue2177.pdf` (8.16),
`issue16038.pdf` (6.16), `issue16316.pdf` (5.06), `issue6769.pdf` and `issue6769_no_matrix.pdf`
(4.81, similarity 0.84–0.86). Their artefact pairs are on disk after a run. This side has not
opened them; the list is the offer.

---

## 5. Speed, offscreen

Page one of every document, both backends, same display list. **The GPU figures include the
readback to system memory**, which a windowed host does not pay — `doc/RENDER_LIBRARY.md` §6.1
measures that at 55% to 92% of a frame — so these are offscreen numbers and say nothing directly
about presenting.

| scale | pages drawn | `render-cpu` | quorra | median page |
|---|---|---|---|---|
| 1.0 — the page's own | 950 | **2.63 s** | 6.21 s | quorra **2.64×** |
| 2.0 — a window's | 940 | **6.72 s** | 12.64 s | quorra **3.18×** |
| 4.0 | 419 of 952; the rest refused, see §2 | 4.12 s | 10.30 s | quorra **3.77×** |

**Quote the total against the median and say which.** The totals ratio *improves* with scale
(2.36× → 1.88×) while the median ratio *worsens* (2.64× → 3.18×), and both are true: this
viewer's CPU rasterisation grows with the pixels, so the heavy pages close the gap in the total,
while the median page is small enough to be dominated by a per-frame floor that does not shrink.
The scale-4 row is not comparable with the other two, because half the corpus is refused and the
survivors are the pages whose resources fit; it is here because leaving it out would hide the
refusals rather than the number.

For contrast, the *presented* path — `pdf-viewer` on `Xvfb`, ISO 32000-2 through quorra — puts
page one on screen in **44.6 ms** and turns to page 6 in **9.3 to 17.4 ms** a page. That is the
number the swap was made for, and this gate is deliberately not it.

**The instrument had the same shape of defect the library does, and it is recorded here so that
nobody trusts an earlier draft**: a refused frame is a fast frame, so the first scale-4 run
reported a median of `0.00×`. Only frames that were produced are timed now.

---

## 6. One thing this side owes back

`cargo test -p conformance` fails on ten citations in `crates/render-quorra` — `§4.5`, `§2.2`,
`§4.6` — which are sections of `doc/RENDER_LIBRARY.md` rather than of ISO 32000-2. This tree's
rule is that a bare `§` means one document, and the checker already accepts another document's
sections when the document is named first (`RFC 3986 §5.2` is the standing example). Writing
`RENDER_LIBRARY.md §4.5` clears it. Nothing about the library is wrong here; it is a convention
of the caller's repository that the brief did not state, and it is stated now.
