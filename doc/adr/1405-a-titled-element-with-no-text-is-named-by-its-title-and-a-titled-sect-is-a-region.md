# 1405 — An element with no text is named by its title, and a titled `Sect` is a region

Session 1284. Status: **accepted** and built.
Context: `crates/viewer-core/src/accessibility.rs` (`AccessibilityNode::titled`, `finish`, the walk's
`title`), `crates/viewer-accessibility/src/role.rs` (`map`), `crates/viewer-accessibility/src/tree.rs`,
`crates/viewer-confined/src/protocol/panels.rs` (the flag on the wire),
`crates/viewer-core/tests/{untagged_widgets,accessibility_census}.rs`,
`crates/viewer-accessibility/tests/tree.rs`.
Amends: ADR 1394 section 2's "`/T` is read only for a `Form`". Keeps its order.
Clauses: ISO 32000-2 §14.7.2 (Table 355), §14.8.4.4 (Table 365), §14.9.3.

## 1. The reading

Table 355 is the structure element dictionary, and its `/T` row is stated for every element: "The
title of the structure element, a text string representing it in human-readable form. The title
should characterise the specific structure element, such as Chapter 1, rather than merely a generic
element type, such as Chapter." The clause's own example is a section's title.

ADR 1394 read `/T` for a `Form` alone, on the argument that "no other type's content can leave it with
nothing to be called by". That is not so: §14.8.4.4's grouping types — `Part`, `Sect`, `Div`,
`Aside`, `NonStruct` — enclose "a grouping of structure elements", so their own content items are
usually none and they crossed AT-SPI named `''` whatever title the producer stated.

## 2. Decision

- The order stays ADR 1394's: `/Alt` or `/E` (a substitution), the element's own text, its `/T`, then
  for a `Form` the field's `/TU` and qualified name. `/T` is now read for every element. It names an
  element only where the element has no text of its own, and substitutes for nothing: a titled
  section's children are still published and still say what is in it.
- `AccessibilityNode::titled` says the name is the `/T`. A host cannot tell a title from a line of
  the element's own text by the string, and the role depends on it.
- **A titled `Sect` is `accesskit::Role::Region`**, which the AT-SPI adapter publishes as a landmark:
  §14.8.4.4 makes a `Sect` a grouping "with consideration for their hierarchy", and its title is the
  name that hierarchy is navigated by. This is a documented choice of the platform's word, not a
  reading: no clause maps a structure type onto a platform role. Not a heading: a heading role on a
  container would make everything in the section the heading's text. `Div` stays `Section` whatever it
  states, because Table 365 makes it "orthogonal to the semantic structure"; `Part` stays `Group`,
  "without consideration for their hierarchy". Both carry their title as their name.

## 3. Measured

The census counts elements named by their `/T` (`accessibility_census`, printed and floored).
Before this change the count outside `Form` was zero by construction — `/T` was not read there.
