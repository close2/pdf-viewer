# 2384 — Table 380's decoration and position reach the accessibility tree on the element's own text

Session 1775, slot 5 of batch 117. Status: accepted, built. Context: `doc/questions/Q634`, `A634`;
review 1768's finding 8; ADRs 0301, 0394, 0716, 1535; ISO 32000-2 §14.8.4.1, §14.8.5.3,
§14.8.5.4.1, §14.8.5.4.4. ADR 2385 is unused.

## Decision

§14.8.5.4.4's row stood `inapplicable` on "exports to no format", and `viewer_core`'s accessibility
tree is an export: §14.8.5.4.1's NOTE 1 names the use, to "export it to some other document format
with at least basic styling preserved", and the platform's text interface is such a format. A634
adopts Q634's recommendation, so Table 380's entries that a sighted reader sees drawn cross to it.

- **Read once, ranked once.** `pdf_model::structure::Tree::inline_statements` reads an element's
  attribute objects once and ranks `/TextDecorationType`, `/TextDecorationColor`, `/BaselineShift`
  and `/TextPosition` by §14.8.5.3's first three priorities. A name a cell does not list is the
  entry's default and a colour outside "the range 0.0 to 1.0" is unstated — `Tree::writing_mode`'s
  reading of the same kind of cell.
- **Inheritance is the walk's, because two of the four depend on §14.8.4.1's category**, which for
  a contextual type is settled by where it sits. `viewer_core::accessibility::Inline::enter`
  applies each cell's reach: `/TextDecorationType` "inheritable only for directly nested ILSEs" and
  not to "nested BLSEs"; `/TextDecorationColor` and `/TextPosition` plainly inheritable;
  `/BaselineShift` summed along the chain of inline-level elements and reset at any other kind,
  because a child's shift is "measured relative to the shifted baseline" of its parent and a block
  moves its whole line, leaving nothing on it to be raised above. An element of no standard type has
  no category and is never inline.
- **A `Figure`, `Form` or `Formula` resets the sum as a block does.** §14.8.5.4.6 gives their shift
  its own meaning — "the value of its BaselineShift attribute shall be used to determine the position
  of its after edge relative to the text baseline" — which is where an inline illustration's bottom
  sits and not a subscript of any text drawn inside it. The rule is the clause's and no
  measurement's: the 266 elements of `doc/PDF32000_2008.pdf` stating `/BaselineShift` on their own
  attribute objects are all `Figure`s placed `Inline`, 265 of them lowered, and none draws text, so
  the census's counts are the same with the rule and without it. That document's 1 352 lowered and
  49 raised elements are `Span`s whose class states the shift — Table 86's `x00 y00` indices among them.
- **A summed shift's sign is a superscript or a subscript — a choice, not a derivation.** The cell
  states a distance; "[t]he shifted element may be a superscript, a subscript, or an inline graphic".
  Positive is "toward the before edge", which is what a platform's superscript means by position,
  and only text runs carry it; the inline graphic is the next item's. **The nearest
  statement wins**: an element's own `/TextPosition` outranks its own shift, because it names the
  position the shift only measures, and an element's own shift outranks a `/TextPosition` it
  inherited.
- **On the runs, never on the node.** `viewer_accessibility::tree` sets `accesskit`'s underline,
  overline, strikethrough and vertical offset on each of the element's own `Role::TextRun`s.
  `accesskit_consumer` inherits all four down its own tree, so a paragraph's underline set on its
  node would reach the block nested inside it, which the cell forbids.
- **An unstated decoration colour crosses as opaque black — a choice with its cost.**
  `accesskit::TextDecoration` cannot be built without a colour and the entry's default, "the current
  fill colour in effect at the start of its associated content", is not in the readback.
  `AccessibilityNode::decoration_colour` keeps `None`, so the model says the truth and the black is
  the bridge's alone. No adapter this tree links reads it: `accesskit_atspi_common` 0.19.1 publishes
  an underline as `single` and a line-through as `true`.
- **`/TextPosition` is built although the brief listed it among the three left.** The brief's reason
  was "no platform counterpart"; `accesskit` 0.24.1's `VerticalOffset` is its counterpart word for
  word — "Position is elevated, like for superscript" against `Superscript` — and it decides the
  reading where a file states it, so leaving it would have kept the guess where the file answers.
- **The confined window's wire carries the three fields**, held to their closed sets and the colour
  to its range on the host's side.

## What stays unread, and why

`/LineHeight`, `/GlyphOrientationVertical`, `/TextDecorationThickness`, `/RubyAlign` and
`/RubyPosition`: each parameterises how a line, a glyph, a rule or a ruby is set, `accesskit` 0.24.1
has no property for any of them, and each serves the reflowing view A634 keeps out as a layout
engine. The ruby *roles* already cross (`RB`'s `Ruby` and `RT`'s `RubyAnnotation`, `role.rs`).

**AT-SPI shows two of the four today.** `accesskit_atspi_common` 0.19.1's text attributes are
`underline` and `strikethrough` among ten; it has none for an overline or a vertical offset, so those
two are in the tree for an adapter that reads them and absent from the Linux bus. That is the
bridge's gap, recorded rather than worked around.

## Cost

The walk reads each element's attribute objects once more. The tree is built when a host asks
`Query::AccessibilityTree`, after page one's frame, so the launch path does not run it; the
round's A/B of `launch_path` says so in its record.
