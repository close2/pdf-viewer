# 1740 — A toolkit notch is past a frame without its encode

Session 1740, 2026-10-24. Batch 113, slot 5 (measure). ADRs 2314, 2315; no Q filed.

**The split** (ADR 2314). ISO 32000-2 page 20, 2 659 commands (2 580 glyphs), from 100 % at 1.6.
The toolkit windows' read-back lane draws a notch on `Coverage::Cpu` (`lane::coverage_for`), where
`quorra` takes `Compute` on a moved view; three notches of eight reach the device before the page
passes `oversize`'s bound, and the first draws pages 20 and 21 (OneColumn). Headless, `zoom_frame`
min of five, load 9.7: device 9.0 to 15.4 ms a page, encode 62 to 72 % of it, the glyph atlas's
rasterisation 68 to 72 % of the encode. On the owner's display, one loop run, three drives at loads
6.7 to 5.0: settled 17.7 to 26.4 ms, encode 9.2 to 15.2 (49 to 58 %), read back 2.5 to 4.7, copy into
a `QImage` 1.3 to 3.9, paints 2.5 to 4.7; the stand-in shows 3.1 to 6.9 ms after the wheel.

**The lever** (ADR 2315, rejected). Under `Compute` the encode is zoom-invariant already (raster's
ADR 0087 replay): 0.3 to 0.5 ms, its replayed digest equal to a fresh walk's. Under `Cpu` it is not:
the glyph key carries the linear part and the device-grid phase, the lane is chosen by device size,
the thin-mark width and the rectangle lane are device-space. At its ceiling, encode zero, the
owner's notches leave 8.0 to 13.0 ms: one of nine under 8.33, drawing the stand-in's picture. The
read-back lane on `Compute` instead: 10.0 to 14.9 ms, and 7.30 % of a frame's pixels change (worst
14 of 255). Neither is built; `doc/todo/75` section 7 says so.

**Hypotheses.** "The encode is 60–80 % of a notch": not held on the owner's display (49 to 58 % of
the settled notch; 58 to 70 % of its device frame). "Not zoom-invariant for glyphs and thin marks":
held for `Cpu`, not for `Compute`. "A re-used encode with per-view glyph re-quantisation gets a notch
under 8 ms": not held — the floor outside the encode is past the frame. Premise corrected: ADR 0070
is raster's (thin marks), not this tree's; ADR 2132 is `render-cpu`'s window grid, which the device
lane's whole-page notch does not reach.

**Where.** I worked in scratchpad/r1740/ and in the owner's tmp/r1740/. One `--clock` hold of 65 s
held the headless arms, an Xvfb drive of the round's own and the loop hand-over (36 s, exit 0, the
window activated by its pid and confirmed active, no capture). HEAD's tree only: no source changed,
so no A/B and no arms are owed.

**Gates.** No crate touched, so no tier 1 crate check and no `tools/state.sh docs`; the
`t2-turn_path` and `t2-toolkit_launch` A/B and the six arms are not owed (no encode changed). Every
`zoom_frame` arm, the Xvfb drive and the loop exit 0. `cargo test -p conformance --no-fail-fast`:
exit 101 on `conformance` alone, two unattributed quotations in siblings' `pdf-archive` and
`pdf-transform` edits; `--test evidence --test adr_status --test records` exit 0 (7 passed).
