# 2836 — The text of the pages on the screen is one question on the boundary

Session 2009, a ledger round (§9.10.1). Status: implemented. Lifts `doc/todo/76` row 5, released by
`doc/questions/A800` item 2 (ADR 2800); carries out the pipe's one message `doc/todo/63` sized.

## Context

`doc/todo/63` left one message undone on purpose: `viewer_core::Query` had no "this page's text",
so the readback crossed the boundary only as a `Selection`, which needs a drag, or as §14.9's
spoken nodes. Its gate was "when a host wants a page's text", and A800 lifted every such gate.
Read against the code, nothing else held it: every interpreted page already keeps its text
(`Interpreted::text`), `Query::Find` searches it and `Query::Selection` copies slices of it.

§9.10.1 states why a processor wants it — "the information content of text … This need arises
during operations such as searching, indexing, and exporting of text to other file formats" — and
the clause's own two questions, appearance and content, are the two this tree already carries as
separate facts.

## Decision

`Query::Text` answers `Answer::Text(Vec<PageText>)`: one entry per page on the screen, each naming
its page, the text borrowed from the interpretation.

- **The pages on the screen, not any page.** `Query::Readback`'s population and its reason:
  `Viewer::query` takes `&self` and is asked at pointer speed, and reading a page means
  interpreting it (`viewer-core/src/search.rs` prices a page at 5.7 ms). A host turns to the page
  and asks. An empty list where no page has been read, `Answer::None` where nothing is open —
  the distinction `Readback` and `Frame` already draw.
- **Content order, the same characters `Find` and `Selection` read.** One readback, so what a
  program reads of a page and what a person finds on it cannot differ. §14.8.2.5's logical order
  stays `Query::LogicalSelection`'s, which needs the structure tree to reach the range.
- **No report and no count beside it.** What §9.10.2 cannot name is `Query::Readback`'s count,
  asked beside this one.
- **On the confined pipe** as query kind 38 and answer kind 38, owned on the far side as
  `viewer_confined::PageText`.

## Consequences

`viewer-core` states thirty-eight questions and `viewer-confined` carries all of them. The C ABI
(`viewer-ffi`) has no accessor for it: its entry points are per question rather than a generic
query, so a C host's `quorra_page_text` is a separate, effort-only addition. `pdf-retrieve`'s
substitution, `doc/todo/63`'s other half, is unchanged by this and still owed.
