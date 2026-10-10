# 1489 — A heavy page is timed through the window path, in all four windows

Slot 4 of batch seventy-seven, 2026-10-10, an instruments round on the owner's own request (the
operator bound, the loading bar, the cost of loading; slot 4 is the timing). ADR 1815. No ledger row.

**Premise, held**: the drive runs the windows under Xvfb, `viewer-confined` sends the smaller of list
and raster (ADR 0607), and nothing timed the window path. **Hypothesis, held, and two things it
lacked.** The page crosses as `Payload::Raster`: its list is 431 743 784 B against a scale-1 raster
of 16 058 624 B (26.89×). But `quorra` does not reach the device either: it refuses a 423 206 976 B
scene against its 268 435 456 B budget and draws on the processor, on Xvfb and on the owner's 890M.
And at HEAD every window was deaf until the open joined, the confined host until the page landed.

**The instrument.** Step 80, `heavy_open`, outside the default list; `tools/state.sh heavy` runs it
as a clock walk at nice 0, then the byte count as a small walk; `HEAVY_BIN` measures an export.
Calibrated (trap 13): a one-rectangle page reads a 1.26 s first frame and a 0.00 s key wait in
`quorra`; a two-page file is refused by name. Slot 2 took steps 75–79, through the orchestrator.

**Xvfb, llvmpipe**, HEAD 59bdf2d7's export → the tree with slots 1486–1488 in (62 paths, ADRs
1809–1811, 1813), load 3.0 and 1.4 at the starts. Seconds by the window's trace — first frame; open
joined; longest key wait — and the bar at 1.000–1.001 s after: `quorra` 15.20 → 12.00, 5.53 → 2.65,
4.84 → 0.95; `quorra-gtk` 14.78 → 8.73, 7.76 → 2.88, 7.14 → 0.38; `quorra-qt` 14.99 → 8.31, 8.32 →
2.51, 0.00 → 0.00 with its window mapped at 8.31 → 1.06; `quorra-confined` 13.64 → 10.73, 6.61 →
3.66, 13.54 → 6.70. Peaks (`VmHWM`/`Anonymous`, MiB) held: `quorra` 4 041/3 786 → 4 043/3 786,
`quorra-gtk` 1 489/1 373 → 1 495/1 378, `quorra-qt` 1 388/1 320 → 1 389/1 320, confined host
132/33 → 131/33, `pdf-view-worker` 1 727/1 664 → 1 728/1 698.

**Owner's display** (XWayland, 890M, no key pressed, both arms in one clock hold at load 1.0–1.3):
first frame, open joined. `quorra` 23.50 → 12.33, 8.56 → 2.67; `quorra-confined` 14.64 → 11.02,
5.68 → 2.54; `quorra-gtk` 16.57 → 9.47, 6.88 → 3.31; `quorra-qt` 18.15 → 9.86, 5.75 → 2.57; bars at
1.000–1.003 s. An after arm taken beside slot 3's arms (load 11.3) read `quorra` at 32.94 s: unused.

**Unfinished**, measured and built nowhere: the confined host still answers no key for 6.70 s;
`quorra`'s page is a 9.23 s processor draw behind the scene budget (9.57 s at HEAD); its 4 GiB peak.

**Files.** `tools/drive-windows.sh` (`heavy_open`, header) and `tools/state.sh` (`section_heavy`,
the lists), each replaced by rename; `doc/HANDOVER.md` (one clause); ADR 1815; this record.

**Gates.** `bash -n` on both scripts: exit 0. `tools/state.sh heavy` before (`HEAVY_BIN`, queued
648.2 s, held 61.8 s) and after (built in the hold, held 44.2 s; byte count 118.4 s): exit 0, 4 works
each. `cargo test -p conformance`: exit 0. `tools/batch.sh check`: exit 0. Duration 7 300 s.
