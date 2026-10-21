# 2146 — A standard font stated by name alone is given the face it is drawn from, with its descriptor and widths

Session 1655, the archive slot of batch one hundred and two. Status: **accepted**, **built** in
`pdf-transform`. It **answers** the largest refusal round 1647's census of the real corpora found
(ADR 2130, `doc/history/1647`). Context: ADRs 0133, 0270, 0396, 1209, 1441; `doc/questions/A47`;
`doc/pdf-a-conversion-limits.md` section 4.9. Code: `crates/pdf-transform/src/archive/fonts/described.rs`,
`fonts::embed_faces`, `Rewriter::describe_font`, `Described`.

## 1. What was refused

ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 want the program of every rendered
font embedded, and section 6.2.11.4.1's NOTE 5 takes no exemption for the standard 14. ISO 32000-2
Table 109 lets one of §9.6.2.2's fourteen state no `/FirstChar`, `/LastChar`, `/Widths` or
`/FontDescriptor` ("optional in PDF 1.0-1.7 for the standard 14 fonts"). A program goes into a
descriptor, so `embed_faces` refused such a font. Its sentence said that `/StemV` was a measurement
nothing in the file states. At 2b that refusal stopped 192 pdf.js documents and 168 of `doc/corpora`'s.

That premise did not hold. Table 120 itself says of `/StemV`: "A value of 0 indicates an unknown stem
thickness". ADR 1099's appended-page font already writes a descriptor from a program. Every other
entry Table 120 requires is a statement the program makes or a measurement of its outlines.

## 2. Decision

**The face the viewer draws the font from is embedded unmodified.** Three things are written
beside it. A descriptor read off that program. The widths the page is already laid out by. And,
where the face asks for one, the dictionary kind it is.

- **The descriptor** (Table 120). `/FontBBox`, `/ItalicAngle` and the fixed-pitch flag are the
  program's own statements: a CFF Top DICT's, or an sfnt's `head` and `post`. `/Ascent`, `/Descent`
  and `/CapHeight` are measured exactly with `kurbo` over the outlines Table 120 names. For a Latin
  face that is the glyphs whose names the Adobe Glyph List maps to a printable ASCII character,
  which leaves out every accented character, and `H` for the capital. For the two symbolic faces it
  is every glyph. `/StemV` is the CFF Private DICT's `StdVW`, and 0 for an sfnt. `/Flags`
  (Table 121) carries Serif for Times, FixedPitch where the program says so, Italic for a non-zero
  angle, and Symbolic for Symbol and ZapfDingbats, Nonsymbolic otherwise (ADR 1099's reading).
  `/FontName` is the `/BaseFont`, as Table 120 requires. §9.6.3's TrueType name rule is a
  "should", and the dictionary's name is kept.
- **The widths.** Table 109's descriptor row: "For the standard 14 fonts, the entries FirstChar,
  LastChar, Widths , and FontDescriptor shall either all be present or all be absent." Writing the
  descriptor therefore obliges the widths. Those written are `LoadedFont::advance` of the source
  for every code from the first to the last non-zero one, so the advances are the standard metrics
  the viewer laid the page out by (`pdf_font::standard_metrics`). A file that states `/Widths`
  keeps them. Route 1 only: a face whose own advance disagrees with a shown code's by more than
  section 6.2.11.5's thousandth of an em is refused by name.
- **The kind.** The ten CFF faces go under the `Type1` dictionary as `/FontFile3 /Type1C`. The four
  Liberation faces are `glyf` programs, and Table 124 opens no key to one under `Type1`, so the
  dictionary becomes §9.6.3's `TrueType` with `/FontFile2`. ISO 19005-2 section 6.2.11.6 wants a
  non-symbolic TrueType font's encoding to be `MacRomanEncoding`, `WinAnsiEncoding` or a dictionary
  on one. Such an encoding is carried untouched. Any other is restated as a `WinAnsiEncoding`
  dictionary whose `/Differences` give every code the glyph name it had. §9.6.5.1 makes an encoding
  exactly that table of names. A symbolic CFF face is restated the same way with no base, because
  §9.6.5.1's base is then the program's built-in encoding, and Foxit's Dingbats program's is not
  Annex D.6's.

**It is proved before it is written.** `described::proved` serialises the converted font, its
descriptor and the program into a document of their own. It loads that font through
`LoadedFont::load` and holds it to the source font. Every code the page shows must give the same
outline. The comparison undoes the one change the viewer makes to a substitute: it winds the
substitute counter-clockwise, and leaves an embedded program as drawn (ADR 0396). All 256 codes must
give the same advance, and the scale outlines are drawn at must be the same. The converted font
must be drawn from its own program. A font that fails is refused by name.

**Refused by name**, each a sentence in `described.rs`: a font that is no object (the corpora's
Isartor-style test files write it inside the resource dictionary); a font that is not a `Type1`
naming one of the fourteen; a font the viewer draws from a machine face instead (ADR 0270), since
that face is not ours to embed; a face whose advances are not the widths; a program the operator
named with `--font`, since the descriptor describes the shipped face; and a result that would be
drawn differently.

## 3. The licences

Liberation Sans is SIL OFL 1.1. Its preamble exempts "any document created using the fonts", and it
is embedded unmodified only. The Foxit faces are BSD-3-Clause. Their binary-redistribution condition
asks for the notice "in the documentation and/or other materials provided with the distribution", and
the tree has embedded them on A47's reading without one. `doc/questions/Q550` puts that reading to
the owner. It also names a finding about the older route, which restates Liberation's advances and
keeps its reserved name.

## 4. What it costs, measured

- **Pixels.** A CFF face draws identically: `IdentityToUnicodeMap_charCodeOf.pdf` (Times-Roman) is
  byte-identical at 150 dpi. A Liberation face is wound clockwise once embedded, and the CPU
  rasteriser's coverage differs at a few anti-aliased edge pixels by one level of 255.
  `4PP-Highlighting.pdf` at 100 dpi: 1 pixel. `NegativeFontSize.pdf`: 22 pixels. Every embedded
  TrueType program in the tree is drawn this way (ADR 0396).
- **Selection geometry.** `/Ascent` and `/Descent` are now stated, so a highlight's height is
  theirs and no longer the em box. That is not a mark on the page.
- **The census depends on the machine.** ADR 0270 chooses a machine face over the compiled-in one,
  so the count of "another face is drawn" refusals (36 in `doc/corpora` at 2b) is this machine's.

The real corpora at 2b under `as-if-printed`: pdf.js 618 → 750 converted and 336 → 204 refused;
`doc/corpora` 179 → 260 and 290 → 209. ADR 2147's group is included; its rows are the Table 124
sentences, and every other font row of the census is this ADR's. The veraPDF corpus census is unchanged: 2b 525/86, 4 266/93.
