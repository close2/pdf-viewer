# 3629 — The validator's grammar reads every content class the tokens decide

Session 2232. Status: **accepted and built**; extends ADR 3542 and ADR 3614. Context: round 2228's
finding (ADR 3614 item 3) that the converter's fence reads a repair log the survey's token walk
filled, while the photograph step interprets every page and logs classes the walk never read, so
the decision is made on a partial log. Clauses: ISO 32000-2 §7.3.6, §7.8.2, §8.6.8, §8.9.7,
§14.6.2.

## Context

After ADR 3614 the grammar read seven of the interpreter's content classes. Of the rest logged
inside content, some are a function of the tokens alone or of the tokens and a name the walk
already resolves: `model/operands-left-over` (§7.8.2's "operands shall not be left over when an
operator finishes execution"), `model/property-list-unresolved` for a well-typed `BDC` (§14.6.2),
and the inline-image bound (`model/content-past-a-bound` where the scan reaches its lookahead).
The inline-object and inline-image classes were already read: the survey runs the same
`inline_dictionary` and `inline_image::scan` the interpreter does.

## Decision

1. **`Grammar::operator` answers a `Found`**: the deciding class as before, and §7.8.2's leftover
   beside it — a fixed operand list with more operands before it than it states. One walk, no
   second interpretation (ADR 3542).
2. **The count is the interpreter's.** An array's brackets are operands of their own
   (`Operand::ArrayOpen`, `ArrayClose`) and its elements one each, as the interpreter keeps them;
   a keyword inside an array is stepped over without ending the operands (§7.3.6,
   `operator-in-TJ-array.pdf`); past the interpreter's operand bound the array is no longer
   believed. After a `d1`, and in a stream the walk marks uncoloured (an uncoloured tiling
   pattern's cell, or what such a glyph invokes), §8.6.8's colour operators are ignored with
   their operands and answered with nothing, since the interpreter drops them before asking.
3. **A `BDC` naming its property list asks the walk**: `names_a_property_list` says the last of its
   two operands is a name, the survey looks the name up in `/Properties` (`peek`, no record), and
   `property_list(resolved)` answers the class — the interpreter's own condition.
4. **The survey logs the inline-image bound** where its scan settles by the bound alone, as the
   interpreter's `nested.rs` does.
5. The interpreter keeps logging every class for the viewer; the log deduplicates class and site.

## Consequences

A class the grammar now reads appears in the checker's log before the fence asks, so a page
holding one is photographed under `as-if-printed` rather than refused afterwards. What is left to
the interpreter, and why, is ADR 3630's table.
