# 1510 — A zero-length round stroke is a disc both backends paint from its area

Slot 2, batch eighty-one, 2026-10-12, pixels; the owner's "improve performance" (brief §3). ADRs
1857, 1858; no question. No ledger row moves; §8.5.3.2's row gains its code, test and a sentence.

**Premise** held: `thin_segment` refused `from == to`, so the witness's 28 684 round dots took
`split_degenerate`, one outline each on the device. **Hypothesis 1** held: the disc is the capsule
at length zero; only the shader's `normalize(0)` lacked a direction. **Hypothesis 2 did not**: the
gpu lane's move on `bug946506.pdf` is not conflation. Raster culls those 254 commands already; its
census counted a glyph beyond the page as a second placement and sent it to the atlas. Planted
`placed_once = false`: whole and culled frames the same bytes at 1× and 4×.

**Built.** `thin_segment` admits a round dot unless no pixel of it holds a level (that stays
`point_mark`'s: a corner dot would be four levels for two, `sub_pixel_coverage.rs` named it). The
scene admits equal points under a round cap; the shader takes (1, 0) at length zero; the stroke road
draws two round caps (`dot_pieces`). `render_whole` draws from `reaching`; `offscreen_cull`; rungs.

**The figure** (witness, fit, HEAD's export → this): device resources **32 791 → 4 107**; device
against oracle, bytes differing 649 717 → 617 668 (`Cpu`), 558 710 → 553 396 (`Compute`); shipped
window against one strip 142 → 63 pixels. Clocks "not judged" (trap 149; load 10–50, then 5–28):
quickest fit one thread 539.3 → 589.8 ms, shipped 579.8 → 597.6 ms, device `Cpu` 907 → 931 ms,
`Compute` 923 → 1 107 ms, inside each arm's own spread (539–831 ms); no gain is claimed.

**Pixels** (looked at, trap 1): the witness, 37 427 pixels by ≤ 13 levels, ink +0.044 %, each dot
its disc, not a one-pixel circle at a twentieth of the alpha; `issue14953.pdf`, 6 pixels by one
level at a stroked font's joints, golden line regenerated. **Unfinished**: `m h` dots, a quiet clock.

**Files.** `pdf-render/src/thin_segment.rs`; `render-cpu/src/lib.rs`, `tests/thin_segment_ladder.rs`;
`render-raster/src/{lib,stroke}.rs`, `examples/offscreen_cull.rs`; raster-gpu `src/{raster.rs,
raster/stroke.rs, encode/line.rs, shaders/line.wgsl}`, `tests/line_lane.rs`; raster-scene `src/{error,
scene/{builder,command,validate}}.rs`; the golden; the ledger; ADRs 1857, 1858; todo 69; state-of-play.

**Gates.** HEAD + this patch exported, own target. `rustfmt --check`, `clippy -D warnings
--all-targets` on the five crates: exit 0. `nextest`: 1 347 passed, 5 skipped; three plants failed.
`cargo test -p conformance`: exit 101 only on `records`, naming 1512's and 1514's. `raster_golden`
(`--tree 12`): exit 101, held 973, moved 1; division 176, ≤ 1 308 pixels, ≤ 3 levels, 12 named;
regenerated, exit 0. Six arms against `arms-1509`: each exit 0; moved `issue14953.pdf` 1× on every
lane, `bug946506.pdf` gpu 0.1101 → 0.1141 (1×), 0.0324 → 0.0351 (4×). `turn_path` (`--clock`):
exit 101 twice, 2 then 6 outside; a third hold cancelled at the budget. Queued: walks 553 s, clocks
6 087 s, golden 1 272 s. Duration 14 000 s.
