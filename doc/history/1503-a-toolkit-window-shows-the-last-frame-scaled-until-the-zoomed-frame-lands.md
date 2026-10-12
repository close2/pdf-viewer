# 1503 — A toolkit window shows the last frame scaled until the zoomed frame lands

Slot 2 of batch eighty, 2026-10-12, a host round on the owner's target. ADR 1843; no Q.

**Premise, held**: `PageArea::paintEvent` drew `frames_` at their positions, replaced only when a
render landed; the viewer moves a page's origin and extent at the zoom and keeps its pixels
(`Viewer::arrange`, `OnScreen::frame`). **Hypothesis, half held**: GTK had the same shape and the
same defect — the old frame at its own size at the new corner, photographed in both windows (red
block 202 × 201 at the notch against 251 × 252 landed). A resample on up to eight cores was not
needed: the toolkit's own scaled draw costs 1.9–4.2 ms on one core at 1000 × 1000 (Qt's raster
engine, `scratchpad/r1503/paintcost.cpp`), and GTK's is GSK's.

**Built.** `viewer_host::stand_in::placed` pairs each frame with the extent `Query::PageGeometry`
gives the page now, where the shape is the same to within each side's rounding; Qt paints into
that rectangle with `SmoothPixmapTransform`, GTK through `GtkFixed`'s child transform (a size
request cannot shrink a picture below its texture — the first build's notch out stayed large).
The part-way picture of ADR 1825 replaces the stand-in, answering its §3; each window says
`page N stands in at …` on `frames`. Drive step 82 `zoom_stand_in` compares the block's box on both.

**Figures, s, notch to the first picture at the new size** (HEAD's binaries against this round's,
the owner's drawing, two alternating runs, one clock hold per display). Xvfb: `quorra-qt` 0.900,
0.592 → **0.000, 0.001**; `quorra-gtk` 1.192, 0.626 → **0.006, 0.007**. Owner's display:
`quorra-qt` 1.212, 1.458 → **0.002, 0.001**; `quorra-gtk` 1.104, 1.320 → **0.000, 0.000**. Notch to
the landed frame unmoved within its spread (1.45–3.33 s both arms). Photographed on both displays.

**Unfinished**: a page arriving with a notch out has no frame; a rotation keeps the old frame unscaled.

**Files.** `crates/viewer-host/src/{stand_in,lib}.rs`, `crates/viewer-qt/src/{host,bridge}.rs`,
`crates/viewer-qt/cpp/window.{h,cpp}`, `crates/viewer-gtk/src/host.rs`, `tools/drive-windows.sh`
(step 82), `doc/todo/30`, `doc/state-of-play.md`, `doc/crate-map.md`, ADR 1843, this record.

**Gates.** `rustfmt --check --edition 2024` on the five Rust files, `bash -n`: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy -p viewer-host -p viewer-gtk -p viewer-qt --all-targets`: exit 0 (after a
sibling's `pdf-render` lint cleared). `cargo nextest run` on the three: 300 passed. `cargo test -p
conformance`: 431 passed; at the end 430, 1 failed on 1508's record's unfilled Gates. Step 82 (`--tree 6`, queued 2 501 s): works in both windows, wrong in
both with HEAD's binaries (the plant). Drive whole (`--tree 12`, queued about 3 300 s, held 946 s):
274 works, 2 wrong — step 83 (round 1508's) in `quorra --cpu` and `quorra-gtk`; HEAD's `quorra-gtk`
fails it with the same 3 468-pixel seam, so it predates this round, and GTK's unscaled frames went
back to `GtkFixed::move` after the drive, re-driven: step 82 works. `launch_path` (`--clock`,
queued 378 s, held 287 s): exit 0, 31 banded, 0 outside. Duration 14134 s.
