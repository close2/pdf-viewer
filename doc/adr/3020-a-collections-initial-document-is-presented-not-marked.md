# 3020 — A collection's initial document is presented, not marked

Session 2082. Status: **accepted, not built.** The change is in the viewer crates, which another round holds while
this one runs. Supersedes ADR 0231's "the panel obeys the outcomes the only way a panel over a page can" and
ADR 0202's "the container's pages stay on the screen" on the point named below. Their other decisions (the panel,
the four `Initial` values, `/View H`) stand. Context: review 2038 (finding D02). Clauses: ISO 32000-2 §12.3.5.1
Table 153 (`/D`, `/View`), §7.6.7.

## Context

Table 153's `/D` identifies "the document that shall be initially presented in the user interface", with three
fallbacks: the container, "the first item from the list of files", or "an empty preview window". ADR 0231 meets
the four outcomes by setting the initial document's panel row in bold. The window keeps showing the container's
pages, as ADR 0202 decided on the strength of §7.6.7's unencrypted wrapper.

Neither half reads the clause through. Emphasis on a row is not presenting a document. §12.3.5.1's prose says the
initial document "may be the container PDF or one of the embedded documents as specified by the D key", and that
is the thing presented. §7.6.7 points the other way from how ADR 0202 used it. The wrapper must name the encrypted
payload as the initial document and set `/View` to `H` "[t]o enable automatic display of an encrypted payload", so
that "PDF processors that already have the necessary security handler" can "immediately present the encrypted
payload". The wrapper's own page is for a processor that cannot open the payload.

## Decision

On first open, a collection whose `Initial` names an embedded document this viewer can open presents that
document. It is presented by the route a `GoToE` target already takes (`viewer_core::open`, with `Open::parents`
keeping the container one step back, ADR 2964), and the panel still shows the collection. A container `Initial`
keeps the container's pages. Where the named document cannot be presented, the container's pages stay and the
window says why. Examples are a payload under a security handler this program does not have, a file that is not a
PDF, or a read refusal. That is the wrapper's case, and the row stays marked. "The first item" is presented the same
way when it is a PDF. An empty name tree keeps the empty-panel answer ADR 0231 gives.

## Consequences

- D02 stays `open` until a viewer round builds it. A test drives each of the four `Initial` outcomes through
  `viewer_core` and asserts which document's first page is on screen. It fails on today's tree for an embedded
  `/D`.
- §12.3.5.1's ledger note keeps ADR 0231's description until then. It does not claim the presentation.
