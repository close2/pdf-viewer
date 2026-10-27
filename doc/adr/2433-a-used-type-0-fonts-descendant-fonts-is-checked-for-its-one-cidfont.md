# 2433 — A used Type 0 font's `/DescendantFonts` is checked for its one `CIDFont`

Status: implemented. Session 1799, the archive slot of batch one hundred and twenty. Context: ADR
2413, whose premise half on `issue12823.pdf` did not hold and was left owed; ADR 2178, the earlier
narrow row split out of the same `Unchecked` sentence; `doc/todo/72`'s owed paragraph.

## The gap

ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2 require the fonts used in a file, in
whatever rendering mode, to conform to the base standard's font clauses. Its row here,
`fonts/font-programs-conform-to-their-own-specifications`, is `Unchecked`, because judging a font
program against sfnt, CFF or Type 1 is a validator nothing in this tree is. One part of the
sentence needs no such validator: ISO 32000-2 §9.7.6.1's Table 119 makes `/DescendantFonts`
required of a Type 0 font, a one-element array naming its `CIDFont`, and §9.7.4.1's Table 115 makes
`/Subtype` `CIDFontType0` or `CIDFontType2` what says a dictionary is one. `issue12823.pdf` writes
`[ null ]`, and §7.3.10's null is no dictionary; HEAD converted it into files veraPDF 1.30.3 fails
at 2b and 4.

## The decision

1. **A new implemented row**, `fonts/descendant-fonts-holds-one-cidfont`, under the same two
   clauses, carried beside the `Unchecked` row and the widths row in `coverage.rs`. It reports a
   used Type 0 font whose entry is missing, is not an array, holds other than one element, or holds
   an element that is not a dictionary with one of the two `CIDFont` subtypes.
2. **Its population is the fonts a text-showing operator ran with**, `Survey::fonts`, in any mode —
   not every font dictionary the table names. The clause binds the fonts *used*, whatever the mode
   they are shown in. The first version walked every font dictionary and refused
   `PDFBOX-5784.pdf`, whose Type 0 font with no `/DescendantFonts` only the form's `/DR` names and
   no content shows; HEAD's output of it passes veraPDF at 2b and 4, which agrees with that reading
   rather than defining it.
3. **The converter refuses it by name** (`NO_CIDFONT_TO_DESCEND_TO`, `NotBuiltYet`). A font with no
   `CIDFont` states no glyph selection, widths or character collection, so nothing can be restated;
   building one is the limits document's section 4.9 substitution for a font with no dictionary to
   substitute into, which is not built.

## What it costs

One pass over the survey's fonts, which the embedding row already walks. `issue12823.pdf` is
refused at 2b and 4 under both shipped profiles where HEAD wrote a file veraPDF fails; no other
document of the census moves on this row (`doc/history/1799`). The general sentence's remainder
stays `Unchecked`, and the widths row keeps its own, wider population.
