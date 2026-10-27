# 1822 — A placement is answered from the pair one layout measured

Session 1822, a "clause" round in batch 122 (slot 7, R2 of ADR 2464); 2026-10-27. ADR 2478; Q688 unused (ADR 2479 became the merge's).

**Contract.** §12.2 on the merge's red `window` run: CI 40486886833 on 4ad00b04, job `window (WinUI 3)`.

**Premises.** The brief's reading of the log held. `placed.pdf` asks 752 by 212, then the pane
closes, the presenter is `Overlapped`, and it asks 752 by 35. No placement line comes in 120 s,
and the hold's line appears 0 times. Both deck verdicts work. ADR 2446's premise, that its hold
would engage, did not hold.

**Hypothesis** (the second ask came from a pair the hold did not guard): held. The code has two
routes to the stale answer with the hold off. Either `ClientSize` read unchanged straight after
`ResizeClient`, which clears the hold at once, or a page area size was told between the act and
the layout. The log cannot tell which. Both come from one cause: the window gave the library its
inner size read from `AppWindow` when it answered, against a viewport from the last layout that
told it. Fix (ADR 2478): `Resize` records the root `Grid`'s and the page area's sizes from the
one layout pass that tells the library. `Place` answers from that pair, so an early answer repeats
the resize rather than halving it. The hold now only saves placement frames. The placement line
is read after the batch, at the dispatcher's low priority. Three probe lines say what the next
run needs to tell the routes apart: the measured pair beside the client size, the client size
read after a resize, and an empty page area. The step's conditions and messages are unchanged.
No row moved: §12.2 stays `partial` and its note names this run. The next green run is read at
the merge.

**Files.** `hosts/windows/Quorra.App/MainWindow.xaml`, `MainWindow.xaml.cs`, `Probe.cs`;
`doc/conformance/ledger.toml` (§12.2's note, one hunk); `doc/todo/65`; ADR 2478. I worked in
`scratchpad/r1822/`.

**Gates.** `tools/dotnet-check.sh`: exit 0, 0 warnings, 0 errors; the chrome mapping test passed.
`cargo run -p conformance --bin cited`: exit 0. `tools/state.sh --round 1822 ledger`: exit 1, on
§13.2.6.2's row at line 4936, a sibling's mid-edit. My §12.2 hunk, laid on HEAD's ledger, parses
(Python `tomllib`). `cargo test -p conformance --no-fail-fast`: exit 101, and none of the reds is
mine. They are that same ledger line (every test that loads the ledger), quotations in `media.rs`
and `colour_transform.rs`, Table A.20 and A.28 in `pdf-jpx-write`, and `batch.rs:2132`'s reflink
under `/tmp`. No Rust changed, so the six arms, `raster_golden` and the windows' drives cannot
move. PowerShell is not installed, and the step is unchanged. Duration 1583 s.
