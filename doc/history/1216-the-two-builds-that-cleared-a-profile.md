# 1216 — The two builds that cleared a profile

The corpus-named round, on `doc/todo/66`'s last two answers `only-metadata-loss` owed at part 4.
`tools/state.sh remedies` now reads 0 for that profile at all six targets.

- **`/Info` moves into the packet** (ADR 1269), answering ISO 19005-4 section 6.1.3's two rows.
  §14.3.3 deprecates the dictionary and Table 349's NOTEs name the XMP counterpart of every key, so
  the values move and the dictionary goes. `pdf_model::xmp::supplement` is written beside `restate`
  — which was the wrong tool, since it removes a namespace before writing into it — and is
  **additive by construction**: §14.3.4 permits an addition only into a silence, so the writer reads
  what the packet already states and has no way to overwrite. The shapes are not a choice either:
  ISO 19005-2 section 6.6.2.3.1 makes a `dc:title` written as a simple value fail the converter's
  own output, and §14.3.3's EXAMPLE prints the `rdf:Alt` and the `rdf:Seq`. A catalog stating a
  `/PieceInfo` keeps `/Info` holding `/ModDate` alone — the clause's carve-out, §14.5's reason —
  and then §14.3.4's fourth `shall` binds, so a date the packet contradicts refuses by name.
- **The attachment machinery** (ADR 1270), answering `original = "attach"` and `keep-xfa =
  "attach"`. `crates/pdf-transform/src/archive/attach.rs` files a producer's XMP packet or XFA
  resource as an embedded file at PDF/A-4f and 4e, through `filing.rs` — the one writer of
  §7.11.4's stream, §7.11.3's specification and §7.9.6's `/Names` node — with §14.13.3's catalog
  `/AF` beside it. Annex K is what says the array form of `/XFA` is its streams end to end. Which
  targets admit the answer is read off the requirement table rather than a flavour list, so a
  configuration asking for it elsewhere is an error naming both.

Two findings the round did not expect:

- **§14.13.2 states the media type outright** — "[i]f the MIME type is not known, the value
  ' application/octet-stream ' shall be used" — and the same sentence answers `/Params` `/ModDate`
  from the file. A documented choice was half-written before the clause was read; `CLAUDE.md`'s own
  rule about recording a silence, at a site where the silence had not been written down yet.
- **`unmapped = "extension-schema"` cannot be built**, and the catalogue offered it in error. ISO
  19005-2 section 6.6.2.3.2's container describes a schema *a packet uses*; a `/Info` key is in
  none, so putting one in needs a namespace URI, which is the property's identity rather than a
  label for it. It is recognised, validated and refused by name, with `--remedy-sites` listing it.

Beside them: `embedded-files/pdfa-4f-carries-embedded-files` is answered by the attachment — its
refusal said *attaching one would be adding content no source states*, which is not true of a
conversion that files one out of the document's own bytes; `keep-everything.toml`'s `/XFA` row is
target-qualified, because the mechanism is; and `config.rs`'s
`a_target_qualified_row_wins_however_late_the_file_states_it` reads which *mechanism* the winning
row asks for, now that counting unbuilt answers no longer discriminates between them.
