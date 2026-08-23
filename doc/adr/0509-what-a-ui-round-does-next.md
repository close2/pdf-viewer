# ADR 0509 — What a UI round does next, and the criterion that orders it

Status: accepted, 2026-08-23. Session 678, the first round taken on the project owner's *"we should
start investing time into the UI (and its API for the native versions)"*. Adds `tools/state.sh
hosts`; corrects two stale doc comments in `viewer-core`; writes the ordered reading into
`doc/todo/30`. **Decides nothing about any of the items and designs nothing** — it decides the
*order* and the criterion, so that the next three rounds can be decisive without re-surveying.

## 1. Why an ordering is the deliverable

`doc/todo/30` is a list of gaps and the owner asked for an *investment*. The difference is that a
list is taken in whatever order a round notices, and a round that notices from the file notices in
the order somebody wrote it down — which is how `viewer-gtk`'s `/TI` sat behind a diagnosis nobody
re-checked for seventy-seven sessions (ADR 0508). So this round surveyed the four consumers and the
boundary, and what it produces is a ranking with the reason attached to each place.

The criterion, stated once so that a later round can disagree with it rather than guess:

1. **What a reader can do with a document and cannot do here**, first — `CLAUDE.md`'s scope
   sentence is *"what a reader does with an open document"*, and a capability nobody has beats a
   capability two hosts out of three have.
2. **Then what costs no new message.** The boundary's proudest claim is that six consumers over
   sixty rounds have asked for one `Command` and one `Event` between them; an item that needs no
   channel is an item that is only work.
3. **Then what makes the level-hosts decision *checkable*.** "All three stay level" is a rule with
   no instrument, and a rule with no instrument decays the way a ledger row does.
4. **A toolkit floor is not a reason to rank an item last**, but it is a reason to write down what
   was actually tried. ADR 0508's whole lesson.

## 2. What the survey found, before the ranking

Four consumers, and they are not four of a kind. `viewer-ui` is tier 2 and draws its own chrome;
`viewer-gtk` and `viewer-qt` are tier 1 and place somebody else's widgets; `viewer-ffi` draws
nothing at all and hands a caller data. **"All three hosts stay level" therefore cannot mean one
thing**, and today it is false in both directions rather than in one:

- `viewer-ui` is *ahead* on everything that reads a document — six sidebar tabs against three,
  §12.3.4's thumbnails, §12.4.3's articles, §14.3.3's properties, §12.3.5's collection,
  §12.5.6.14's popup windows, the caret and in-field selection, §12.5.6.10's markup keys, and a
  copy key;
- and *behind* on everything that changes one — no undo or redo binding, no native form controls
  (it never sends `Command::Delegate`), and a password prompt that reads a line from **the
  terminal** and calls `std::process::exit(1)` if there is not one.

And the fourth consumer is not in the decision's sentence at all. `tools/state.sh hosts` is what
counts it, and the number is the finding: **every `Command` reaches the C ABI and twelve of
thirty-one `Query` variants do not.** `doc/ui-boundary.md` and `doc/todo/30` both say the ABI's
entry points are "the whole vocabulary", which was true when ADR 0346 wrote it and stopped being
true as the vocabulary grew — and the sharpest instance is a C caller that can start Annex O's
document-wide search with `pdfv_find_start` and cannot draw one match, because `Query::Find`'s
per-page geometry never crosses.

## 3. The order

**1. A selection that can leave the program.** All three windowed hosts draw a selection and none
of the three can give it to another application. `viewer-ui` has an in-process `String` and its own
comment says why — *"reaching the platform's [clipboard] … belongs to the platform, so `viewer-core`
has no command for it"*, and *"a native host embedding `viewer-core` owns that end by
construction"*. Neither native host took that end: there is no `gdk::Clipboard` in `viewer-gtk` and
no `QClipboard` anywhere in `viewer-qt`. This is first on criterion 1 without competition — a
viewer whose text cannot be copied is not a viewer somebody uses — and on criterion 2 it needs no
message at all: `Query::Selection` answers with the text and `Query::LogicalSelection` with
§14.8.2.5's reading order, which is the one to prefer and which already declines rather than
guessing where the structure tree does not reach every byte. The C ABI has `pdfv_selection_text`
already.

**2. A statement of what a key means, in `viewer-host`.** Four consumers, four key tables, and they
already disagree: `f` opens the find bar in GTK and arms §12.5.6.6's free-text drag in `viewer-ui`;
Up and Down scroll by 60 px in `viewer-ui` and turn the page in GTK and Qt; Escape clears the
selection in both native hosts and **quits the program** in `viewer-ui`. Nothing anywhere states
what a key means, so "the hosts stay level" is a rule about features with no purchase on the thing
a person actually touches. The argument for `viewer-host` is `Presenting`'s and `Clock`'s, verbatim
(ADRs 0470, 0473): *which sentence a window is obeying* is shared and `gdk::Key` against `Qt::Key`
against `winit::keyboard::Key` is what a toolkit is. This is the fourth application of that
argument and the first one that would be *checked* rather than agreed to — a table is a value, and
a test can read it.

**3. `viewer-ui`'s password prompt.** It is the only place in the tree where the program answers a
document by writing to `stderr` and reading `stdin`, and it exits the process when the terminal is
not there — so the tier-2 host cannot open an encrypted document from a desktop launcher at all,
while both native hosts have had a modal dialog since their first session. `viewer_ui::chrome`
already draws a modal card (the About panel), so this is drawing rather than architecture, and it
is the largest of the three ways `viewer-ui` is behind.

**4. The three panels the native hosts do not have.** Thumbnails (§12.3.4), articles (§12.4.3) and
document properties with §14.3.2's XMP beside them (§14.3.3) are drawn by `viewer-ui` and are
absent from GTK and Qt, each of which says so in a comment rather than by accident.
`Query::Thumbnail`, `Query::Articles` and `Query::Properties` all exist and all answer. This is the
plainest instance of the level-hosts debt and it costs no message; it is fourth rather than first
because it is a panel a reader can do without and item 1 is not.

**5. The C ABI's other half.** Twelve queries, counted by `tools/state.sh hosts` rather than
written here, and the two that matter most are `Query::Find` — a search a C caller can run and
cannot show — and `Query::Opening` with `Query::Preferences`, without which a C caller gets none of
Table 29's `/PageMode`, none of `/PageLayout` and none of Table 147 unless it re-derives them. Both
native hosts obey the catalogue on open; a C host cannot. The ABI's own protection is a *count of
event kinds*, which is the right instrument for the thing that arrives unasked and no instrument at
all for a question: **a `Query` added after the last sweep leaves a C caller with no symbol and no
signal**, which is exactly how twelve accumulated. The mechanism worth adding with the entry points
is the one that would have caught them — a test that enumerates `Query` and asserts an entry point
per variant, the same shape `PDFV_EVENT_KIND_COUNT` has for events.

**6. Undo and redo in `viewer-ui`.** Both native hosts bind `z` and `y`, `pdfv_undo` and
`pdfv_redo` exist, and `viewer-ui` names the two commands only in its trace formatter. A host that
can add an annotation and cannot take it back is the half-feature, and it is a keyboard binding
once item 2 exists.

**7. Table 233 bit 19's editable combo box in `viewer-gtk` — and this one is a real toolkit
floor.** *"If set, the combo box shall include an editable text box as well as a drop-down list"*;
`QComboBox::setEditable` obeys it and `GtkDropDown` is not editable, while `GtkComboBoxText` is
deprecated in the release this crate binds. `viewer-gtk` carries the flag and reports it, which is
the honest answer. It is written down here so that the next round does not spend itself
rediscovering it — **and with ADR 0508's rule attached: before writing that an item is blocked on a
toolkit, call the API the block names.** That entry cost seventy-seven sessions and the check cost
one screenshot.

## 4. Where the API forces a host into an awkward shape

Two, both concrete, neither an argument for changing the vocabulary today.

**The per-page answers are two shapes and a host learns which one variant at a time.**
`Answer::Reports`, `Answer::Readback` and `Answer::Accessibility` carry one entry per page with the
page named on it (ADR 0445), and `Answer::Frame` carries a `FrameView` apiece (ADR 0441); while
`Answer::Fields`, `Answer::Popups` and `Answer::Selected`'s quadrilaterals are flat lists with no
page anywhere on them. Both are right for what they carry — a quadrilateral already in the
viewport's own pixels needs no page, and a *report* about a page is meaningless without one — but
the enum does not say which rule it is following, and this round found the cost: `Query::Fields`
and `Answer::Fields` both said "the page being shown" while `Viewer::form_fields` had walked
`open.on_screen` since `Command::Layout` existed. The code was right and both comments were wrong,
which is the failure mode that costs a *host author* rather than a gate — a host reading the
documentation would have placed controls for one page of a column. Corrected in this round.

**`Command::Delegate` is a policy about a document and `Query::Fields` is an answer about a
screen**, and the pair is only safe because both now follow the arrangement. A host asks for the
page without widget appearances once, on open, and asks for fields per frame; if those two ever
disagree about which pages they cover, the result is a form with holes in it and no report anywhere
— the appearance is gone and the control was never built. That is not a defect today and it is the
kind of pairing worth a test rather than a comment.

## 5. What this ADR deliberately does not do

It designs no clipboard type, chooses no key table, and proposes no new message. Every item above
is stated as *what* and *why*; the *how* belongs to the round that takes it, which is the round with
the compiler and the screenshot. The one thing asked of that round is ADR 0508's rule: check the
API before writing down a blocker.
