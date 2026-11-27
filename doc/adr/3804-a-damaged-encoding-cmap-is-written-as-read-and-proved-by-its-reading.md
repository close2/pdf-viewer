# 3804 — A damaged encoding `CMap` is written as read, proved by its reading

Session 2275. Status: **accepted and built** in `pdf-font` (`cmap::CMap::reads_alike`,
`CMap::sections_as_read`, a name inside a section) and `pdf-transform` (`archive/font_maps/
encoding_cmap.rs`, `font_maps::restate_damaged_encoding`, the report's `restated_font_maps`), with
`font/cmap-damaged` mapped to `fonts/font-maps-read-as-written` in `pdf-archive`. Supersedes ADR 3782's
decision. Clauses: ISO 32000-2 §9.7.5.3, §9.7.5.4, §9.7.6.2, §9.7.6.3, Table 118; ISO 19005-2 and
ISO 19005-4 section 5.1.

## Context

ADR 3782 left `font/cmap-damaged` refused: a lookup proof (ADR 3781's, code by code) proves which CID
each code selects and not how §9.7.6.2 cuts a string into codes, which the codespace ranges decide.
It asked for a draw proof over the shown strings.

## Decision

1. **The proof is the reading.** `CMap::reads_alike` compares what §9.7.6.2's extraction and
   §9.7.6.3's selection read and nothing else: the codespace ranges in order, the character and notdef
   mappings of each code length (singles, and ranges in the order the lookup tries them), and the
   writing mode. Two maps alike there cut every string into the same codes and select the same CIDs,
   so the glyphs and advances drawn are the same for every string, shown on a page or not — a
   stronger proof than a draw of the strings the survey met, and one no string can escape.
2. **The map is written again from the reading** (`CMap::sections_as_read`): `codespacerange`,
   `cidchar`, `cidrange`, `notdefchar` and `notdefrange` sections of at most a hundred entries (Adobe
   Technical Note #5014), framed by the stream dictionary's `/CIDSystemInfo` and `/CMapName`, which
   Table 118 requires there, and the writing mode read. Read back by the same parser it must be whole
   — not damaged, not cut by a bound, its `/WMode` usable — and alike. A map built on another through
   `/UseCMap` is written as one map holding both, and the entry goes, as ADR 3781 does for a
   `/ToUnicode`. Refused by name (`encoding_cmap::NOT_WRITTEN`): an in-file `usecmap`, an unusable
   `/WMode`, a reading a bound cut.
3. **The reader is corrected where this proof found it silent**: a name inside a
   mapping section cleared the operands before it, so `<41> 36 <42> /B` read as no mapping at all
   and logged nothing. Inside a section a name is now an operand §9.7.5.3 does not admit — the entry
   it stands in reads as no mapping, the map as damaged — and the entries before it are kept.
4. The loss is `font-maps-as-read`, which both shipped profiles authorise: the entries dropped are
   the producer's words, which no reader reads by §9.7.5.3. The report names each map restated
   (`Conversion::restated_font_maps`, which also carries the lines ADR 3570's restatements always
   wrote and nothing rendered).

## Consequences

A composite font whose embedded encoding `CMap` holds entries outside §9.7.5.3 converts under both
profiles. A damaged map that also uses `beginbfchar` takes ADR 3428's restatement first, which keeps
the damage, and the output's own check refuses it by name.
