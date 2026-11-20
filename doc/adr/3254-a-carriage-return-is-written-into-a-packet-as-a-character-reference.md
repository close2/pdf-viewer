# 3254 — A carriage return is written into a packet as a character reference

Session 2138, an archive round in the `lane3` worktree. Status: **accepted** and built. Context:
ADR 2680 and 2681 (a key Table 349 does not define, carried and its leftovers), ADR 3238 (which
left this reading open); `crates/pdf-model/src/xmp.rs` (`escaped`),
`crates/pdf-transform/src/archive/prepare/metadata.rs` (`carried`). Clauses: ISO 32000-2 §14.3.3
and Table 349, §7.9.2.2; ISO 19005-4 section 6.1.3; XML 1.0 sections 2.2 and 2.11.

## Context

A Table 349 value is a §7.9.2.2 text string and may hold a carriage return. XML 1.0 section 2.11
has a processor read a carriage return, alone or before a line feed, as one line feed before it
parses anything, so a value written raw comes back changed. ADR 2680 therefore refused a carried
value holding one (`xml_carries`), while a Table 349 value holding one was written raw.

## Decision

1. The packet writer escapes a carriage return as `&#xD;`. A character reference is not a line
   end, so section 2.11 leaves it, and a reader gets back the carriage return the dictionary
   stated. XFA's writer (`xfa/xml.rs`) already spells it so, for §12.7.4.3's line breaks.
2. A carried value holding a carriage return is now carried: the reason ADR 2680 gave for
   refusing it is gone, and the carried rows ask `xml_admits` (ADR 3253) as the Table 349 rows do.
   Its refusal sentences now name only the characters the `Char` production excludes.

## Tests

`a_carriage_return_in_a_value_survives_end_of_line_handling` applies section 2.11 to the written
bytes by hand before reading them, so the test does not rest on this module's reader;
`both_stating_profiles_carry_a_key_table_349_does_not_define` does the same through a conversion
for a carried `/Batch` and `/Subject`'s `dc:description`. veraPDF 1.30.3 passes the outputs.
