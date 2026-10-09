# 1475 — A choice selection runs its keystroke, and an arrow says so

Slot 2 of batch seventy-five, 2026-10-09, a host round. ADRs 1786 and 1787; Q371 not used. No
ledger row changed status. §12.6.3's `implemented` row gained one sentence and two tests for Table
199's `/K` at a selection.

**Premise.** Held in the code. `set_field`'s match ran `/K` for `Entered::Text` only. The wire
already carries `key_down` (`pdf-script/src/wire.rs`), so `wire::VERSION` did not move.
`ChoiceList` is `chrome.rs`'s, but `quorra`'s keys reach it in `bin/quorra/{window,typing}.rs`.

**Hypothesis.** Held. The option the selection names goes through `keystroke_verdict` as
`event.change`, with its export value as `changeEx`. A rewrite to another option's text selects
that option.

**Built.** (1) `selection_verdict`. (2) `quorra`'s drawn list takes Up and Down, which reverses
`doc/todo/30`'s "takes no keyboard". (3) GTK's list, and Qt's list and closed combo box, mark an
arrow press. Each window sends `Command::Keys` with the arrows down around that edit alone.

**Found on the drive.** One arrow selection logged two `/K` runs, and the first carried the earlier
option. `Open::commit` replayed the whole log at every edit, so every logged keystroke script ran
again under the keys held now. `refused_keystroke` also ran each typed value's script a second time
on a copy. A commit now applies only its own entry. Only an undo or a redo replays the log, and the
copy is tried only for a one-call `AF` keystroke (ADR 1787).

**Unfinished.** A choice runs no `/K` commit form and no `/V`, since ADR 1579 makes a pick whole
when made. `quorra-confined` runs no scripts, and the C ABI has no keys entry point (ADR 1771).

**Gates.** `rustfmt --check --edition 2024` on the 13 Rust files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy --all-targets`, exit 0 each, for `pdf-model`, `pdf-script` with and without `engine`,
`viewer-core`, `viewer-ui`, `viewer-gtk` and `viewer-qt`. `cargo nextest run`, exit 0 each:
`pdf-model` 2 016 passed (an earlier run had 1 failure in a sibling's Type 3 work); `pdf-script`
`engine` 170, plain 22; `viewer-core` 374; `viewer-ui` 162; `viewer-gtk` 15; `viewer-qt` 34;
`viewer-confined` 101; `viewer-ffi` 32. Plants (trap 13): with `Chosen` let through, 6 of 6
`list_keystroke` tests failed; with the full replay back in `commit`, 3 of 4 `script_keystrokes`
tests failed. `cargo test -p conformance --no-fail-fast`: 428 passed, 0 failed. Tier 2, as a
`--tree 12` walk on a release build: `tools/drive-windows.sh` whole, exit 0, with 261 works,
0 wrong and 6 not offered. Step 73 works in all three windows: GTK and Qt read `K 1 Beta keyDown
false; K 2 Gamma keyDown true`, and `quorra` reads `Alpha true` then `Beta false`. Lock: the step
probe waited 635.6 s and held 28.8 s; the whole drive waited 930.7 s behind slot 5 and held 853.0
s. `launch_path` was not owed, since no open path changed. Duration 5 130 s.
