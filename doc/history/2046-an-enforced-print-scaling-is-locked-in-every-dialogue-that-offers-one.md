# 2046 — An enforced print scaling is locked in every dialogue that offers one

Session 2046, a single ledger round, the signatures lane (`doc/todo/65` section 6's only row; §12.2,
`doc/questions/A834` item 2). ADR 2908; no question.

**The slice, and why.** The brief's items 1 and 2 together — the hold in `quorra-gtk` and the C ABI's — are one
fact, `Event::Printing`'s `scaling_held`, reaching the two windows ADR 2840 left with a sentence; item 3 is named,
not built: the WinUI host has no print dialogue and this VM no .NET SDK.

**The premise.** Held: GTK and the C window said `held_note` and locked nothing; no `A*` after A834 cites
`/Enforce`. One part did not hold: the row's other gap, `/HideMenubar`, is the departure ADR 1622 kept, so with
`/Enforce` done the row is `departed`, not `implemented` or `partial`.

**Built.** `viewer_host::printing::Defaults::held_scaling`. `viewer-gtk`: `hold_the_scaling` answers
`create-custom-widget` with a tab saying why, from which GTK's `GtkPrintUnixDialog` loses `SCALE` from its manual
capabilities at `Priority::HIGH`, and `custom-widget-apply` sets the scale back before GTK builds the job (read in
GTK 4.18.6's `gtkprintoperation-unix.c` and `gtkprintunixdialog.c`). `viewer-ffi`: `quorra_event_printing_scaling`,
`quorra_owed_print_scaling`, `Owed::Print::held`; header, C# interop, counts 377 → 379 and 358 → 360.

**Tests.** Three GTK tests drive GTK's own signals and dialogue (locked, the job's scale, the unheld control);
`c/drive_a_window.c --enforced` reads both new calls at `off`, `warn`, `on`, `ask` no and yes against ADR 2840's
table. **Plants**: the capability left in, the scale not set back, the window's record never held: the two GTK
tests and the C window test failed; reversed, all pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`viewer-host`, `viewer-gtk`, `viewer-ffi`): 0. nextest of the
three under `xvfb-run -a`: 458 of 458, 2 skipped, the C programs included. Fuzz `cargo check --bins`: 0.
Conformance after `git add -N`: 474 passed. Tier 2: `selection_census` 0 (974); `accessibility_census` 0 (114
with structure); `awkward_classes` 0 (none killed); `corpus` and `raster_golden` not owed (no crate they build
changed). `state.sh --round 2046 docs` 0.

**Habit, and what is left.** A row's status after its gap closes is read from its own history first: §12.2 was
`departed` before an audit found `/Enforce` owed. Left: the WinUI card reading `quorra_owed_print_scaling` when it
gains a print dialogue; `/HideMenubar` stays the owner's departure.
