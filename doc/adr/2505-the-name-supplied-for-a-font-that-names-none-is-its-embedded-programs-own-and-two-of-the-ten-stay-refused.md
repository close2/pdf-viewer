# 2505 — The name supplied for a font that names none is its embedded program's own, and two of the ten stay refused

Status: accepted, the first half built in `fonts::described::Layout::font_name` and the second half
recorded and not built. Session 1835, the archive slot of batch one hundred and twenty-four. Context:
ADR 2504 (the remedy this completes), `doc/questions/A687` (the owner's word, whose Owes line names both
halves), `doc/questions/A48` (no value is guessed), ADR 1014 (a page of the document's own content),
ADR 2476 (the descriptor and the `/BaseFont` it must equal).

## 1. The name supplied

ISO 32000-2 §9.8.1's Table 120 says of `/FontName`: "The PostScript name of the font. This name shall
be the same as the value of BaseFont in the font or CIDFont dictionary that refers to this font
descriptor." A font that states no `/BaseFont` has no name to equal, and the descriptor this conversion
writes needs one: ISO 19005-2 section 6.2.11.2 holds every font to the base standard's font clauses,
Table 120's among them, and veraPDF's test 3 of that clause (the equality of the two names) failed the
one hand-built document of the ten (`T02-04-01_001_resources-basefont-missing`) in round 1821's
experiment.

**The standard does define the name, once there is a program to read it from.** §9.6.2.1's Table 109,
the `BaseFont` row, for a `Type1` font: "For Type 1 fonts, this is always the value of the FontName
entry in the font program". §9.6.3, for a `TrueType` one: "If the TrueType or OpenType font program's
"name" table contains a PostScript language name, it should be used." The conversion embeds the shipped
face, so the program is in hand: it writes that program's own PostScript name as both the descriptor's
`/FontName` and the dictionary's `/BaseFont` (`described::postscript_name`): the `name` table's name ID 6
of an sfnt face, the Compact Font Format name of a bare CFF face. For the Liberation Sans face every
font of this kind is embedded in, that is `LiberationSans`. Round 1835's first build wrote the face's
description hyphenated instead (`Liberation-Sans-Regular`), a name neither the file nor the program
states; the validator's reading of those two clauses ended it.

**A choice remains only where a program states no usable name** (none, an empty one, or one with a
space or a control byte, which a PDF name should not carry): the shipped face's description with its
spaces replaced by hyphens, a pure function of the face so two runs write the same bytes. The fallback is
written down rather than left as a failure.

What it costs: a reader that looks a font up by `/BaseFont` finds a name the producer never wrote,
though one the embedded program states; the `xmpMM:History` clause and the report both say the file
stated no widths for the font, and the loss `layout-from-this-viewer` names the whole substitution.
The test `the_name_supplied_is_the_postscript_name_the_embedded_program_states` reads name ID 6 from the
program the output carries and compares.

Where the file states a `/BaseFont` that is a name, `/FontName` is its bytes and the dictionary's entry
is untouched (ADR 2504). Where it states one that is not a name, the font stays refused: replacing the
entry is not an interpretation the standard defines.

## 2. The two that stay refused

A687's Owes line allowed that the two misspelt dictionaries "convert under the same loss where the
viewer draws their page, or are named". Both are named and neither converts:

- `T02-04-01_006_resources_subtype_wrong.pdf` states `/BaseFont /Times-Italic` under `/Subtype /Type7`.
  The standard defines no such subtype; §9.6's simple fonts are `Type1`, `MMType1` and `TrueType`, and
  reading `Type7` as `Type1` is a guess about what the producer meant.
- `issue17929.pdf` spells the descriptor key `/GontDescriptor` and states `/Widths`. The descriptor
  and its `FontFile2` program exist under the misspelt key, so the file carries a program the output
  could use; but reading `/GontDescriptor` as `/FontDescriptor` is again a guess, and the font states
  widths that were written for that program rather than for the face this conversion would embed.

A48 forbids the guess, and the standard defines neither misspelling, so no remedy that reads the
misspelt text as the correct text can be argued from the specification. This is the same boundary the
other `NotBuiltYet` sentences name (`NOT_A_SIMPLE_FONT`, `WIDTHS_WITHOUT_A_DESCRIPTOR` in `described.rs`),
and the census names each by document.

**The owner's stance of 2026-10-27, as the round's brief relays it, is recorded and nothing of it is
built.** Failing an elegant remedy that reads the misspelling, the loss could be stated on a page
appended to the output under the profile, the construction of ADR 1014: a page composed solely of what
the document already holds, saying that a font the file named could not be converted and what it was.
That page carries a statement and no mark of the producer's, and ADR 1014 names what an appended page
owes (page labels, the outline, the structure tree, a sentence in the report). It is a different
mechanism from this ADR's, it changes which documents convert only by turning a refusal into a
conversion with a statement on it, and it needs its own round and its own census. A687 asked for the
loss on a font the viewer lays out, which is built.
