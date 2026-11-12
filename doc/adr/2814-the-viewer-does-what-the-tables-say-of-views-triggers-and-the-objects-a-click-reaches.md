# 2814 — The viewer does what the tables say of views, triggers and the objects a click reaches

Round 2000, the audit-fix round (helper VW, viewer behaviour).
Status: **accepted and built** in `viewer-core` (`open/destinations.rs`, `viewer/outcomes.rs`,
`viewer/navigation.rs`, `viewer/layout.rs`, `viewer/answers.rs`, `viewer.rs`, `interact.rs`, `open.rs`), `pdf-model`
(`action.rs`, `link.rs`, `view/annotations.rs`, `view/script_sites.rs`, `popup.rs`, `attachment.rs`), `pdf-signature`
(`signature.rs`), `viewer-host` (`documents.rs`, `panel.rs`), `viewer-gtk` (`tree.rs`) and `viewer-qt`
(`bridge.rs`, `host/printing.rs`, `cpp/window.cpp`).
It answers the viewer-behaviour findings of `doc/reviews/1983-specification-audit.md` and supersedes, each for the point
named below and nothing else: ADR 0162 (the `/FitBH` and `/FitBV` coordinate), ADR 0164 (`/PV` and `/PI` riding with
the current page), ADR 0177 (a link's active area), ADR 0181 (the `/SigFlags` gate), ADR 0244 (the expansion cap),
ADR 0295 (an empty `/Contents`), ADR 1180 (the first `/PrintPageRange` pair only), ADR 1602 section 4 (the order
between a script and a non-script action of one chain) and ADR 1723 (a window of its own for text notes only).
Clauses: ISO 32000-2 §7.7.2 (Table 29), §12.3.2.2 (Table 149), §12.3.3, §12.2 (Table 147), §12.4.2 (Table 161),
§12.5.6.4, §12.5.6.5 (Table 176), §12.5.6.7, §12.5.6.8, §12.5.6.10, §12.5.6.12, §12.5.6.13, §12.5.6.15, §12.6.2
(Table 196), §12.6.3 (Table 197), §12.7.3 (Table 225), §12.8.5.2.

The owner's instruction of 2026-11-11 — "Please use a round to fix all findings" — is the decision; this record is
the reading under each change.

## 1. A destination's coordinates are default user space, and a null keeps the coordinate

Table 149 calls `/FitBH`'s operand "the vertical coordinate top" and `/FitBV`'s "the horizontal coordinate left", in
the words it uses for `/FitH` and `/FitV`; only the magnification is the bounding box's. ADR 0162 classed the two as
"already page space" and skipped the page's transform, so a crop box not at the origin moved the view. Both now go
through the transform; only `/Fit`'s and `/FitB`'s corners, which the viewer works out itself in the display list's
space, do not.

"A null value for any of the parameters left, top, or zoom specifies that the current value of that parameter shall
be retained unchanged." The parameter is the coordinate, not the pixel scroll: a zoom change moves every pixel, and a
page turn reset the scroll to the top before the view was applied. `Open::retain_corner` reads the point of default
user space at the window's corner before a turn that carries a view, and `apply_view` fills each null from it (or
from the current scroll, read before the zoom changes, on the same page).

## 2. Table 29's `/OpenAction` action is performed

Table 29: "A value specifying a destination that shall be displayed or an action that shall be performed when the
document is opened." Only a go-to action's destination was taken. `interact::open_action` now performs the rest of the
chain at the first present, after the open sequence's scripts, as a link's chain is performed at a click; the root
go-to stays the view the document opened to, and the chain's ECMAScript actions stay the open sequence's.

## 3. `/PV` and `/PI` are the window's visibility

Table 197: `/PV` "when the page containing the annotation becomes visible", `/PI` "when the page containing the
annotation is no longer visible in the interactive PDF processor's user interface". ADR 0164 derived both from the
current page, which in a two-page or continuous layout raises `/PV` late for a neighbour and `/PI` for a page still
on the screen. `Viewer::visibility` now diffs the arrangement's pages against `Open::visible` at every settle and
raises the two for what joined and what left; `/PO` and `/PC` stay the current page's. **Limit kept**: the *scripts*
of `/PV` and `/PI` still ride with the current page's open and close (`ViewState::run_page_scripts`), because the
open sequence runs page one's before any visibility has been settled; a neighbour's `/PV` script waits for its page to
become current.

## 4. One chain, one order

Table 196: `/Next` is "an array of action dictionaries that shall be performed in order". ADR 1602 section 4 ran a
chain's scripts and its other actions in two passes and named the order between them as "the one thing not kept".
`ViewState::perform_annotation_chain` runs each script where its `JavaScript` refusal stands in
`pdf_model::action::read`'s list, performing the actions between; a link's click goes through it. Where the two walks
disagree on how many scripts the chain holds (each has its own bound) the scripts run first, as before.

The same table makes an array that names one action twice two performances. §12.6.2 NOTE 1's "self-referential
actions ought not be executed more than once" is a cycle — an action reached again through its own `/Next` — so both
walks now keep the *path* rather than every object seen, and `MAX_VISITS` bounds the occurrences a nested repetition
could otherwise multiply.

## 5. What a click reaches

Table 176's `/QuadPoints` "comprise the region in which the link should be activated". ADR 0177 routed every
annotation by `/Rect`, so a press inside a link's rectangle and outside its quadrilaterals pressed the link.
`view::annotations::active_area_covers` takes the link's region (`link::region_covers`, the one `link::at` uses) and
`/Rect` for every other subtype.

§12.5.6.7, §12.5.6.8, §12.5.6.10, §12.5.6.12 and §12.5.6.13 each say "When opened, it shall display a popup window
containing the text of the associated note" (or "they shall"); Table 172 makes `/Popup` optional. ADR 1723 gave a
window of its own to `/Text` only; `popup::opens_a_window` names all of them, and one stating `/Popup` is still shown
in that one.

§12.5.6.15: "Interactive PDF processors shall use this entry rather than the optional Desc entry". ADR 0295 used
`/Contents` only where it was non-empty; present is the condition, and an empty one leaves `/Desc` unshown (a host
shows the type and size in the line it leaves empty).

## 6. Labels, signatures, outlines and print ranges

Table 161: "(If the P entry is also missing or empty, the page label is an empty string.)" The viewer answered no
label for an empty one, and a host then wrote its own "Page n". The empty label is now the label; the title bar shows
the index alone beside it.

Table 225 bit 1: "If set, the document contains at least one signature field." ADR 0181 read the converse — a clear or
absent bit as "no signature fields" — and skipped the walk; the clause states no converse, and §12.8.5.2 says "The
existence of one or more document timestamps shall be determined by examining signature fields". The walk is made
whatever the flag says; it runs off the first frame.

§12.3.3 lets a document ask for an outline item to open. The GTK sidebar stopped opening rows at 4096 (ADR 0244's
small-document evidence cannot speak for that boundary); the cap is gone, and the walk ends because
`pdf_model::outline` reads at most `MAX_ITEMS`.

Table 147's `/PrintPageRange` is pairs, several where the document states several. The Qt bridge forwarded the first
pair only (ADR 1180); it now forwards all of them as a `QPageRanges`, and prints the pages the dialogue settled on,
gaps included.
