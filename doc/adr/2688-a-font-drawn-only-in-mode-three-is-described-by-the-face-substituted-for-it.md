# 2688 — A font drawn only in mode 3 is described by the face substituted for it

Session 1934, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/fonts/described/undrawn.rs` (`undrawn`,
the proof and its refusals), `archive/fonts/described.rs` (`Measured::descriptor` names a program
only where one is embedded), `archive/font_widths.rs` (`laid_out` takes the route instead of
refusing; `WidthArrays::written`), `archive/prepare.rs` (the descriptors among the added objects),
and the stated interpretation's sentence in `archive/decision/fonts.rs` and `archive/rewrite.rs`.
Tests: `type1_widths::a_standard_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_four`
and `type1_widths::a_truetype_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_both_parts`.
Context: ADR 2686 (the row, and the refusal by name this replaces), ADR 2687 (the published
widths), ADR 2146 (the embedding route's descriptor, measured from the shipped face), ADR 2504 and
`doc/questions/A687` (a non-standard name laid out by this viewer).
Clauses: ISO 32000-2 §9.3.6 (Table 104), §9.6.2.1 (Table 109), §9.8.1 (Table 120), §9.8.2
(Table 121); ISO 32000-1 Table 111; ISO 19005-2 sections 6.2.11.2 and 6.2.11.4.1 (NOTE 2), ISO
19005-4 sections 6.2.10.2 and 6.2.10.4.1.

## Context

ADR 2686's row holds a used simple font to Table 109's four entries; its remedy writes the widths
where a descriptor exists, and the embedding route writes all four where it embeds a face. A font
used only in rendering mode 3, which Table 104 describes as "Neither fill nor stroke text
(invisible).", is not rendered, and 6.2.11.4.1's NOTE 2 exempts it from embedding, so no route
reached it and it was refused by name: seven census documents, OCR layers over scans, in a
standard name at part 4 and in `TrueType` `Arial`, `TimesNewRoman` and `CourierNew` at both parts.

## Decision

1. **The dictionary is given `/FirstChar`, `/LastChar`, `/Widths` and a `/FontDescriptor`, and no
   program.** The descriptor is an object of its own, since Table 109 makes the entry an indirect
   reference.
2. **The widths are the advances this reader lays the glyphs out by** (ADR 2686 decision 2): for
   one of the fourteen the published metrics §9.6.2.1 makes a processor provide; for any other name
   the published metrics of the face its name resembles, which this reader already lays it by.
3. **The descriptor is the face this program ships for the font, measured as ADR 2146 measures
   it.** §9.8.1 says what a descriptor is for when the program is absent: its metrics "enable a PDF
   processor to synthesise a substitute font or select a similar font when the font program is
   unavailable". The face is the substitute this reader selects, compiled in, so the output is the
   same on every machine. `/FontName` is the dictionary's `/BaseFont` (Table 120); `/FontBBox`,
   `/ItalicAngle`, `/Ascent`, `/Descent`, `/CapHeight` and `/StemV` are the face's; `/Flags` is
   Table 121 read off the same face: `FixedPitch` where its program says so, `Serif` for the Times
   face, `Italic` where its angle is not zero, and `Symbolic` for Symbol and ZapfDingbats or
   `Nonsymbolic` otherwise. The tree ships the fourteen's published *widths* only, not the AFM
   header values, so for the fourteen too the descriptor's other numbers are the shipped face's.
   Where no shipped face answers the font, or it cannot be measured, or the font has no
   `/BaseFont` name, there is no value to derive and the font is refused by name.
4. **Proved, not trusted.** The dictionary as written is loaded beside the source, and each of the
   256 codes must advance by the same amount, bit for bit, and read as the same text (§9.10.2's
   route through `/ToUnicode` or the glyph name). A descriptor can change both (its flags take
   part in choosing the substitute and the encoding), and an invisible layer is there for its text
   and where it sits. A difference is refused by name.
5. **A stated interpretation, not a loss, and not "none visible".** No mark moves, because mode 3
   paints none, and nothing of the file is dropped. But the source is a dictionary the base
   standard does not admit, and the output states this reader's choice of layout and substitute;
   another reader might have placed the invisible text by another face. That is ADR 2686 decision
   3's situation, and its row's sentence now says so. `layout-from-this-viewer` (A687) is not
   charged: it names a layout a reader *sees* drawn in a face this conversion embeds, and here
   nothing is drawn and nothing is embedded.
