# 2416 — A column's stand-in carries the last frame page by page

Status: implemented. Session 1791, 2026-10-26, slot 4 of batch one hundred and nineteen, a "loop"
round on the owner's zoom-out complaint and ADR 2408's second lever. No clause: what a stand-in
shows is this program's own approximation (`doc/todo/37`), and no ledger row moves. Amends ADR 0461's
item 2 (a placement per page, priced and declined) and ADR 0442 (one texture under one placement).
Context: ADRs 0378, 0391, 0443, 0699, 0702, 1843, 2408; traps 1, 13, 276. Code:
`viewer_device::stale::placement` (`Part`, `Carry`, `carrying_on_quads`, `page_by_page`,
`page_texels`), `Stale::reproject_on_quads`, `DeviceWindow::present`, quorra's
`surface::device_frame`; `raster_gpu::Layer::within` and `Texels`, `present.wgsl`. Tests:
`stale::tests::a_zoom_of_a_real_column_stands_in_from_its_last_frame_page_by_page`, the two column
tests re-read for both surfaces, and raster-gpu's `a_part_of_a_texture_is_drawn_alone_and_filtered_inside_itself`
and `a_part_past_the_texture_is_refused`.

## 1. The premise, read in the code

The brief called the column stand-in's page offset a defect to fix first. **No page was drawn off.**
On a zoom in a column one placement puts the second page `GAP × (1 − k)` device pixels from where the
new view puts it (1.5 to 4.0 px; the window's trace said 3.20 and 3.95 at the owner's scale), past
`AGREEMENT`'s half pixel, and `one_placement` refused it (`Refusal::Rearranged`) — so the last frame
was never put up and the stand-in was the 512-pixel retained pages alone (ADR 2408 §3). The offset is
why the sharp layer was refused, not a picture on the screen; the defect is the refusal, and the test
that fails on HEAD asserts what replaces it: a column zoom on the device plans `LastFrame`, not
`Refused` or `RetainedPages`.

## 2. Decision

1. **On a surface that draws in quads, a column's zoom carries the last frame a page at a time.**
   For each page both the held picture and the new view have, the page's own `was⁻¹ ∘ now` (the
   composition `one_placement` already made, now one helper, `moved`), and the page's rectangle of
   the held picture — its box `[0, 0]`–`page_size` under the placement it was drawn at, grown outward
   to whole texels and clipped to the picture. The gap between the pages is the window's again: the
   medium and the retained layer show through it.
2. **The presenter shows a part of a texture**: `raster_gpu::Layer::within` names the texels a layer
   shows; outside them it contributes nothing, as outside the texture, and a linear tap is clamped to
   the part's outer texel centres so the next page's edge is never filtered in. `None` is the whole
   texture through the shader path it had, its bounds test and normalisation unchanged in value
   (`textureDimensions` for the extent the uniform carried). The sharp pass (ADR 0699) shows the same
   part at twice the texels.
3. **One placement stays where one is true.** A scroll, a single page and every view change
   `AGREEMENT` admits still go up as the whole texture, so `AGREEMENT`, 609's test and its
   measurement stay. The processor's window, which resamples one window of pixels, still refuses a
   column's zoom and stands in from the retained pages; so does `could_stand_in`, the superseded-frame
   policy both windows ask (ADR 0657), which this decision does not move.
4. **Rule 3**: the trace says `<n> of <m> pages carried one by one: one placement would put them
   <d> px apart … so each page goes up under its own and the gap between them is kept`; the frame
   line's word is the base's own (`approximated`, `approximated, over a retained page`).

**Why ADR 0461's price does not apply**: it priced one texture per page — N renders or an ask to
quorra, and three times the page half of a frame's memory. This is one texture drawn once a page:
no render, no texture, no memory added; the cost is a textured quad per page.

## 3. Measured

The Radeon 890M (`rendering with AMD Radeon 890M Graphics (RADV STRIX1)`), natively on Wayland in a
nested headless KWin at 2880 × 1800, scale 1.6, as ADR 2408 §2; Control+wheel through KWin's EIS
input 1.5 s apart, four in, six out, two in. No page a corpus arm digests can move: the one shader
changed is `present.wgsl`, whose pipeline exists only for `Presenter::present`, and the corpus gate
draws through `Device::render` and a readback, so HEAD's six arms stand as they are. One `--clock --nice 0` hold of 669 s, two runs per document per build,
interleaved, each arm started at a one-minute load of 1.83 to 3.95 (the 60 s cap reached on two), the
render node held by none of this account's processes, `pp_dpm_sclk`'s middle line at 825 to 1451 MHz.

| ISO 32000-2 pp. 101–102, `OneColumn` | notch to sharp | inside 8.33 ms | stood in |
|---|---|---|---|
| HEAD, 24 steps | 6–19 ms | 7 | 0 |
| this change, 24 steps | 6–20 ms | 11 | 2, both from the last frame carried one by one, up 1 ms after the notch, presents 0.25–0.43 ms |
| `issue19802.pdf` p. 1, HEAD / this change | 2–20 / 2–28 ms | 22 / 21 | 1 / 1 |

The nested output runs at 60 Hz, so rule 5 stands in only past 16.7 ms and few steps do; two
unclocked runs on the same column before the hold, under load, show the change itself: HEAD stood in
four of four steps `approximated from a retained page` (presents 0.72–9.95 ms), the change two of four
`approximated, over a retained page` with `2 of 2 pages carried one by one` (presents 0.35–1.62 ms).
No sharp frame moved: the sharp ranges are one population. Under Xvfb (llvmpipe, one thread, so a
frame is long enough to photograph) a notch out of the same column photographed mid-stand-in shows
both pages of the last frame at the new view's places, the gap clean, text minified by the one
bilinear tap a single page's stand-in already has (trap 1).

## 4. What stays

The band a zoom-out reveals has no picture sharper than the retained page (ADR 2408 §4). The toolkit
windows stand in at every notch (ADR 1843); whether they wait one refresh is Q657, the owner's.
