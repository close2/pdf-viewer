# 1275 — A document a reader names opens beside the one showing, and a tab says its title

Session 1219. Status: **accepted**.
Context: `crates/viewer-host/src/documents.rs` (`Named`, `Arriving`, `Arrivals`, `titled`),
`crates/viewer-host/src/policy.rs` (`open_chosen`), `crates/viewer-host/src/drawing.rs`
(`DrawRequest::document`, `Drawing::inside_document`), `crates/viewer-host/src/keys.rs`
(`WindowAct::OpenDocument`, `ctrl_meaning`), `crates/viewer-core/src/viewer.rs` (`Viewer::open`,
`Viewer::adopt`, `Viewer::rendered`), `crates/viewer-gtk/src/host.rs` and `src/bin/quorra-gtk.rs`,
`crates/viewer-qt/src/{host.rs,bridge.rs,bin/quorra-qt.rs}` and `cpp/{window.h,window.cpp}`,
`crates/viewer-ui/src/chrome.rs` (`FindBar::draw_labelled`),
`crates/viewer-ui/src/bin/quorra/{arrivals.rs,arguments.rs,app.rs,dispatch.rs,window.rs,overlays.rs}`.
Builds: ADR 1264 (the strip of tabs and what it left owed), ADR 1263 (`Viewer::adopt`), ADR 1240
(the chooser that spells a path and decides nothing), ADR 0809 (a document opened on disk).
Clauses: ISO 32000-2 §14.3.3 (Table 349), §7.9.2.2, §12.2 (Table 147's `/DisplayDocTitle`),
§7.6.4.1, §12.11.6, Annex O.

ADR 1264 built tabs and named three things a person still could not do: choose a file, name a
second one on a command line, and see a document's own title on its tab. All three are built. What
a later round should not re-open is below.

## 1. Every route ends in one function, and it is not `read_chosen`

A file dialogue, a typed path and a command line's later words are three ways a *reader* names a
document. All three end in `viewer_host::open_chosen`, which is the gate `CLAUDE.md`'s levels attach
to — `may_choose_file`'s argument for the other chooser (ADR 1240 section 1). It takes the path as
given, as `read_chosen` does and `read_import` does not, because the path is the reader's own. It
refuses what cannot be a document — nothing there, or not a regular file — by name. It has **no size
bound**, and that is the difference from `CHOSEN_FILE_LIMIT`: a form's file is read whole into
memory, while a document is opened on disk and read where its offsets point.

## 2. One at a time, and the command line's documents wait for the first frame

`viewer_host::Arrivals` holds what is still to open and starts one only when the one before it has
opened, failed or been declined. A document can ask a §7.6.4.1 password or a §12.11.6 question on its
way in, and a window asking two such questions at once asks a person to know which prompt is which.
Each document keeps its own prompt count on its `Arriving`, because the count in the window's fields
belongs to the one in front.

A command line's later paths start **at the first frame**, not when the first document opens:
`CLAUDE.md` section 2 says nothing page one does not need happens before page one. They open
*behind* the first, which is the document the launch was for. A first document that will draw no
frame starts them itself: it failed, it has no pages, or it was cancelled or declined. A file a
person chose comes to the front, because they asked to read it.

The next document never starts from inside the pump that settled the last. GTK uses an idle
callback, Qt a flag the C++ side reads with a zero-length `QTimer`, and `quorra` a flag its loop
reads in `about_to_wait`. Starting inside the pump would nest a second command loop in the first.

## 3. Every one is `Command::Open`, and `Viewer::open` asks `adopt`

A reserved name and `Command::Open` are the whole route. ADR 1263 found that the action path had lost
five of the reader's answers. `Viewer::open` had held its own copy of the list that `adopt` now
holds, which is two lists for one rule. It now calls `adopt`, so a host naming a file and a link
naming one hand a document the same answers. `a_second_document_a_host_names_gets_every_answer_the_first_did`
opens two documents after the answers are given and asserts all five on both.

## 4. A tab says Table 349's `/Title`, which is a choice the standard leaves open

No clause describes a tab strip. §12.2's `/DisplayDocTitle` is about the window's *title bar* and
names XMP's `dc:title`, and `quorra`'s title bar still obeys it. The tab uses §14.3.3's `/Title`
where the document states a text string with something to read in it, and the file's name
otherwise. The clause's own EXAMPLE has a dictionary holding "just the creation and last
modification date", so its tab is the file's name, and that is the fixture. The decoding is
`pdf_syntax::text_string` inside `pdf_model::metadata::Information`, so no second decoder exists.
The value comes from the `Query::Properties` answer each window already asks for its document
panel, so naming a tab costs no second decode of §14.3.2's stream.

## 5. What was found, most of it by driving the three windows

ADR 1264's tabs had been compiled and tested and never driven. Driving them with three documents
found six defects. Each is fixed, and each is a sentence here so that nobody re-argues it.

- **A page was keyed by its index alone, in two places.** `viewer_host::Drawing` replaced a queued
  request for "page 0" with the next one, whichever document it came from. `Viewer::rendered` looked
  for a token only in the focused document, although tokens come from one counter for all of them.
  So a document opened beside the first, then given back the front, never drew. A request is now
  (document, page). An answer is found in whichever document holds its token. A draw for a tab
  behind the front is not measured against the front's arrangement.
- **`quorra` kept one list of drawn pages for the window.** The core asks for a page once, so the
  list left in the window's fields was drawn under the next tab. The list now travels with the tab
  in `Showing`, and a page of a tab behind the front goes to that tab.
- **GTK spent Ctrl + Tab on moving the keyboard focus** before the window's own key controller saw
  it. One capture-phase controller now takes that one press.
- **GTK's `switch-page` idle carried a stale index.** Two switches inside one pump swapped two tabs
  back and forth for as long as the loop ran. A switch is now answered only when its index is still
  the notebook's current page.
- **GTK built a tab's panels before `Command::Focus` had run**, so they were filled from the
  document that had been in front. They are now built after the pump. Qt's behind case does the same.
- **`quorra` drew the strip of tabs over its find bar**, and so it would have covered the Ctrl + O
  line too. The strip is now drawn under the bar.

Reading the code found two more:

- **Three windows split a fragment three ways.** `quorra` asked the filesystem first, so a file
  called `a#b.pdf` opened. The native windows split at the `#` first, so it did not. The one reading
  is now `viewer_host::Named::from_argument`, and it is `quorra`'s.
- **`quorra` never asked for a tab's lists again.** The outline, the files, the threads and Table
  349 were taken once, when a document opened. `App::take_the_lists` now runs on every switch.

## Cost

No test drives a dialogue: the seam in front of each one is what is tested, which is ADR 1240's
cost again. Qt's `arrive` waits 50 ms and tries again when the host is held, which is a poll during
that wait.
