# 1716 — The Qt lag is the re-raster under load, and the first Wayland size is held

Session 1716, 2026-10-23. Batch 110, slot 5 (host). ADRs 2266, 2267; no Q filed.

**The drive.** `quorra-qt` posts Control+wheel notches to itself under `PDFVIEWER_ZOOM_DRIVE`
(ADR 2266), so the owner's Wayland window was driven without synthetic input. One loop run (the
owner's word of 2026-10-23), 90 s inside one `--clock` hold: Wayland, `eDP-1` 2880×1800 at 1.6 and
120 Hz, Radeon 890M; every window activated by its pid through a KWin script and confirmed active.

**Hypothesis did not hold.** No notch waited for a re-raster before showing anything: the scaled
stand-in (ADR 1843) painted 3.6 to 8.9 ms after the wheel in the quiet run, 11 to 40 ms in the
others. The settled frame: 11 to 23 ms quiet, 26 to 111 ms loaded. **Dominant term**: the re-raster
per notch, its processor half — scene encode 1.3 to 70.3 ms, window raster 9.1 to 72.0 ms — against
a device execute of 0.2 to 0.4 ms; the spread is the machine's. The loop wrote no load of its
own; read 2 min after it ended, the 1, 5 and 15 min load averages were 30.27, 44.08, 34.50 on 24
threads, and every loop figure here was taken under them. The one Qt-owned term, the copy of an unchanged raster (at most 10.0 ms), cannot bring the
loaded figure under a 120 Hz frame, so it is not built. Device bring-up there: 135 to 227 ms.

**First Wayland size** (ADR 2267): every owner launch drew page 20 at ratio 2 first, 26.8 to
88.4 ms, against the brief's under 5 ms. Qt now holds the first Wayland size until the configure
or the first paint. Nested KWin at 1.6: the first sharp paint at a median of 134 ms after the first
size against 179 ms, no ratio-2 draw; at 2 and 1 no first frame later.

**Leftovers.** The scale hook alone: `zoom-probe-wayland.py --rescale 1.6:1.25` reads 1:1 at both
with no resize after the change; with the hook removed, no new frame. GTK captured by identity on
the owner's display: 1:1, every byte. `zoom-probe-drive.py` forces X11 only under `--x11`.
**Unfinished**: deep-zoom drift, which wants GTK's own in-program drive.

**Gates.** `cargo fmt --check -p viewer-qt`, warnings-denied clippy `--all-targets` for
`viewer-qt` and `conformance`: exit 0. `cargo nextest run -p viewer-qt`: 43 passed, 1 skipped, exit 0.
`cargo test -p conformance --no-fail-fast`: exit 101 on `variables` (the drive's variable is read in
C++, which the gate did not read; it now reads `.cpp` and `.h`) and on `names` (a neighbour's
`viewer-ffi` edit); `--test variables --test evidence --test adr_status` then exit 0. Qt drive under
Xvfb on the export, this tree and HEAD: exit 0 both, 120 works, 2 wrong, 1 not offered, the same
steps (`83-transition-frames`, `88-archive-refused`). `zoom-probe-wayland.py` at the seven scales:
1:1 at each, exit 0; `--rescale`: exit 0, control exit 1. Loop hold 91 s, exit 0. `t2-toolkit_launch`
(Qt), two `--clock` holds: exit 101 both, the tree's `device_page_one_ms` 5.8 to 6.0 against
the floor 5.9, HEAD's 6.0 to 6.2. Layout, not the change: the device thread's instructions under
callgrind 142.80 against 142.15 million, and under `setarch -R` in a third hold, exit 0, the tree
6.9 and 7.2 ms against HEAD's 6.9 and 7.0 (load 7.8 to 8.3); the band stands (ADR 2267).
