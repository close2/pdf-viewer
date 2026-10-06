# 1365: a screen reader finds the window, and types into a field

This round had the UI slot of batch fifty-seven. It wrote ADRs 1565 and 1566, moved no ledger row
and wrote no question.

**The two sections read against the tree.** In `doc/todo/31`, "window focus is told to no adapter"
held: `update_window_focus_state` had no caller. "`SetValue` on a text field is the one worth taking
next" was false: `accesskit_atspi_common` raises `SetValue` only from `Value`, as a number, so a text
value cannot arrive that way. Every item in `doc/todo/30`'s "What is left" was already struck. Its
closing "what is left is two" was also stale, because §12.3.5's collection is in all three windows.
`tools/state.sh windows` had five `UNREAD` rows and one `SPENT` row.

**Ranked by a document's demand**, the first two a reader meets were these. First, the window's
focus, which matters for every document. Second, a field clicked through AT-SPI, which matters for
every form: both native windows refused it by name.

**ADR 1565.** `Bridge::focused` is now called by each window at bring-up and on every change. The
sources are winit's `Focused`, GTK's `is-active` and Qt's `ActivationChange`. The confined window
publishes no tree, so it has nothing to tell.

**ADR 1566.** `Clicked::Aimed` carries the widget annotation. The native windows give the keyboard to
the control placed over it, and `Clicked::note` lost its `placed` parameter. These were small named
hunks in `viewer-host` (`form.rs`, `tests/host_mappings.rs`).

**The drive** gained `32-window-active` and `33-aimed-field`, each run on two documents. Before the
change: 32 was wrong in all three windows, and 33 was wrong in `quorra-gtk` and `quorra-qt`. After:
all six work. In 32, the pointer leaves the window before the keyboard goes to the root. Otherwise X
hands the keys to the window under the pointer, and GTK reports itself active.

**Prose.** `doc/todo/30` and `31` now state what is: 850 → 201 lines and 397 → 155 lines. Their
`grep -c hundred-and-` counts went 67 → 0 and 30 → 0. `doc/todo/30` now leads with the native windows'
missing policy words: `--trust-anchors`, `--reference-files` and `--reader-*`. `tools/state.sh
windows` gained five readings, and the spent `Query:View` reading was deleted; 0 `UNREAD`, 0 `SPENT`.
`doc/state-of-play.md` gained one sentence.

**Gates.** `rustfmt --check --edition 2024` on the twelve Rust files: exit 0. `bash -n` on
`drive-windows.sh` and `state.sh`: 0. `RUSTFLAGS=-D warnings cargo clippy` on `viewer-host`,
`viewer-accessibility`, `viewer-gtk`, `viewer-qt` and `viewer-ui` with `--all-targets`: 0. `cargo
nextest run` on the same five crates: 0, with 446 passed. `cargo test -p conformance`: 0. The whole
drive ran behind the lock: exit 0, 121 works / 0 wrong / 3 not offered, 811 s.
