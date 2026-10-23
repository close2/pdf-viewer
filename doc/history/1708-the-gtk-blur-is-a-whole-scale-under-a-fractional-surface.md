# 1708 — The GTK blur is a whole scale under a fractional surface

Session 1708, 2026-10-23. Batch 109, slot 5 (host). ADRs 2250, 2251; Q571 withdrawn.

**The display.** One loop run (the owner's word of 2026-10-23 admits UI questions) read his
session: `wayland`, no `QT_QPA_PLATFORM`, `GDK_BACKEND`, `QT_SCALE_FACTOR` or `GDK_SCALE`, panel
`eDP-1` 2880×1800 at 1.6; HEAD's `quorra-gtk` on `GdkWaylandDisplay` at scale 2, this tree's at
1.6, `quorra-qt` on `wayland` at 1.6. Every trial of batches 106–108 forced X11
(`tools/zoom-probe-drive.py:46–49`, verified), hence Qt's 1.59375 and GTK's 1 there. Nothing was
left for Q571 to ask, so it is not filed.

**Hypothesis held, for GTK; Qt was already right.** On a nested headless KWin set by
`kscreen-doctor` (ADR 2251) GTK's `scale_factor` is 2 where `GdkSurface:scale` is 1.6: HEAD's
source is shown at 0.7992 of its size — blur by construction. **Change** (ADR 2250): the
surface's fractional scale (property by name, floor `v4_10` kept), re-measured on
`notify::scale`; the area's device size counted in whole covered pixels; a finished frame put on
the surface's pixel grid. Scale alone: 96.77 % identical, largest difference 102.

**The Qt chain** (Xvfb, 1.6, `--cpu`, ISO page 20), measured before touching anything: the
dominant term is the re-raster per notch, nine of 8.25–16.06 ms; settled frame painted
10.1–25.4 ms after the wheel; a paint 0.23–2.63 ms; no visible lag on this processor. The
per-step `PDFVIEWER_ZOOM_PROBE` read (`viewer-qt/src/host/input.rs:178`, verified) is read once.

**Gates.** `tools/zoom-probe-wayland.py` (new): this tree 1:1, every byte, at 1, 1.25, 1.5,
1.6, 1.7, 1.75 and 2, exit 0; HEAD `resampled` at the five fractional scales, exit 1 (control).
`cargo fmt --check`, warnings-denied clippy `--all-targets` and rustdoc for `viewer-gtk`,
`viewer-qt`: exit 0. `cargo nextest run -p viewer-gtk -p viewer-qt` (export): 67 passed, 2
skipped, exit 0. Drive of both windows under Xvfb on the export's copy: exit 0, 238 works, 5 not
offered, 3 wrong; HEAD's binaries read the same steps wrong (`88-archive-refused` in both,
`83-transition-frames`, which GTK passed here). `t2-toolkit_launch` under `--clock`: exit 0,
every figure inside its band (the first try lacked the sandbox worker, trap 10, exit 101).
`tools/state.sh --round 1708 docs`: exit 0. `cargo test -p conformance --no-fail-fast`
(worktree): exit 0 (an earlier run failed on a sibling's ledger row naming its then-unwritten ADR).
Loop hold: 43 s, exit 0. Builds: 501 s, 304 s, 197 s, 146 s under `--tree 6`.

**Owed.** The owner's-display pixel comparison: the loop's monitor capture showed another window
in front, so the 1:1 verdict is the nested compositor's. A native-Wayland zoom-anchor (drift)
test, which needs input a compositor admits. Qt's first viewport at DPR 2 before 1.6, a
launch-path cost to price on the owner's display.
