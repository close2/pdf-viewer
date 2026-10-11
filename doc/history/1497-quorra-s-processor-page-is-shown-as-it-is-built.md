# 1497 — `quorra`'s processor page is shown as it is built, and asked for where it is faster

Slot 2 of batch seventy-nine, 2026-10-10, a loop round on the owner's target. ADRs 1831, 1832; Q393.

**Premise, held**: `compose_pages` was not progressive; `CpuRasterizer::showing` was the hook.
**Hypothesis, held on the owner's display, not under Xvfb**: there HEAD's device draws every frame,
first frame 12.5–19.0 s, gesture 15.2 and > 20.3 s; llvmpipe binds 128 MiB and still refuses, so
Xvfb measured the refusal path (6.8–10.8 s, gesture 3.5–4.7 s).

**Built.** `Costs` prices each lane per command per arrangement (seeds 1 788 and 667 ns, ADRs 1823,
1819); a frame predicted past `WARN` on the device takes the smaller prediction, said in the trace
and title. `compose_pages_watched` + `Assembly` show the processor's draw as it is built where the
window has no picture of the page; the title carries the share. A processor page's whole-page
picture is drawn on the processor, interruptibly; no sharp pass after a processor frame.
Owner's report (`--cpu` zoom): not reproduced as a blank window; rule 4 refused at most notches
(1-core resample 124–185 ms against 26–40 ms frames) and its sample was spent at the next tick, so
the same notch then stood in late. Fixed both (ADR 1832). `quorra-qt` was not looked at.

**Figures, s** (HEAD `f9b240c2…` against `728bb691…`, two alternating runs, one clock hold each).
Owner's display: first frame 18.98, 12.51 → **6.73, 5.20**; first picture part-way 4.19, 3.30;
gesture > 20.3, 15.22 → **1.43, 2.11**. Xvfb: 10.79, 7.58 → **4.92, 6.32**; part-way 3.31, 4.48;
gesture 4.66, 3.47 → **1.50, 1.48**. `--cpu`, 2500 × 1600, `tracemonkey.pdf`, notch to true frame:
192, 168 → **47 ms** mean (`df650acb…`). Photographed: the page fills in correctly part-way.
**Principle conflict, said not settled**: page one on the processor departs from principle 2's
"page one goes to the GPU"; Q393 recommends amending it, and meanwhile the tree does ADR 1831.

**Unfinished**: `--cpu`'s composer is not progressive; the device is never priced on a page the
seeds gave the processor; a look allocates a texture pair. `state-of-play` is slot 6's this batch.

**Files.** `crates/viewer-ui/src/software.rs`, `crates/viewer-ui/src/bin/quorra.rs`,
`crates/viewer-ui/src/bin/quorra/{renderer,surface,app,stale}.rs`, `tools/drive-windows.sh`
(step 81 offered in `quorra`), ADRs 1831, 1832, `doc/questions/Q393`, this record.

**Gates.** Built in a private export of HEAD plus these files (siblings' `raster-gpu` was mid-edit).
`rustfmt --check --edition 2024`, `bash -n`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p
viewer-ui --all-targets`: exit 0. `cargo nextest run -p viewer-ui`: 179 passed, 2 skipped; the new
`a_frame_nobody_waits…` fails with the `WARN` gate planted out (trap 13). `cargo test -p
conformance`: 430 passed. Step 81 (`--tree 6`): works in `quorra` and `quorra-gtk`. Drive whole
(`--tree 12`, queued 3 420 s, held 901 s): 270 works, 0 wrong. `launch_path` (`--clock`, queued
1 348 s, held 205 s): exit 0, 31 banded, 0 outside. Duration 12 800 s.
