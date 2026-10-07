# 1654 — A popup's runs are each set in their own face, in one paragraph's order, spaced and scaled

Status: accepted and **built**. Session 1409.
Builds on ADR 1642 (a popup window draws its rich text run by run, in each host's toolkit), ADR 0133
(the chrome's faces are the binary's), ADR 1413 and ADR 1417 (UAX #9 for text this program lays
out), ADR 1634 section 2 (CSS2's generic families named as §9.6.2.2's faces).
Code: `crates/pdf-model/src/popup/rich.rs` (`RichRun::letter_spacing`, `horizontal_scale`,
`vertical_scale`, `RichSpacing`); `crates/viewer-host/src/popup.rs` (`letter_spacing`, `scaled`,
`toolkit_unapplied`); `crates/viewer-ui/src/chrome/rich.rs` (the layout), `chrome.rs` (`Family`,
`face_of`, `glyph`, `draw_set`); `crates/viewer-gtk/src/host.rs` (`pango_span`'s `letter_spacing`);
`crates/viewer-qt/src/host.rs`; `crates/viewer-confined/src/protocol/panels.rs` (the wire, greeting
`PDFVCF08`). Tests: `viewer-ui`'s `panel.rs` (`a_rich_run_is_set_in_the_face_its_family_names`,
`a_right_to_left_paragraph_is_ordered_across_its_runs`,
`a_runs_letter_spacing_and_horizontal_scale_are_drawn`), `chrome::rich::tests`, `viewer-host`'s
`popup::tests`, the confined round trip; `tools/drive-windows.sh` steps 51 to 54.

## 1. What ADR 1642 left, and why each is a host's

Table 172's `/RC` "shall be displayed in the popup window when the annotation is opened", and
ADR 1642 drew it in three windows with three things said rather than drawn: `quorra` set every run
in its own Helvetica, `quorra` drew a note holding a right-to-left character plain, and no window
carried a run's `letter-spacing` or chapter 27's two font scales. All three are now carried: the
run states them (`pdf_model::popup::RichRun` gained the spacing, in the unit it was stated in, and
both scales), and each window draws what its toolkit can and says the rest.

## 2. A face per run in `quorra`

`quorra` has no toolkit to ask for a family by name, and its chrome's rule is that what it draws
comes from the binary (ADR 0133). So a run's `font-family` search path is walked to the first name
that is one of §9.6.2.2's five families or a name `pdf_font::substitute` treats as metric-compatible
with one (Arial, Times New Roman, Courier New, a PostScript name's `MT`/`PS` set aside), and the run
is set in that family's compiled-in face in its weight and posture. **A path naming none of them is
set in the family `pdf_font::substitute::Request::derive` classifies its first name into** — the
reading a field's appearance falls back to on a machine with no fonts — and nothing is said: CSS2
section 15.5's matching ends in a face of the user agent's choosing when no family matches, so the
property has been carried out, not dropped. The sentence `quorra` used to say ("the face …") is
gone. The other faces load on first use, so the launch path parses none.

## 3. One paragraph, one order

The runs' characters are one paragraph to UAX #9: levels are resolved over the whole of it (rules
P2 and P3 find its direction from its first strong character, whichever run holds it), joining is
over the whole of it, and each laid-out line is ordered by rules L1 and L2 glyph by glyph, every
glyph keeping its run's style; rule L4 mirrors. **A choice**: a paragraph whose direction is right
to left and that states no `text-align` begins at the right — its own start edge. Qt puts an
unaligned block there by itself, so `viewer_host::popup::html` writes no `align` for a paragraph that
states none; a GTK label of one line stays where its `xalign` puts it, so `quorra-gtk` sets that from
`viewer_host::popup::right_to_left`, the same reading `quorra` lays the paragraph out by. Each was
looked at on the drive's step 52 photograph. A list
tag is chapter 27's generated number and is set as one left-to-right label at the paragraph's left,
outside the order.

## 4. Spacing and scales

`quorra` applies both as `pdf_model::rich_text::lay_out` applies them to a field: a glyph advances
`(width × em + spacing) × horizontal scale`, §9.4.4's `(w0 × Tfs + Tc) × Th` with chapter 27's two in
place of `Tc` and `Th`, and is drawn `em × vertical scale` tall. A spacing given as a percentage is of
a space in the run's face (chapter 27, page 1204), which only the host that picks the face knows,
so it crosses as a share. **A choice**: a spacing is held within the run's own em, as ADR 1642 holds
a rise, so a spacing of a page's width does not put one letter in the window and the rest outside.

Pango's markup takes `letter_spacing` and Qt's rich text `letter-spacing`, so both toolkit windows
draw a length — each in its reference pixel, 96 to the inch: Pango's unit is the layout's logical
pixel and not the point its documentation names, which step 53 measured (eight points spaced a gap
six points wide until it was converted). **Neither states a glyph scale** — Pango's span attributes have none (its
`font_stretch` chooses a face's width, it does not scale one) and Qt's CSS parser has no
`font-stretch` (`QCss::Property` lists `LetterSpacing` and `WordSpacing` and no stretch). Qt's
`QTextCharFormat::setFontStretch` would scale, but only on a document built format by format rather
than from the markup its label is handed; that construction is the route, not taken here. So a scale,
and a spacing given as a share of a space, are said under the note in those two windows
(`viewer_host::popup::toolkit_unapplied`), and the drive's step 54 is not offered there.

## 5. The wire

`RichRun` widened, so the confined greeting moved to `PDFVCF08`: a host and a worker from builds
either side of this would read a run's fields out of step.
