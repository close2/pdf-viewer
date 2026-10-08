# 1690 — A popup's right-to-left tabs reach leftward in both toolkits, from stops handed per width

Session 1427. Status: **accepted** and **built** in `quorra-gtk` and `quorra-qt`; a decimal stop in a
right-to-left paragraph **said** in both. Builds on ADRs 1679 (`quorra`'s leftward tabs), 1666 (a
popup's tab stops) and 0596 (trap 17). Context: ISO 32000-2 §12.5.6.2 (Table 172's `/RC`),
§12.7.4.3; XFA 3.3 chapter 2's *Tab Stops* (page 61) and chapter 27's (pages 1205 to 1207), held,
cited and never quoted.
Code: `crates/viewer-host/src/popup.rs` (`from_start_edge`, `toolkit_unapplied`);
`crates/viewer-gtk/src/host.rs` (`leftward_tabs`, `pango_tabs`' `from_right`);
`crates/viewer-qt/cpp/window.cpp` and `window.h` (`RichNoteView::handLeftwardTabs`, `resizeEvent`,
`tabs`). Tests: `viewer-host`'s `a_right_to_left_paragraphs_stops_are_handed_from_the_lines_start_edge`
and `a_decimal_tab_is_said_only_where_the_paragraph_reads_right_to_left`, `viewer-gtk`'s
`a_right_to_left_stop_reaches_pango_from_the_right_edge_by_its_name_there`; `tools/drive-windows.sh`
step 59 in the three windows.

## 1. What both toolkits do, measured

The chapter places a stop from the left margin and has a right-to-left paragraph's tab reach the
next stop on its left. Both toolkits measure a stop in a line read right to left from its start
edge, the right: a stop at 100 px in a line 400 px wide stood at 300 px from the left in Pango (a
layout, 1.58.2) and in Qt (a `QTextDocument`, 6.11.2), at 200 px in a line 300 px wide, and Qt
measures from the block's right margin where one is set (350 − 100 in a 400 px document with a
50 px margin). Both then reach leftward on their own: two stops at 100 and 200 px put the two words
after two tabs at 300 and 200 px. **So a stop is handed as its distance from the line's start edge,
nearest it first, and a stop at or right of that edge is dropped** (`viewer_host::popup::from_start_edge`),
which is the subtraction `quorra` already makes (ADR 1679) moved to the toolkit's own origin.

The sides are named differently. Pango names an alignment by the edge the text starts from: in the
right-to-left layout a `Left` stop put the right edge of the text after the tab at it, a `Right` one
its left edge — so `TabSide::Left` (the text's left edge at the stop) is Pango's `Right` and the
reverse. Qt names a kind by the edge as drawn: a `LeftTab` put the text's left edge
at the stop in both directions, so a side keeps its kind.

## 2. Per width, and why the note in `quorra-gtk` is a label under a ruler

The right edge is a number only once the toolkit has allocated the note. `RichNoteView` already laid
its document out at its width; it now keeps each right-to-left paragraph's stops and sets them again
in `resizeEvent` (and in `paintEvent`, should one come first) when the width changes.

A `GtkLabel` reports no allocation and `viewer-gtk` forbids the `unsafe` a subclass would take, so
the label is the main child of a `GtkOverlay` with a `GtkDrawingArea` over it that draws nothing,
takes no pointer, and is allocated the label's own size, which its `resize` reports. Two
arrangements were measured and refused on the way. The label as an overlay child over the drawing
area, the title bar's arrangement, is placed at its preferred size — a wrapped label's height at its
narrowest — and a one-line paragraph took 35 px of an 18 px overlay, its words drawn through the
next line (photographed). And a tab array set inside the allocation draws at once but its resize is
dropped: a paragraph the stops wrapped onto two lines stayed 18 px tall. So the stops are set when
the width differs from the last handed (`resize` comes on every allocation, the ones the change
causes among them) and the label is measured again once from the main loop.

## 3. A decimal stop, said

Both toolkits place a decimal stop's text in a right-to-left line as though what is stored before its
full stop stood on the stop's right, which holds for Hebrew letters and not for a number, read left
to right inside the paragraph: `123.45` at a decimal stop standing at 300 px put its full stop at
306 px in Pango and 304 px in Qt, where the same stop at 100 px in a left-to-right line put it at 98
and 100 px. The offset depends on the number's digits, so it is not handed as another kind of stop;
both windows say "a decimal tab in a right-to-left paragraph", which replaces "a tab in a
right-to-left paragraph" and fires only where such a paragraph holds a tab and states a decimal stop.
"A tab past the last stated stop" still fires on its own condition, whatever the direction.

## 4. A leader, still said, and the composition that would draw it

Neither API has a fill: Pango's `PangoTabArray` holds a position, an alignment and a decimal point
(`pango-tabs.h`, and no attribute in `pango-attributes.h` names a leader), and Qt's
`QTextOption::Tab` a position, a kind and a delimiter (`qtextoption.h`; `qtextformat.h` names no
leader either). Trap 17 says a catalogue is not the question, and it applies: both windows now paint
or can paint over the laid-out text — `RichNoteView` draws its document with a `QPainter` and
`QTextLine::cursorToX` gives each tab's extent; the ruler above can draw from the label's
`pango::Layout` at `layout_offsets` — so a leader is one composition away in both, and building it is
a host round's work of its own (each pattern, a drive step per toolkit). Until then it is said.
