# 1791 — A column's stand-in carries the last frame page by page

Session 1791, slot 4 of batch 119, a "loop" round; ADR 2416 and Q657; ADR 2417 unused.
No ledger row moved; no pixel a corpus gate holds can move (only `present.wgsl` changed, which the
corpus gate's `Device::render` and readback never reach), so HEAD's six arms stand as they are.

**Premise.** "The second page lands 1.5 to 4.0 px off" did not hold as a picture: no page was drawn
off. One placement would have put it 3.20 to 3.95 px off at the owner's scale, so `one_placement`
refused the last frame (`Refusal::Rearranged`) and the column stood in from the 512-pixel retained
pages alone. The refusal was the defect. The test that fails on HEAD is
`a_zoom_of_a_real_column_stands_in_from_its_last_frame_page_by_page`, which plans `LastFrame` where HEAD
plans `Refused`/`RetainedPages`; planting HEAD's plan back fails it.

**Built** (ADR 2416). `raster_gpu::Layer::within` shows only part of a texture, with a linear tap held
inside that part. On a surface that draws in quads, `Stale::reproject_on_quads` carries a column's
zoom one page at a time: each page goes up under its own `was⁻¹ ∘ now`, showing its own texels, with
the gap kept. One placement still serves every view change `AGREEMENT` admits. The processor's window
and `could_stand_in` are unchanged. ADR 0461's price (one texture per page) does not apply: there is
one texture and a quad per page.

**Measured** (Radeon 890M, nested KWin at the owner's mode, as ADR 2408 §2). One `--clock --nice 0`
hold (669 s), loads 1.83–3.95, render node held by none of this account's processes, sclk 825–1451 MHz. On the ISO 32000-2 pp. 101–102 column,
notch to sharp was 6–19 ms on HEAD and 6–20 ms with the change, over 24 steps each. The nested 60 Hz
output stood in 0 and 2 times; both of the change's stand-ins came from the last frame carried one by
one, 1 ms after the notch, and presented in 0.25–0.43 ms. `issue19802.pdf` read 2–20 and 2–28 ms. Two
unclocked runs before the hold showed the change: HEAD `approximated from a retained page` on 4 of 4
steps (presents 0.72–9.95 ms), the change `2 of 2 pages carried one by one` (presents 0.35–1.62 ms). A
photograph under Xvfb mid-stand-in showed both pages at their new places, the gap clean (trap 1). The
hypothesis held: one part per page, presents inside 4.5 ms, no sharp frame moved.

**Q657** asks the owner about lever 3 (the toolkit windows wait one refresh), with 1787's figures.
**Instruments.** The session worked in `scratchpad/r1791/`; its Xvfb `:91` was stopped with TERM, lock and socket removed.

**Gates.** `cargo fmt --check` exit 0 and `RUSTFLAGS="-D warnings" cargo clippy --all-targets` exit 0,
on each of raster-gpu, viewer-device and viewer-ui. `cargo nextest run`: viewer-device 95 passed,
viewer-ui 137, raster-gpu 784, each exit 0. `RUSTDOCFLAGS="-D warnings" cargo doc --document-private-items`
exit 0 on viewer-device, raster-gpu and viewer-ui. Hold exit 0, every arm's measure exit 0.
`turn_path` A/B exit 101 in both arms (loads 5.76 and 2.90 at start, bands missed on different pages
in each, so the machine and not the change). Drive under Xvfb on the export, `quorra` and
`quorra-confined`: 43 works, 0 wrong, 6 not offered. `cargo test -p conformance --no-fail-fast` exit 0.
