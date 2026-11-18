# 2982 — An annotation without a handler draws its normal appearance whatever the pointer does

Session 2076. Status: **implemented.** Context: review 2038 (finding B68). Clauses: ISO 32000-2 §12.5.1,
§12.5.5, Table 167 (`Invisible`), §13.6.3.1.

§12.5.1: "A PDF processor shall provide annotation handlers for all of the conforming annotation types. The set of
annotation types is extensible." §12.5.5 states the three appearances as three situations: the normal appearance
"shall be used when the annotation is not interacting with the user", the rollover when the cursor enters "the
annotation's active area", the down while the button is held there. An annotation whose subtype has no handler in
this reader does not interact with the user — nothing handles its events — so it is never in the second or third
situation, and its `/R` and `/D` are appearances for an interaction nothing carries. §13.6.3.1's NOTE states the
same direction for the one unknown-type case the standard spells out: "PDF processors need to follow the approach of
falling back to the normal appearance".

So `annotation::stored_appearance` keys `/N` for every pointer state where the subtype is not one of Table 171's
(`STANDARD_SUBTYPES`, `has_handler`), and `annotation::hover_changes` answers false for its `/R`, so the viewer
neither redraws nor swaps the stream. Table 167's `Invisible` row is unchanged: it decides whether such an
annotation is drawn at all, and this decides which of its appearances.

Tests: `viewer-core` `unknown_annotation_pointer_spec_audit` (Square keeps N/R/D; the extension stays N).
