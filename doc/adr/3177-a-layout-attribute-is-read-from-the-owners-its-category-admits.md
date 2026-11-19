# 3177 — A layout attribute is read from the owners its category admits, and the other 31 `inapplicable` rows hold

Session 2119. Status: accepted. Rows: §14.8.5.4.1 (`inapplicable` to `implemented`); §14.8.3.2,
§14.8.5.4.6 and §14.8.5.4.7 (`inapplicable`, reasons restated); the other 28 `inapplicable` rows
re-read and unchanged. Context: ADRs 1535, 2384, 3171; `doc/questions/A634`; review 1768.

## The re-reading

With no row `partial` (ADR 3171), each `inapplicable` row was read against its clause and the titles
around it in `doc/md/`, asking whether something a reader does with the feature is in scope.

| rows | holds because | verdict |
|---|---|---|
| §10.6.1–§10.6.4 | §10.6.1: "Halftoning is not required for such devices" — every raster this program transmits, screen, print or `render` export, is continuous-tone; no route writes a bilevel or few-level spool | confirmed |
| §14.2 | procedure sets "shall be used only when the content stream is printed to a PostScript language compatible output device"; no output is one | confirmed |
| §14.10 and its 21 subrows; §14.11.4, §14.11.6.1, §14.11.6.3, §14.11.7 | "deprecated with PDF 2.0", which §3.15 defines as what "should be ignored by a PDF processor"; `/PZ`, the one permission addressed to a viewer, stays declined (ADR 1172) | confirmed |
| §14.12.4.2 | DPM is "application-specific information communicated between the creator of PDF data and a receiving system", a production workflow's | confirmed |
| §14.8.3.2, §14.8.5.4.6, §14.8.5.4.7 | each serves a processor making its own layout decisions; the reasons said "exports to no format", which A634's accessibility tree made false, and now say what that export does and does not carry (`accesskit` has no indent and no text-column property; an illustration's `/BaselineShift` is never read as text shift). A reflowing view would flip them, and A634 keeps it out | confirmed |
| §14.8.5.4.1 | see below | built |

## Decision

§14.8.5.4.1's sentence — layout attributes "shall be defined in attribute objects whose O (owner)
entry has the value Layout or whose owner is any other owner excluding List, Table, PrintField and
Artifact" — is read on the reader's side, as §14.8.5.6's and §14.8.5.8's parallel sentences already
were (`Tree::print_field`'s claim aside, below). The owner "determines the interpretation of the
attributes defined in the object" (§14.8.5.2), so an entry under an excluded owner is not the layout
attribute of that name. `Tree::layout_attribute` and `Tree::inherited_layout_attribute` rank only the
admitted objects; `writing_mode`, `block_spacing` and `inline_statements` read through them, and
`bounds` admits `Artifact` beside them for Table 385's `/BBox`. The cost is nil for a conforming file
and a value lost for a non-conforming one, which is the clause's own division.

No question is owed: the only scope change the re-reading met, a reflowing view, A634 answered.

## Left

`Tree::print_field` says §14.8.5.6's owner restriction is "applied by `Self::attribute` already",
and `attribute` admits every ranked owner; §14.8.5.6's row owes either the narrowing or the sentence.
§14.8.5.4's aggregate note still says "nothing in it exports".
