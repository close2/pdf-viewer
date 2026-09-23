# 1242 — A page's photographs, decoded beside each other

Date: 2026-09-23. Branch `batch-1239-1244`, worktree `/home/AI/pdf-viewer-rounds`, shared with five
siblings. ADR [1321](../adr/1321-a-pages-photographs-are-decoded-beside-each-other.md). No ledger
row is owed: this changes when a decode happens, never what a clause makes of its samples.

## What was asked, and what was taken

ADR 1272 section 5's parallel decode, under the owner's answer A121: nothing presented before its
photograph. Measured first (`examples/image_decode_census`: 39 of 958 first pages name two images
or more), then built: a pool task reads the content stream ahead and starts each decode, and the
`Do` waits for its raster, so the list is the serial run's. Pinned A/B in one sitting, two exported
trees, `md5sum`'d: the three witnesses' interpretation −36% to −62%, single-image pages unchanged,
`raster_golden` held 974, moved 0.

## Three things worth keeping

**Where a decode is issued is decided by the witnesses, not by the `Do`.** Two draw their
photographs through forms or tiling cells, the third spreads them over three thousand lines: no
look through the reader's window from a `Do` finds the next image; a walk from the page does.

**The first unpinned A/B said the single-image page was 20 ms slower, and it was right twice for
different reasons.** A pool thread on the Zen 5c class decodes a photograph half again as slowly
as the interpreter on a Zen 5, so the first image is now the interpreter's; and asking rayon its
size starts its threads, so the question moved behind the dictionary checks. Then pinned, the
page is the page it was — and unpinned, on an identical code path, its median still moves by
thirty milliseconds, which is the habit's lottery.

**A decode ahead that disagrees with its `Do` costs twice.** `bug1771477.pdf` sets `/Perceptual
ri` before its images; offered under the initial intent, both were decoded twice at once, slower
than serial. The walk follows `ri`, `gs` and `q`/`Q` now, and declines `/OC`, which
`issue12007_reduced.pdf`'s hidden forms showed is otherwise a decode the scope waits for.

## Files touched

`crates/pdf-model/src/image.rs` (`mod ahead`, `RasterCache::{decode_ahead, stop_ahead}` and the
miss path of `parts`), `crates/pdf-model/src/content.rs` (the scope in `interpret_into`), and in
`crates/pdf-model/`: `src/content/image.rs`, new `src/image/ahead.rs`, `tests/decode_ahead.rs`,
`examples/image_decode_census.rs`; `doc/todo/36`, `doc/performance.md`, ADR 1321, this file.
