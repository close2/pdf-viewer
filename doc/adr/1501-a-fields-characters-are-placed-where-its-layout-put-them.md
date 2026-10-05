# 1501 — A field's characters are placed where its layout put them

Session 1333. Status: **accepted**.
Context: `pdf-model`'s `variable_text` (`Asked::glyphs`, `Glyph`, `LaidOut::glyphs`, `comb_glyphs`),
`appearance::glyphs`, `view::ViewState::field_glyphs` and `view::FieldGlyph`;
`viewer_core::AccessibilityNode::value_lines` and `accessibility::value_lines`;
`viewer-confined`'s node encoding (greeting `PDFVCF06` → `PDFVCF07`);
`viewer_accessibility::tree::held`; `tools/drive-windows.sh` step `29-field-extents`. Amends ADR
1489 section 3, whose run carried a field's value and no character positions.

## 1. Why the positions are this tree's to give

ADR 1489 left a field's run without places on the argument that its characters are laid out in an
appearance stream rather than read back from a content stream. The argument holds and points the
other way: §12.7.4.3 has the processor "construct an appearance stream dynamically at rendering time", and
the translation of every `Tm` it writes is "positioning values it determines to be appropriate". The
layout that writes them is this tree's own, so where each glyph goes is a fact it already holds —
`variable_text` sums the same advances to place a caret and to shape a selection (ADRs 0211, 0225).

## 2. The construction

`Asked::glyphs` asks the walk that writes the stream for each glyph it placed: its line, the byte
range of the value it shows (from its code's byte to the next code's, so a character with no code
or a joined pair belongs to the glyph before it), and its box between the caret's descent and
ascent and as wide as its advance — a comb glyph as wide as its cell, which Table 231 bit 25 makes
its position. The boxes go through the `/DA`'s `Tm` (`Frame::place_quad`) and, in
`appearance::glyphs`, through the map `selection` takes onto the page — Table 192's `/R`, the
appearance's `/Matrix` and §12.5.5's algorithm onto `/Rect`. `ViewState::field_glyphs` answers by
widget, because an accessibility node holds the widget rather than a point. `viewer-core` maps each
glyph's box into device pixels with the rectangle mapping the node's bounds take
(`Viewer::device_rect`), and `accessibility::value_lines` groups them into `TextLine`s under the
lines' invariant, leaving out a glyph whose bytes are not whole characters of the value or that has
no place on the page. Asked only by the accessibility tree's two routes (an element's object
reference, an unreached widget), never by drawing.

## 3. Order, and what the run says

A line is in display order, left to right — UAX #9's rule L2 as the layout applied it (ADR 1413) —
so a right-to-left value's run holds its characters reversed, as a page's own readback holds a run
its producer stored for display (ADR 1465); `value` keeps the logical order. `tree::held` builds
the field's runs from `value_lines` with the paragraph's own `line_runs`, so `GetCharacterExtents`
and `GetOffsetAtPoint` answer on a field. Where the layout placed nothing (an empty field, a value
it refused) the run of ADR 1489 stands, with no positions.

## 4. Checked

`viewer-core`'s `untagged_widgets.rs`: "123" three boxes left to right, abutting, inside the
widget; U+05D0 U+05D1 U+05D2 in display order; a comb's three equal cells; a tagged `Form`
element's "Ada". `viewer-accessibility`'s `tree.rs`: the run's positions, widths and rectangle.
`accessibility_census` counts "field characters with extents", a floor new with this decision, and
names every filled field with none. The drive's `29-field-extents` asks AT-SPI for
`GetCharacterExtents(1)` on a field holding "123" in each window and checks the box lies inside the
field's own extents.
