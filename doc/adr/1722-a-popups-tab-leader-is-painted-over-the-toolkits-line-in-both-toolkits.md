# 1722 — A popup's tab leader is painted over the toolkit's own line, in both toolkits

Session 1439. Status: **accepted** and **built** in `quorra-gtk` and `quorra-qt`; the C ABI still
**says** it. Builds on ADRs 1690 §4 (the composition this builds), 1679 (`quorra`'s leaders), 1660
(a field's leader pieces) and 0596 (trap 17). Context: ISO 32000-2 §12.5.6.2 (Table 172's `/RC`),
§12.7.4.3; XFA 3.3 chapter 2's *Tab Stops* and *Tab Leader Pattern* (pages 61 to 65), held, cited
and never quoted.
Code: `crates/viewer-host/src/popup.rs` (`reached_stop`, `leader_cycles`, `rule_thickness`,
`rule_pieces`; `toolkit_unapplied` no longer says a leader, `leader_unapplied` is the C ABI's);
`crates/viewer-ui/src/chrome/rich.rs` (`draw_leader` on the shared grid);
`crates/viewer-gtk/src/host.rs` (`measured_label`, `Leaders`, `tab_runs`, `paint_leaders`);
`crates/viewer-qt/src/bridge.rs` and `host.rs` (`QtTab`'s leader fields, `qt_tab`, four bridged
functions); `crates/viewer-qt/cpp/window.cpp` and `window.h` (`RichNoteView::paintLeaders`).
Tests: `viewer-host`'s `a_tab_reaches_the_nearest_stop_beyond_it_in_its_paragraphs_direction`,
`a_leaders_cycles_lie_on_the_margins_grid_and_a_partial_one_is_blank`,
`a_rule_leader_is_drawn_in_its_styles_pieces`; `tools/drive-windows.sh` step 62 in the three windows.

## 1. The composition, one per toolkit

Neither tab API has a fill (ADR 1690 §4), so the leader is drawn over the text each toolkit laid
out, from the toolkit's own answer for where the tab is:

- **GTK**: the paragraph's label is the main child of a `GtkOverlay` and a `GtkDrawingArea` of its
  size lies over it — the arrangement ADR 1690 built for a right-to-left paragraph, now taken by
  every tabbed paragraph one of whose stops states a leader. Its draw function reads the label's own
  `pango::Layout`: `index_to_pos` gives each tab's extent, `index_to_line_x` and the layout's
  iterator its line's baseline, and `layout_offsets` where the layout stands in the label. Dots and
  content are a layout of the tab's run's own Pango span (its face, size, rise, letter spacing and
  colour; no underline), appended once per repetition to a `GtkSnapshot` whose render node draws
  onto the area's cairo context — `gtk4` binds no Pango-on-cairo call, and a node draws a layout
  as GTK's own text nodes do, with no new dependency.
- **Qt**: `RichNoteView` already paints its `QTextDocument` with a `QPainter`; after the document,
  each tab of a paragraph with a leader is found in its block's `QTextLayout`, its extent is
  `QTextLine::cursorToX` on either side of the tab character, its baseline the line's `y` plus its
  ascent, and the face and colour the fragment's `QTextCharFormat`. A stop carries its leader across
  the bridge in `QtTab`.

## 2. What is shared, so that one leader is not drawn three ways

**The stop a tab reached is asked of its extent, not counted**: the nearest stop beyond the tab's
start in the direction the paragraph reads (`viewer_host::popup::reached_stop`), which is chapter 2's
rule and the one both toolkits lay a tab out by — a run longer than the room before a stop carries
its tab on to the next one, which counting tabs would get wrong. The repetitions' grid from the
paragraph's left margin, the partial repetition left blank, the dashed and dotted rules' pieces and
a rule's thickness where none is stated are `quorra`'s (ADR 1679), moved to `viewer_host::popup` so
that `quorra`, the GTK host and — through four bridged functions — the C++ side all draw by them.

## 3. Which toolkit layout gives the extent, and what is still said

Both do, for every tab: Pango's `index_to_pos` and Qt's `cursorToX` answer for a tab character as
for any other, in either direction, so no toolkit window says "a tab leader in a popup window" any
more and `toolkit_unapplied` has lost the sentence. Measured by step 62 (16 pt runs, stops 200
points from the margin, 267 px at 96 to the inch): the dots, the rule and the content reach 256, 264
and 258 px from their H in `quorra`, 259, 264 and 249 in `quorra-gtk` and 260, 265 and 261 in
`quorra-qt`, each ending short of the blue H at the stop; a right-to-left paragraph's dots reach
leftward 354, 246 and 255 px (the toolkit windows are narrower). Photographed in all three: the
content leader repeats fewer plus signs in `quorra-gtk` because its run is set in the machine's
face, whose `+` is wider than `quorra`'s — the run's own face, which is the rule.

The C ABI hands a stop's position and alignment and not its fill, so a note it hands over still carries it (`leader_unapplied`, ADR 1679). A tab
that reaches no stop takes the toolkit's own default advance and has no leader, which "a tab past
the last stated stop" already says.

Not measured, and left as it is: Qt's rise (super- and subscript) is a vertical alignment the
format states and not a length, so a raised run's dots stand on the line's baseline in `quorra-qt`;
GTK's piece carries the run's own `rise`.
