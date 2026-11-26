# 3717 — A redaction drops the private data and the thumbnail its removal made stale

Session 2254, the fix round of audit 2253 (finding B03). Status: **accepted**.
Context: `doc/questions/A834` (a redaction removes, never covers), ADR 1162 (a protected source),
ADR 2868 (the overlay after the removal), RFC 0002 section 9 (no clock); ISO 32000-2 §12.3.4,
§12.5.6.23, §14.3.3 and §14.5, Tables 31, 93, 350 and 351.

## Context

`redact`'s `build_page` wrote the edited content and carried every other page entry, so a
redacted page kept its `/PieceInfo`, the `/LastModified` that dates it, and its `/Thumb`. §14.5
makes the page's date the test of whether a data dictionary is current: "their modification dates
can be compared with those in the corresponding entry of the page object or form dictionary … to
ascertain whether the data dictionary corresponds to the current content", "only for equality".
Carried unchanged, the two still compare equal, so private data — "all of the private data needed
by the PDF processor other than the actual content", an editor's own copy of the artwork as often
as not — is declared current after its content was removed. The thumbnail is a picture of the page
before the removal. The catalog's `/PieceInfo` and the information dictionary's `/ModDate` stand in
the same relation for the document.

## Decision

- §12.5.6.23 decides it before §14.5 does: a processor applying a redaction "shall remove all traces
  of the specified content" and "shall also be diligent in their consideration of all content that
  can exist in a PDF document", and A834 makes the removal physical. Private data that may hold the
  removed content, and a thumbnail that does, are traces.
- This verb has no clock (RFC 0002 section 9), so it cannot write a new `/LastModified`. The remedy
  is to stop asserting the old one: a redacted page is written without `/PieceInfo`,
  `/LastModified` and `/Thumb`; a form `XObject` whose content the removal edited without
  `/PieceInfo` and `/LastModified`; and, where any page was redacted, the catalog without
  `/PieceInfo` (the catalog is then replaced rather than copied, so nothing only that entry reached
  is written). Tables 31 and 93 require `/LastModified` only beside `/PieceInfo`, and Table 349
  requires `/ModDate` only beside the catalog's, so the output keeps every `shall`.
- The information dictionary's `/ModDate` crosses as the source stated it: with the catalog's
  private data gone nothing compares a data dictionary against it, and a date this program chose
  would be an invention.
- Each entry dropped is a `Departure` in the redaction's report — the page, or none for the catalog,
  and the entries by name — so the change is never silent.
- A page the redaction did not touch keeps all three: its content is the content they describe.

## Consequences

`crates/pdf-transform/tests/stale_private_data.rs` pins the drops and the report;
`metadata_writers_14_3_14_5_spec_audit.rs`'s B03 test passes. Left: other places a removed text
may survive (a structure element's `/ActualText` or `/Alt`, a page's own `/Metadata` packet) are
not read by this decision and are named in the round's record as the next to examine.
