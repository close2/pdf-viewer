# 3047 — A font outside the fourteen stating `/Widths` and no descriptor converts by its own widths

Session 2087. Status: **accepted and built.** In `pdf-transform`: `described::Layout::TheFiles` and `layout_of`;
`WIDTHS_WITHOUT_A_DESCRIPTOR` is removed. Supersedes ADR 2504's second refusal. Context: ADRs 2504 and 2505 (a font
naming a face with no widths), 2898 (the order where no widths are stated), 2522 and 2938 (the fourteen stated by name
alone), 3046 (a code such a font gives no width). Clauses: ISO 32000-2 §9.2.4, §9.5, §9.6.2.1 (Table 109), §9.8.1;
ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.5.

## The reading

Table 109 makes `/FontDescriptor` "Required; optional in PDF 1.0-1.7 for the standard 14 fonts", so a simple font
outside the fourteen that omits it is missing an entry the standard requires. What the standard still says of it:

- **Its widths govern.** §9.2.4 positions glyphs "according to their widths", and `/Widths` is where Table 109 states
  them; nothing makes them depend on the descriptor, which describes "the font's metrics other than its glyph
  widths". A code outside the range has no `/MissingWidth` to read (ADR 3046).
- **Substitution is the processor's.** §9.5 says a font dictionary carries "information that can be used to provide
  a substitute when the font program is not available", and §9.8.1 that the descriptor's metrics let a processor
  "synthesise a substitute font or select a similar font". Neither says which face, nor what to do when the
  descriptor itself is absent. **That is what the standard leaves to the processor**, and this tree's answer is
  unchanged and stated as a choice: the request is derived from `/BaseFont` alone (`substitute::Request::derive`,
  `doc/questions/A687`), the face is chosen by it, and the face is drawn to the file's widths (ADR 0358).

ADR 2504 refused this font in the archive because "its layout is the file's own and the face embedded is not the
one the widths were stated for". The first half is why it needs no loss, and the second is equally true of every
descriptor-bearing font that embeds no program, which the descriptor route converts. So the refusal rested on
nothing the clause says.

## Decision

`layout_of` gives such a font `Layout::TheFiles`: `describe` embeds the shipped face as for a font that states
nothing, writes a descriptor measured off it under the file's `/BaseFont` (or the program's PostScript name where the
file states none), keeps every width the file stated as the object it wrote (`keep_stated_widths`), and states for the codes outside the range the advances the
window laid out (ADR 3046), so the 256 advances read back are the window's. **No `layout-from-this-viewer` loss**:
the layout is the file's. The face's own losses stand, and where the face's advances are not the file's widths
`advances_are_the_faces` still refuses by name (`WIDTHS_NOT_THE_FACES`, its sentence no longer claiming the font is
one of the fourteen); restating a shipped face to arbitrary widths is not built here.

**ADR 2522's conversion-defined fourteen need nothing new.** A name the conversion defines is a `Type1` font with a
standard `/BaseFont` and no `/Encoding`, so its codes take `StandardEncoding`, every name of which the shipped face
holds; its empty slots are ADR 2948's. It is `Layout::Standard` and was already converted.

## Proof

`layout_from_this_viewer.rs`: `a_font_stating_widths_but_no_descriptor_keeps_them_beside_the_descriptor_it_is_given`
(`Type1` and `TrueType` `/Arial` at Arial's widths, 2b and 4, no layout loss, the file's widths read back) and
`a_font_stating_widths_but_no_descriptor_moves_no_code` (every code's advance equal before and after); veraPDF
1.30.3 passes the four outputs. Planted: `layout_of` returning the old refusal fails both.
