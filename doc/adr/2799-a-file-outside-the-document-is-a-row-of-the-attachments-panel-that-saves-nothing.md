# 2799 — A file outside the document is a row of the attachments panel that saves nothing

Session 1994, a ledger round (§14.13.2, §14.13.4). Status: implemented. Answers
`doc/questions/A45`'s `Owes:`, released by `doc/questions/A800` item 2 (ADR 2800). Builds on ADRs
0918 and 1186.

## Context

§14.13.2 gives an associated file two forms in one sentence: "[t]he file specification for an
associated file represents either a file external to the PDF file or an embedded file stream (see
7.11.4, "Embedded file streams") within the PDF file." `pdf_model::attachment::external_associated`
read the external form (ADR 0918) and `viewer_core::notes::about` said it once when a document
opened. The owner's A45 kept that note and asked for the panel row "when the attachments panel is
designed, not before — and decide it once for the whole panel", with its shape: "it states plainly
that the file is outside the document, carries no save action at all, and shows the relationship in
place of the size." The panel exists — `Query::Attachments` lists in `viewer-gtk`, `viewer-qt`,
`viewer-ffi` (the WinUI window) and `quorra` — so the gate was effort only.

## Decision

1. **A question of its own, `Query::ExternalFiles`**, answered with
   `Vec<pdf_model::attachment::ExternalAssociatedFile>` — not a change to `Answer::Attachments`.
   An `Attachment` is a stream with a name; the external form has no stream, so every consumer of
   the embedded list (extraction, previews, collections, `pdf-vfs`) would have had to learn a case
   it can do nothing with. It crosses `viewer-confined`'s pipe whole (query and answer kind 37).
2. **The population is the embedded list's**: §14.13.3's catalog `/AF`, then §14.13.4's `/AF` on
   the page in view, for the cost ADR 1186 measured. A specification naming the same file with the
   same relationship in both is one row, because an external file has no stream whose identity
   could tell two apart; one that states no name is kept, since its relationship is still a fact.
3. **One rule for the whole panel** is `viewer_host::panel::file_rows` and `external_rows`: every
   file the panel can name is a row, whichever form it takes, and the form decides only the action.
   An external row is the name (or "A file the document does not name"), "Outside this document —
   <relationship>" as its second line, and `RowAction::Inert`. Under a §12.3.5 collection the rows
   follow the collection's, since a collection arranges the files the document holds. The empty
   panel's sentence is said only where neither form names a file. `quorra`'s own sidebar draws the
   same rows from `external_rows`, so the wording is one in every window.

## Consequences

- §14.13.2's and §14.13.4's notes say the external form reaches the panel; their status
  (`implemented`) does not move, because §14.13.1 says the relationship gives a processor no
  instruction — the row is what a reader is shown, not a requirement newly met.
- §14.13.5 to §14.13.9's carriers still reach no caller; their objects are not on the panel's path.
- No corpus document states an external associated file (ADR 0918), so both fixtures are built
  (trap 8).
