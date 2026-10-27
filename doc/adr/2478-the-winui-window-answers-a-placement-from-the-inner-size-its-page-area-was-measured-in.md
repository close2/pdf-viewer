# 2478 — The WinUI window answers a placement from the inner size its page area was measured in

Session 1822, the clause slot of batch 122 (ADR 2464's R2).

Status: implemented in `hosts/windows/Quorra.App/MainWindow.xaml`, `MainWindow.xaml.cs` and
`Probe.cs`. It has not run: this machine has no Windows runner, so the next CI run on main is its
first execution, read at batch 122's merge. Amends ADR 2446, whose hold is kept with a smaller job.
Context: ADRs 2308 (the decision is the library's and the act is the window's) and 2436 (the step).

## Premise

ADR 2446 made the window hold a `QUORRA_OWED_PLACE` record from the moment it carried out a
resize until its page area's next size had reached the library. The merge's run, CI 40486886833
on 4ad00b04, job `window (WinUI 3)`, is red the same way as the run before it. On `placed.pdf` the
window wrote four probe lines: a resize asked to 752 by 212, the pane closed, the presenter
`Overlapped`, and a resize asked to 752 by 35. No placement line came within 120 s. The hold's
line, "probe: the placement waits for the page area to take the resize", appears 0 times. **So the
premise that the hold would engage did not hold.** Both deck verdicts held again.

## Finding

The second answer still pairs the client size from after the resize with the viewport from
before it, and the hold was off when that answer was given. The code has two ways for that:

- `ResizeClient` returns, `AppWindow.ClientSize` still reads the old size, and the "platform left
  the size as it was" check clears the hold at once. The size then changes before the layout that
  tells the library. The page area's next size and the client size move at two moments.
- A page area size is told to the library between the act and the layout that takes the resize.

The log cannot say which, because the window wrote nothing about either. But both have one cause.
The window read its inner size from `AppWindow` when it answered, while the library's viewport
came from the last layout that told it. `Presenting::place` answers `inner − viewport + page`, so
those two must come from one moment. A flag that guesses when the two agree depends on an order
of platform events that this machine cannot observe.

## Decision

- **The pair is read in one layout pass.** When `Resize` tells the library the page area's size,
  it also records the window's root content element (`Root`, the XAML `Grid`) and the page area,
  both in logical pixels, from the same pass. `Place` gives the library that recorded inner size
  and no longer reads `ClientSize`. The frame (outer less client) is still read from `AppWindow`
  at the answer, because one read gives both halves and a client resize leaves it unchanged. An
  answer given before the layout takes the resize is now the same resize again, not a stale one.
- **The hold stays, as economy only.** It saves the library's placement frames (`PLACING_FRAMES`,
  four) from repeated asks. It no longer guards correctness, so a hold released early costs one
  repeated ask.
- **The placement line is read after the act.** The window reads where it is once the batch has
  ended, at the dispatcher's low priority, not inside the act. The `AppWindow` reads this log
  cannot vouch for come after the act's own messages and layout.
- **Three probe lines are added**, none matching a pattern the step holds:
  - "probe: measured: the window W by H around the page area w by h last told, the client area
    W' by H' now", at every answer;
  - "probe: the client area reads W by H after the resize";
  - "probe: the page area is w by h device pixels, so no size is told", once per empty episode.

  The step's conditions and messages are unchanged.

## What the next run must show

On `placed.pdf`, a `probe: placed:` line whose page area is three units wide for each one tall
within 3 px, and whose window sits at the work area's centre within 1 px. Both `/FitWindow` and
`/CenterWindow` read `works`, and `/NonFullScreenPageMode` reads `works` with 0 lines of a pane
opened. Every `measured` line has its window size equal to the client size, to within a pixel,
once the layout has settled. If the window size differs from the client size by a constant, the
root element is not the client area, and that is the next finding. A red run is diagnosed from
these lines: a resize's client size read straight after the act says which of the two sizes moved
first, and an empty page area is said, not left silent.

## Consequences

- §12.2 stays `partial`, and its note names CI 40486886833, until a green `window` run is read
  at a merge with its id and its verdict lines.
- There is no unit test of the ordering. What is under test is the toolkit's order of events,
  which only the published window on a Windows runner shows. `tools/dotnet-check.sh` compiles the
  window against the real SDK references, with stubs for the XAML names.
