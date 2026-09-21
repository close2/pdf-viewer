# 1155 — The levels reach a menu, the question reaches a person, and a document may depart

**The two owner decisions, read first.** (a) *The gestures follow the HTML mockups*: the owner's
2026-09-03 word asked for HTML mockups of **adding embedded files** in the GUIs, per platform, and
`grep -rn mockup doc/` finds only references to that request — **no mockup exists in the tree**. It
binds the attach and detach gestures, which this round did not build. (b) The session-885 ruling —
"no drag-and-drop, no command palette, no file dialog … by the owner's word that the mockups are
being reviewed first" — defers that same feature rather than forbidding dialogues, so no file dialog
was built and the *ask* prompt, which names no file and opens none, was.

**Measured first (trap 8).** All three windows answered `Event::Asking` with
`viewer_host::unanswerable` and `proceed: false`, and none had a menu bar of any kind.

**Built.** `Command::Restrict(RestrictionScope)` — `Window(RestrictionPolicy)` or
`Document(RestrictionOverride)`, the override one *optional* level per operation so that a departure
in copying does not detach annotating from the window's level. It lives on `Open`, so it ends with
the document; `RestrictionPolicy::under` is the whole layering, asked in `Viewer::standing`.
`viewer_host::restriction` is one decision for three bindings: the rows, the two scopes, the state a
menu edits, `asked`, `declined`, `chosen`. GTK nests it into a `gio::Menu` behind
`set_create_popup_func` and asks in a modal `gtk4::Window`; Qt into a `QMenuBar` refilled on
`aboutToShow` and a `QDialog`; `quorra` draws the rows on a card answered by the arrows, Enter and
Escape. `Key::R` opens it in all three, and each builds its menu when opened and never before. Wire:
a scope byte and `NO_DEPARTURE`. ABI: `quorra_restrict_document_operation`,
`QUORRA_RESTRICT_INHERIT`, entry points 187 → 188, `QUORRA_ABI_VERSION` unmoved.

**A clause decided against a document.** §12.2's `/HideMenubar` is read, answered and deliberately
not obeyed: the only menu bar these windows have is the reader's own levels, and a file that could
hide it would take away the control over itself (principle 3). `/HideToolbar` and `/HideWindowUI`
are obeyed; Table 29's full screen still takes the bar.

**Calibrated (trap 13).** The override's two tests in `restriction_levels.rs` — window `On`,
document `Ask`, the copy asks and a `yes` performs it; annotating stays refused; the next document
opened is refused again — were **planted against**: with `under` returning `self`, both fail and the
other four pass. **Gates**: fmt, the seven-crate clippy line, nextest, doc, `conformance`, the ABI
tests, `awkward_classes`, `launch_path` (26 figures banded, 0 outside) and `raster_golden` (974
held, 0 moved) pass. **One pre-existing failure, not this round's**: `viewer-confined`'s
`a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw` fails on base `9f145bff` in a
control worktree exactly as here — the draw it interrupts finishes inside `HOST_UNFINISHED`.
§7.6.4.2 gains four tests and the scope sentence, §12.2 the `/HideMenubar` reading and its test;
neither status moved. ADR 1145 is the argument; `doc/todo/38` is what-is.
