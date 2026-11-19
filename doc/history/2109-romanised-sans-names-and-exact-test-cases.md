# 2109 — A romanised Hei, Gothic, Dotum or Gulim name is a sans request; a witness asks its premise

Session 2109, a pixels round in the lane4 worktree on four leftovers of rounds 2104 and 2107. ADRs 3137, 3138.

**Premises.** Item 1 held (`SimHei` over a `Serif` flag was a serif request, planted below); item 2 was open and is
answered by reading; item 3 held; item 4 held in part: six `powi`/`powf` calls, one round 2107's, one other exact.

**Item 1 (ADR 3137).** `names_a_cjk_sans`, the counterpart of `names_a_cjk_serif`: the Hei, CJK Gothic, Dotum, Gulim
and Meiryo families by name, and ADR 3117's native words. Not the bare `gothic` (`Letter Gothic` is monospaced) nor
`hei` (`Fahrenheit`). Test `a_romanised_hei_gothic_dotum_or_gulim_name_is_a_sans`; planted (the call out of
`family_of`), it fails on `SimHei` with `Serif`; restored, it passes.

**Item 2 (ADR 3138).** Not built. §9.7.6's `/Encoding` maps the show strings, and nothing ties it to the name's bytes
(§7.3.5 says UTF-8). A byte-encoding `CMap` a producer names is a reading already or a narrower form of one, so it
could only break a tie, and Japanese, the one collection with two unrelated encodings, cannot tie on a style word:
`a_japanese_style_word_names_a_style_in_its_own_encoding_only`.

**Item 3.** `composite_without_a_machine_face` asks `LoadedFont::machine_offers_a_substitute` of every page font with
its program dropped (`without_program`), per witness, before drawing; a witness without one prints "skipped: <path>:
this machine offers no face composite font /<name> can be drawn from without its program". Here all four run and pass;
calibrated by `cargo test --test text_suite -- --test-threads=1`, where `machine_face_written` turns the faces off
first: `issue11651` and `435321` skip by that sentence, the other two (simple fonts) still pass.

**Item 4.** Sites: `pdf-render` `shading.rs` box_down (2107's, untouched here), `collapsed.rs` test
`the_line_test_compares_exact_products` (`2f64.powi(-27)` → `1.0 / f64::from(1u32 << 27)`); render-cpu
`stroke_join_shapes_audit.rs` (an audit file, tolerance 0.2), render-raster `own_space_groups.rs` (`powf(1/2.4)`,
tolerance 2 levels), `dashed_close.rs` ×2 (printed and bounded areas): left, none reaches an exact assertion. Miri
seeds 0..32 pass on the new form, and on the old form too: `1.0 + e` absorbs powi's error, so this states the case
exactly rather than fixing a failure.

**Golden.** Census of the golden list (Sonnet, 974 files): eight page-one fonts match a new word; five embedded; three
unembedded (`noembed-eucjp`, `noembed-sjis` `MS-Gothic`, `noembed-identity-2` `MS-PGothic`) state PANOSE serif style
11, sans, so were sans already; `issue8372` `AdobeHeitiStd` has no Serif flag. No golden page expected to move. But
`panose.rs`'s ratchet moved: the name now decides MS-Gothic before PANOSE, so it measures with the name taken out too.

**Gates.** Over `-p pdf-font -p pdf-model -p pdf-transform -p pdf-render`: fmt 0; clippy 0; `pdf-sandbox --bins` 0;
nextest 100 (`panose` ratchet, restated), then 0 (4878 of 4878; audit-diff 0 new, 0 gone); `cargo test --test
text_suite` 0; rustdoc 0 ×3; Miri seeds 0..32 0; conformance 101 (this record's gates unwritten), then 0.

**Left.** pdf-colour's integer `powi` scales (e-sRGB); `Century Gothic` reads serif by `century`; `HiraMin`.
