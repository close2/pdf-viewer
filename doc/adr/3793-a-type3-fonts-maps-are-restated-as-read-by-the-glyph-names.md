# 3793 — A Type 3 font's maps are restated as read, proved by the glyph each code names

Session 2273. Status: **accepted** and built. Context: ADR 3570 (the row and the loss), ADR 3753
(`StandardEncoding` is no predefined encoding), ADR 3781 (a damaged map written as read); ISO
32000-2 §9.6.4, Table 110, §9.6.5.1, Table 112, §D.1; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

`pdf_model::type3::encoding` read a Type 3 font's `/Encoding` or `/BaseEncoding` naming
`StandardEncoding` as no table and logged it (ADR 3753), but read any other name §9.6.5.1 does not
predefine as no table in silence (record 2263). And `archive::font_maps` restated only simple
fonts' names: a Type 3 font naming either was left as written, so the output's own section 5.1
check refused it. Its encoding dictionaries were restated, but `draws_alike` answered `true` for
every font that is not simple, so the restatement was not proved.

## Decision

1. **The reader logs an unknown name** as `font/encoding-name-unknown`, as `pdf_font`'s
   `base_encoding` does for a simple font: any name outside Table 109's three is checked first.
   And **the validator reads every Type 3 font** (`table::fonts::load_every_font`), as the page
   interpreter does: it had skipped them as fonts with no program, so a Type 3 font's readings
   reached its log only where something else interpreted the page, and the font-maps row and
   section 5.1 passed a Type 3 font whose maps were read past.
2. **The converter restates a Type 3 font's maps.** A name the reader reads as no table is written
   as `<< /Type /Encoding /Differences [] >>` rather than removed, because Table 110 requires the
   entry and makes it a dictionary; a `/BaseEncoding` so named is removed; and a `/Differences`
   code past one byte is read past with the names after it, as `type3::encoding` reads it.
3. **The proof is the glyph names.** §9.6.4 draws a Type 3 glyph in two lookups — the code in
   `/Encoding` gives a name, the name in `/CharProcs` gives the description — and only the first
   reads an entry the restatement touches; `/Widths` and `/FontMatrix` are written as they were.
   So `Type3Font::read` of the font before and after must name every code alike, or both refuse
   alike; otherwise the font is refused by name. An encoding dictionary two fonts would restate two
   ways is refused too.

## Cost

A Type 3 font naming `StandardEncoding` drew nothing before and draws nothing after; a reader
holding a table under that name will now see none. That is the loss `font-maps-as-read` already
names, which both shipped profiles authorise.
