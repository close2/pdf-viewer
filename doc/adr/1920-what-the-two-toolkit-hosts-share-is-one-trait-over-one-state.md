# 1920 — What the two toolkit hosts share is one trait over one state

Session 1542, slot 3 of batch eighty-six, a host round of the project owner's switch of 2026-10-14
to "code improvements/refactoring". Status: **accepted** and **built** in `viewer-host`,
`viewer-gtk` and `viewer-qt`. Context: ADR 0246 (the second host), ADR 1264 (`Showing` travels
with the tab), ADR 1906 and ADR 1908 (the hosts as module trees, and §3's count of what they share).
Code: `viewer_host::native_window::{NativeWindow, WindowState, Showing, DOCUMENT}`. It cites no
clause and moves no ledger row.

## 1. The decision

`viewer_host::native_window` holds the toolkit-free half of a toolkit window's state and the
methods both hosts answered alike:

- **`WindowState`**: the viewer, `Showing`, the `Documents<Showing>`, the arrivals, the drawing,
  the retained and oversized pictures, the warned page, the trace, the search's needle and pages
  left, the clock, the armed transition and the shown face, the viewport and the extent, page one's
  ahead and its read look, and the submitter. A field is here because a shared method reads it.
  Each `Host` holds one as `state`, and its own code reaches the fields as `self.state.<field>`.
- **`Showing`** and **`DOCUMENT`**: one type and one constant, where each host had its own copy.
- **`NativeWindow`**: a trait whose required methods are what a toolkit does differently inside
  the shared methods. That is `say` (GTK's status label, Qt's update flag), `dispatch` (GTK's pump,
  Qt's drain), `repaint` (GTK's `refresh`, Qt's frame flag) and `later_open_the_next` (GTK's idle
  callback, Qt's flag), plus `state` and `state_mut`. **Thirty-two provided methods** are the shared
  code. 28 of them were identical in both hosts once comments are set aside. The other 4 were
  near-identical: `reported`, `write_saved` and `write_extracted` differed only in `&self` against
  `&mut self`, and `show_the_read` only in its repaint.

Each host's `impl NativeWindow` forwards the four required methods to its own inherent methods of
the same name. A path through the type finds the inherent method before the trait's, so call sites
in the hosts did not change. `Host::open`'s literal starts from `WindowState::new`. Qt's `open` no
longer needs `too_many_lines`, so the expectation was removed, because clippy now reports it
unfulfilled.

## 2. What moved and what did not

- **Every remaining function is unchanged token for token** (comments included) once `.state` is
  taken out of the tree's copy (`scratchpad/r1542/py/prove.py`). That is 221 of GTK's functions
  and 356 of Qt's. The ones that differ: `open` (the literal); three functions where rustfmt added
  a trailing comma after `self.state.` lengthened a line; and Qt's `find`. `cxx` calls `find` as a
  method of `Host` itself, so Qt keeps a forwarding `find` with that name.
- **Every provided method is HEAD's code** in both hosts, token for token apart from comments
  (`bodies.py`), with the trait's path to the state written back as `self.`. The differences are
  these: `named(path)` became `documents::label(path)`, and both hosts' `named` is that body
  exactly; `repaint()` stands where each host repainted; a receiver is now `&mut self`; GTK's
  `write_extracted` clones the directory instead of borrowing it, because `say` now takes
  `&mut self`, and that copies one path a write; and rustfmt added trailing commas.
- **Visibility**: `WindowState`'s and `Showing`'s fields are `pub` because two other crates read
  them. Each was tried private in turn (`narrow.py`). Only `warned` compiled private, so it is
  private. `Host::state` is `pub(crate)` because `access.rs` reads it from outside the `host`
  module.
- **Docs**: a shared method's doc comment is the GTK one with the toolkit's name taken out. Where
  the two hosts' docs had diverged, the longer one was kept.

## 3. What is not shared, and why

- **`run`** (identical in both hosts). It tells §14.7's bridge that a command changes the tree
  (`viewer_accessibility::republishes`), and `viewer-host` does not depend on
  `viewer-accessibility`. Sharing `run` would cost a dependency edge or one more required method
  for a 27-line loop. **`fetch_policies`** (a 3-line loop) would need `fetch_policy` as a required
  method, and that method differs between the hosts. **`Debug::fmt`** names toolkit fields.
- **`react`'s match is not split.** Its alike arms are now one-line calls into the provided methods.
  Splitting the match would have needed one of three things: a wildcard in one of the two halves,
  a dead arm repeating the shared variants, or every toolkit answer as a required method. The first
  two lose the exhaustiveness the match is written for: a variant added to `Event` fails to compile
  in both hosts. The third turns a four-method trait into the hosts' whole event interface.
- **The other eight of ADR 1908 §3's near-identical methods** differ in what the toolkit does in
  the middle: a placement flag against a fit, GTK's center-window sentence, Qt's single question
  slot, a submitter's first look. Each is a difference of behaviour, not of spelling.

**Gates.** `doc/history/1542` gives the drives before and after, the test counts and clippy.
