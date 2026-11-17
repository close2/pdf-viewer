# 2938 — The pack answers a `TrueType` dictionary re-filed as `Type1`, and a standard font stated by name alone

Session 2062. Status: **implemented.** Context: ADR 2934 (the pack, whose section 3 left this unanswered), ADR 2630
(requirement 2), ADR 2147 (a dictionary re-filed for a shipped face of the other kind), ADR 2278, ADR 2146 (the
fourteen's widths stated), ISO 32000-2 §9.6.2.1's Table 109, §9.6.3, §9.6.5.4 and §9.9's Table 124, ISO 19005-2
section 6.2.11.

## 1. The question

A `TrueType` font dictionary that states a descriptor and no program, whose shown codes only the pack's face draws
(a `TimesNewRoman` showing a beta), was refused by Table 124's sentence: the pack's faces are bare CFF programs, and
`row_for` restated the dictionary's kind for a shipped or a machine's face only.

## 2. What the text admits

- Table 124's `FontFile2` row is a "TrueType font program", which "shall include these tables: "glyf", "head",
  "hhea", "hmtx", "loca", and "maxp""; its `FontFile3 /OpenType` row puts a program under a `TrueType` dictionary
  only "if the embedded font program contains a "glyf" table". A bare CFF program goes under no key of a `TrueType`
  dictionary. Its row is `FontFile3 /Type1C`: "This entry may appear in the font descriptor for a Type1 or MMType1
  font dictionary".
- §9.6.3: "A TrueType font dictionary may contain the same entries as a Type 1 font dictionary", the differences
  being the `/Subtype`, how `/BaseFont` is derived, and the program's key. So the same dictionary under
  `/Subtype /Type1` states the same widths, encoding and descriptor, and nothing else in it moves.
- ADR 2630's requirement 2 is the mirror: a `glyf` face is refused for a `Type1` dictionary, and Liberation is
  re-filed as `TrueType` instead (ADR 2147). Re-filing in the other direction is what `restated_kind` already does
  for the Foxit faces, which are bare CFF too.
- ISO 19005-2 section 6.2.11 asks an embedded program of every rendered font and that its dictionary and program
  agree with ISO 32000; its section 6.2.11.6 rules on a `TrueType` font's encoding, which a `Type1` dictionary is
  not bound by. Nothing in it forbids a converter to state the kind its program is.

## 3. The decision

**Build it.** A `TrueType` dictionary given a pack face is re-filed as `Type1` with the face under `/FontFile3
/Type1C` (`described::pack_restated`): `/Subtype /Type1`, and where the file states no `/Encoding`, the source's own
glyph names as a `/Differences` encoding, because §9.6.5.1 gives a `Type1` font with none the program's built-in
encoding where §9.6.5.4 gave the `TrueType` one a different route. The report's word is `restated-type1`, as for a
shipped face. The proof is the pack's (`pack_face_proved`), read back with the restated entries in: every shown code
selects the pack face's own glyph, and all 256 advances are the source's. `restatement_proved` is not asked, because
it compares with the window's drawing of the shipped face, and the window did not draw the pack face; the loss is
`machine-face`, as on the `Type1` route.

## 4. The standard fourteen stated by name alone

The second dictionary ADR 2934 left: one of the fourteen with no `/FontDescriptor` (Table 109 permits it), showing a
code the shipped face lacks. `described::describe` embeds the shipped face there and holds it to the window, so such
a page was photographed. Nothing in the text separates this case from the descriptor route: ISO 19005-2 section
6.2.11.4.1 asks a program of the font either way, and Table 109's `/FontDescriptor` row makes `/FirstChar`,
`/LastChar`, `/Widths` and the descriptor "all be present or all be absent" for the fourteen, which ADR 2146 already
answers by stating the widths the page is laid out by. **Built** (`described::from_pack`): where the shipped face
lacks a shown code and the pack's face draws them all, the face is restated to the font's own advances, a descriptor
is measured off it as `Measured` measures a shipped face (`/Flags`, `/FontBBox`, `/ItalicAngle`, `/Ascent`,
`/Descent`, `/CapHeight`, `/StemV`), the widths are stated, and the pack proof and ADR 2939's subset follow, under
`machine-face`. Only the standard's layout takes it: a font outside the fourteen with no descriptor is laid out and
named by this viewer (ADR 2898), and the name the shipped face gives it is not the pack face's; it keeps `describe`.
A code the standard metrics give no width keeps the width the window laid it out at, which is zero.

## 5. Evidence

The fixture of ADR 2934 as `/Subtype /TrueType /BaseFont /TimesNewRoman` converts under `as-if-printed` at 2b and
4 with the three codes drawn by Noto Serif Regular under a `Type1` dictionary; veraPDF 1.30.3 passes both
(`tests/archive/lgc_pack.rs`, `a_truetype_dictionary_is_refiled_as_type1_for_the_packs_bare_cff_face`). A
`Times-Roman` and a `Courier-Bold` stated by name alone, showing a-ogonek, n-acute, z-dot-above and l-slash,
E-ogonek, z-caron, are written as text where the page was photographed, and pass veraPDF at both parts
(`a_standard_font_stated_by_name_alone_takes_the_packs_face_and_a_descriptor_of_its_own`). The plants: the pack arm
of `row_for` given back Table 124's refusal fails the `TrueType` test, and the new route switched off fails its own.
