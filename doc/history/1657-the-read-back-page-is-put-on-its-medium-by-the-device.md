# 1657 — The read-back page is put on its medium by the device

Session 1657, slot 3 of batch one hundred and two, a "pixels" round, 2026-10-21: ADR 2124's last
lever, profiled first (habit 59); ADR 2150. No ledger row moves; Q552 not used.

**The profile.** Stamps around `QuorraRasterizer::rasterize`'s three passes on the toolkit page
(620 × 876, release, RADV, warm): premultiply 1.17–1.20 ms, `impose_within` 0.79–0.92, demultiply
0.17–0.18 — the whole of the 2.16–2.28 ms outside the scene and the device call. ADR 2124's reading
of the code holds, measured. **The premise held; the hypothesis held in shape**: warm 4.0 → 2.8 ms,
not 2.5, and still above the processor's 2.2, because the read back gained 0.4 ms.

**Built.** `raster_gpu::Target::ReadbackOnto(Backdrop)` and `onto.wgsl`: the copy put onto an
opaque colour in a storage buffer, the host unpads and converts nothing; `render-raster` asks for
it where a uniform opaque medium and the crop are all a page owes; `pdf_render::{crop_pixels,
medium_bytes}` state the rectangle and the colour once; the stage line says `outside both`.
Exactness: the round trip is `min(c, a)` over all 65 536 pairs, the crop one rectangle, over-opaque
alpha 255 — each a test; two plants (a truncation, a colour one level off) fail them.

**Measured** (one `--clock` hold, HEAD's export and this one interleaved, load 3.2–3.9, pinned):
medians of 14, GTK/Qt — page 2 4.0/4.1 → 2.8/2.8 ms, its outside both 1.6 → 0.0, read back 0.4 →
0.8/0.9; page 1 (cold) 8.9/8.3 → 7.9/7.4. `device_page_one_ms` 8.8, 8.5 → 7.6, 7.8 (GTK), 8.1, 8.1
→ 7.1, 7.0 (Qt). `quorra`'s `turn_ms` 4.09/4.48/4.66 → 2.51/2.69/3.07, past each band's floor.

**Unfinished.** `doc/checks/launch-path.toml`'s three `turn_ms` bands must be taken again (slot 7's
file, and the merge's); a kept staged buffer (the 0.4 ms) is unpriced.

**Shared files** (`-U0`): `doc/state-of-play.md`, `doc/todo/30`, `raster/doc/PLAN.md`.

**Gates.** `rustfmt --check` and `cargo fmt --check`: 0 for `raster-gpu render-raster pdf-render
viewer-device`; `-D warnings clippy --all-targets`: 0 for the first three in the worktree and for
`viewer-device` in the export (the worktree's is a neighbour's `pdf-transform`, exit 101); nextest
749, 108, 289 passed and 85 in the export; `state.sh docs` 0;
`cargo test -p conformance` exit 0. Six arms against `/home/AI/arms-1655/`: 0 of 5 799 frame
digests differ. `raster_golden` (export): held 974, moved 0. The hold: `t2-toolkit_launch` four runs
exit 0; `t2-launch_path` exit 101 on both — HEAD on `opt_demo.pdf`'s warm open and script figures
over their high edges, this tree on the three `turn_ms` floors and `PDF20_AN001-BPC.pdf`'s warm open
(0.402 against 0.400); `t2-turn_path` two runs each exit 0, all 33 figures inside. Drive of both
toolkit windows on the device lane: GTK 112 works, 4 not offered; Qt 115 works, 1 not offered; 83
`wrong` in both, and re-driven three times a build it fails in HEAD two of six and here three of
six. Duration: 10 769 s.
