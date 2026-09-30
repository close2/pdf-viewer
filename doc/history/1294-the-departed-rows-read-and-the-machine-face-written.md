# 1294 — The thirty-nine departed rows read, and the machine face written into the file

The ledger slot of batch forty-five. ADR 1425.

**The write.** A value set in a face from this machine (ADR 1414) is now saved with that face
subset and embedded instead of owed. `pdf_font::embed::for_embedding` keeps the glyphs `/W` states,
glyph 0 and the composite closure, rebuilds `glyf`/`loca`/`hmtx`, restates `hhea`/`maxp`, carries
`cvt `/`fpgm`/`prep`/`OS/2` and writes **no `cmap`**, because §9.9.1 says one "shall not be present"
under a `CIDFont` (the brief asked for one; the clause is what was built). `/BaseFont` and
`/FontName` get §9.9.2's tag. CIDs stay the face's glyph indices, with a `/CIDToGIDMap` stream where
the subset renumbered. The `OS/2` `fsType` decides the write: restricted or bitmap-only is refused
and owed with its sentence, and no-subsetting embeds the whole face. `variable_text::for_a_file`
builds the file's form at all five save sites. The streams become objects in the update
(`promote_resource_streams`) and in the archive (`streams_numbered`). An added annotation that
cannot be written is now named in `Written::unappeared` rather than dropped silently. Table 121's
flag is `Symbolic`, as §9.8.2 requires; `raster_golden` held all 974 pages.

**Fixture.** `machine_face_written.rs` saves an Arabic field value (`qpdf --check` clean), turns
the machine's fonts off, re-opens the file and draws it: byte-identical to the screen at 2x.
Planting `/Identity` for the renumbered subset fails it. `fsType` 0x0002 on this machine's own face
stays owed. `view.rs`'s field-tree walks now take `MAX_FIELD_ANCESTRY` (256), not 32, so none stops
silently short of where the construction reports.

**The 39 departed rows.** None is (b): no departure turned out to be the cost of something built
since. Twenty-four are (a) as they stand; fifteen are (c) and were rewritten as what is, each
status unchanged: §7.5.5 (the serializer writes `/Encrypt`, ADR 1162); §8.7.4.5.7 and §8.7.4.5.8
(the first sentence now names the departure and ADR 1217); §10.7.4 and §11.6.6 (`render-quorra`
renamed `render-raster`); §10.7.5 (Table 51, not 58); §11.7.5.3 (`/BG`/`/UCR` evaluated since ADR
1207); §12.5, §12.5.6 and §12.7.4 (their children are `departed`, not `partial`/`reported`);
§12.5.2 (the licence decline, ADR 1425); §12.7 (`/CO` is Table 224's, carried by `merge`);
§12.7.4.1 (one bound); §12.7.4.3 (the write, the corpus count); §12.7.8.3.1 (`/Annots` placed,
ADRs 1224 and 1297). §9.9.1 and §9.9.2 name `embed.rs`. The counts are departed 39, partial 10,
reported 4, implemented 665, both before and after.

Left: §10.4.2.3 does not answer §10.3.2's "should follow the method described in 10.4.2.3" (ADR
1207 raised it). Several notes still carry a retired `partial` sentence that a later one corrects
(§11.3.4, §12.3, §12.3.5, §12.3.5.1, §12.7.5.3, §12.7.8.3.3). A `GSUB`-only Arabic face is still
not chosen. `render-quorra` is still named in eight `implemented` rows.
