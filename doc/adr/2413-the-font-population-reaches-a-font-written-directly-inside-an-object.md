# 2413 — The font population reaches a font written directly inside an object

Status: implemented. Session 1789, the archive slot of batch one hundred and nineteen. Context:
`doc/todo/72`'s owed paragraph, which named `Test-plusminus.pdf` and `issue12823.pdf`; ADR 2178 (the
widths-array row); `crates/pdf-archive/src/table/fonts.rs`'s module comment, which stated the gap as
an under-report.

## The gap, and the half of the premise that did not hold

`fonts::for_each_font`, the population thirteen rows of the font tranche walk, was every
`/Type /Font` dictionary that *is* an object the cross-reference table names. ISO 19005-2 and ISO
19005-4 section 6.1.4 bound the population by that table, but what they exempt is an indirect object
no section names — not a dictionary written inside one that is named. ISO 32000-2 §9.7.6.1 lets a
Type 0 font's `/DescendantFonts` hold its `CIDFont` directly, and §7.8.3 lets a `/Font` resource
dictionary hold a font directly; neither is less a font for it.

- **`Test-plusminus.pdf` held.** Its Type 0 font writes its embedded `CIDFontType2` directly into
  `/DescendantFonts`, with no `/CIDToGIDMap`. veraPDF fails the output at 2b on 6.2.11.3.2-1 and at
  part 4 on 6.2.10.3.2-1; `fonts/cid-to-gid-map-present` never saw the descendant.
- **`issue12823.pdf` did not.** Its fault is not a direct font: one Type 0 font's
  `/DescendantFonts` is `[ null ]`. veraPDF's 6.2.11.2-1 and -3 (6.2.10.2 at part 4) fail on the
  missing `CIDFont`'s `/Type` and `/BaseFont`. That is ISO 19005's "fonts conform to §9.6 and §9.7"
  sentence, whose row here, `fonts/font-programs-conform-to-their-own-specifications`, is
  `Unchecked`; this walk does not move it, and the report names it as owed.

## The decision

1. **The population is fonts, not objects.** `Examination::direct_fonts` walks, once per
   examination, the direct structure of every object the table names — a dictionary's values, a
   stream's dictionary's values, an array's items, inward through every direct dictionary and array
   — and keeps every direct `/Type /Font` dictionary with the object holding it. References are not
   followed: what they name is an object of its own and is visited on its own turn. Once rather than
   per row, because thirteen rows read it.
2. **A direct font is reported at no object, its holder named in words** (`CIDToGIDMap, in a font
   written directly inside object 2 0`); a font that is an object of its own is reported at it, as
   before. Naming the holder as the object would be a wrong instruction: the converter reads a
   finding's object as the font to edit (`archive::sites`' `objects_named`, `font_widths`,
   `descriptor_sets`), and the holder is a page, a resource dictionary's object or a Type 0 font.
   With no object each of those refuses by name, and `cid_to_gid_maps` says why in a sentence of its
   own. The cost is section 6.2.2's exemption, which drops a finding by object and so keeps a direct
   font's; that over-reports, the direction `Findings::exempting` already argues for.
3. **Two callers tell the two kinds apart** through `FontSite::own_object`:
   `fonts_naming_an_undefined_cmap`, whose list the converter edits object by object, keeps fonts
   that are objects of their own, and `to_unicode_present`'s glyph-name test matches the survey's
   selection of a direct font by its dictionary, the survey naming such a font by no object.

## What it costs

One walk of every object's direct structure per examination, linear in what the parser built.
`Test-plusminus.pdf` is refused at 2b and 4 under both profiles where HEAD wrote a file veraPDF
1.30.3 fails, and `hesr13613-sup-0001-Authormatrix.pdf`, refused already, gains the same reason. A
qpdf scan finds 94 documents in the corpora holding a direct font; the converter's reports on every
other one are HEAD's word for word, and the cross-check stays at 0 disagreements
(`doc/history/1789`).
