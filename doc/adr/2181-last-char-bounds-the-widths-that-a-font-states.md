# 2181 — LastChar bounds the widths that a font states

Session 1672. Status: **accepted**, in `crates/pdf-font/src/metrics.rs`. Context: ISO 32000-2
§9.6.2.1 Table 109 and §9.8.1 Table 120; ADR 2163 section 2.

Table 109 assigns array entries to codes from FirstChar through LastChar and sends codes outside
that range to the descriptor's MissingWidth. The reader previously accepted every array entry,
so a malformed long array extended the range past the endpoint the dictionary actually stated.

Stop reading entries when their code exceeds a stated LastChar. A short array invents no entries;
a reversed range supplies none. Where LastChar is absent, retain the existing recovery over the
array rather than adding a second malformed-input policy in this change.

The regression fixture states FirstChar = LastChar = 65, Widths [722 999], and MissingWidth 250.
Code 66 advances by 0.250, not by the ignored trailing width. This is also the boundary the
archive remedy must preserve: removing trailing entries is safe only after the reader itself
ignores them. Round 1671's conversion test checks the display list and every character advance
across serialization, as well as the output's second archival verdict.
