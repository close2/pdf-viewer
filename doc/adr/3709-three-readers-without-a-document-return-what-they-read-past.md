# 3709 — Three readers without a document return what they read past, or decline

Session 2252, the silent-repair campaign's census burn-down, fifth pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3574 (the census and its `HELD` ratchet), ADR 3621 (a repair made
without a document is returned to the reader that holds one), ADR 3669 and ADR 3691 (the list this
takes up), ADR 1197 and ADR 1660 (rich text and its list tags); ISO 32000-2 §7.4.9, §12.7.4.3,
§12.10.3 and §12.10.4.

## Context

Round 2247 left three files whose readers hold no document and returned nothing of what they read
past: the rich-text markup reader (a string that is not XML, an `ol start` or `li value` that is no
number, a list tag a style could not write), the JPEG 2000 header reader (a Cross-Reference or
Codestream Registration box short of its fields, a codestream short of a length) and the Well Known
Text reader (a nameless system, datum or unit, an EPSG code that is no number, an absent
`Auxiliary_Sphere_Type`).

## Decision

- **Rich text** (§12.7.4.3, XFA 3.3 chapter 27). `markup::parse_noting` returns the walk's repairs
  beside the text, as `text_string_noting` does. Two classes: `model/rich-text-malformed`, a `/RV`
  or `/RC` that is not well-formed XML (the plain text is drawn instead, ADR 1197), and
  `model/rich-text-attribute-unusable`, an `ol start`, `li value` or `xfa:spec` read as absent.
  `rich_text::for_field` and `for_free_text` record them at `Site::WholeFile`; a `/V` that only
  looks like markup records the second class alone where it is drawn as rich text, because a `/V`
  that does not parse is plain text, and an imported FDF `/RV` records nothing on the document.
- **A list tag outside its style's range is not a repair; it was a defect.** ADR 1660 sends a
  number outside a style's range to its fallback (section 8.5 of the 2011 draft), `decimal` for
  the Latin, Greek and Roman styles; those three answered no tag and the item was drawn with none.
  They now fall back, and `ListStyle::tag` returns a `String`.
- **JPEG 2000 declines rather than repairs** (§7.4.9, ISO/IEC 15444-1 Annex I, T.801 M.11). The
  module's own contract is that "a box this module does know but that is too short for its own
  fields is an error"; a short `cref` or `creg` box is now `HeaderError::ShortBox`, where it was
  read as empty. A series too short for `Qmcc` carries `Collection::count` as `None`, as `kind` and
  `reversible` already did, and section 6.2.8.3's row compares it with `Some(1)`. A codestream that
  ends inside a marker's length or `Psot` ends the walk there, which reads nothing in its place.
  No class is added: what this reader cannot read is the validator's finding already.
- **Well Known Text** (§12.10.3 and §12.10.4, ISO 19162). `system::from_wkt_noting` returns three
  classes: `model/wkt-name-absent` (the name is the empty string), `model/wkt-identifier-unusable`
  (the component is known by its name) and `model/wkt-auxiliary-sphere-type-absent` (type 0, the
  only one `resolve` admits). `measurement.rs` records them where it reads a coordinate system
  dictionary's `/WKT`, which parses the string once more there: the system itself is read again
  wherever it is asked for, and the record must be made where the document is held. A nameless
  unit is no longer named `metre`; `metre` is ISO 19162 section 7.4.4's default only where no unit
  is stated.
- **Mapping and the converter's answer.** All five classes map to section 5.1 alone. Each is
  outside content: the whole-file rewrite carries the string as the file states it, and the
  output's own check refuses by the class's name where it reads the entry again (ADR 3621's rule).

## Consequences

- `HELD`: `rich_text/markup.rs`, `jpeg2000.rs` and `geospatial/system.rs` 6 → 0 each.
- A JPX image with a short `cref` or `creg` box now fails `Headers::parse`, which the validator
  reports and the converter's JPEG 2000 rewrite declines, where both read past it before.
