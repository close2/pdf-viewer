# 3627 — A field is named by `/TU` to a person, and an outline row carries the document's language

Session 2231. Status: accepted. Context: ADRs 1198, 1405, 3499; review 2229 (B03, B04).

## Decision

Every sentence `submission::compose` owes a person, and the edit log's notes (`field_name_of`), name
a field by `FieldName::shown` — §14.9.3's `/TU`, else the qualified name; what is submitted keeps the
qualified name, which addresses the field for a program. viewer-core's script question names the
field by the event's `/TU` where the field states one.

`Outline` carries `language` (the catalog's `/Lang`, §14.9.2.1's "outline entries"), it crosses the
confined wire, and `PanelRow::language` carries it to every outline row. GTK states it as a Pango
language attribute on the row's label (GTK's AT-SPI text interface reports it; `GtkAccessible` has
no language property); Qt's item views give a row no settable locale, and the comment at
`push_rows` says so; quorra's accessibility tree holds no panel rows. The C ABI carries it as
`quorra_outline_language`.

## Left

`layer_rows` does not fill `language`: `Answer::Layers` carries no document language, and the
carrier (a document-language query, or the answer gaining one) crosses the confined protocol and the
C ABI; it is the next round's (§14.9.2.1's "names for optional content groups"). A title's own
§7.9.2.2.2 escape is not yet read into its row.
