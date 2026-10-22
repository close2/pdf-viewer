# ADR 2200 — The native window lends effective chrome

WinUI received `QUORRA_OWED_CHROME` but could not read the state it was told to apply. The
vocabulary-layer preference query takes a different handle and returns document preferences,
not `Presenting`'s effective state. Casting handles or duplicating the preference decision would
break the boundary.

An additive `quorra_window_chrome` entry point copies four `uint32_t` fields, each zero or one,
from the window's resolved chrome. It changes no existing struct or ABI version. The header,
C# declaration and null-pointer contract are checked alongside the other entry points. A native
stateful witness opens hidden chrome, ordinary and full-screen documents and switches among them;
each change must both owe CHROME and expose the corresponding state.

WinUI consumes that record through its owned wrapper. Its navigation toggle and status line
are window UI. The Permissions menu preserves ADR 1145's exception: HideMenubar cannot remove
the reader's control over restriction levels; presentation hides it with other windows. There is
no toolbar, so HideToolbar has no widget to hide. HideWindowUI does not close an outline pane. `other_windows` independently
controls panel availability, with the user's open/closed preference restored after temporary
hiding. Repeated hidden records cannot overwrite that remembered preference.

The pure managed transition test runs on Linux; the host is compiled against the real Windows
SDK. Neither proves XAML layout or live Windows behavior. Full-screen placement/title-bar,
presentation clocks/transitions and modal-dialog suppression remain separate debts, as do FitWindow and
CenterWindow. This change does not mark §12.2 complete.

The existing Present and LeaveFullScreen actions now also update the FFI window's shared
presentation state, clock ownership and repaint obligations before returning the same physical
window action to C. Escape therefore restores effective chrome even where native placement is
not implemented. The library resolves NonFullScreenPageMode into a one-shot CHROME panel payload;
`quorra_owed_chrome_panel` returns NoAnswer to preserve choice, -1 to close, or a tab index to show.
Ordered records preserve pending requests across plain chrome refreshes; taking the batch consumes
them, so a later refresh cannot override the reader's subsequent panel choice. Managed code copies
the payload before freeing the batch and does not interpret PDF page modes.

A stateful tab witness exposed that effective presentation had stayed global while the host
parked other document state. Showing now offers optional private presentation parking, initialized
only when the front document's catalog is obeyed; the active WindowState remains authoritative.
This round adopts it only in the FFI window. Focus, close and replacement restore mode without
re-reading the catalog or replaying panel commands. Both On and Off reach the core. A temporary
background-open roundtrip preserves the original front clock and transition state; real activation
clears outgoing transients before incoming render events, so synchronization cannot erase a fresh
page face. Escape choices survive revisits. GTK and Qt have not adopted this support: their own
physical window, panel and timer lifecycle needs separate witnesses before that migration.

Real document activation invalidates undelivered chrome commands from the document being left;
a temporary background open preserves those of the front document. The core's presentation mode
still applies globally to its open documents: restoring chrome and Escape choices is not a claim
that a parked PresSteps chain resumes at its old navigation state.
