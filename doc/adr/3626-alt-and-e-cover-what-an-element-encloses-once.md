# 3626 — `/Alt` and `/E` cover what an element encloses, once; structured content stops a Span's language

Session 2231. Status: accepted. Context: ADRs 2976, 3261; review 2229 (B02, B05).

## Decision

`structure::enclosing`'s one walk up `/P` reads `/Alt` and `/E` beside `/ActualText`, outermost
winning (Table 355: `/Alt` describes "the structure element and its children"; §14.9.5's `/E`
substitutes for the tagged text). The element stating the phrase is carried as `phrase_owner`, and
the interpreter's `described_elements` set speaks it at the element's first content item and records
the rest as `alt: Some("")` — replaced by nothing — the construction §14.9.4's `/ActualText` already
takes with `replaced_elements`. An empty `/Alt` in a file is still read as none.

A sequence naming a structure element whose chain states no `/Lang` takes the catalog's default,
or `""` (§14.9.2.2's unknown) where the catalog states none: §14.9.2.3 withholds a `Span`'s language
from "nested marked-content that is contained in the structure hierarchy". `EnclosingMemo` reads the
catalog once per page.

## Cost

Every structured sequence on a tagged page now carries a `Described` (its language), one small
allocation per `/MCID` sequence. A child element's node in viewer-core is named by an ancestor's
`/Alt` where its text is so replaced, as it already is by an ancestor's `/ActualText`.
