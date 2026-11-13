# 2851 — The per-shape flag plan removes what its hidden shapes hide

Session 2016, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/in_place.rs`: `partition_flags`,
`shape_flags`; `archive/in_place/linked_popups.rs`: `prove`, `pair`).
Context: ADRs 1234, 2210, 2218; R16 of `doc/pdfa-blocking-issues.md`.
Clauses: ISO 19005-2 and ISO 19005-4 section 6.3.2; ISO 32000-2 §12.5.3 (Table 167), §12.5.6.14
(Table 186), §12.5.6.19 (Table 191), §12.7.2.

## 1. What the census said

`as-shown` answers `annotations/printable-and-visible` per object (ADR 2210): `hidden-at-rest` is
discarded, `visible-nonprinting` kept with section 6.3.2's flags. Six documents stopped there at
both parts on refusals `as-if-printed`'s site-wide removal does not make:

- a hidden widget (`160F-2019`, `annotation_hidden_noview`, `widget_hidden_print`): refused "could
  leave its field structure inconsistent";
- a hidden markup annotation with a closed popup whose own flags conform
  (`PDFBOX-6018-099267-p9-OrphanPopups`, `annotation_hidden_print`): ADR 2218 asked both members to
  be hidden, and a conforming popup is in no row, so it had no answer to give;
- a popup stating `Invisible` (`pr7352`): every shape with that bit set was stopped;
- `160F-2019`'s hidden widgets are kids of their fields, and their `/Parent` was read as a popup's.

## 2. The decision

- **A hidden widget leaves its page and its field stays.** §12.7.2: a terminal field "may have
  children that are widget annotations", which §12.5.6.19 makes the field's appearance on a page.
  The removal takes the widget out of `/Annots` and nothing else, as the site-wide removal does;
  the field keeps its value in the form.
- **A closed popup leaves with its hidden parent.** Table 186's `/Open`, false or absent, keeps it
  closed, and Table 167's Hidden on the parent says "do not render the annotation or allow it to
  interact with the user", so no reader shows it. Every other condition of ADR 2218's proof stands
  (sole reciprocal links, one page, no appearance, no actions); a popup a row names still needs its
  own hidden answer, and one kept visible refuses the pair. The report names the popup with its
  flags. A popup no page holds (`OrphanPopups`) has no `/Annots` entry to lose: the parent leaves
  alone, held to having no incoming link from any page annotation.
- **A widget's `/Parent` is its field, not a popup link.** §12.5.6.19's Table 191 makes it "[a]n
  indirect reference to the widget annotation's parent field"; the popup proof read it as Table
  186's and refused every hidden kid widget (`160F-2019`).
- **`Invisible` on a standard subtype is read as the bits beside it.** Table 167: it "[a]pplies only
  to annotations which do not belong to one of the standard annotation types". The subtypes are the
  ones ISO 19005-4 section 6.3.1 admits (Table 171 less `Sound`, `Screen`, `Movie`, whose removal
  is that section's). The kept annotation's `/F` is written with the bit clear, which changes
  nothing drawn or printed.

Under `as-shown`, both parts. No other profile states shapes; `as-if-printed`'s site-wide removal
already did all three. No loss is new: each is the `hidden-annotation` removal or ADR 1285's flags
the shape answers already carry, named per annotation in the report.

## 3. What it freed, and what it did not

`annotation_hidden_noview`, `widget_hidden_print`, `annotation_hidden_print`, `OrphanPopups` and
`pr7352` under `as-shown` at 2b and 4; `160F-2019` at both too (its push buttons are ADR 2751's).

## What it costs

A field whose only widget was hidden is no longer on any page; it was on none a reader saw.
