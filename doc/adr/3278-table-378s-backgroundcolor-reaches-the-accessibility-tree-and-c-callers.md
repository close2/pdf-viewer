# 3278 — Table 378's `/BackgroundColor` reaches the accessibility tree, and C callers get both

Session 2144. Status: accepted. Rows: §14.8.5.4.2 (`implemented`; `/BackgroundColor` moves from
inapplicable to exported). Context: ADRs 2384, 3177, 3191; `doc/questions/A634`.

## Why it is in scope

A634 settles it without a question. The export takes what a platform counterpart carries and what
a sighted reader sees drawn. `accesskit` 0.24.1 has `background_color`, and
`accesskit_atspi_common` publishes it as AT-SPI's `bg-color` text attribute. The fill is drawn on
the page by the producer's content, so it is seen.

## The reading

ISO 32000-2 §14.8.5.4.2, Table 378: the colour "to be used to fill the background of a table cell or
any element's content rectangle", "not inheritable", and "If this attribute is not specified, the
element shall be treated as if its background were transparent."

`Tree::background_colour` reads the element's own statement under §14.8.5.4.1's owners
(`layout_attribute`). A value that is not three numbers in `0.0..=1.0` states no colour, as
`/TextDecorationColor`'s does. The shared reader is `inline::rgb_colour`.

**What reaches the text is a choice, and it is written down.** The entry is not inheritable, so no
descendant holds the value. A descendant that states none is transparent, though, and a
transparent element shows what is behind it. §14.8.3's block layout lays a child inside its
parent's content rectangle. So `viewer_core` carries, behind each element's own text, the
element's own fill or else the nearest one stated above it. The case this decides is the common
one: a paragraph inside a coloured `TD` (the Tagged PDF Best Practice Guide's example for the
entry) is read on the cell's colour. Where nothing on the way up states one, nothing is set: the
page shows through, and the page's colour is no attribute's.

## Where it goes

- `AccessibilityNode::background`, carried across the confined protocol after the alignment
  (`encode_drawn_as`, one shared `read_colour` that holds the range on the untrusted side).
- `viewer_accessibility` sets it on the element's runs, where `bg-color` is read, and not on the
  node, as the alignment and the decoration are set (ADR 3191).
- The C ABI: `quorra_structure_layout(structure, entry, node, &alignment, background,
  &has_background)` answers Table 379's alignment (`QUORRA_ALIGNMENT_*`, carrying ADR 3191's
  alignment to C callers, which round 2122 left) and this background in one call. It is a new
  function and not a wider `quorra_structure_style`, so `QUORRA_ABI_VERSION` does not move, as
  ADR 2422 argued. The entry-point count goes from 384 to 385. The C# declaration and constants
  are compiled only by CI's Windows job; `header_and_interop_agree` compares them with the
  header here.
