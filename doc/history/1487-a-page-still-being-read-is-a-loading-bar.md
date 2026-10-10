# 1487 — A page still being read past the warning is a loading bar, in all four windows

Slot 2 of batch seventy-seven, 2026-10-10, a host round on the project owner's own request: "As long
as it isn't a resource problem (only a time problem) we should allow it, and possible display a
loading bar." ADR 1811. No ledger row: the owner's request, not a clause.

**Premise, half held.** `ask_the_host`, `WARN` and `run.rs`'s count are where the brief said; the
interpreting process is not. `quorra`, `quorra-gtk` and `quorra-qt` hold their viewer in process and
each *joined* its opening thread (ADR 1531) with no ceiling, so the owner's page held the toolkit's
thread for the whole read. Only `quorra-confined` interprets in the worker; it joined the same way.

**Hypothesis, changed in its transport.** The figure is input bytes over input bytes of the page's
`/Contents` (`Pump::taken`, `ContentReader::progress`) beside an operator count, published from one
line in `run.rs`'s 64-operator check (ADR 1811 section 2 names it) to a thread-installed
`pdf_model::progress::Progress`. On the owner's page, three alternating rounds
(`examples/loading_cost`, `--clock`): 7.613 s bare, 7.548 s watched, −0.85 %, noise under the 1 %
asked. Across the pipe neither `ask_the_host` (a round trip that stops the interpreter) nor a shared
counter (no safe shared memory): a `NOTICE` frame (`0xF2`, 24 bytes) every 32 768 operators, handed
to a listener; the greeting is `PDFVCF14`.

**The windows.** `viewer_host::Loading` waits at most `WARN` (1 s); then the window answers its
toolkit, looks every 33 ms and joins when ready. GTK and Qt show their own bar under the sentence,
`quorra` and `quorra-confined` a 20-cell bar in the title. Owner's file, under neighbours' load: the
sentence at 1.04–1.10 s, page read at 7.2–8.1 s, first frame at 14.5–17.9 s. Step `75-loading-bar`
of `tools/drive-windows.sh` (own fixture: 50 M `0 g`, a red fill) photographs the bar moving, then
the red page, in all four. Looked at (trap 1): GTK's bar at 78 %, Qt's at 83 %, `quorra`'s red page.

**Unfinished.** A page interpreted after the launch is read on the toolkit's thread with no bar (the
core's rule 4). An in-process read has no stop but the close, driven in GTK alone (ended in 86 ms).
`quorra`'s processor fallback on the owner's page (423 MB of scene refused) is silent for ~10 s:
`mind_a_long_draw` warns only on `--cpu`.

**Gates.** `rustfmt --check --edition 2024` on every file touched: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy --all-targets` on `pdf-syntax`, `pdf-model`, `confined-transport`, `viewer-confined`,
`viewer-host`, `viewer-gtk`, `viewer-qt`, `viewer-ui`: exit 0; `cargo nextest run` on them: 2 900
passed, 23 skipped; `-p pdf-vfs`: 81 passed. `cargo test -p conformance`: 430 passed, 0 failed.
`tools/state.sh --round 1487 launch` (`--clock`, 0.0 s queue, 356.8 s held): exit 0, 31 figures
banded, 0 outside. `awkward_classes` (`--tree 12`, both workers built in the hold): 1 passed, killed
0. `tools/drive-windows.sh` whole (`--tree 12`, 900 s held): exit 0, 268 works, 0 wrong, 6 not
offered. Queues: 438.4, 449.2 and 555.8 s on three small walks. Duration in the report.
