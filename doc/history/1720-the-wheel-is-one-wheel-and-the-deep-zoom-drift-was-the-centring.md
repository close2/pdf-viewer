# 1720 — The wheel is one wheel in both windows, and the deep-zoom drift was the centring

Session 1720, 2026-10-23. Batch 111, slot 1 (host). ADRs 2274, 2275; no Q filed. Worked in
`scratchpad/r1720/`.

**The instrument.** The nested KWin is the probe's own, and its `org.kde.KWin.EIS.RemoteDesktop`
hands a libei sender the pointer, Control and the wheel, so both windows were driven through the
compositor; the premise that no compositor admits synthetic input holds for the owner's session
only. `tools/zoom-probe-wayland.py` gains `--wheel` (the count, read off the output) and `--drift`
(the anchor split into the core's and the window's), so no GTK in-program drive was built.

**The owner's question: yes.** His receipts: 46 Control events, every one source 0, angle ±120 and
pixel ±15, each counted as 15 pixels, 0.3 of a step — three notches in four did nothing. Qt 6.12
reports the axis value for every source (6.10 withheld it for wheels). At 1.6 on HEAD, Qt: one zoom
step of four notches, 24 device pixels a notch, a move on every eighth; GTK: four, 77, the fourth and
twelfth. **Change** (ADR 2274): `WheelTravel::of`, `DiscreteWheel` (GTK 4.24.1's count),
`SCROLL_NOTCH`, and, as the owner asked, `ZoomTravel`: a click is a step, less than a click a
proportion at once (GTK zooms on a second controller without `DISCRETE`). Qt now: four, 76, the
fourth and twelfth; sixteen Control eighths are sixteen factors of 1.0283 in both windows.

**The drift: the hypothesis did not hold.** Not a rounding: each window's frame stayed within 1.1
device pixels of the core's placement; the core carried it all, 20.5 page units in GTK's window and
38.1 in Qt's at 9.3 times, from the steps on which the page is narrower than the viewport and
centred. **Change** (ADR 2275): a gesture holds the point it was pointed at; GTK reads the pointer on
`enter` (its first step was anchored at the centre). After: 0.57 and 0.78 units, under one fitted
pixel (0.90, 1.02), and ten steps out land where they began.

**Gates.** On the export: `cargo fmt --check`, per-file `rustfmt --check`, warnings-denied clippy
`--all-targets` and rustdoc for `viewer-host`, `viewer-core`, `viewer-gtk`, `viewer-qt`: exit 0 (clippy
first 101 twice, a float compared and a 119-line function, fixed). `cargo nextest run` of the four:
768 passed, 5 skipped; the new headless test is red on HEAD's core (234.6 units, the pointed 100).
The probe's export copy: `--wheel` and `--drift` exit 0 in both windows. Drive under Xvfb, four
windows: 401 works, 7 wrong, 10 not offered; HEAD's three toolkit windows 284, 6, 6; no step wrong
in both tree drives and right in HEAD's. Arms: `cargo tree -i` puts `viewer-core` and `viewer-host`
outside `render-raster`'s and `pdf-sandbox`'s graphs. `t2-toolkit_launch`, one `--clock` hold, tree,
HEAD, tree: exit 0 each at load 1.7 to 1.9, every figure in its band (the first hold lacked the
sandbox worker, trap 10, exit 101). Release builds: exit 137 three times at the 6 GiB ceiling, then
2171 s and 773 s under 12. `cargo test -p conformance --no-fail-fast` (worktree): exit 0.

**Unfinished.** A touchpad: EIS states no axis source, so no finger could be sent (ADR 2274).
