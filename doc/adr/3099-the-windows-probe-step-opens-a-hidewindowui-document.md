# 3099 — The Windows probe step opens a `/HideWindowUI` document

Session 2099. Status: **implemented, not yet run** (no .NET or PowerShell on the development machine; CI's
`window` job is its first run). Context: ADR 2436, ADR 3040–3042, record 2086 (D53: "The CI step with a
`/HideWindowUI` document is still owed"), `doc/todo/30`. Clause: ISO 32000-2 §12.2, Table 147.

## Decision

1. **A third document in the same step.** The `window` job's Table 147 step already writes its documents with its
   own `Write-Document` — numbered objects and a counted cross-reference table, no corpus bytes — and reads the
   window's `--probe` log back. It now writes `bare.pdf`, whose catalog states `/ViewerPreferences << /HideWindowUI
   true >>` beside the same outline, and waits for the line `WindowProbe.Controls` writes.
2. **The held sentence is the collapsed one.** The step holds the last controls line to `probe: the status line is
   collapsed, the pane toggle is hidden`: Table 147's "A flag specifying whether to hide user interface elements in
   the document's window (such as scroll bars and navigation controls), leaving only the document's contents
   displayed", read on the two controls `MainWindow.ApplyChrome` sets.
3. **The calibration is the placed page.** The step's first document states no `/HideWindowUI`, so by the entry's
   "Default value: false" its last controls line must say both controls are visible. A probe that wrote the
   collapsed line for every document fails there instead of passing here.
4. The step's ceiling rises from eight to ten minutes for a third window of up to two, and the job's from 36 to 38.

## Cost

The step is checked only where it runs: its PowerShell is parsed by nothing on this machine, so a typing error in it
is found by the first `window` run on main, which the merge reads.
