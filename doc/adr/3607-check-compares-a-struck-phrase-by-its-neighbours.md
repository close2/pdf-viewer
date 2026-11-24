# 3607 — `spec-errata check` tells a struck phrase from the same phrase in another sentence by its neighbours

Session 2226. Status: **accepted**. Context: ADR 0254, ADR 0255, ADR 3589, ADR 3603,
`tools/spec-errata`.

## Context

`check`'s other-clause population held 1134 lines after round 2225. 531 of them were #106's
*shall be an indirect reference*. #106 strikes that phrase from some table cells. Quotations of
other cells, which keep it, contained the phrase, so they landed on every strike of it: seven
pages, one line each. Sixty more were quotations of a caption the conversion spells letter by
letter (`Ta b l e`), which fold to *Table* and match any struck passage that names a table.
`Landing::in_clause` cannot tell either kind from a finding. The outline's section is the
section a page opens with, and a quotation's clause is this tree's.

## Decision

1. **A note keeps the page's text beside the strike** (`Note::beside`, up to `BESIDE_BYTES`
   either way). `Landing::beside_the_strike` takes `CONTEXT_CHARS` folded characters of the
   quotation either side of the struck passage and looks for them in the page's run. The run is
   cut where the page runs out. A quotation that puts other words around the passage is the same
   phrase in another sentence. `check` counts these under their own line and does not list them.
2. **The comparison is withdrawn only where it was made.** A site note has no page. A quotation
   inside the struck words, or an elided one, is not one run of the page. Each of these stays
   where `in_clause` put it, so the change can only remove landings it has shown to be
   coincidences.
3. **`MIN_WORDS` counts runs of two characters or more, and digits** (`words`), so a spaced-out caption is
   one word.

## Consequences

Of the 1134, `check` now counts 327 as repeated phrases and no longer lists them. The word floor and the reconciliation it feeds remove 102 more. 705 stay listed as other-clause landings.
A later round reads the real ones in that list, sorted by erratum in round 2226's record. The
remaining bare-phrase quotations, such as `shall be an indirect reference` quoted alone, cannot
be placed from the quotation, and they stay listed.
