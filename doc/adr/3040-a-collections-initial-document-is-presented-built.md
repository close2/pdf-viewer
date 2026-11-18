# 3040 — A collection's initial document is presented: ADR 3020 built

Session 2086. Status: **accepted, built.** Builds ADR 3020, whose record stays as written ("accepted, not built").
Context: review 2038 (finding D02). Clauses: ISO 32000-2 §12.3.5.1 Table 153 (`/D`), §7.6.7.

## Decision

ADR 3020's decision stands and is now the code. Three points it left to the round that built it are decided here.

1. **Where.** `viewer_core::interact::initial_document` reads the collection on the first open of a document a
   person or a host opened (its `parents` empty), after Table 29's `/OpenAction`, which is the container's own and is
   performed "when the document is opened". An open action that already went to another document leaves nothing to
   present. The embedded document is opened with the container's limits and becomes the window's document through the
   outcome a `GoToE` replacement already takes, with the container as its one parent (ADR 2964).
2. **The panel around it.** ADR 3020 says "the panel still shows the collection". A document whose root parent holds a
   `/Collection` while it holds none answers `Query::Collection`, `Query::Attachments` and `Command::Extract` with the
   container's (`viewer::answers::collection_frame`), so every window's panel goes on listing the collection's files
   and a row still extracts its file from the container. A member that is itself a collection answers for itself.
3. **"The first item".** Table 153's `/Sort` order where the collection states one, the `/EmbeddedFiles` tree's own
   order otherwise: the order the panel lists the files in. A first item that is not a PDF keeps the container's
   pages and is said, as a named document that cannot be presented is.

Where the document cannot be presented (no embedded file stream, a stream that does not decode, a file that is not a
PDF or is under a security handler this program does not have, no pages), the container's pages stay and the window
reports why. The panel row stays marked in every case (ADR 0231's `mark_initial`).

## Consequences

- D02 is `fixed`. `crates/viewer-core/tests/collection_initial_document.rs` drives the four `Initial` outcomes and a
  non-PDF `/D` through `viewer_core`, and asserts which document's pages are shown and that the panel's answers stay
  the container's.
- §12.3.5.1's ledger note now records the presentation.
