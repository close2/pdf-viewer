# 2342 — A stream-mapped CIDFontType2 over a name-keyed CFF is re-keyed to its glyph indices

Session 1754, the archive slot of batch one hundred and fifteen. Status: implemented. Amends ADR
2326, whose refusal of a stream map it answers. Context: `doc/history/1746` ("Found, not fixed");
`doc/todo/72`.

## The reading

ADR 2326 retypes a `CIDFontType2` over a CFF-flavoured `OTTO` program to `CIDFontType0` where the
map is `Identity`, because Table 124 admits the program only there and §9.7.4.2 then uses each CID
of a name-keyed program "directly as GID values". A map stated as a stream sends CID *c* to the
glyph index in its bytes 2*c* and 2*c* + 1 (Table 115), so under the new subtype the same CIDs
would select other glyphs. What makes them select the same ones is changing the CIDs, and three
statements are keyed by CID: the `Type0` font's encoding (§9.7.6.2: codes to CIDs), `/W` (§9.7.4.3)
and `/W2`. The descendant's program, `/DW`, `/CIDSystemInfo` and `/ToUnicode` (keyed by code) are not.

So `archive::font_files::glyph_indices` restates the two that matter and refuses the third:

- each `Type0` font naming the descendant under `Identity-H` or `Identity-V` gets an embedded `CMap`
  (§9.7.5.3, Table 118), its one codespace the same two bytes, its `cidrange` runs sending every code
  to the glyph index this reader drew it with, and a `notdefrange` onto CID 0 for the codes that drew
  glyph 0 — §9.7.6.2 consults the notdef mappings for a code no character mapping covers. The
  `CMap` states the descendant's own `CIDSystemInfo`, which ISO 19005-2 section 6.2.11.3.1 requires
  of a `CMap` that is no identity, and one `CMap` is written per writing mode;
- `/W` states, for each glyph a code reaches, the advance its codes had, written as the number that
  reads back to the same `f32` in this reader.

**The glyphs and advances are this reader's**, asked of `pdf_font::LoadedFont` for every two-byte
code, not a second reading of the map and the widths: the re-keyed font is the page as drawn.

**Refused, each by a sentence**: another encoding (re-keying a `CMap` that is not the identity is not
built); a `/W2`, or a descriptor's `/CIDSet`, whose keys are the CIDs that change; a `Type0` font
written inside another object, whose `/Encoding` the rewrite cannot reach (every object is searched,
dictionaries inside it included); a code reaching no glyph or one past the program's `maxp` count;
two codes reaching one glyph at two advances, since one glyph index is now one CID with one width.

It stays `Mechanical`, under `FontProgramRefiled`: nothing drawn moves, and the widths array is
re-keyed rather than restated — every code keeps its advance, which is what `archive_corpus`'s third
property protects (`doc/pdf-a-conversion-limits.md` section 4.9).

## What it moved

`bug1186827` converts at 2b and 4 under both shipped profiles: its CIDs are Unicode values whose map
sends 186 of them to glyphs 1–186 in nine runs — the brief's "*c* − 31" is the first run only — and no
glyph is reached at two widths. Page 1 is byte-identical to the source's at 100 dpi and 360, looked at;
veraPDF 1.30.3 passes the four outputs. `archive_corpus`'s
`a_refiled_composite_font_draws_every_code_as_its_source_did` holds every one of the 65 536 codes to the
source's glyph, advance, program advance and outline, and fails with a planted off-by-one in the runs.
