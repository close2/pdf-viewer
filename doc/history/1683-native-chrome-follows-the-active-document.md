# 1683 — Native chrome follows the active document

Session 1683, 2026-10-22. WinUI §12.2 chrome follow-up; ADRs 2200/2201.

**Change.** The native window lends four resolved chrome flags through an additive C query.
WinUI maps navigation/status controls separately from outline availability, preserves pane choice
through temporary hiding and retains the reader's Permissions exception. One-shot panel commands
travel in ordered CHROME records and are copied before batch release; later refreshes cannot
reapply an old command. Document activation discards commands belonging to the document left.

Present and Escape update shared mode before returning the same physical window action. Escape
restores ordinary hide flags and the catalog's exit panel choice. A tab witness exposed global
presentation state leaking between documents. Optional shared parking, adopted only by FFI,
preserves each document's effective chrome and Escape choice without rerunning its catalog.
Focus, close and replacement synchronize both On and Off. Background opening preserves the front
clock and face; real activation clears outgoing transients before incoming render events.

**Stability.** Independent review found background clock resets, late clearing of incoming faces
and action clocks, and stale panel-command identity. Focused native witnesses pin these boundaries,
including a two-page duration fixture proving actual core Off/On, not merely a chrome snapshot.

**Gates.** Scoped warnings-denied clippy and all 48 native unit tests passed. Final real Windows
SDK compile had zero warnings/errors; pure managed mapping passed, exit 0, 41 s, 0.22 GiB. The custom-profile C invocation linked an old gates library
while nested builds produced dev; direct linker reproduction confirmed missing old ABI symbols.
That run was stopped and its evidence retained. Supported dev coverage comes from the batch
workspace suite; a redundant queued retry was canceled to avoid racing its fixed C fixture paths.

The final workspace dev suite passed all 7,551 tests, exit 0, including both C clients, the new
stateful chrome witness, header agreement and unsafe-boundary checks; 539 s hold, 8.06 GiB peak.

**Found, not fixed.** C ABI tests derive artifact paths from their test profile but nested builds
default to dev; custom profiles can test stale libraries/workers. The harness should build the
matching profile and verify its artifacts. The supported dev rerun avoids that harness defect.

**Limits.** Real Windows SDK compilation and pure managed tests do not prove live XAML behavior.
Native fullscreen/title-bar placement, FitWindow/CenterWindow, modal suppression and the FFI
presentation timer/transition pump remain unfinished. GTK/Qt have not adopted presentation parking.
Core mode still applies globally: parked PresSteps continuation is not claimed. §12.2 stays partial.
