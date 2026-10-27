# 2446 — The WinUI window answers a placement only once its page area has taken the resize

Session 1806, the clause slot of batch 121 (ADR 2370's ledger rule).

Status: implemented in `hosts/windows/Quorra.App/MainWindow.xaml.cs` and `Probe.cs`, and in the
messages of the `window` job's Table 147 step in `.github/workflows/ci.yml`. It has not run yet:
the next CI run on main is its first execution, read at batch 121's merge. Context: ADRs 2308 (the
placement's decision is the library's, and its act is the window's), 2436 (the step and its probe).

## Premise

ADR 2436 moved §12.2 to `implemented` on the step's first green run. That run, CI 40455259365 on
60f975c8, job `window (WinUI 3)`, was red on two of the five verdicts. Both deck verdicts held: Table
29's `/FullScreen` and, after Escape, `/NonFullScreenPageMode /UseOutlines`. On the placed page the
window's whole log was four probe lines: a resize asked to 752 by 212, the pane closed, the
presenter `Overlapped`, and then a resize asked to 752 by 35. No placement line followed in 120 s.

## Finding

**The second answer paired two sizes from either side of the first resize.** `Presenting::place`
answers `inner − viewport + page`, so the window's inner size and the library's viewport must
describe the same moment. `drive_a_window.c` keeps that pairing: it tells the library the new
viewport in the same step as its resize. The WinUI window does not. `AppWindow.ResizeClient` changes
`ClientSize` at once, but the page area's `SizeChanged`, and with it `quorra_window_resize`, comes
only at the next layout pass. A frame answered in between owes `QUORRA_OWED_PLACE` again, and that
answer reads the new client size against the old viewport and page.

The log's two answers fit this and nothing else. Call the first ask's inner height `H`, viewport
height `V` and page height `p`. Then 212 = `H − V + p`, and a stale second ask gives
`212 − V + p` = 35, so `V − p` = 177 and `H` = 389. Both widths are 752, which needs only that the
page filled the viewport's width. With a fresh viewport, the second answer would be 212 (the page
fits) or taller (the pane's close widened the page). It could not be 35.

**Why no placement line came after.** A 35 px client height leaves the page area no height.
`Resize` tells the library nothing for an empty area. So no frame is drawn and no further
`QUORRA_OWED_PLACE` is owed, and the window sits until the step's deadline.

**The ignored-otherwise verdict is the same failure.** It counted 0 pane or full-screen lines,
which is the entry ignored. But it is held at the placement line, so with no placement line it
reads WRONG.

## Decision

- **A resize carried out makes the window wait.** From just before `ResizeClient` until the page
  area's next size has been told to the library, a `QUORRA_OWED_PLACE` record is not answered, and
  the probe writes "probe: the placement waits for the page area to take the resize", once per
  resize. The library owes the record again when that size arrives: a size taken while the fit is
  still owed asks it again (`native_window::launch`). The wait is set before the act, so a layout
  that tells the area's size inside `ResizeClient` clears it. It is cleared at once where the client
  size did not change, because then no new size will come.
- **The step's conditions are unchanged.** Only its messages say more: a missing placement line
  names the number of resizes asked, and the ignored-otherwise line says whether it was held at a
  placement line.

## Consequences

- §12.2 stays `partial` until the step is green. The next run will show one of three things: a
  placement line, a resize that keeps waiting, or a figure. Each is read from the probe lines.
- The wait is a bool and has no unit test. The `Quorra.Chrome.Tests` project covers only the pure
  chrome mapping, and what is being tested is the toolkit's ordering, which only the published
  window on a Windows runner shows.
- Checked here: `tools/dotnet-check.sh` (exit 0, 0 warnings) and the workflow's YAML (PyYAML). No
  PowerShell is installed on this machine, so the step's changed lines were not parsed by
  PowerShell's own parser.
