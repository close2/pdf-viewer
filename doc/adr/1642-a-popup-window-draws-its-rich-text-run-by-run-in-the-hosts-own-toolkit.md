# 1642 — A popup window draws its rich text run by run, in each host's own toolkit

Status: accepted and **built**. Session 1403.
Builds on ADR 1634 (the subset of XFA 3.3's chapter 27 this tree reads), ADR 1635 (which entry
wins where two disagree), ADR 0191 (this program draws a popup window), ADR 0613 (the window is a
host's, shared through `viewer_host::popup`), ADR 1466 (the window's paper and edge).
Code: `crates/pdf-model/src/popup/rich.rs` (`RichNote`, `RichParagraph`, `RichRun`, `Measure`,
`RichAlign`), `popup.rs` (`Popup::rich`, `Comment::rich`), `rich_text.rs` (the `parts` re-export);
`crates/viewer-core/src/query.rs` (`PopupWindow::rich`); `crates/viewer-confined/src/protocol/panels.rs`
(the wire); `crates/viewer-host/src/popup.rs` (`size`, `rise`, `family`, `not_drawn`, `html`,
`thread_html`, `hex`); `crates/viewer-ui/src/chrome.rs` (`draw_rich`); `crates/viewer-gtk/src/host.rs`
(`rich_note`, `pango_span`); `crates/viewer-qt` (`QtPopup::rich`, `rich_thread`, `not_drawn`).
Tests: `pdf-model`'s `popup::tests::a_rich_note_*`, `viewer-host`'s `popup::tests`, `viewer-ui`'s
`panel.rs::a_rich_note_is_drawn_in_the_colour_its_run_states`, the confined round trip,
`tools/drive-windows.sh` step `50-popup-rich`.

## 1. What the clause asks, and whose it is

Table 172's `/RC` is "[a] rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA)
Specification, version 3.3 ) that shall be displayed in the popup window when the annotation is
opened". §12.5.6.14 gives the popup "no appearance stream", so the window is a host's and there is
no content stream to lay the string into: what crosses the boundary is the string *read* — by the
same `rich_text` walk and cascade a field's `/RV` takes — as paragraphs of runs, each run carrying
its characters, `font-family`'s search path, its size, weight, posture, colour, underlines, line
through and rise, and each paragraph its alignment, list level and tag. Where a line breaks and
which machine face answers a family is the toolkit's; what the producer specified about each
character is `pdf-model`'s.

## 2. Which entry is drawn

`/RC` is drawn where its characters are `/Contents`'s, or where there is no `/Contents`; where the
two disagree, `/Contents` — Table 166's "[t]ext that shall be displayed for the annotation" — is
the window's text, plain. That is ADR 1635's free text rule, reached through the same
`rich_text::for_free_text` so the two cannot come apart; §12.5.6.2's NOTE 1 expects the two to be
textually equivalent, so a file that breaks it is the file's. `/RC` and `/DS` are read from the
group source, because §12.5.6.2 lists "Contents (or RC and DS )" among the group attributes.

## 3. Sizes, and the two bounds

§12.5.6.4 lets the processor choose "a font and size" for a note's window, so a run's relative
sizes are multiples of the host's own text size and its absolute ones are points
(`pdf_model::popup::Measure`, `rich_text`'s own split). **A choice**: `viewer_host::popup::size`
holds a run between half and three times the base, so a `sub` of a `sub` stays legible and a
`72pt` heading stays inside the document's rectangle; chapter 27 states no bound. `quorra` takes a
point as CSS2's reference pixel, 96 to the inch, which is what the two toolkits take it to be.

## 4. What each window does not draw, and says

The runs' `letter-spacing`, chapter 27's two font scales and its tab stops are not carried — no
toolkit label takes them — and `RichNote::unapplied` names each beside what `rich_text` itself
does not apply; every window says the list under the note (`viewer_host::popup::not_drawn`).
`quorra` draws in its own Helvetica and names each other face the note asks for; it draws a note
holding a right-to-left character plain, saying so, because the order UAX #9 gives a paragraph
across runs is not laid out here (ADR 1634 section 6 names the same residue for a field).

## 5. The document's text never becomes markup

Pango and Qt each take a markup string. Every character of the note is escaped and every attribute
is one this program writes: a size and a rise as numbers, a colour as six hex digits, a family only
where every character is a letter, digit, space, hyphen or low line — so a name holding a quote
cannot end its attribute and open one the document wrote. Qt's labels are `Qt::RichText` with links
and text interaction off; `a_rich_note_reaches_qt_as_escaped_text_in_spans_this_program_wrote`
plants an `<img>` and a family carrying `color:red` and finds neither.
