# 2544 — A document opened beside is asked for its report and its open sequence when its own frame is shown

Session 1855, a script round in batch 126. Follows ADR 1044 (when a host asks for the report) and ADR 2525's last
Consequence, which owed this.

Status: implemented.

## Context

ADR 2525 found that `quorra-gtk` and `quorra-qt` send `Command::Report` and `Command::Presented` for the first
document only, and named `viewer-host`'s `report::Due` "one per window" as the cause. **That premise did not hold**:
`Due` lives in `native_window::Showing`, which `Documents::focus` swaps with each tab, so every document has its own
and `opened()` arms the new one. The miss is the *moment*. Both hosts took the `Due` only in
`NativeWindow::answered`, which runs when the drawing thread answers the viewer for a page. A document named behind
the first (the command line's later paths, ADR 1275) is focused for a moment, its page is drawn then, and the front is
given back before the drawing lands; when a person brings its tab forward the window shows a frame the viewer already
holds, and no answer for a page ever arrives for it. Driven under Xvfb with two documents and `--trace`, the GTK window
before this change logged `Report` and `Presented` once, at 0.228 s, and never for the second document, though it was
in front from 25.277 s on: its §12.8 signatures were never reported, its Table 32 scripts never ran and its XFA merge
never activated.

## Decision

1. **Two moments ask, and a host needs both** (`viewer_host::report::owed`): the viewer answered for a page of the
   document in front, and the window put on its screen a frame of the document in front. The first stays in
   `NativeWindow::answered`, now only for a page of the focused document, since both commands are about the focused one
   and a page of a tab behind is not a frame of it on the screen. The second is each toolkit's paint: GTK's `refresh`
   and Qt's `painted`, where a frame of the focused document is held.
2. **The paint does not run the commands; it asks for them from the main loop** once the paint has let go, naming the
   document then and checking it is still in front when the loop comes back: GTK by an idle callback
   (`later_ask_what_it_owes`), Qt by the flag C++ already reads at the end of every update (`arrival_due`), answered in
   `arrive`, which now carries the report owed and the next document separately. No C++ changes.
3. `Due` stays one per document and true once per opened document (ADR 1044); `is_owed` reads it without taking it,
   so that the paint can ask without spending it on a document that is no longer in front by the time the loop runs.

## Consequences

- Driven under Xvfb with two documents and two tab switches, both windows now log `Report` and `Presented` once for the
  first document and once for the second, at its first time in front (GTK 25.262 s, Qt 25.295 s), and not again on the
  return to the first. `viewer-qt`'s test `a_document_opened_behind_is_asked_for_its_report_once_its_frame_is_shown`
  drives the Qt host's half with no display, and fails with the paint's hook taken out.
- The report of a document whose page was drawn while behind costs nothing until it is shown, as page one's does.
- `quorra`'s own window asks at its surface's present already (ADR 2525) and is unchanged. `viewer-ffi`'s window takes
  the same `answered` path and was not driven here; whether a WinUI tab brought forward shows a held frame without an
  answer is a question for the `window` job, not settled by this ADR.
