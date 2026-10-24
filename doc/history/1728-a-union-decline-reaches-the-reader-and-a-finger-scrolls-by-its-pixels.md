# 1728 — A union decline reaches the reader, and a finger scrolls by its pixels

Session 1728, slot 1 of batch 112, a host round; 2026-10-23. ADRs 2290, 2291; no `Q`. I worked in
`scratchpad/r1728/`.

**Premise.** Held: nothing carried `CpuRasterizer::declined` past the draw, and GTK's `DISCRETE`
controller turned ten touchpad pixels into a notch (HEAD's input: 77 device pixels an event at
1.6). Not held: one field on `Rendered` — `quorra` answers `Presented` and a window of a page past
its window answers no token, so the value is `Command::Declined`, keyed by the page (ADR 2290); and
Qt needed no touchpad change. Found: `inks.pdf` at 16x declines two marks past `UNION_WORK`.

**Change.** `Command::Declined`, said once a condition by `Event::Reported` and kept with the page's
notes, carried from `Underway::finish`, `viewer_host::drawing`'s page and window draws, both
`quorra` processor lanes, the confined worker and its wire, and the C ABI's `quorra_raster`; GTK,
Qt and `quorra-confined` name the pages in the title, `quorra` counts the item.
`PDFVIEWER_PLANT_OPEN_ROW` plants ADR 2280's open row; `WheelTravel::distance` is the one distance,
GTK's finger its smooth controller's (ADR 2291).

**Measured.** Nested KWin at 1.6, `zoom-probe-wayland.py --touchpad`, a source-less continuous axis
since no finger could be sent (ADR 2291): GTK 16 device pixels an event, 192 for 192; Qt 77, as at
HEAD. The drive's `89-union-declined` (planted folding strokes, `--cpu`) works in all four windows,
its control silent; GTK's photograph shows the disks, the title and the status line.

**Files.** New: ADRs 2290, 2291, this record, two tests. In `crates/`: `render-cpu` (3 + a test),
`viewer-core` (5), `viewer-host` (4 + 2 tests), `viewer-device` (7), `viewer-ffi` (5),
`viewer-confined` (4), `viewer-{gtk,qt}` (`host/{input,window}.rs`), `viewer-ui` (7), `pdf-model`'s
`drawn_while_read` test; `tools/{drive-windows.sh,zoom-probe-wayland.py}`; the §10.7.4 row's note,
`doc/ui-boundary.md`, `doc/state-of-play.md`.

**Gates.** On my export, its 47 paths checked equal to HEAD plus my hunks: `rustfmt --check` per
file 0; `cargo fmt --check` for the nine crates 0; `-D warnings clippy --all-targets` for the nine 0
(101 twice first: two unfulfilled expectations, a `match` for an `if let`); crate-scoped rustdoc `-D
warnings` 0; `cargo nextest run` of eight crates 1320 of 1320; `cargo test -p viewer-confined` 101,
45 of 46 in `confined`, the failure reading `wire::crossing` and the interpreter, HEAD's both here.
`raster_golden` held 974, moved 0. Drive under Xvfb, dev binaries: 403 works, 9 wrong, 10 not
offered, HEAD's 400, 8, 10; the one step wrong here and right in HEAD's, Qt's `48-script-withdrawn`
at load 30, works rerun at load 2.8. `t2-toolkit_launch`, one `--clock` hold, tree, HEAD, tree at
load 1.4 to 1.6: exit 0 each, every figure in its band, the tree within 3 ms of HEAD. Worktree:
`cargo test -p conformance` 0 (101 at the ledger edit, on two siblings' files then mid-write);
`--test evidence --test adr_status --test records` 0 for this record.
