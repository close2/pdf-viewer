# ADR 0156 — The quorra backend, over the corpus

Status: accepted, 2026-08-03. Session 189. Asked for by the project owner: *a complete corpus
check using the new renderer, and its performance measured.*

## Why a fifth gate

`render-quorra` arrived with eleven cross-backend scenes and four real pages of the
specification's own PDFs. That is a better suite than the Vello backend had, and trap 12b is
the standing warning about what it can still miss: fourteen scenes passed and the first real
page at a real window's size came back **blank**, because a suite of small scenes tests small
scenes. Four real pages is four.

`crates/render-quorra/tests/corpus.rs` is the same comparison over **974 documents' first
pages**. Both backends are handed the *same display list*, so nothing here is about PDF
semantics: a difference is a difference between two rasterisers and a refusal is a command the
new backend cannot draw. `CLAUDE.md` keeps the CPU backend as the correctness oracle; this is
what holds a second backend to it at the corpus's scale.

Three ratchets, all held by name: what quorra **refuses**, what it draws **differently at the
edges** (structural similarity above 0.99 — the same shapes in the same places), and what it
draws **differently in shape** (at or below it). The differing pages write both renders to
`target/tmp/quorra/<stem>/`, because a list of names and three numbers cannot tell a missing
mark from a soft edge.

## What it found, at the page's own scale

957 pages compared: **900 agree, 50 differ, 7 refused, 17 not comparable.**

- **`issue4260_reduced.pdf`, mean 14.19 and similarity 0.49, is trap 2's shape.** It is a page
  of ruling lines written as zero-height rectangles; §10.7.4 says no shape may disappear;
  `pdf_render::collapsed` is where that rule lives *so that both backends inherit it* (ADR
  0154, three sessions old); and this backend does not ask for it. `stroke.rs` asks
  `pdf-render` for §8.5.3.2's degenerate strokes and nothing asks for §10.7.4's degenerate
  fills. The artefacts say it in one look: the CPU backend draws the grid, quorra draws the
  surrounding box and nothing inside it. **A device decision added to `pdf-render` does not
  announce itself to a backend**, which is the whole reason those decisions live there — and
  the whole reason a corpus-scale comparison is worth having.
- **Twenty-eight of the fifty differ only at the edges**, twenty of them one document family at
  mean 1.52 and worst tile 5.09. That is a page of dense text measured against a different
  glyph rasteriser, and `real_pages.rs` measures the specification's own pages at 1.18: the
  same floor on a heavier page, not a defect list.
- **Six of the seven refusals give a reason whose own arithmetic does not support it**: "frame
  needs N bytes of instance data, over the stated budget of 33554432", with N equal to 21 093,
  114 140, 1 170 768 — every one of them *under* the budget it is compared against. Either the
  message names the wrong budget or the comparison is against the wrong quantity. It is the
  backend's to answer; the list is where it will show when it does.

## What only a corpus-scale run could find

At four times the page's own scale, **533 of 952 pages are refused**, and the reason is not the
pages:

```text
resource upload refused: uploading would hold 536871036 resource bytes
(536870896 already resident), over the stated budget of 536870912
```

536 870 896 bytes already resident is the 512 MB budget, full. `tiling-pattern-box.pdf` is
refused in the full run and **passes on its own**, which is the whole proof: the rasteriser's
outline, image and ramp caches hold an `Arc` clone of everything they have ever been shown and
never evict, so a long-lived instance fills its budget and then refuses every frame after it.
Per-document runs cannot see it — each starts empty — and a viewer left open across a
document is exactly the long-lived instance this describes.

**And the instrument had the same shape of defect first.** A refused frame is a fast frame, so
timing one reported the backend that drew nothing as the quickest there is: the median ratio at
4× came back as `0.00×` before the timing was restricted to frames that were produced. A
suspiciously clean measurement is a reason to check the instrument.

## Performance, measured

Offscreen, page one of every document, on an AMD Radeon 890M (RADV, integrated), release build.
The GPU figures include the readback to system memory that a windowed host does not pay —
`doc/RENDER_LIBRARY.md` §6.1 measures that at 55% to 92% of a frame — so these are the
*offscreen* numbers and say nothing directly about the window.

| scale | pages drawn | CPU backend | quorra | median page |
|---|---|---|---|---|
| 1.0 (the page's own) | 950 | **2.63 s** | 6.21 s | quorra **2.64×** |
| 2.0 (a window's) | 940 | **6.72 s** | 12.64 s | quorra **3.18×** |
| 4.0 | 419 of 952, the rest refused | 4.12 s | 10.30 s | quorra **3.77×** |

**Quote the total against the median and say which.** The totals ratio *improves* with scale
(2.36× → 1.88×) while the median ratio *worsens* (2.64× → 3.18×), and both are true: the CPU
backend's cost grows with the pixels, so the heavy pages close the gap in the total, while the
median page is small enough to be dominated by a per-frame floor that does not shrink. That is
the same axis ADR 0136 measured `rasterrocket` on, from the other side.

The scale-4 row is not comparable with the other two — half the corpus is refused and the
pages that survive are the ones whose resources fit — and it is in the table because leaving it
out would hide the refusals rather than the number.

**What this does not measure**: the window. A presented frame skips the readback, keeps the
resources resident between frames and is what the swap was made for. `frame_race.rs` is the
example for that and this gate is deliberately not it.
