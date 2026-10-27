# 1806 — A placement waits for the resize, and an offset is read for viability

Session 1806, a "clause" round in batch 121 (the ledger slot); 2026-10-27. ADRs 2446 and 2447; Q672 unused.

**Contract.** Slot 1: §12.2 on CI's red Table 147 step; §13.2.6.2/§13.2.6.3 against clause 13's entry.

**Premises.** Both held. In run 40455259365's `window (WinUI 3)` log, the deck's two verdicts work,
and `placed.pdf` resizes to 752 by 212 and then to 752 by 35, with no placement line in 120 s.
`media.rs` executes both tables' subtype rules for a `/B`.

**Hypothesis** (green, or one placement figure): did not hold. The finding is an ordering. The
second `QUORRA_OWED_PLACE` paired the client size from after `ResizeClient` with the library's
viewport and page from before it: `V − p` = 177 and `H` = 389 fit both answers, and a fresh
viewport could not give 35 (ADR 2446). A 35 px client leaves the page area empty, which `Resize`
never tells the library, so nothing is owed again. The ignored-otherwise WRONG is the same failure,
because that verdict is held at the placement line. Fix: the window answers no placement from its
own resize until the page area's next size has reached the library, and the probe says when one
waits. The step's conditions are unchanged; two messages say more. §12.2 stays `partial`, and the
next run is read at this batch's merge.

**Rows moved.** §13.2.6.2 and §13.2.6.3 go from `out-of-scope` to `partial` (ADR 2447): "The
rendition is non-viable if the interactive PDF processor does not recognise the value of this
entry". It is read for whether a `/B` names the start, and it is owed outside `MH`. Seeking and
timing stay the player's. §13.2.6 becomes an aggregate. The new test
`an_offset_names_its_clips_start_only_as_zero_seconds_or_frame_zero` fails on a planted `/F <= 0`.
The plant's patch came from `git diff` while slot 4's one-line hunk was in `media.rs`, so reversing
it removed that line for seconds. It was re-applied at once.

**Files.** `hosts/windows/Quorra.App/MainWindow.xaml.cs`, `Probe.cs`; `.github/workflows/ci.yml`;
`crates/pdf-model/tests/media_viability.rs` (one hunk); `doc/conformance/ledger.toml` (§12.2,
§13.2.6, §13.2.6.2, §13.2.6.3); `doc/todo/01`, `doc/todo/65`; ADRs 2446, 2447. I worked in
`scratchpad/r1806/`.

**Gates.** `tools/dotnet-check.sh` exit 0, 0 warnings. PyYAML parses the workflow, but the changed
PowerShell was not parsed, since none is installed. `pdf-model` against HEAD plus my hunk (habit 138):
`cargo fmt --check` exit 0, `-D warnings` clippy `--all-targets` exit 0, nextest 2274 passed.
`media_viability` in the worktree: 15 passed. `inapplicable`, `cited` and `quotations`: exit 0 each.
`tools/state.sh --round 1806 ledger`: exit 0. `cargo test -p conformance --no-fail-fast`: exit 101,
on two reds that are not mine (`readings.rs:312`'s doc path, 1808's record length). Duration 2660 s.
