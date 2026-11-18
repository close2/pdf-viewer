# 2971 — A TrueType encoding dictionary fills its undefined slots from `StandardEncoding`

Session 2074. Status: **accepted and built.** In `pdf-font`: `truetype::fill_dictionary_gaps_from_standard`. Finding
2038-B50. Read against ADRs 2948 and 2956, which it does not change. Clauses: ISO 32000-2 §9.6.5.4, Table 112,
Annex D.2 and D.4.

## Context

§9.6.5.4's paragraph for a nonsymbolic TrueType font builds a code-to-name table: "If the Encoding entry is a
dictionary, the table shall be initialised with the entries from the dictionary's BaseEncoding entry … Any entries
in the Differences array shall be used to update the table. Finally, any undefined entries in the table shall be
filled using StandardEncoding." The tree started from `StandardEncoding` only where the dictionary named no base, so
a named base's empty slot stayed empty: `MacExpertEncoding` leaves code 168 blank (Annex D.4), and the code fell to
the processor's own tiers and drew `dieresis` where Annex D.2's `currency` is the table's answer.

## Decision

After the base and the `/Differences`, every empty slot of a nonsymbolic TrueType font's dictionary-valued
`/Encoding` takes `StandardEncoding`'s name for that code. A named `/Encoding` (`/MacRomanEncoding`,
`/WinAnsiEncoding`) is "initialised with the mappings described in Annex D" and is not filled: the sentence belongs to
the dictionary branch only.

ADR 2956 reads a name-keyed program's (Type 1, CFF) empty slot as naming `.notdef`, and ADR 2948 advances a
name-alone standard font by `.notdef`; both are about the Type 1 format's 256-slot vector and stand. This is the
TrueType paragraph's own sentence for its own table, and the two do not meet.

## Consequences

Test: `pdf-model` `truetype_standard_fill_spec_audit` (its MacExpert case now passes; its controls stand).
