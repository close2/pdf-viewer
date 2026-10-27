# 1801 — The WinUI window runs Table 147 on CI, and the turn test holds its read

Session 1801, a "host" round in batch 120; 2026-10-27. ADRs 2436 and 2437; Q667 unused.

**Contract.** Slot 4: §12.2's WinUI half as a CI smoke step held to Table 147; main's red turn test.

**Premises.** *§12.2 is `partial` for the Windows run alone*: held, by the note's first sentence.
*The title step is advisory until a run passes*: the Actions API names it `success`, with no
failure annotation, in the five runs on main from 004f8e73 to b46ff72e (2 to 7 s), so it is a gate
now. *The read lands before the second key*: held, from `drain`, `take_the_drawn`, `take_the_turn`.

**Hypothesis** (one probe mode, one step): held. Under its `--probe` word, `Probe.cs` writes what
the window reads back after acting. The step runs it on two documents it builds itself and holds
four things: `/FitWindow` (a 3:1 page area after a resize was asked), `/CenterWindow` (within 1 px),
`/NonFullScreenPageMode` ignored with no full screen, and the outline pane after Escape leaves
`/PageMode /FullScreen`. **Row:** §12.2 stays `partial`. Its note and lists now say the move to
`implemented` is owed on that step's first green run (ADR 2436).

**The race** does not reproduce here. The old test failed 0 of 200 alone (load 37 to 39), 0 of 200
pinned to one core, and 0 of 200 whole-binary runs on four cores. A 300 ms pause planted between
the keys fails it 20 of 20 with CI's "left: 0", and the same plant passes the new test 20 of 20.
Each plant was reversed by its own patch. The fix is a test-only `Window::holding` flag that keeps
`drain` and `tick` from `take_the_drawn`; the non-test statement is unchanged (ADR 2437). The new
test failed 0 of 200 at load 33 to 37, with a `viewer-ffi` build beside it.

**Files.** New: `hosts/windows/Quorra.App/Probe.cs`, ADRs 2436, 2437, this record. Edited:
`MainWindow.xaml.cs`; `ci.yml`'s `window` job; `crates/viewer-ffi/src/window.rs` (`holding` hunks),
`window/turn_tests.rs`; the ledger's §12.2 (`-U0`); a passage each of `doc/todo/01`, `30`, `65` and
`doc/state-of-play.md`. I worked in `scratchpad/r1801/`.

**Gates.** `tools/dotnet-check.sh`: exit 0, 0 warnings. The YAML parses with PyYAML (no `actionlint`
here), and the job's four pwsh steps parse with the PowerShell SDK's parser: 0 errors. The step's
verdicts on canned logs: 5 `works` on a good log, and 3 `WRONG` then a throw on an empty one.
`qpdf --check` (evidence) passes both documents. `cargo fmt --check -p viewer-ffi`: exit 0.
Siblings' mid-edits broke `pdf-syntax` and `pdf-archive`, so clippy and the tests ran in an export
of HEAD plus my hunks (habit 138). There, `-D warnings -p viewer-ffi --all-targets`: exit 0, and
`cargo test -p viewer-ffi`: lib 55/55 and six integration targets green. `a_c_program_drives_the_abi`
ran in the worktree instead: 8/8. The 200-run loop: 0 failures. `cargo test -p conformance
--no-fail-fast`: 468 passed, 1 red, `records`' gates rule on 1802's and 1805's records, not mine. No arm and no drive: the non-test build is unchanged, and the C# runs on Windows alone.
