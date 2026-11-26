# 3733 — A write answers at a holder only the classes the proof's file no longer needs

Session 2258. Status: **accepted and built** in `pdf-transform` (`archive/as_drawn.rs`:
`still_repaired`, `Drawn::left_repaired`, `Drawn::wrote_out_of`; `archive/fenced.rs`'s
`content_repair_holders`; `archive/base_standard.rs`'s `settled`). Amends ADR 3558's answer at a
holder. Context: ADR 3679's withdrawn section-closing write, which took `050734`'s page out of
`as-if-printed`'s photographs. Clauses: ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

ADR 3558's fence answered every repair of a written-out class at a page or form once one operator
had been written out of every stream of it (`wrote_out_of(holder)`). A write that answered one
class, or one operator of a class, therefore answered every other repair of the listed classes at
the same holder too, including ones the walk never wrote: an operator whose operands stand in an
earlier stream of the `/Contents` array, a colour operator in a form begun in two spaces, a text
operator whose matrix the walk cannot state. Under `as-shown` the output's own check then refused;
under `as-if-printed` the page was no longer photographed and the output failed section 5.1.

## Decision

1. **The proof's second file is read for what it still needs.** Where the write stands, every
   repair the second file logs inside a content stream at a page or form, of a class that breaches
   section 5.1 and that no other rewrite writes out, is kept with its holder and class.
2. **A class is answered at a holder only where it is gone from it.** `content_repair_holders` and
   `settled` answer a repair by the write only where its holder's streams were all rewritten, an
   operator was written out of one, and the second file no longer needs that class there. A page
   holding a class the write left keeps the answer it had: refused by that class's sentence under
   `as-shown`, photographed under `as-if-printed`, the write going with the content.
3. **Bounded**: where the second file logged a refused class at more sites than the log keeps
   (`SITES_KEPT_PER_CLASS`), the holders past it are unknown and the write is not shown to answer
   anything (`None`, the proof's refusal).

Reading the file again, rather than counting what the walk wrote, holds for every class at once,
including a repair the walk never meets.

## Consequences

`a_page_holding_a_class_the_write_left_is_photographed_though_another_was_written` fails without
item 2 and passes with it. ADR 3734's section-closing write can stand.
