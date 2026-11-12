# 2844 — A page's text on the boundary is a reading, not a copy

Session 2013, a ledger round (§9.10.1, Table 22). Status: implemented. Succeeds ADR 2836, which put
`viewer_core::Query::Text` on the boundary and left two things open: what the question is under the
reader's restriction levels, and the C ABI's accessor for it.

## Context

`Query::Text` answers the text of the pages on the screen to any host, at every level. Text leaves
the document by the other routes only through the reader's level: `Command::Copy` asks Table 22's
bit 5 (ADR 1144), and `quorra_selection_copy_text` answers only with what that copy released
(ADR 2087). So the question had to be settled before a C caller was handed the same characters by a
second symbol: is the page's text a reading, or a copy that leaves the process?

What the standard says:

- Table 22's bit 5: "Copy or otherwise extract text and graphics from the document. However, for
  the limited purpose of providing this content to assistive technology, a PDF reader should behave
  as if this bit was set to 1."
- Bit 10: "PDF readers shall ignore this bit", so the accessibility carve-out ISO 32000-1 gave a bit
  of its own is now inside bit 5's own words.
- §9.10.1 names the uses of a text's information content: "searching, indexing, and exporting of
  text to other file formats". The first two are what the viewer's own `Query::Find` and §14.9's
  `Query::AccessibilityTree` already do with these characters, gated by nothing.

And what `CLAUDE.md` says: a restriction's policy is asked once, in a place a host can supply, at
four levels, and a refusal that cannot become an ask is the thing it names to avoid.

## Decision

**`Query::Text` is a reading, answered at every level; the copy stays `Command::Copy`.**

1. **The level is asked at the gesture.** A query is `&self`, answered at pointer speed, and raises
   no event, so `ask` and `warn` cannot exist over one (`doc/todo/38`'s reading, ADR 1144). Gating
   the question would make bit 5 a refusal that can never become an ask, at `on`, and a silent
   proceed at the other three — the shape the restrictions section forbids.
2. **The characters are already read without a level.** `Query::Find` searches them and
   `Query::AccessibilityTree` speaks them; the second is bit 5's own carve-out. A gate on
   `Query::Text` alone would withhold nothing a host cannot already read.
3. **What leaves the program is the host's gesture, and it has its route.** Text bound for a
   clipboard or a file is bit 5's "copy or otherwise extract", asked by `Command::Copy` and read
   back by `quorra_selection_copy_text` (ADR 2087).

**The C ABI** answers `Query::Text` in `Query::Readback`'s shape, because the population is the
same: `quorra_text_pages`, `quorra_text_page` and `quorra_page_text` (C's two-call idiom), declared
in `quorra.h` and `Quorra.Interop`, driven by `c/open_a_page.c`, and named by
`every_query_reaches_the_abi.rs`'s exhaustive arm. Its documentation states the reading and points
at `quorra_copy` for a copy.

Held by `viewer-core/tests/restriction_levels.rs::the_pages_text_is_a_reading_the_copy_level_does_not_withhold`
and `viewer-ffi`'s `session.rs::a_pages_text_is_a_reading_and_answers_where_the_copy_is_refused`, each
on a document whose `/P` clears bit 5 at the level `on`: the copy is refused and the text answers.

## Consequences

The owner's word is not needed: the decision follows from Table 22's text and `CLAUDE.md`'s rule,
and it changes no level and no default. Were a host ever to export page text as a gesture of its
own, that gesture is a `Command` asked at bit 5's level, not a gate on this question. `pdf-vfs`'s
`PageText` path, which `pdf-vfs/src/layout.rs` leaves unconsulted on `doc/todo/38`'s reading, is
unchanged.
