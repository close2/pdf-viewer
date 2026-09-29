# 1394 — A `Form` element with no text is named by its title, then by its field

Session 1278. Status: **accepted** and built.
Context: `crates/viewer-core/src/accessibility.rs` (`Gathered::title`, `Readback::fields`, `finish`),
`crates/viewer-core/src/viewer.rs` (`referenced_objects`, `Referenced::fields`),
`crates/viewer-core/tests/{untagged_widgets,accessibility_census}.rs`.
Clauses: ISO 32000-2 §14.8.4.7.2, §14.9.3, §14.9.5, §12.7.4.2, Table 226, Table 355.

## 1. The reading

§14.8.4.7.2's `Form` is "Either an association between content enclosed by the Form structure
element and a corresponding widget annotation or a mechanism to include a widget annotation in the
structure tree". In the second shape the element's only content item is §14.7.5.3's `/OBJR`, so its
own text is empty and it crossed AT-SPI named `''`: all 272 corpus `Form` elements did.

Four names are stated for it, in two places:
- On the element (Table 355): `/Alt`, "An alternative description of the structure element and its
  children in human-readable form", which §14.9.3 makes "a complete (or whole) word or phrase
  substitution for the current element"; `/E` likewise for an abbreviation (§14.9.5); `/T`, "The
  title of the structure element, a text string representing it in human-readable form";
  `/ActualText`, a replacement for enclosed *content*, which an `/OBJR` alone does not have.
- On the field: §14.9.3, "An alternative name may be specified for an interactive form field (see
  12.7, "Forms") which, if present, shall be used in place of the actual field name when an
  interactive PDF processor identifies the field in a user-interface" — Table 226's `/TU` — else
  §12.7.4.2's fully qualified name.

## 2. Decision

`/Alt` or `/E` first, as before, and as a substitution. Otherwise the element's own text; where that
is empty, the element's `/T` (the producer named this element), then the field's `/TU`, then its
qualified name — `pdf_model::view::FieldName::shown`. None of the last three is a substitution, so
nothing below the element is withheld. `/T` is read only for a `Form` (§14.7.3's mapped type): no
other type's content can leave it with nothing to be called by, and naming a `Sect` by its title is a
separate question this does not answer. The widget's `/Contents`, which §14.9.3 offers for an
annotation "that does not already have a text representation", is not consulted: the field's name is
the one §14.9.3 says a user interface shall show for a field.

## 3. Measured

`Form` elements naming a widget: 272 tracked, a new floor; crossing with no name: 272 before, 0
after, held at zero by the census.
