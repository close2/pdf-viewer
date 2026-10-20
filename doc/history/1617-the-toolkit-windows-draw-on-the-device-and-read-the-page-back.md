# 1617 — The toolkit windows draw on the device and read the page back

Session 1617, slot 3 of batch ninety-seven, a "pixels" round, 2026-10-20: ADR 2049's G1 and Q1;
ADRs 2070-2071.

**Built.** `viewer_device::renderer::ReadBack`: the device on a thread of its own, brought up beside
the document's read, each page drawn by `QuorraRasterizer` into a `Readback` target and handed back
as `Rendered::Raster`, the lane chosen by `quorra`'s `Costs` and `Refusals`, ADR 1831's sentence in
the title. `viewer_host::drawing::PageLane` is the seam: `needs_render` offers a page to the lane
first, `take_the_drawn` answers its pages with the drawing thread's loop. GTK wakes on
`MainContext::invoke`; Qt takes the pages on its drawing poll. `--cpu` on both is the arrangement
before. Both toolkit manifests join `WINDOWS`; the rule-2 test was planted (GTK left out) and failed.

**The premises that did not hold.** `FrameLines` and `FrameClock` serve the stand-in policy of a
window that presents layers; the toolkits keep `viewer_host::stand_in`. Qt's wake is none: a push
into `QApplication::exec` is a second hand-written `unsafe` token. And `viewer-device` alone was not
enough: the ask and the answer are `NativeWindow`'s provided methods, so `viewer-host` gained the
trait and a field rather than each window a third copy.

**The hypothesis.** One copy per page held. The frame time stayed inside the drive's steps but in
`page_as_built`, whose 200 000 fills the device drew in 1.10 s with nothing to glance at; that step
now launches the toolkits `--cpu`, its subject being the drawing thread (ADR 1825).

**Figures** (ADR 2071, one `--clock` hold, five rounds, medians, device against `--cpu`): first frame
GTK 146 against 138 ms, Qt 135 against 98 ms; page one 13 against 4 ms; device up 36-64 ms, RADV
headless. The device is the default, on principle 2; Qt's 30 ms before its first resize is a lever.

**Shared files**: `viewer-host`'s `drawing`, `loading`, `native_window` (additive), `Cargo.lock`
(two lines), `tools/drive-windows.sh` (`-U0`). **Unfinished**: step 27 is still not offered in the
toolkits; their launch has no band; `quorra`'s `predicted` and `Prediction::said` say one thing twice.

**Gates.** `cargo fmt --check` and `RUSTFLAGS=-D warnings cargo clippy --all-targets` over
`viewer-device viewer-host viewer-gtk viewer-qt`: 0, 0. `cargo nextest run` the four: 413 passed, 2
skipped. `cargo test -p conformance`: 0 failed. `tools/state.sh hosts`: 45 of 45, 36 of 36. Launch
gate in the hold: exit 0, 31 banded, 0 outside, so `quorra` is unchanged. Drive of the export's
binaries: `quorra` 39 works, `quorra-confined` 2, `quorra-gtk` 41 and 2 not offered, `quorra-qt` 42
and 1, each toolkit alike on both lanes; Qt's 43 photographs differ device against `--cpu` by at most
315 pixels (glyph edges, the status line's path), GTK's alike but for launch shots taken before the
`--cpu` window mapped. Arms not owed: `cargo tree -i viewer-host` is the viewer crates (habit 119).
`tools/state.sh docs` not owed: none of its crates touched. Duration: 6 100 s.
