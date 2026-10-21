# 2178 — A simple font's widths array states the range the reader already uses

Session 1671, archive. Status: **accepted**. Context: ADR 2163 section 2.

ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2 require fonts to
follow their base standards' font clauses. The purchased texts were read again;
this record paraphrases them. ISO 32000-2 §9.6.2.1's Table 109 states:

> An array of ( LastChar - FirstChar + 1) numbers, each element being the glyph width for the
> character code that equals FirstChar plus the array index.

The existing `fonts/font-programs-conform-to-their-own-specifications` requirement
remains unchecked: reading font formats is not validating every format rule.
The independently checkable dictionary count is now
`fonts/widths-array-covers-character-range`, for Type1, MMType1 and TrueType fonts.
It judges a stated array and a readable byte-code range in the existing xref
`/Type /Font` dictionary population, including fonts never rendered and rendering
mode 3. Fonts written directly inside resource dictionaries remain outside that
existing traversal. Missing arrays, malformed ranges and Type 3's separate Table
110 interpretation also remain gaps, not claims this check makes.

The converter's `FontWidthArrayRestated` is a `Stated` interpretation. A nonempty
short array narrows LastChar to its last covered code. A long array loses only
entries past LastChar. It never changes a retained advance to match a program.
Table 109 makes codes outside the range use MissingWidth; codes without a width
already use that default in this reader. The report names this interpretation,
and the file's existing provenance machinery records the decision.

The long-array construction depends on the font reader respecting LastChar
(ADR 2181). Without that correction, truncation would change what this reader
shows, even though it would implement Table 109. Empty arrays stay refused by
name: the font loader can use substitute metrics when it finds no stated width,
so inventing one or manufacturing an empty byte-code range is not this argument.

Preparation follows the validator's finding objects. It produces just the entries
to replace, checks refusal before output, and the existing second verdict checks
the assembled document. The tests hold every byte-code advance and the page's
display list equal before and after, for both short and long arrays and both parts.
