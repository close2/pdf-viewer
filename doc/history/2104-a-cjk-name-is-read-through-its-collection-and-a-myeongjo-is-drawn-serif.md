# 2104 — A CJK font name is read through its collection, a Myeongjo is drawn serif, and a missing descriptor is said

Session 2104, a pixels round in the gwg worktree on four pdf-font leftovers (records 2062, 2064, 2100). ADRs 3117-3119.

**Premises.** Items 1 and 2 held (`issue3521`'s GBK 黑体 folded to nothing; Adobe-KR: Noto Sans CJK 21 061, Serif
21 014). Item 3 had moved: round 2067 (ADR 2948) and round 2087 (ADR 3046) advance an empty slot by the painted
`.notdef` wherever no descriptor states `/MissingWidth`; a descriptor keeps Table 120's entry, stated or 0. Nothing
built. Item 4 held in part: the font was drawn (substitute by name, its `/Widths`) and nothing was said when it stated
`/Widths`.

**Item 1 (ADR 3117).** §7.3.5: a `/BaseFont` read as text "should be interpreted according to UTF-8". A name that is
not UTF-8 is, by a documented choice, read through Table 116's byte-encoding `CMap`s of its collection and the
collection's `-UCS2` table (`substitute/native_name.rs`); a reading counts where it names a serif or sans style and no
reading disagrees. The name decides before the flags. `issue3521` now draws "我们都是黑体字" in sans (looked at).

**Item 2 (ADR 3118).** `Repertoire::breadth` counts a collection's BMP characters: both Noto families map 21 001 of
Adobe-KR's, so the family decides and the 47 supplementary ideographs are borrowed (ADR 3067) where shown. Measured
over every catalogue face: only Adobe-KR's order changes; Droid Sans Fallback stays behind in every collection.

**Item 4 (ADR 3119).** Table 109: `/FontDescriptor` "Required; optional in PDF 1.0-1.7 for the standard 14 fonts".
Drawing unchanged (a documented choice on a malformed file); `LoadedFont::states_no_descriptor` and pdf-model's
`note_missing_descriptor` say it, and the corpus walk classes it as the file's.

**Tests and plants.** `native_name`'s three, `a_cjk_name_in_its_collections_encoding_names_its_style`,
`a_collections_breadth_is_counted_in_the_basic_multilingual_plane`,
`a_font_outside_the_fourteen_stating_no_descriptor_is_found`. Planted (reading off, the BMP bound removed, the
fourteen's exemption dropped): each new test fails; restored, all pass.

**Golden.** Census of the 974 golden files (Sonnet): non-ASCII names in `SimFang-variant`, `XiaoBiaoSong`,
`issue20489`, `issue2128r`, `issue3521`, `issue5010` (none UTF-8); no Adobe-KR font. Each moves only where it is a
composite font of a collection and the read style differs from the flags. Not edited.

**Gates.** Over `-p pdf-font -p pdf-model -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0;
`pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 100 (4556 of 4557: `rich_text`'s pair-kerning fixture, a
descriptor-less `/Arial`, now also drew the new note; given a descriptor, `forms_suite` 392 of 392); rustdoc 0;
conformance 101 on this record's gates and `batch`'s warm-build test under load (alone: 1 passed), then 0. Fuzz not
owed: two methods added, no signature changed.

**Left.** Romanised sans names (`SimHei`, `MS Gothic`) as sans requests; the Type 0 `/Encoding` as the reading's
first `CMap`; a page with an Adobe-KR Myeongjo to look at.
