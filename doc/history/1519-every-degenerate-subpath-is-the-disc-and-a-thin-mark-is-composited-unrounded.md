# 1519 — Every degenerate subpath is the disc, and a thin mark is composited unrounded

Slot 5, batch eighty-two, 2026-10-12, pixels; the owner's "improve performance" (brief §3). ADRs
1875, 1876; no question. No ledger row moves; §8.5.3.2's row gains two tests and a sentence.

**Premise (a)** held: `thin_segment` matched `[m, l]` only. Census of 958 first pages: `m h` 27
strokes on 2 pages (25 admitted); one dot in a longer path, past the block. **Premise (b)** held:
witness at the fit, device against oracle, worst 14, mean 0.1893. **Hypothesis (b) did not hold**:
not order (two overlapping dots in both orders agree across the lanes to a level). An `f64` model
from §11.4.7's transparent group: area rounded to a level, then the store, reproduces the processor
to 20 bytes; the store alone reproduces the device (mean 0.021). That first rounding was the whole
residue, and the oracle was the farther from exact (mean |d| 0.359, signed −0.259; device 0.277).

**Built.** `pdf_render::segment_ends` (`m h` and its spellings as the point to itself); `thin_dots`
(a path's dots as discs where all are admitted and no two share a pixel, else the path stays
`split_degenerate`'s), painted after the rest in `render-cpu`; the kernel composites the area
unrounded and rounds once at the store, ADR 1374's floor kept. Slot 6 swapped `render-raster`'s
`thin_line` to `segment_ends` on request, so the device takes `m h` too.

**The figure** (witness 1417 × 1001, HEAD export → HEAD + this, device against oracle): `Cpu` bytes
differing 619 361 → 265 222, mean 0.1893 → 0.0715, worst 14 → 13, SSIM 0.99965 → 0.99988; `Compute`
555 598 → 201 117, mean 0.1584 → 0.0405; `Gpu` 767 343 → 417 157, mean 0.5061 → 0.3895. The worst
left is the non-thin marks'. Bytes, so no window clock on Xvfb or the owner's display is owed. The
processor's fit page at one strip: HEAD 1 129 / 1 167 / 1 181 ms, this 1 501 / 1 076 / 1 061 ms.

**Pixels** (looked at, trap 1): witness 264 945 pixels by ≤ 14, ink −0.143 %, the racks' dark bias
leaving; `issue12810.pdf`'s dot 11 darker as the disc's area; `issue12295.pdf` ≤ 5.

**Files.** `crates/pdf-render/src/{thin_segment,degenerate,lib}.rs`; `crates/render-cpu/src/{thin_segment,
lib}.rs`, `tests/thin_segment_ladder.rs`; `crates/pdf-model/tests/raster_golden.tsv`; the ledger; ADRs
1875, 1876; `doc/todo/69`; `doc/state-of-play.md`; this record.

**Gates.** `rustfmt --check`, six files: exit 0. `clippy -D warnings --all-targets`, `pdf-render` and
`render-cpu`: exit 0. `nextest`: 481 passed; three plants failed (a level first; `m h` refused;
`thin_dots` `None`), reverted. `cargo test -p conformance`: exit 101 only on `records`, naming 1515's and 1517's.
`raster_golden` (`--tree 12`, own export): exit 101, held 970, moved 4 (`bug1978317`, `issue12295`,
`issue12810`, `issue14953`, raster only); division 176, ≤ 1 308 pixels, ≤ 3 levels; regenerated,
exit 0. Six arms against `arms-1515`: each exit 0, verdicts HEAD's, device digests unmoved, the mean
moved down on 4 pages at 1× and 1 at 4×. `turn_path` (`--clock`, HEAD and this, twice): exit 0, 33
banded, 0 outside, each. Queued 18 454 s summed, held 7 646 s. Duration 9 900 s.
