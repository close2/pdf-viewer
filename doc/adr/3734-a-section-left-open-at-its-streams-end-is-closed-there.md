# 3734 — A marked-content section left open at its stream's end is closed there

Session 2258. Status: **accepted and built** in `pdf-transform` (`archive/as_drawn.rs`:
`Walk::sections_left_open`, `CLOSES_A_SECTION`). Reverses ADR 3679's second item, whose reason ADR
3733 removes. Clauses: ISO 32000-2 §14.6.1, Table 352; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

Table 352 has `BMC` and `BDC` each begin a sequence "terminated by a balancing EMC operator", and
§14.6.1 says "each sequence shall be entirely contained within a single content stream". A page's
`/Contents` array is one stream for that purpose (§7.8.2). This reader ends a section the content
leaves open where the stream ends: nothing it draws depends on where a sequence ends.

## Decision

1. At the end of a page's last content stream, or of a form's, pattern's or glyph procedure's own,
   each section the walk followed whose opening operator stays is closed by an `EMC`, each written
   after an end-of-line marker so that a comment the stream ends in cannot take it.
2. **Not** inside a text object the producer left open (the sequence would cross its edge), after
   operands no operator took (the `EMC` would take them — `050734`'s unterminated string is this
   case), or past the sections the walk follows (`MOST_SECTIONS`).
3. ADR 3558's proof decides, and ADR 3733 answers the class only where the second file no longer
   needs it. The loss is `content-as-drawn`: where the producer meant the section to end is this
   reader's reading, not the clause's.

## Consequences

A page whose only repair is a section left open converts under both profiles with no photograph.
`050734` stays photographed under `as-if-printed`: its section is open inside a string the reader
read to the end of the data.
