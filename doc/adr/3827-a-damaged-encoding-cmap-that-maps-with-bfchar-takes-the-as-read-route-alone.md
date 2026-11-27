# 3827 — A damaged encoding `CMap` that maps with `bfchar` takes the as-read route alone

Session 2281. Status: **accepted** and built in `pdf-transform` (`archive/encoding_cmaps.rs`,
`archive/rewrite/streams.rs`). Amends ADR 3428 item 1 and closes the consequence ADR 3804 left.
Clauses: ISO 32000-2 §9.7.5.3, §9.7.5.4 c), §9.7.6.2; ISO 19005-2 section 6.2.11.3.3, ISO 19005-4
section 6.2.10.3.3, both parts' section 5.1.

## Context

Two findings can fall on one encoding `CMap`. The first is `fonts/encoding-cmap-maps-codes-to-cids`,
for a `beginbfchar` section, which §9.7.5.4 c) reserves for a `ToUnicode` map. ADR 3428 answers it
by restating each section as `begincidchar` and leaving every other byte as it was. The second is
`font/cmap-damaged`, for an entry §9.7.5.3 does not admit. ADR 3804 answers it by writing the whole
map again from its reading. When a map had both, ADR 3428's module either refused the document,
because its own parse of the sections failed or its proof failed on the damage, or it wrote the
restated map first, so the damage was still there for the output's check to refuse by name.

## Decision

1. **The reading answers both.** `pdf_font::cmap` reads a `bfchar` destination as the CID it names
   (ADR 3428). The map written from the reading (`CMap::sections_as_read`) states every mapping as
   `cidchar` or `cidrange`, so no `bfchar` section and no damaged entry is left in it. ADR 3804's
   `reads_alike` proof covers both changes.
2. **`encoding_cmaps` leaves a map it reads as damaged** to `font_maps`. It does not restate the
   sections and it does not refuse.
3. **`FontMapsAsRead` is asked before `EncodingCmapRestated`** when the rewrite replaces a stream. A
   map whose own text reads whole but whose `/UseCMap` base is damaged is still restated by both
   modules. The map written from the reading, with the base folded in, is the one that wins.

## Consequences

A composite font whose embedded encoding `CMap` both maps with `bfchar` and holds an entry outside
§9.7.5.3 converts under both shipped profiles, with the loss `font-maps-as-read`. It reads alike,
and its output holds (`font_as_read::a_damaged_encoding_cmap_that_maps_with_bfchar_is_written_as_read`,
which tests the damage in a section of its own and inside the `bfchar` section). An undamaged
`bfchar` map keeps ADR 3428's restatement, which changes no other byte and costs no loss.
