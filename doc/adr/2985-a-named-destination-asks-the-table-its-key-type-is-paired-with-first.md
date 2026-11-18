# 2985 — A named destination asks the table its key's type is paired with first

Session 2076. Status: **implemented.** Supersedes ADR 0054 on the order of the two lookups. Context: review 2038
(finding B08). Clause: ISO 32000-2 §12.3.2.4.

§12.3.2.4: "In PDF 1.1, the correspondence between name objects and destinations shall be defined by the Dests entry
in the document catalog" and "[i]n PDF 1.2 and later, the correspondence between strings and destinations may
alternatively be defined by the Dests entry in the document's name dictionary". ADR 0054 read "alternatively" as a
sentence about where a document keeps its table and asked the catalog's dictionary first for either key type, so a
string destination whose bytes were also a key of the catalog dictionary resolved there rather than in the name tree
the clause pairs strings with.

The clause pairs each table with a key type, and that pairing now decides the order: a name asks the catalog's
`/Dests` first, a string the name tree first. The other table is still asked where the paired one has no entry —
the key is the same bytes (Annex J.3.3, J.3.4) and no clause makes a destination kept there unreachable — which keeps
every file ADR 0054 resolved resolving.

Tests: `viewer-core` `named_destination_tables_audit`.
