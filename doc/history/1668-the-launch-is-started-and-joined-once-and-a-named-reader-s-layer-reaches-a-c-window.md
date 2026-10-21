# 1668 — The launch is started and joined once, and a named reader's layer reaches a C window

Session 1668, slot 6 of batch one hundred and three, a "host" round; 2026-10-21. ADRs 2172 and
2173. No Q file. No ledger row moved: the round changes where the launch is written, not a clause.

**Built.** ADR 2172: `viewer_host::native_window::launch` — `Launch::start` (the reader's commands,
the named directories' refusals, the opening commands and page one anticipated on the opening
thread) and `Launch::state`; `NativeWindow::sized` (the viewport, the extent told to the drawing
ahead, the first size's bounded wait and join, a later size told or kept). The toolkit's part is
typed: `Refusals::{Terminal, Sent}`, `Sizing { FirstSize, Fitting }`, and `Joining::StillOpening`,
on which GTK alone arms its timer. `WindowState::sized` replaces GTK's and Qt's `opened` and the C
window's `joined`; GTK's `Host::open` takes the launch whole. ADR 2173: the audience reaches a C
window on `QUORRA_OWED_PANELS`' layer row and the frames; `drive_a_window.c`'s `the_audience`
reads both for `--reader-name Ada`, `Grace` and nobody.

**The premise and the hypothesis.** The copy held: three windows, line for line. Two premises did
not: no window says a refusal in a dialogue or a status line — GTK and Qt both print `note:` on the
terminal, the C window sends a `SAID` record — and the C window's later sizes never asked the fit
again, a difference of shape (nothing there calls `Presenting::placed`), so it became `Fitting`
rather than a step taken. The hypothesis held in shape: GTK lost 184 lines for 55, Qt 137 for 28,
the C window 102 for 23, and `launch.rs` is 302 with its types' documents. The audience premise
did not hold: the layer a named reader sees was already owed (the panels record, the frames), so
no kind was added — one would have answered `Query::Layers` twice. `quorra`'s launch
(`viewer-ui`) is its own order and was not moved.

**Found, not fixed.** `tools/drive-windows.sh` edited by a sibling mid-drive ran the steps twice
(`local: can only be used in a function`, line 4878); the after drive was re-run on HEAD's copy.
`viewer-host`, `viewer-ffi`, `viewer-gtk` and `viewer-qt` hold 129 rustdoc errors, none in a line
written here; none of the four is in ADR 1938's held list.

**Gates.** In `tools/batch.sh export 1668 --as after` (HEAD, this round's paths, `-U0` hunks of the
four shared files): `cargo fmt --check` on the four crates 0; `RUSTFLAGS="-D warnings" cargo
clippy --all-targets` 0; `cargo nextest run` 0, 402 of 402 — both C programs (`-Werror`) and
`header_and_interop_agree` among them. The audience step planted wrong (`(Ady)`): exit 1, "on 0
for Ada". In the tree: `cargo test -p conformance` 0, 64 suites; `tools/state.sh --round 1668
hosts` 0, Command 45 of 45, Query 35 of 36, 349 entry points (none added here). The drive at
`:1668`, `drive`, `reader_words`, `loading_bar`, `zoomed_window`, HEAD's binaries before and the
export's after, both on HEAD's script: 134 works, 0 wrong, 6 not offered, both; all 140 verdicts
identical, the seen column apart only in paths, ports and the bar's share (the first after run, on
the edited script, had Qt's `01-center-window` wrong in one of two passes). Duration: 3400 s.
