# 1496 — A straight stroke inside two by two pixels is painted from its closed-form area

Slot 1 of batch seventy-nine, 2026-10-10, a pixels round. ADR 1829. No ledger row: the owner's target.

**Premise, held**: ADR 1817's census at the fit, which `tile_price` re-reads: 4 365 065 marks under
¼ px. HEAD's one-thread fit raster is 2.9 s now, not the 7.3 s quoted. **Hypothesis, held in kind
and not in size**: per mark, the area differs from HEAD's by under a level — the stroker's flattened
outline, and rounding. Overlapping dots composited one by one sum to 12 levels at the witness's
worst pixel.

**Built.** `pdf_render::thin_segment` admits one `m l` of non-zero length in **any direction**: no
dash, a solid `Normal` paint, axes kept, the ink within two columns and two rows. The axis-only cut
left 667 904 hatching dots at 13.3 G of 34.3 G drawing instructions. The area is the band clipped
to the pixel, plus the disc beyond each round end, in the frame where the pen is the unit disc.
`render_cpu::thin_segment::draw` composes as `scan::intersected` does, then blends as the
high-precision pipeline does. The witness: 4 367 627 marks admitted.

**Before → after**, HEAD export → HEAD plus this, one clock hold, fast cores: fit raster on one
thread **2 933.1 / 2 835.0 ms → 834.0 / 844.8 ms**; four-times window 913.1 / 926.1 → 624.6 ms.
**Xvfb**, `tools/state.sh heavy` with `HEAVY_BIN`, first frame: `quorra-gtk` 5.93 → 4.16 s (raster
2.76 → 0.79 s), `quorra-qt` 7.57 → 3.62 s, `quorra-confined` 12.69 → 3.60 s. `quorra` 7.08 → 7.28 s:
its device refusal (2.48 → 2.60 s) comes first, and its processor draw went 1.92 → 1.03 s.

**Pixels.** `raster_golden` moves three first pages, raster only, each looked at; `issue12295.pdf`
is darker on 6 746 pixels, by up to 10 levels. Arms against `/home/AI/arms-1496`: 0 digests moved in
all six, oracle means on those three pages only. The witness: 48 761 pixels differ, 43 615 by one.

**Unfinished.** `drawn_row_by_row` forbids a cut at strokes under `unmeasurable_width`, though this
kernel draws them pixel by pixel; that is slot 5's plan. The 834 ms is the command loop's, with
`device_bounds` 9.9 % of callgrind. `oracle.rs`'s 7.000 for `issue12295.pdf` is now 0.6 % stale.

**Files.** `crates/{pdf-render,render-cpu}/src/thin_segment.rs` (new) and both `lib.rs`, `render-cpu/src/scan.rs`,
`render-cpu/tests/thin_segment_ladder.rs` (new), `raster_golden.tsv`, ADR 1829, `doc/todo/69`, this.

**Gates.** `rustfmt --check`, clippy `-D warnings` on `pdf-render` and `render-cpu`: exit 0. On
`pdf-model`: 101 in the tree, from a sibling's `callgrind_interpret.rs`; 0 on HEAD plus this.
nextest: 460 passed; `pdf-model` 2 036 passed, 19 skipped. `conformance`: 405 passed, 1 failed (1498's record).
`raster_golden` (`--tree 6`, queued 0 s, held 100 s): 3 moved, then held 974. Division test:
exit 0. Six arms (`--tree 12`, queued 1 290 s, held 1 633 s): exit 0. `tile_price` (`--clock`,
133 s queued, 81 s held) and `heavy` ×2: exit 0. Duration 8 200 s.
