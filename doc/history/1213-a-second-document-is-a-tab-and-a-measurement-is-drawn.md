# 1213 — A second document is a tab, and a measurement is drawn

The HOST-UI round of batch thirty-three.

## What was found

**ADR 1227 had put a claim about the *window* inside the reading of the *file*.** Table 203's
`/NewWindow true` was answered with "this view has one, so the remote document replaces what was
open", and the same ADR wrote down that a round building tabs would have to change it. The clause
was read right; the sentence was in the wrong crate.

**Table 204 is the stronger of the two entries, and the first design could not have reached it.**
§12.6.4.3's Table 203 states the `true` case with no modal verb at all; §12.6.4.4's Table 204 states
it with a `should`. Carrying the name on `Command::Supply` — the variant-shape mechanism
`doc/ui-boundary.md` prefers to a message — would have answered only the weaker one, because an
embedded go-to reaches its target inside the document already open and asks no host for anything.

**A document reached through an action has never had this reader's answers.** `resume_remote` and
`jump_into` build their replacement with `Open::around`, which is not `Viewer::open`, so §8.10.4's
target documents, §8.11.4.4's audience, Table 166's clock, §10.8.3's simulation and §12.4.4's
presentation were all lost on that path. Each is documented as applying to every open document.

**Round 1177 left the rubber band owed and nothing could see it.** No gate covers what a window
draws for itself, and the sentence in that round's own record was the only instrument.

## What was built

`Command::Beside(Option<DocumentId>)` in the core — a name a host has free for a document opened
beside the one showing, read against both tables, consumed when one opens under it — with
`Outcome::beside`, `Viewer::adopt`, command kind 35 on the confined wire and `quorra_beside` on the
C ABI. `viewer_host::documents` is the bookkeeping three windows share, `Restrictions::depart` the
setter the per-document menu scope needed, and `ctrl_meaning` gained Ctrl + Tab and Ctrl + W.
`viewer-gtk` holds a `gtk4::Notebook`, `viewer-qt` a `QTabWidget` and `viewer-ui` a strip it draws
itself, each with one view moved into whichever tab is in front and each hiding the strip for a
single document. And §12.9's traced path is drawn over the page in all three.

**Left owed**: no window opens a file a *person* chose — no chooser, and no second path on a
command line — so the only route to a second tab is an action; a tab says the file's name and never
§14.3.3's `/Info /Title`; and no window was driven by hand, only compiled and tested.
