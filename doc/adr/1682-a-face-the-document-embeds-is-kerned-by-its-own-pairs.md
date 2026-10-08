# 1682 — A face the document embeds is kerned by its own pairs

Session 1424. Status: **accepted** and **built**. Builds the property ADR 1660 section 7 left
named by `Owed::RichTextUnapplied`, for every face but §9.6.2.2's fourteen, which wait on
`doc/questions/Q308`; amends ADR 1660 section 7, which is not edited. Context: ISO 32000-2
§12.7.4.3, §9.4.3, §9.4.4, Table 231 bit 25; XFA 3.3 chapter 27 (*Kerning*, pages 1203 and 1204)
and the version 2.8 change list (*Pair kerning support*), held and cited by section; ISO/IEC
14496-22 (OpenType), `GPOS` chapter (*Lookup type 2: pair adjustment positioning*, the
introduction's advance and placement conventions) and `kern` chapter, fetched for this round from
Microsoft's published text and not held. All cited and paraphrased, none quoted except where the
code quotes the format's one sentence on a consumed second glyph.
Code: `crates/pdf-font/src/pairs.rs` (the reader), `crates/pdf-font/src/loading.rs`
(`LoadedFont::pairs`, `pair_adjustments`), `crates/pdf-font/src/embedding.rs` (`with_tables`, the
fixture's way in), `crates/pdf-model/src/rich_text/layout.rs` (`kern`, `Atom::kern`, `adjustments`,
the line-end rule), `crates/pdf-model/src/rich_text/style.rs` (`Kerning`),
`crates/pdf-model/src/variable_text.rs` (`show_adjusted`).

## 1. What the property asks, and whose numbers answer it

§12.7.4.3 hands a rich text field's formatting to XFA 3.3, and chapter 27 names `kerning-mode` with
two values, `none` and `pair`; the change list defines pair kerning as kerning by the two adjacent
glyphs alone. The numbers are the face's: a font program states its pairs, and nothing in either
standard states pairs of its own. So a run is kerned by the program of the face each glyph is
drawn in, where that program is the document's — a substituted face's pairs would be another
typeface's, which is the whole of Q308's question about the fourteen.

## 2. Where a program states pairs, and how each is read

- **`GPOS`'s `kern` feature**, its pair-adjustment lookups (type 2, and type 9's extension of it),
  both subtable formats. Within a lookup the first subtable that states a pair applies; a pair
  whose second value format is not empty takes its second glyph out of the next pair, as the
  format says; lookups apply one after another over the run, in lookup-list order.
- **The `kern` table**, version 0, formats 0 and 2: values add up across subtables and an override
  subtable replaces the sum. Its pairs are a left-hand and a right-hand glyph, so read right to
  left the second glyph of the logical pair is the left-hand one.

**Applied as the format states it**: an `XAdvance` changes the room between a glyph and the glyph
after it in logical order — the `GPOS` introduction states that in right-to-left text the second
glyph begins at minus the first one's advance — and an `XPlacement` moves the glyph alone, in
Cartesian `x` whatever the direction. In the appearance stream the room stands after the glyph as
it is displayed, or before it where it reads right to left, and both are §9.4.3's `TJ` numbers in
thousandths of text space, which §9.4.4 scales by the size and `Th` and by nothing else — so the
measured advance and the drawn one are one formula.

## 3. Choices, each named

1. **`GPOS` before `kern`**: a program stating a `kern` feature is not read in its `kern` table.
   OpenType makes `GPOS` the only way a CFF-flavoured program kerns and states no order otherwise.
2. **The feature by tag alone**, every script's lookups together, as `pdf_font::vertical` reads
   `vert`: a PDF states no script or language system for a field's text. The cost, named: a
   program registering two different `kern` lookups for one pair under two scripts applies both.
3. **Only pair adjustment.** A contextual lookup under the feature positions by context, which is
   not the two adjacent glyphs the change list names; it is not applied and not said.
4. **No device tables or variation deltas**: they are adjustments at a pixel size or a design
   location, and an appearance stream is laid out at neither.
5. **Within a run, within a face.** A pair is two glyphs of one styled run set in one face; a
   character a later face of the search path draws pairs with nothing, and neither does a tab.
6. **A comb takes none**: Table 231 bit 25 divides the field into equally spaced positions.
7. **A line's last glyph keeps no room**, nor the last before the spaces a line end trims: the
   glyph it was kerned against is not beside it. Breaking measures the line as it would end.

## 4. What is said rather than done

`Owed::RichTextUnapplied` names `kerning-mode:pair` with the reason wherever a run asks it of a
face that gives none — a face this program chose (the fourteen, a stand-in), a bare CFF or Type 1
program, a program with neither table, an unreadable one, one over the budget below — and a pair's
vertical value (`YPlacement`, `YAdvance`, which a horizontal `TJ` cannot place) or a `kern`
subtable of minimum or cross-stream values is said the same way. The parse still notes the
property, so a reader that sets no glyphs — a host's popup window (ADR 1666) — keeps saying it;
the layout takes the note back and says only what it could not kern.

## 5. Bounds and cost

Nothing at load: the pairs are read the first time a run asks, once per face per layout. A
program whose `kern` feature or `kern` table reaches more than 512 pair subtables
(`pdf_font::pairs::MAX_SUBTABLES`) is refused, because each pair visits every subtable at worst.

## 6. Evidence

The fixtures are Liberation Sans with its `GPOS` and `kern` taken out and one known table written
in (trap 8): ten unit tests in `pairs.rs`, one planted defect (the consumed-second rule) failing
the test written for it; `tests/rich_text.rs` draws `AVAV` with `A V` at −300/2048 and reads
`[(A) 146.48438 (VA) 146.48438 (V)] TJ` and 3 to 4 pixels less ink, and the page was looked at.
The corpus was not censused for `kerning-mode:pair` this round; the fixtures are the defence.

## 7. What stays

§9.6.2.2's fourteen, until Q308 is answered — the three rows stay `partial` on that alone.
