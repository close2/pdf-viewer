# 1416 — The first frame's excess is the page's own work, and nothing a warm-up could make is moved

Pixels slot of batch sixty-five. ADR 1668; ADR 1669 and Q312 not used.

**Re-taken** (ADR 1668 section 1). `examples/first_frame` now prints each frame stage by stage from
`FrameCost`, with its minor page faults and raster's phases for frames 1, 2 and 10, and it marks
the frame boundaries where `strace` and `callgrind` split. ISO 32000-2 page 7, eight interleaved
runs a target, behind the lock: frame 1 less the steady frame is 6.70 ms at 596 × 842, 7.74 at
1191 × 1684 and 17.87 at 2382 × 3368 (section 9 said 13.3, 14.3, 18.1). Encode holds 4.1 to 5.8 ms
of it, the scene 0.5. At 4× the readback adds 2.4 and the host pass over the read-back raster 7.6,
which frame 2 also pays. A one-second settle makes the 1× excess 6.62 against 5.77, because the
first submission then meets an idle device (ADR 1658).
**Counted** (section 2). At wgpu's boundary under `callgrind`, frame 1 makes two textures the tenth
does not: the 8 MiB glyph atlas and the white stand-in. It also makes two HAL encoders and 25 driver
buffer objects under recording, which is ADR 1606's pool. 57 M of its 78.4 M extra instructions are
the glyphs' coverage, and its 2 028 faults are mostly the encode's own, 85 of them the atlas sheet.
**Moved, measured, declined** (sections 3 and 4). Made on the warm-up thread before the warm set,
the two textures were found 24 of 24 times. Upload fell 0.28 → 0.18 ms (1×) and 0.38 → 0.20 (4×),
but the frame's excess stayed at 6.12 → 6.15, while the warm set was delayed 6.18 → 6.63 ms.
Made after the warm set, they lost the race to the 1× upload in 7 of 12 runs. Made synchronously,
0.16 to 0.23 ms bought nothing outside the spread, and prefaulting the sheet moved nothing. Not
built. `doc/QUORRA_FEEDBACK.md` section 9 is answered and withdrawn; `doc/performance.md`'s launch
sentence and §3e, and `doc/todo/36`, now say so.

**Premise.** Held: section 9 was open, the example exists, and ADR 1606's finding stands. Not held:
"first-use resource creation" was most of the cost. On this tree the excess is the page's own
coverage. Only 0.1 to 0.2 ms is device-lived creation a warm-up could take.

**Gates.** `rustfmt --check` on `first_frame.rs`: 0. `RUSTFLAGS="-D warnings" cargo clippy -p
render-raster --all-targets`: 0. `cargo nextest run -p render-raster`: 0, 100 passed, 3 skipped.
`cargo test -p conformance`: 0, 403 passed. Tier 2 is not owed: the diff touches no code a frame
runs (`raster/`, `render-raster/src`, `pdf-render` and `pdf-model`'s content are unchanged), so
the six arms in `/home/AI/arms-1414/`, `raster_golden`, `turn_path` and `launch_path` are HEAD's
by construction. The experiment patch was reversed before these ran.

**Unfinished.** Two costs are not first-use and are left: the 4× host pass of frames 1 and 2, and
the encode's 46 thread spawns per frame of new geometry. For slot 4: no bring-up figure changed, but
`doc/todo/42` section 5 still quotes "~12 ms of first-use allocation", which ADR 1668 retires.
