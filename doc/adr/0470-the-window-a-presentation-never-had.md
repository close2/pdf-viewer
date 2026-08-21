# ADR 0470 — The window a presentation never had

Status: accepted, 2026-08-21. Session 638.

## Context

Since the hundred-and-fiftieth session `viewer-core` has advanced a slide show, since the
three-hundred-and-ninety-third it has shaped §12.4.4.1's transition frames (ADR 0230), and since
the four-hundred-and-eighty-first it has kept §12.4.4.2's presentation *mode*, because that
subclause conditions a state machine on it (ADR 0316). What no host had was the **window**: a
presentation played inside a sidebar, a tool bar and a status line. `doc/todo/32` said so in one
line — "[f]ull screen. Chrome, and therefore the host's" — and ADR 0316 said it twice, both times
on the same reasoning: "§12.4.4.1 says a processor 'may allow a document to be displayed in the
form of a presentation or slide show' and says nothing about a window".

**That reasoning was right about §12.4.4 and wrong about the standard.** The window is specified;
it is specified in two other clauses; and one of those two states it as a `shall`-flavoured
obligation on the document being *opened*.

## What the standard states, and where

Read verbatim against `doc/md/ISO_32000-2_sponsored_EC3.md` and the standard's own PDF.

**Table 29 (§7.7.2), `/PageMode`** — "[a] name object specifying how the document shall be
displayed when opened", one of whose six names is

> FullScreen Full-screen mode, with no menu bar, window controls, or any other window visible

**Table 147 (§12.2)**, the same subject in the smaller — three flags, each with the default
`false`:

> HideToolbar … A flag specifying whether to hide the interactive PDF processor's tool bars when
> the document is active.

> HideMenubar … A flag specifying whether to hide the interactive PDF processor's menu bar when
> the document is active.

> HideWindowUI … A flag specifying whether to hide user interface elements in the document's
> window (such as scroll bars and navigation controls), leaving only the document's contents
> displayed.

**And Table 147's `/NonFullScreenPageMode`**, which is the way back out and which nothing in this
tree had ever acted on:

> The document's page mode, specifying how to display the document on exiting full-screen mode:
> … This entry is meaningful only if the value of the PageMode entry in the catalog dictionary
> (see 7.7.2, "Document catalog dictionary") is FullScreen; it shall be ignored otherwise.
> Default value: UseNone.

So the standard says four things about what a full-screen window *shows* and one about what
happens when it stops, and it says nothing at all about how it starts, how it ends, what the
pointer does, or whether a click advances. §12.4.4 supplies the timing, the transitions and the
states; these two clauses supply the window. Between them there is nothing left to invent except
the gestures.

## The decision

**`viewer_host::presentation` is where full screen is decided, and all three hosts adopt it.**

`Presenting` is one value per open document: the chrome §12.2 permits, `/NonFullScreenPageMode`
where Table 147's condition makes it meaningful, and whether the window is full screen now.
`Chrome` is four booleans for the four sentences above, stated as *permissions* — `true` means no
clause asks for this to be hidden — so a host hides what it is told to and never widens the
answer. `Presenting::mode` is what a host sends as `Command::Present`.

**It is in `viewer-host` rather than in a host for the reason [`arrangement`] is** (ADR 0246, ADR
0442): the toolkit call differs in all three — `GtkWindow::fullscreen`, `QWidget::showFullScreen`,
`winit`'s `set_fullscreen` — and *which sentence is in force* does not. The third copy of a
decision is where two hosts stop agreeing.

**And it needed no new message on the boundary**, which is `doc/ui-boundary.md`'s claim tested
rather than repeated. `Command::Present(PresentationMode)` already exists and already means this;
`Query::Opening` already answers Table 29's `/PageMode`; `Query::Preferences` already answers
Table 147 whole — the second and third since the hundred-and-thirty-seventh session, added
because a *clause* needed a channel. Nothing was added and no variant changed shape, which is the
fifth round since the six-hundred-and-seventh to add nothing at all.

## What is chosen rather than derived, and why each way

- **Entering full screen enters §12.4.4's presentation mode, and leaving leaves it.** The standard
  describes exactly one full-screen mode and exactly one presentation mode and never distinguishes
  a window in the first from a window in the second. A reader who asked for a slide show and got a
  window with a sidebar in it has been given neither, and a document that asked to open full
  screen has asked for the thing §12.4.4 is about. One key — `p` — in all three hosts.
- **Escape leaves.** No clause states how full screen *ends*; it states only what is displayed
  afterwards. `CLAUDE.md` principle 3 is the argument for choosing a key at all rather than none:
  a document that could put a reader in full screen and keep them there would be imposing a
  restriction the reader cannot switch off. In `viewer-ui` this displaces Escape's other meaning —
  it exits the program — only while a presentation is running.
- **A click does not advance the page and the pointer is not hidden.** Both are conventions of
  other slide shows and neither is in the standard. Taking the click would *remove* two things the
  standard does define for a page being displayed — §12.5.6.5's link activation and §12.4.2's
  selection — in order to add one it does not.
- **Where `/NonFullScreenPageMode` is not meaningful, a host puts back what the reader had.** The
  entry is ignored unless the catalog asked to open full screen, so leaving a full screen the
  *reader* asked for is not that entry's subject; `Presenting::on_exit` answers `None` there and
  says so, rather than substituting `UseNone`, which is Table 147's default for a question nobody
  asked.
- **A document asking to open full screen gets it.** Table 29 says "shall be displayed when
  opened" and the other five names in that column have been obeyed here since the
  hundred-and-seventieth session and the two-hundred-and-sixty-sixth. Obeying the sixth is
  consistency rather than a new policy, and Escape is what makes it safe.

## The erratum the reading found

`cargo run --release -p spec-errata -- emit doc/*.pdf` reports, under §12.2, an insertion by
**issue #275** (state Review/Completed) whose text is `UseAttachments (PDF 2.0) Attachments panel
visible`. The caret's own rectangle — read out of the annotation dictionary and compared against
the page's text boxes — sits at the start of the line beginning *This entry is meaningful only
if*, which is the line immediately after `UseOC`'s in **`/NonFullScreenPageMode`'s own cell**.

`pdf_model::viewer_preferences` refused that name there, with the comment "`UseAttachments` is a
name this entry does not define". It is defined; the printed table is short by one row. The reader
now accepts five of Table 29's six for that entry and refuses only `FullScreen`, whose refusal was
right and stays: a window cannot exit full-screen mode into full-screen mode, and Table 147 makes
that name the entry's own condition rather than one of its values.

It is not a curiosity. `/NonFullScreenPageMode /UseAttachments` is the value the round's fixture
states, and all three hosts were driven leaving full screen onto §7.11.4's attachments panel — so
the erratum decides what a person sees.

## What each host does now

| | full screen | `/HideToolbar` | `/HideWindowUI` | `/HideMenubar` | "any other window" |
|---|---|---|---|---|---|
| `viewer-ui` | `set_fullscreen(Borderless)` | reported: it has none | reported: it has none | reported: it has none | the sidebar, the About card, the find bar |
| `viewer-gtk` | `fullscreen()`, `set_decorated(false)` | the header bar's buttons and the `GtkSearchBar` | the status line and its separator | reported: it has none | the `GtkNotebook`, taken out of the `GtkPaned` |
| `viewer-qt` | `showFullScreen()` | the navigate and find `QToolBar`s | `statusBar()` | a `QMenuBar` if one is ever added | the `QTabWidget` |

Two host-side notes worth keeping:

- **GTK's header bar is the tool bar *and* the window controls in one widget**, so `/HideToolbar`
  hides the four buttons and not the bar: a document asking for no tool bar has not asked a reader
  to lose their close button. Full screen takes the whole titlebar, because Table 29's sentence
  names the window controls separately.
- **The `GtkNotebook` is removed from the `GtkPaned` rather than hidden**, because a `GtkPaned`
  carries a `position` that was set and a hidden start child leaves that space standing.

## Verified on screen

Under `Xvfb :78` at 1200×900 with `lavapipe`, on a fixture written by
`pdf-model/examples/presentation_fixture --opens-full-screen` — Table 29's `/PageMode
/FullScreen`, `/HideToolbar true`, `/HideWindowUI true`, `/NonFullScreenPageMode /UseAttachments`.

`viewer-gtk` and `viewer-qt` both open presenting: the photograph is the page and
`pdf_render::SURROUND` edge to edge, with no panel, no tool bar, no status line and no titlebar.
Pressing `p` brings all of it back **on the Files tab**, which is `/NonFullScreenPageMode` and
therefore the erratum, on the screen. `viewer-ui` prints the same sequence and opens the Files tab
after Escape, with §8.11's bullets put back — NOTE 2's restore, visible.

**Two things this machine cannot show, and both are the instrument rather than the code.** There
is **no window manager**: `mutter` here is Wayland-only, and full screen on X11 is a request to a
window manager (`_NET_WM_STATE_FULLSCREEN`), so the window stays the size it was and only the
chrome can be photographed. And **`xwd` returns stale content for a window that has not
repainted** — several captures were byte-identical across a rebuild and across two GTK renderers,
and changed only after a key press. Both are recorded in `doc/environment.md`, because both cost
this round time.

## Consequences

- §12.2 goes from a clause read and handed over to one two hosts *obey*: `/HideToolbar` and
  `/HideWindowUI` change what is on the screen now, and `/NonFullScreenPageMode` decides a panel.
- Table 29's `/PageMode` is obeyed for five of its six names in `viewer-ui` and for four in the
  two native hosts, which have no §12.3.4 thumbnails tab; the sixth and fifth are said out loud.
- `doc/todo/32`'s one non-transition item is closed. What that file still owes is the five styles
  ADR 0230 refused, unchanged by this round.
- `viewer-gtk` and `viewer-qt` gained `Command::Present` and therefore §12.4.4.2's state machine.
  What they still do not have is §12.4.4.1's *clock* and its transition frames, which are
  `viewer-ui`'s alone — `doc/todo/32` carries it as the next item under "all three hosts stay
  level".

[`arrangement`]: ../../crates/viewer-host/src/arrangement.rs
