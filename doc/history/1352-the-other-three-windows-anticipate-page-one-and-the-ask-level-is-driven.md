# 1352 — The other three windows anticipate page one, and the `ask` level is driven

The UI slot of batch fifty-five. ADRs 1539 and 1540. No ledger row moved; no question written.

**Premise checked.** `quorra-gtk` and `quorra-qt` hold a `viewer_core::Viewer` of their own and do
not go through `viewer-ffi`'s session, so `viewer-ffi` is untouched. In all three windows the
document was opened only after the toolkit (GTK, Qt) or the device (confined) was up, so
anticipation was worth building in each.

**Anticipation (ADR 1539).** GTK and Qt open the file and start a thread before the toolkit is
asked for anything. The thread makes a viewer with no viewport, sends `opening_commands`
(`Restrict`, `Delegate`, `Separations`, `Open`, now one function) and then `anticipate`. The first
allocation (resize) joins it. The confined window's `Host::anticipate` starts the worker, sends a
0×0 `Resize` and the `Open` from a thread before the window exists, and the worker's `perform`
anticipates after every open. The first A/B found page one interpreted twice in both native
windows: `anticipate` did not carry `Command::Delegate`'s answer, so the first `settle` threw the
page away. Fixed in `viewer.rs`'s `anticipate`, with a test seen to fail without the fix.

**Measured** (`--trace=launch`, Xvfb, release, ten interleaved launches per arm and document,
medians of first frame, ms, before → after): confined WTPDF 104.0 → 99.5, ISO 32000-2 125.0 → 98.0,
bug1815476 177.5 → 144.5; GTK 167.0 → 137.5, 198.0 → 176.5, 165.5 → 146.5; Qt 100.0 → 88.5,
148.5 → 147.0, 136.5 → 119.5. The time after the toolkit or device was up fell on every row (GTK
32 → 13.5 on WTPDF). Qt's ISO 32000-2 row is level because its first resize came 10 ms later with
a second busy thread beside it.

**The `ask` drive (ADR 1540).** `31-fragment-fdf-ask-yes` and `-ask-no` in all three windows. Each
checks that nothing is fetched while the question stands, answers it, then reads the server and
the log. The toolkits' dialogue buttons are pressed through AT-SPI's `Action` (`press.py`);
`quorra`'s drawn card is answered with Enter or Escape and photographed first. All six said `works`.

**Gates.** rustfmt --check on my six Rust files 0. clippy `-D warnings` on viewer-core,
viewer-confined, viewer-gtk, viewer-qt and viewer-ui, all targets: 0. nextest on the same five:
602 passed, exit 0. `cargo test -p conformance` 0. Drive behind the lock: exit 0 in 679 s, 111 works, 0 wrong, 3 not offered (`tools/state.sh drive`).
`launch_path` not run: 1354 owns it, and `quorra`, which it measures, does not delegate.

**Left.** In the confined window `bug1815476.pdf` still spends 85–113 ms after the device: its
page crosses as pixels and is rasterised on the worker's one thread (`doc/todo/34`). Hunk outside
my files: `crates/viewer-core/src/viewer.rs` (`anticipate` and one test in its `tests` module).
