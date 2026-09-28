# 1369 — An untagged page's fields are controls, and a relative path below the document is inside it

Session 1266. Status: **accepted** and built.
Context: `crates/viewer-core/src/viewer.rs` (`untagged_widgets`), `crates/viewer-core/src/query.rs`
(`PageStructure::widgets`), `crates/viewer-accessibility/src/tree.rs` (`untagged`, `Band::widgets`),
`crates/viewer-accessibility/src/reading.rs`, `crates/viewer-confined/src/{lib,protocol}.rs`,
`crates/pdf-model/src/view.rs` (`annotation_interacts`), `crates/viewer-host/src/policy.rs`
(`resolve_import`, `ImportRefusal::OutsideTheDocumentsDirectory`).
Amends: ADR 0214 (what an untagged page publishes), ADR 1357 section 4 (its cost), ADR 1155 and
ADR 1227 (the path rule, which stays one function for every purpose).
Clauses: ISO 32000-2 §7.11.2.1, §7.11.2.2, §12.5.1, §12.5.3, §12.7.4.2, Table 226, Table 31,
Table 368, §14.8.1, §14.9.1.

## 1. Untagged widgets: the reading

The brief pointed at §14.9.2 and §14.9.3 for what a processor owes an untagged document. They do
not say it: §14.9.2 is natural language and §14.9.3 alternate descriptions, and §14.9.1 lists
facilities "in support of accessibility" with no `shall` or `should` about a document without
structure. Table 368's "[i]n a tagged PDF, Form shall be used for each PDF widget annotation that
belongs to the real content of the document" binds the producer of a tagged file. So no clause
requires or forbids this; the decision rests on three others.

- §12.5.1: "[w]hen the user activates the annotation by clicking it, it exhibits its associated
  object". A field is interactive whether or not the page is tagged, and ADR 0425 made the click one
  definition for the mouse and the bus.
- Table 226's `/TU`: "[a]n alternative field name that shall be used in place of the actual field
  name wherever the field shall be identified in the user interface". A node is such a place.
- §12.5.1's `/Tabs` (Table 31) states an order for the page's annotations. Publishing the widgets
  in it invents nothing; ADR 0214's refusal was of an invented reading order for *text*.

## 2. Decisions

- `PageStructure::widgets` is a list apart from `nodes`, filled only where the document states no
  `/StructTreeRoot`; a tagged page reaches its widgets through §14.7.5.3. Each is a `Form` node with
  the widget's own control, `/TU` or the §12.7.4.2 name, its `/Rect` and its annotation. §12.5.3's
  Hidden, NoView and ReadOnly withhold a widget as they withhold a click.
- `viewer-accessibility` publishes them after the untagged sentence, built by the same walk as a
  `Form` element in an identifier band moved by 100 000, so they declare `Click` and
  `ScrollIntoView` for the same reasons.
- The census counts untagged pages with published widgets and the widgets (87 and 386, all in the
  tracked population); a tagged page answered with a list is a defect class held at zero. No
  existing floor moved: the honest-untagged count reads `nodes`, which is still empty.

## 3. The path rule

§7.11.2.2: "[a] file specification that does not begin with a SOLIDUS shall be a relative file
specification giving the location of the file relative to that of the PDF file containing it", and
EXAMPLE 1 resolves `ArtFiles/Figure1.pdf` into a subdirectory. The rule exists to keep a document to
its own directory, and a directory below it is inside it. `resolve_import` now splits by §7.11.2.1
(an escaped SOLIDUS is part of a component) and admits every component that is one normal path
component, refusing `..`, `.`, an empty component and an absolute path. It is still the one rule for
every purpose, so §12.7.6.4's import is widened the same way; ADR 1155's "nothing looser than the
import policy" holds because they are one function. Cost: a symbolic link inside that subtree can
point outside it, which was already true of a single file name.

## 4. Driven

GTK, Qt and `quorra` under Xvfb on a built untagged form: the page node holds the sentence, a
`button` named by `/TU` and an `entry`; `DoAction` turned the page in all three. `form_two_pages.pdf`
publishes its two text fields and its button. `issue17846.pdf` with its `/UF` file placed two
directories down opened beside the source at `--remote-documents=open`.
