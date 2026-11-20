# 3282 — A null `/F` on a stream is an absent `/F`

Status: accepted. Session 2145. Context: ISO 32000-2 §7.3.8.1, Table 5, §7.3.9, §7.3.10; audit 2142 finding B06.

## The defect

`Document::is_external` asked whether the stream dictionary had an `/F` key at all, so `/F 9 0 R` naming no object,
or `/F null`, refused a stream whose own bytes are its data as external.

## Decision

Table 5 conditions on whether `/F` "is present". §7.3.9 makes a null value "equivalent to omitting the entry
entirely", and §7.3.10 makes a reference to an undefined object a reference to the null object. So
`is_external` now takes the document (`&self`) and answers false for a direct null and for a reference that resolves
to null; any other value, direct or resolved, is a file specification and the stream is external. Only a reference is
resolved, so a direct file specification dictionary is not copied on every decode.

## Cost

A public signature changes: `Document::is_external(stream)` becomes `document.is_external(stream)`. The callers in
`pdf-syntax`'s serializer and in `pdf-transform` all hold the document.
