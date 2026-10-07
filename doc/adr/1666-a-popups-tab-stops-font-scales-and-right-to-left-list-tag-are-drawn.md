# 1666 — A popup's tab stops, its font scales in Qt, and a right-to-left list item's tag are drawn

Status: accepted and **built**. Session 1415.
Builds on ADR 1654 (a run's face, order, spacing and scales), ADR 1642 (a popup window draws its
rich text in each host's toolkit), ADR 1634 (chapter 27's tab stops in a field's appearance).
Code: `crates/pdf-model/src/popup/rich.rs` (`RichParagraph::tab_interval`, `tab_stops`,
`RichTabStop`, `RichTabAlign`; a tab is a `'\t'` in a run's text); `crates/viewer-host/src/popup.rs`
(`tab_stops`, `TabSide`, `TabStop`, `advances`, `tabbed`, `tabs_unapplied`, `toolkit_unapplied`'s
`scales`, `scaled_face`); `crates/viewer-ui/src/chrome/rich.rs` (`tab_advance`, `lines`, the tag's
side); `crates/viewer-gtk/src/host.rs` (`pango_tabs`, `pango_space`, the isolated tag);
`crates/viewer-qt/src/host.rs` (`rich_paragraphs`, `rich_run`), `cpp/window.cpp` (`RichNoteView`);
`crates/viewer-confined/src/protocol/panels.rs` (the wire, greeting `PDFVCF09`). Tests: `viewer-ui`'s
`panel.rs` (`a_tab_advances_to_the_paragraphs_stop`, `a_right_to_left_list_items_tag_is_at_its_right`),
`viewer-host`'s `popup::tests`; `tools/drive-windows.sh` steps 54 to 57.

## 1. Tab stops

Chapter 27's *Tab Stops* (pages 1205 to 1207, brought in by §12.7.4.3) was read for a field's
appearance (ADR 1634) and said in a popup ("a tab stop in a popup window"), the tab drawn as a
space. It now crosses: a paragraph carries its `tab-interval` and its stated stops, and each
`xfa-tab-count` is that many `'\t'` in the run's characters — white space resolution leaves none of
the string's own, so the character is unambiguous. `viewer_host::popup::tab_stops` is the one
reading the three windows share: the stated stops in order of position, then a default stop every
interval past the last stated, aligned `after`; `after` and `before` are turned into a side by the
paragraph's direction. `quorra` advances a tab to the first stop past its place on the line, the
text after it up to the next tab standing there by the stop's side (a decimal stop at the first
full stop, a field's choice); Pango is handed a `PangoTabArray` (1.50's alignments, so `viewer-gtk` names `pango`
directly, with its `v1_50` feature, where it reached it through `gtk4` — the same crate at the same
version), and Qt
`QTextBlockFormat::setTabPositions`. Positions are from the paragraph's left margin, which in the
two toolkits is the label's start, a list tag inside it; `quorra` measures from the same place.

**The chapter sets no stop where nothing states one, so a tab there advances by nothing**, as a
field's layout reads it; both toolkits put default stops of their own, so a paragraph that states
neither property is handed to them with its tab characters taken out — exact. **Two cases are
said rather than drawn.** A paragraph that states stops and no interval: past its last stated stop
the chapter sets none and each toolkit puts a stop of its own; where a tab reaches that far is the
toolkit's line, so the two toolkit windows name the case ("a tab past the
last stated stop") rather than a count. And a tab in a paragraph read right to left, whose stops
chapter 2 has the text reach leftward (page 61): no window lays that out, so all three say it.
A leader is drawn by none of the three and is said ("a tab leader in a popup window").

## 2. Font scales in `quorra-qt`, and why not in `quorra-gtk`

Qt's CSS reader has no `font-stretch` (ADR 1654 section 4), so the window no longer hands Qt markup
for the note: `RichNoteView` builds a `QTextDocument` through `QTextCursor`, run by run, every
property a number the host computed, and draws it at its width. A run's face is set at its size
times its vertical scale and `QFont::setStretch` at the horizontal over the vertical
(`viewer_host::popup::scaled_face`) — the face pinned by its style name first, because Qt otherwise
matches a stretch below 100 to a condensed design of the family, which step 55 measured at 83 px
against 74 before the pin and 72 after — so a glyph is `em × vertical` tall and advances
`width × em × horizontal` — the reading `quorra` and a field's appearance draw by; its letter spacing
is multiplied by the horizontal scale, as §9.4.4's `Th` multiplies `Tc`. A spacing given as a share of
a space is measured in the face Qt picked (`QFontMetricsF`), and in `quorra-gtk` in the face Pango
picked (a layout of one space), so neither toolkit says it any more. The thread's replies stay Qt
rich text, as before.

**Pango states no per-run glyph scale, and this is reported, not worked around.** Its attribute
list has a uniform `scale`, a `font_stretch` that chooses a face of another width rather than
scaling one, and `font_scale` for superscripts; a matrix is the whole layout's. Drawing each run as
an image or a layout of its own with a transform would take the line breaking away from Pango,
which is the reason the window is a label. So `quorra-gtk` says "a font scale in a popup window"
under the note, and the drive's steps 54 and 55 are not offered there for that reason.

## 3. A list item's tag at the start edge

A tag is chapter 27's generated number, set as one left-to-right label outside the paragraph's
order (ADR 1654 section 3). It now stands at the paragraph's start edge — the right for one UAX #9
reads right to left — with the list indent taken from that edge. `quorra` places it there; the two
toolkits wrap it in UAX #9's LRI and PDI, so it reads as it is written and the paragraph's direction
is found past it, and `quorra-gtk` takes the indent as an end margin, `quorra-qt` as a right one on a
right-to-left block.

## 4. The wire

`RichParagraph` widened, so the confined greeting moved to `PDFVCF09`.
