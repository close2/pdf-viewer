# 1264 — A tab is a document, the window holds one of them, and a measurement is drawn

Session 1213. Status: **accepted**.
Context: `crates/viewer-host/src/documents.rs` (`Documents`, `Close`, `label`, `opened_beside`,
`closed`), `crates/viewer-host/src/keys.rs` (`WindowAct::NextDocument`,
`WindowAct::CloseDocument`, `ctrl_meaning`), `crates/viewer-host/src/restriction.rs`
(`Restrictions::depart`), `crates/viewer-gtk/src/host.rs` (`Showing`, `Ui::documents`,
`show_document`, `hold_the_view`, `opened_beside`, `close_document`, `Chrome::measuring`),
`crates/viewer-qt/src/host.rs` and `cpp/{window.h,window.cpp}` (`Showing`, `documents_`,
`syncDocuments`, `ChromeOverlay::measuring_`), `crates/viewer-qt/src/bridge.rs`,
`crates/viewer-ui/src/chrome.rs` (`DocumentStrip`),
`crates/viewer-ui/src/bin/quorra/{app.rs,window.rs,overlays.rs,dispatch.rs}`.
Builds: ADR 1263 (the name a second document opens under), ADR 1227 section 5, ADR 1145 (the
restriction menu's two scopes), ADR 1190 (what the fourth window owes), ADR 1191 (§12.9's mode and
what was left owed), ADR 1192 (a Control this program does not bind means nothing),
`doc/todo/30`'s "all three hosts stay level".
Clauses: ISO 32000-2 §12.6.4.3, §12.6.4.4, §12.9, Table 29 (`FullScreen`).

Two decisions, taken together because one round took them, and separate everywhere else.

## 1. What is shared about a strip of tabs, and what is a toolkit's

`viewer_core::Viewer` has held a `BTreeMap<DocumentId, Open>` and `Command::Focus` since it
existed; every window put one name in it. What a second one needs on this side is a very small
amount of state and a very exact set of rules about it, and those rules are identical in a
`gtk4::Notebook`, a `QTabWidget` and a strip of rectangles a program draws for itself. So they are
`viewer_host::documents` — this crate's own test applied again, the third copy being where two
hosts stop agreeing.

Three of its rules are decisions rather than mechanics:

- **One `T` per document, and exactly one of them is not parked.** The focused document's state
  lives in the host's own fields, because that is what every line of a window already reads;
  `Documents::focus` swaps the two in one move. The alternative — reading every per-document field
  through an accessor — was rejected on the diff rather than on taste: a field that must not leak
  between tabs then has to be got right at every call site, and here the whole of `T` travels or
  none of it does.
- **Names increase and are never reused.** `reserve` answers the next one, which is what ADR 1263's
  `Command::Beside` holds out. A name reserved and not used is skipped.
- **The last document is not closed here.** `Close::Last` says so and leaves the act to the host,
  because closing the last tab is closing the *window*, and `GtkWindow::close` against
  `QWidget::close` against an event loop that exits is what a toolkit is.

**The keyboard is two rows in `ctrl_meaning` and not one anywhere else**: Ctrl + Tab moves to the
next document and Ctrl + W closes the one in front, on ADR 1192's rule that a modifier this program
binds is one it has a row for. Both go away while a presentation is running, with the strip: Table
29's `FullScreen` shows "no menu bar, window controls, or any other window visible", and a key that
moves to a tab nobody can see is a key whose effect is invisible.

## 2. What travels with a tab, and what does not

The list was settled by asking of each field *is this about the file, or about the window*, and the
three hosts hold the same answer:

| travels with the document | stays with the window |
|---|---|
| the path, the directory, the bytes and Annex O's fragment | §12.7's controls, §12.5.6.14's popups and the placed pictures |
| the title bar's caption, and whether anything is unsaved | the find bar's string and a search in flight |
| §7.6.4.1's attempts, and whether the report is still owed | the draw this window has warned is taking too long |
| Table 29's arrangement (`Command::Layout` is the focused document's) | the restriction menu's *window* levels, `--links=`, `--remote-documents=`, §10.8.3, §6.3.2.2, Table 29's full screen, the panel |
| §12.9's mode and the points put down | |
| what this document departs from the window's restriction levels in | |

The last row is the one that needed a new method. `viewer_host::Restrictions` holds both scopes and
only the document half belongs to a file; `viewer_core` already keeps the departures beside the
document they are about (ADR 1145), so the menu's ticks move with the tab and **nothing is sent** on
a switch. `Restrictions::depart` is the setter that was missing.

Everything in the right-hand column is either rebuilt from a query on the repaint that follows a
switch, or is a host-supplied value `viewer_core` applies to every open document — so parking one
would be keeping a second answer to a question the core answers.

**The three hosts differ in how the swap is spelled, and that is a finding rather than a
compromise.** `viewer-gtk` and `viewer-qt` hold the set as one `Showing` field, because their host
structs are private. `viewer-ui`'s `App` fields are read by name from eight modules, so it keeps
them where they are and moves them through a `park`/`unpark` pair — a constructor and a
destructuring, so a field added to `Showing` fails to compile in both directions. What is level is
the behaviour and the bookkeeping; what differs is what a host is.

## 3. One view, moved, rather than a widget tree per document

Both toolkits want a separate child per tab and this program has one view — a splitter holding the
panels and the page. So every notebook page but the current one is an empty box, and the view is
moved into the page that becomes current. A tab change costs one allocation and happens when a
person clicks; keeping a panel tree, a page picture and a set of controls per document would cost
them all the time, and `CLAUDE.md` section 2 is why that is not a trade.

**The strip hides itself for one document**, in all three, so a window that opened one file is the
window it was — which is also what makes `launch_path` a proof rather than a hope: the one-document
open is the same code, under the same name, with the strip drawing nothing.

`viewer-ui` draws its own strip (`DocumentStrip`) across the top, where its find bar is, and hit
tests a press against it before the panel and the page. `quorra-confined` gains nothing at all, on
ADR 1190's rule: it offers no name, so `Command::Beside` never reaches it with one and a remote
go-to answers exactly as it did.

## 4. The rubber band, which round 1191 left owed

ADR 1191 built §12.9's measurement and recorded what was left: *no window draws the path it is
measuring, only the answer's sentence*. The points have been the host's since that round —
`viewer_host::Measuring` holds them and `Query::Measure` answers what they mean — so this needed no
message and no shared code, only three pictures: a stroked polyline through the points with each
press marked, because the path between two points is a straight line and a third point put down on
top of a second shows nothing otherwise.

It is drawn over everything else the page area holds, and the reason is the same in all three: every
other overlay is something the *document* or a search put there, and this is the one shape the
person is making. The colour is each platform's answer to a clause that states none — the theme's
foreground in GTK, `QPalette::Accent` in Qt, and a red `viewer-ui` writes down as a choice because it
has nobody to ask.

## 5. What this does not close

No window opens a document from a *person's* gesture — no file chooser, and no second path on the
command line. `doc/todo/30` carries both. A tab's label is the file's name and never the document's
own title (§14.3.3's `/Info /Title`), which `Documents::relabel` exists for and nothing calls yet.
