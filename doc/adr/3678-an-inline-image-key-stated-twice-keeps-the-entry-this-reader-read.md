# 3678 — An inline image key stated twice keeps the entry this reader read, and the other goes

Session 2244. Status: **accepted and built**; reverses ADR 3560's first item and ADR 3558 item 5.
Context: `InlineAbbreviations` (veraPDF corpus) and `issue14256` (pdf.js), refused under `as-shown`
by `model/inline-image-key-twice` alone. Clauses: ISO 32000-2 §8.9.7 and Errata Collection 3's
Issue #3; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

ADR 3560 refused the class because "which of the two values this reader used is its lexer's choice"
and no rewrite of an inline image's dictionary was built. Both halves have moved. The erratum the
standard's working group agreed inserts at §8.9.7 that "the abbreviated key name shall take
precedence" where both spellings are present (ADR 3581: the corrected text governs), and it is the
rule `pdf_model::inline_image` already follows; a key stated twice the same way keeps the first, as
`pdf-syntax`'s parser does for every dictionary.

## Decision

1. `inline_image::Scan` carries `set_aside`: the span, key to value, of each entry the reader did
   not use. The reader states it; the converter does not re-derive the rule.
2. `as_drawn`'s walk writes each such span as one white-space character, the dictionary left being
   the one read, under ADR 3558's proof: both files drawn, page by page, the write standing only
   where the display list and text are equal.
3. The loss is `content-as-drawn`. For an abbreviation beside its full name the reading is the
   clause's; for the same spelling twice it is this reader's. One loss covers both, since the
   producer's bytes are gone either way.

## Consequences

`InlineAbbreviations` and `issue14256` convert under `as-shown`. Under `as-if-printed` their pages
are no longer photographed.
