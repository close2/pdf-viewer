# ADR 0231 — A collection opens on one document, and nobody could say which

Status: accepted, three-hundred-and-ninety-fourth session.

## Context

ISO 32000-2 §12.3.5.1, Table 153's `/D`:

> A string that identifies an entry in the EmbeddedFiles name tree, determining the document that
> shall be initially presented in the user interface. If the D entry is missing or is not a valid
> byte string, the initial document shall be the one that contains the collection dictionary. If
> the D entry is a valid byte string that does not match any file in the EmbeddedFiles name tree,
> the interactive PDF processor shall select the first item from the list of files to display in
> its user interface; if no files exist in the name tree, the interactive PDF processor shall
> display an empty preview window.

Four outcomes, three of them `shall`s, and every one decided against the `/EmbeddedFiles` name tree
rather than against the collection dictionary.

`pdf_model::collection::Collection::initial_document` has answered them since the
three-hundred-and-fifty-second session, as four values rather than an `Option`, because they are
four different instructions. **No host could call it for thirty-four sessions.** It takes the
`&Document` that only `viewer-core` holds, and `Answer::Collection` handed a host Table 153 and
nothing else — so the one rule of §12.3.5.1 a *reader* is asked to make was made by nobody.
`doc/todo/01`'s fifth sweep found it in the three-hundred-and-eighty-sixth session and wrote it
into the row and into `doc/todo/34`; the round that found it left it named.

It was never a confinement gap. `viewer_ui::chrome` draws the collection in the same process and
could not ask either, and `viewer_confined`'s protocol reproduced the in-process boundary
faithfully. **The blocker was the answer's shape**, which is `doc/todo/01`'s §14.9.3 lesson — a row
whose reason names a capability can survive the arrival of the capability, because what has to
change is what the program *says*.

## Decision

**`Answer::Collection` carries the resolved `Initial` beside Table 153**, and `viewer_core::query`
resolves it where the document is:

```rust
Collection {
    collection: pdf_model::collection::Collection,
    initial: pdf_model::collection::Initial,
},
```

Two values rather than one, and not an `Option<String>`: a host receiving `None` would have to
consult the name tree to tell "no `/D`" from "a `/D` naming nothing", and the name tree is the
thing a panel does not have. The confined protocol encodes the four cases as a tag byte and, for
`Embedded`, the name — `Reply::Collection` grew the same pair, so the two hosts stay identical.

**The panel obeys the outcomes the only way a panel over a page can.** `viewer_ui::chrome` sets the
initial document's row in **bold**; where `/D` names nothing the tree holds, that is the first file
row, which is the clause's "the first item from the list of files to display in its user
interface"; where the tree is empty, the panel says so instead of drawing nothing. The container
case marks no row, because the container's own pages are what is already on the screen — the
decision ADR 0202 took and gave its reason, §7.6.7's unencrypted wrapper.

**The emphasis is a choice and is written down as one.** The standard states no appearance for an
initial document anywhere; `CLAUDE.md`'s rule for a silence is a documented choice, not a match
with anyone. Bold is the panel's existing vocabulary and costs no row.

## Consequences

- §12.3.5.1 stays `partial`, and for a different reason than it was `partial` for. What is left is
  Table 153's `/View`: `D`, `T` and `C` are three presentations of one collection and this panel
  draws one of them, which is §12.3.5's `partial` in the same words. `H` is met by construction —
  the sidebar is closed until a person opens it.
- **No corpus document states a `/Collection`**, which is measured rather than assumed
  (`crates/pdf-model/tests/collections.rs::no_pdfjs_document_is_a_portable_collection`). So this
  is coverage answering a question robustness cannot see, and the tests are hand-built: four
  synthetic documents in `viewer-core`'s `a_collections_initial_document_reaches_a_host`, one per
  outcome, and a panel test that draws each and compares ink.
- The fifth sweep's own lesson, again: `Collection::initial_document` was implemented, tested and
  correct for thirty-four sessions, and none of that is worth anything until something asks. A
  sweep run in the same round as the work is the cheapest it will ever be.
