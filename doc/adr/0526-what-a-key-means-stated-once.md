# ADR 0526 — What a key means, stated once

Status: accepted, 2026-08-23. Session 687, the third round on the project owner's *"we should start
investing time into the UI (and its API for the native versions)"* and the second item of the
ordering ADR 0509 wrote. **All three windowed hosts now take their key bindings from one value**,
the three disagreements the item was ranked on are settled by decision, and each host carries a test
that fails when it stops translating the whole table. Adds `viewer_host::keys` and
`viewer_host::NOTICE`; costs no new message.

## 1. The item, and what it was ranked on

`doc/todo/30`'s ranked list, second place: *"A statement of what a key means, in `viewer-host`.
Four key tables that already disagree … Same argument as `Presenting` and `Clock` (ADRs 0470,
0473), and the first application of it that a test could check."*

**The count was wrong and the direction was right.** There are **three** key tables, not four:
`viewer-ffi` has no keyboard at all and never had one — `include/pdf_viewer.h` says so in the one
place it mentions a key, *"§12.5.1's tab key. The order is the document's (Table 31's `/Tabs`); the
key is yours"* — because a C caller places its own toolkit and owns its own keyboard entirely. So
the fourth consumer is not behind here; it is a different kind of thing, which is ADR 0509 §2's own
finding applied to its own list.

The three disagreements it named were all real and are all in the diff:

- `f` opened the find bar in `viewer-gtk` and armed §12.5.6.6's free-text drag in `viewer-ui`;
- Up and Down scrolled the view in `viewer-ui` and turned the page in the two native hosts;
- Escape cleared the selection in the two native hosts and **quit the process** in `viewer-ui`.

## 2. Why the decision goes in `viewer-host`, and what shape it takes

The argument is `Presenting`'s and `Clock`'s verbatim (ADRs 0470, 0473): **which sentence a window
is obeying is shared, and `gdk::Key` against `Qt::Key` against `winit::keyboard::Key` is what a
toolkit is.** It is the fifth application of that argument and the first one with an instrument.

The shape is two enumerations rather than one map:

- **`keys::Key` is a closed set of the keys this program binds** — thirty of them, `Key::ALL` — and
  a host's only job is to map its toolkit's key onto one or onto nothing. A `char` and a name would
  have been the obvious type and is the wrong one: it cannot be enumerated, so nothing could hold a
  host to translating all of it.
- **`keys::Meaning` is `Send(Command)` or `Window(WindowAct)`**, because a key table has two kinds of
  row and collapsing them costs the checkability. A `Send` is complete from the key alone and every
  host dispatches it identically; a `Window` names a job whose *doing* differs in every host — a
  clipboard, a find bar, a panel — and whose *meaning* does not. `WindowAct` is matched exhaustively
  in all three hosts with no catch-all arm, which is `doc/ui-boundary.md`'s rule applied one layer
  out from the boundary.
- **`keys::Mode` is on the lookup**, because two rows depend on whether §12.4.4's presentation is
  running and folding that away would have lost the one place the standard actually decides a
  binding.

**`WindowAct::ScrollBy(f32)` is the row worth explaining**, because the table could have built a
`Command::Scroll` itself and deliberately does not: that message speaks *device* pixels and a shared
value has no display to ask. The distance is stated in logical pixels and each host multiplies by
its own scale — which is the conversion the two native hosts already did for a wheel notch and which
`viewer-ui` was not doing at all, so its arrow keys moved half as far on a doubled display as they
were meant to.

## 3. What the standard decides, which is two rows, and what is chosen

The standard names a key exactly twice, and both are in the table with their clause beside them.

§12.5.1: *"Interactive PDF processors may permit the user to navigate through the annotations on a
page by using the keyboard (in particular, the tab key)."* A permission, and the only key in the
standard with a job attached. Tab was bound in one host of three; it is bound in all three now.

§12.4.4.2, twice over — the EXAMPLE *"Pressing an arrow key."* and the requirement's own *"[i]f the
user requests to navigate forward (such as an arrow key press)"*. **Both sentences are inside the
presentation subclause**, which is what settles the arrow-key disagreement rather than a preference:
all four arrows navigate while a presentation is running, and Up and Down move the view while one is
not. The clause is satisfied where it applies, and outside it the reader gets the view movement that
a viewer whose wheel already scrolls (ADR 0442) obviously wants. It also fixes a real gap in the
other direction: `viewer-ui`'s Up and Down scrolled *during a presentation*, so on the tier-2 host
two of the four keys §12.4.4.2 names made no navigation request at all.

§7.7.2's Table 29 decides a third row without naming a key, and it is the one place a clause *takes*
a binding away. `FullScreen` is *"[f]ull-screen mode, with no menu bar, window controls, or any other
window visible"*, so while a presentation is running `f`, `/`, `o` and `?` — the find bar, the panel
of trees, the notices — mean nothing at all. That is `Chrome::HIDDEN` applied to the keyboard rather
than to the widgets.

Everything else is a documented choice and is written down as one in the module. Two are worth
repeating here:

- **Escape clears the selection and leaves full screen first; it never leaves the program.**
  `viewer-ui` called `event_loop.exit()` here, and that is worse than surprising: this program has
  had `Command::Save`, an edit log and a `Query::Dirty` for hundreds of sessions, so the key every
  other program uses to mean *"not that"* could throw away an annotation somebody had just made.
- **`f` opens the find bar and §12.5.6.6's free-text drag moves to `t`.** Two hosts of three already
  meant find by it; `t` was bound by nobody.

## 4. What each host gained, and the one asymmetry that is a tier's rather than a gap

Levelling three tables against one statement is mostly *addition*, and the additions are the point:

- **`viewer-ui`** gained `z` and `y` — which is `doc/todo/30`'s **item 6** falling out of item 2
  exactly as ADR 0509 §3 predicted it would ("it is a keyboard binding once item 2 exists") — and
  `w`, and lost Escape-quits.
- **`viewer-gtk` and `viewer-qt`** gained Tab (§12.5.1), Space, `h` and `k` (§12.5.6.10's two
  markups), `o` and `?`.
- **All three** gained a panel toggle that composes with Table 29 rather than fighting it: the panel
  is on the screen when the clause permits one *and* the reader asked for one, so leaving full
  screen puts back what the reader had rather than what the document last permitted.

**`w` on the tier-2 host is not a gap and is answered rather than ignored.** `viewer_host::ControlFit`
compares a *toolkit's* minimum size against the `/Rect` a document states, and `viewer-ui` sends no
`Command::Delegate` and places no toolkit control — what it draws is the widget's own appearance
stream, which is inside that rectangle by construction. So it answers the sentence a native host
answers for a page whose controls all fit, and says why.

**§12.5.6.6's free-text drag is refused by name in the two native hosts**, which is trap 5 rather
than silence: authoring that annotation is a drag mode plus an editor, both of which are
`doc/todo/33`'s and neither of which those hosts have. `doc/todo/30` carries it as the remaining
asymmetry.

**And one asymmetry is a platform's and cannot be closed here.** On a delegated form a real
`GtkEntry` or `QLineEdit` has the focus, and the toolkit's own focus traversal takes Tab before any
window controller sees it — so what is walked there is the toolkit's order rather than Table 31's
`/Tabs`. Named in §12.5.1's ledger row and in `doc/todo/30`.

## 5. The instrument, which is what makes "the hosts stay level" a rule rather than a sentence

ADR 0509's criterion 3 asked for an item that makes the level-hosts decision *checkable*. This is
that item, and the mechanism is the boundary's own: **a binding added to `viewer_host::keys` fails
to compile in all three hosts.**

Each host carries `every_key_the_table_states_has_one_in_this_toolkit`. It walks `Key::ALL` through a
match that is exhaustive over the enumeration — so a new key is a compile error until that host says
which of its toolkit's keys produces it — and then asserts that the host's *runtime* translation
answers with the same key, so a key named in the test and forgotten in the translation fails rather
than drifting. Three hosts, three toolkits, one list.

`viewer-host` carries the table's own five tests: that `Key::ALL` is the whole enumeration, that no
key it names means nothing, the two `Mode` rows, Escape's ordering, and that Shift moves §12.5.1's
row and no other.

## 6. `viewer_host::NOTICE`, which came with `?` and is a licence obligation

`?` had to mean something in all three hosts, and what it means in `viewer-ui` is the card of
third-party notices — which exists because both licences covering the standard 14 font programs
(§9.6.2.2) require a **binary** distribution to reproduce their notices, and a command-line flag is a
poor answer for somebody looking at a window.

**`pdf-viewer-gtk` and `pdf-viewer-qt` compile in the same font programs and reproduced those notices
nowhere at all** — no card, no dialog, and not even a `--licences` flag. That is a licence obligation
missed rather than a feature gap, found by asking what a key should mean in the host that did not
bind it. The text is now one constant in `viewer-host`, because a notice that differs between two
binaries of one program is two claims about one obligation, and all three hosts put it on the screen:
a `GtkWindow` with a monospace `GtkTextView`, a `QDialog` with a read-only `QPlainTextEdit`, and the
card `viewer-ui` already drew. None of them re-wraps it, for the reason `viewer-ui` already stated:
a BSD licence's paragraphs and a font list's columns are laid out by the file's own line breaks, and
re-flowing text this program is obliged to reproduce would be editing it.

## 7. What asking "what should `?` do here" found, which was not a key defect at all

`?` had to be checked in all three hosts, and in `viewer-ui` it did nothing: the flag was set, the
card's display list was built for the frame, and the window kept the pixels it already had. It was
not this round's doing — the branch that toggles the card is what the old handler had, character for
character — and it was not the card's either. **Every overlay `viewer-ui` draws was unreachable on
the graphics device's path**: the find bar, the panel, §12.5.1's focus ring, the selection, and the
notices card.

`surface.rs`'s `on_the_device` decides whether the frame on hand will do, and the test read

> A rendering of exactly these lists at exactly these targets needs no successor.

— comparing the *pages* and their targets and nothing else. The chrome is drawn into the frame by
the render thread, so a window whose pages had not moved skipped the render and put up a frame drawn
with the *previous* chrome. Two lines fix it and both are in the diff: `Window` keeps the overlays
the last accepted job carried and `of_this_view` compares them, and `Plan::Render` arms the cadence
while a job is in flight — because a view that has not moved plans no approximation, so
`about_to_wait` would otherwise rest on `Wait` and the frame that was just asked for would sit in
the channel unread.

**Measured A/B in one sitting, on the same document under `Xvfb`, with `--trace=frames`.** An idle
window costs nothing: both builds draw their last frame at about 0.9 s and **zero frames after ten
seconds**, which is `doc/todo/36`'s fourth rule holding. Six chrome-changing key presses cost
**14 frames before and 28 after** — one render and one present per change, which is the feature
rather than an overhead. It cannot spin: the tick that collects the new frame finds the chrome
unchanged, asks for nothing, and stops arming.

The general lesson is trap 1's, one layer out from a rasteriser: **the instrument that says a change
happened is not the change.** `about.shown` was `true`, the display list existed, and the screen was
the only thing that could say so.

## 8. Two things this round deliberately did not do

**No new message, and it was checked rather than assumed.** Nothing here added a `Command`, an
`Event`, a `Query` or an `Answer` — the eighth time since the six-hundred-and-seventh session that a
feature landing in every host needed no channel. `viewer-qt` needed two new `QtUpdate` flags, which
is not a boundary change at all: `crate::bridge` states that C++ owns the `Host` and that Rust never
calls a Qt object, so a find bar and a notices dialog go out as flags with the shape `window` has had
since ADR 0470 and `clipboard` since ADR 0519.

**The C ABI gained nothing, and that is a decision rather than an omission.** A C caller owns its
keyboard by construction, so there is no table there to be out of step. What *could* be added is the
table as data — `pdfv_key_meaning`, so that a C host can bind the keys this program binds without
re-deriving them — and that is an addition to the ABI's surface, which is `doc/todo/30`'s **item 5**
and belongs to the round that takes it. It is written down there rather than left to be rediscovered.
