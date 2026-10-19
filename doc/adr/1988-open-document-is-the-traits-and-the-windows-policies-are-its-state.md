# 1988 — `open_document` is the trait's, and the window's three policies are its state

Session 1576, slot 6 of batch ninety-one, a script round of the project owner's switch of
2026-10-14 to "code improvements/refactoring". Status: **accepted** and **built** in `viewer-host`,
`viewer-gtk` and `viewer-qt`. Context: ADR 1920 (one trait over one state), ADR 1972 §3 (what stayed
each host's, and why). Supersedes ADR 1972 §3's second bullet. Code:
`viewer_host::native_window::{NativeWindow::open_document, WindowState, standing, opening_commands}`.
It cites no clause it did not cite before and moves no ledger row.

## 1. The decision

**`NativeWindow::open_document` is a provided method**, written once: it keeps the password a
person typed (ADR 1967) and drains `opening_commands`. ADR 1972 left it a required method because
its body named three of each host's own fields. Those three move into `WindowState`:

- `widget_appearances` — §6.3.2.2's delegation, what `--draw-widget-appearances` changes;
- `restrictions` — the window's levels and the open document's departures (ADR 1145);
- `separations` — §10.8.3's simulation as the window last asked for it (ADR 1228).

Each is a fact about the *window*, which `Showing`'s own comment already said of all three; each is
now read by a provided method, which is ADR 1920's rule for a field of `WindowState`. `links` and
`remote_documents` stay on each `Host`: no provided method reads them.

**The two free functions both hosts held line for line move with them**: `standing`, the
`Restrictions` a command line's `Settings` give, and `opening_commands`, the policies an open sends
before its document. The launch thread each host starts calls the same two, so the open at launch
and every open after it are one statement in one place, which is what `opening_commands`'s comment
had claimed of each host separately. `WindowState::new` takes the window's `WidgetAppearances` and
`Settings` and builds the three fields from them, so no field holds a stand-in until a host fills it.

## 2. What moved

Every moved body is HEAD's code token for token once comments are set aside, the hosts'
`self.restrictions`, `self.separations` and `self.widget_appearances` are read as
`self.state.<field>`, and the trait's `self.state()`/`self.state_mut()` as `self.state`; GTK's `pump`
is `drain`, as ADR 1972 §2 already reads it. The one comment that differed between the hosts'
`opening_commands` is now one, GTK's with Qt's ADR 0245 in it. Nothing a person sees can move: the
same commands go to the core in the same order; the drive of the windows is the witness.

**Gates.** `doc/history/1576` gives the drive before and after, the multiset and the tests.
