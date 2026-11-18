# 2972 — A `/ToUnicode` code keeps its length, and a sequence range increments its last byte

Session 2074. Status: **accepted and built.** In `pdf-font`: `tounicode::ToUnicode` (`SourceCode`, the length-keyed
`singles` and spans, `take_sequence_range`), `LoadedFont::text`, `LoadedFont`'s substitution lookups. Findings
2038-B27 and 2038-B28. Clauses: ISO 32000-2 §9.7.6.2, §9.10.3.

## Context

§9.7.6.2 matches a code against the codespace byte by byte, so `<01>` and `<0001>` are two codes of one value, and a
`/ToUnicode` CMap that maps both has said two things. The map was keyed by value alone, and a later statement
overwrote the earlier: `<01>`, `<0001>`, `<0002>` read `AAβ` where the file says `AΩβ`.

§9.10.3, of `srcCode1 srcCode2 dstString`: "the last byte of the string shall be incremented for each consecutive
code in the source code range", and a last byte past "255 - ( srcCode2 - srcCode1 )" makes "the result of mapping …
undefined". A destination of several scalar values was applied to the low code only: `<0001> <0003> <00660061>` read
`fa` and then nothing.

## Decision

1. The map keys every statement by value and length. A lookup that knows its code's length (a `Code`) matches only
   statements of that length **where the map writes any**; where the map writes none of that length it matches by
   value. The second half is a tolerance, chosen and stated: §9.10.3 says "[f]or simple fonts, character codes shall
   be written as 1 byte in the ToUnicode CMap", and a producer who wrote them as two would otherwise lose every
   mapping. A mixed-length map is read exactly. A caller holding a value only asks by value and is answered by the
   shortest code of that value.
2. A sequence destination is expanded into one entry per code, its last byte incremented, and stops before the byte
   would pass 255, which is where the clause stops defining it; a range so states at most 256 entries.

## Consequences

`ToUnicode::append`, `states` and `char_for` take `impl Into<SourceCode>`: a `u32` asks by value, a `Code` by value and
length, so value-only callers in `pdf-model` and `pdf-transform` compile unchanged. `mappings` lists a value written
at two lengths once per length. Tests: `pdf-model` `tounicode_keeps_source_code_length_spec_audit`,
`tounicode_bfrange_sequence_destinations_spec_audit`, and `tounicode::code_length_tests` (three).
