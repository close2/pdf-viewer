# 1607 — The turn gate holds a long-lived device's seventh frame beside the fresh device's turn

Status: accepted. Session 1385. Builds on ADRs 1513 (the turn gate), 1577 (warm cores); answers
trap 116's second half, which asks for a sequence on one device beside a fresh-device figure.
Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("Perf gates run in CI ... A regression fails the build"); ADRs
1595, 1606; trap 116.
Code: `crates/render-raster/tests/support/frame_cost.rs` (`FRAMES_BEFORE_SEVENTH`,
`long_lived_device`, `Round::seventh`), `crates/render-raster/tests/turn_path.rs` (`Figures`,
the `seventh_ms` key), `crates/render-raster/examples/frame_budget.rs` (the `seventh` row),
`doc/checks/turn-path.toml` (`seventh_ms` on every page), `doc/performance.md` section 3e,
`tools/state.sh` (`section_frame`'s pattern).
Tests: `turn_path.rs`'s `the_check_file_states_a_band_for_every_row` reads all three bands.

## 1. What is built

Every round of `frame_cost::round` draws the page's turn a second time on a device of its own that
has first drawn six frames — the warm-up document's pages 1 to 5 and its first page again, a reader
turning through it and back — so that the page is that device's seventh frame, on the page-turn
lane, against the same interpretation as the turn row. `frame_budget` prints it as `seventh`,
`turn_path`'s child as `seventh_ms`, and the gate judges it against `seventh_ms` in the check file,
a band a page and the file's rule, like `turn_ms` and `step_ms`.

## 2. Why a row, why six, why that document

**A row and not a column or a second table**: section 3e's one table is what `turn_path` bands and
what `tools/state.sh`'s provenance reader reads row by row, and a row keeps the stage columns, which
say where a difference between the two devices is. **Six** because the frames of one device that
paid for `wgpu`'s growing pool of command encoders were the first few — ADR 1595 measured the
overlap paying it back by the sixth on the zoom pair, and `zoom_frame`'s eight frames on one device
of `bug1721218_reduced.pdf` read 80.7 to 82.6 ms on the first and 69.6 to 76.4 from the third on, in
this round's quiet runs — so the seventh is the turn a window's user sees on a device that has
lived. **The warm-up document** because its outlines are in no measured page, so the seventh frame
still uploads every outline its page states and is a turn rather than a repaint.

## 3. Measured

Three runs of five rounds a page in one sitting, 2026-10-07, pinned, warm cores, load 3.8 to 6.0:
on eight pages the seventh frame's range and the fresh device's overlap; on three it is quicker by
its `transfer` and `elsewhere` (`images.pdf` 33.72–34.00 against 35.27–35.47); the text page's
quickest seventh is slower, its `encode` 5.07–5.36 against 3.83–4.51. The bands are the file's rule
over that sitting and two gate runs. The gate: three runs behind the lock, 77 to 108 s each (it was
about 82 s with two devices a round). The second and third judged 33 of 33 and passed; the first
failed on the Type 3 page alone, all three of its rows at 13.0 to 14.5 ms, the two older bands as
well as the new one, and a direct re-take of the same binary read 11.2 to 11.5 minutes later — the
busy end of trap 110, for which no band was widened.

## 4. What it is for

A change whose cost or saving depends on what the device has already done — the encoder pool, an
atlas, a staging belt — now moves a judged figure on the device a window keeps as well as on one
made for the round, so it can no longer read as a loss on one and a win on the other with only one
of the two gated (trap 116, ADR 1595).
