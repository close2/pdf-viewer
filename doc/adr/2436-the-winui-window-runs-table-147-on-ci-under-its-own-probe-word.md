# 2436 — The WinUI window runs Table 147 on CI under its own probe word

Session 1801, a "host" round in batch 120, 2026-10-27.
Status: implemented in `hosts/windows/Quorra.App/` and the `window` job of `.github/workflows/ci.yml`.
It has not run yet: this machine has no Windows runner, so the first CI run after the merge is the
step's first execution. §12.2 stays `partial` until that run is green. Context: ADRs 2301
(the residue is owed rather than departed), 2308 (the placement's decision is the library's, and
its act is the window's), 2309, 2374 (the published window and its title step).

## Premise

§12.2's note held the row `partial` for one thing alone: the WinUI window's half was compiled and
never run. That half is the `/FitWindow` and `/CenterWindow` answer carried out through
`quorra_window_place`, Table 29's `FullScreen` as the `AppWindow`'s full-screen presenter, and the
chrome on leaving it. **Held**: no job started the window except the title step.

The title step was advisory "because no run of it exists yet". The Actions API names it `success`
in the five runs on main from 004f8e73 to b46ff72e, each taking 2 to 7 s, with no failure annotation
on any of the five jobs. That is the answer ADR 2374 waited for, so **the title step is a gate now**.

## Decision

**A `--probe` word on the window, read before the reader's words.** With it, the window writes
sentences to its log (`Probe.cs`, the describer). Each sentence is written once per change in what
it states:

- `probe: asked to resize the window to W by H`: the library's resize answer, before it is carried out.
- `probe: placed: the page area … the window … at …, on the work area …`: written after the
  library's last placement answer is carried out. The values are read back from `AppWindow.Size`,
  `AppWindow.Position`, the page area's laid-out size and the display's work area, in logical
  pixels.
- `probe: the presenter is <kind>`: read back from `AppWindow.Presenter.Kind` after `Present`.
- `probe: the pane is open on <panel>` or `closed`: read back from the navigation view after a
  chrome record.
- Once, at the first page drawn while presenting full screen, the probe presses Escape through the
  same `Press` path the key handler uses (one method, which the key handler now calls too). That
  gives `/NonFullScreenPageMode` an exit to answer.

**The step builds its two documents in PowerShell**, counting the cross-reference offsets, as the C
driver's fixtures do. Both have a 300 by 100 page and a one-item outline.

The first states `/FitWindow`, `/CenterWindow` and `/NonFullScreenPageMode /UseOutlines` and no full
screen. Three assertions are held against it:

- the page area takes the page's 3:1 shape, after at least one resize was asked;
- the outer window sits at the centre of the work area within 1 px, by `viewer_host::centred`'s
  arithmetic;
- no pane opens and no full screen is presented, because the entry "shall be ignored otherwise".

The second states `/PageMode /FullScreen` and `/NonFullScreenPageMode /UseOutlines`. Two assertions
are held against it:

- the presenter is `FullScreen` before Escape;
- after Escape it is not, and the pane is open on Contents.

Each verdict line quotes the Table 147 or Table 29 sentence it holds. The step fails if any one of
them does not hold.

**What the step does not claim.** It does not test the platform's own Escape key. The probe calls
the window's `Press` path, which is everything after the toolkit's key event. It does not show that
a page is drawn right, which no job can see. It does not show the dialogues held while presenting:
ADR 2309's hold is the library's, and `held_tests.rs` proves it.

## Consequences

- §12.2 moves to `implemented` on the step's first green run, read at the merge after it. The row
  already names `.github/workflows/ci.yml` and `Probe.cs`.
- If the first run is red, the log the step prints holds every probe line. A wrong number there is
  a finding about the window, or about `Presenting::place`'s bound on a runner's first frame. It is
  not a reason to loosen the step.
- The `window` job's ceiling rises from 30 to 36 minutes, for this step's own 8.
- Checked here: `tools/dotnet-check.sh` (exit 0) and the workflow's YAML. The step's PowerShell was
  parsed with the PowerShell SDK's own parser, and its verdict logic was run on canned logs, a good
  one and an empty one. qpdf (evidence only) finds the two documents' cross-reference tables sound.
