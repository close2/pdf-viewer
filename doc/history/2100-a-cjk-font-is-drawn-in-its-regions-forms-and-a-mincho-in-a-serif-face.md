# 2100 — A CJK font is drawn in its region's forms, a Mincho in a serif face, and a script no face draws is not refused whole

Session 2100, a pixels round in `pdf-viewer-audit` on the three font items round 2091 left. ADRs 3101 to 3103; 3104
and Q922 unused (Noto Serif CJK is installed here, so no package is asked for).

**Premises.** Held: Noto Sans CJK's five region faces tie on repertoire (44 810 each), style and code pages, so
face 0, JP, drew every collection. Corrected: §9.7.3's `/CIDSystemInfo` is Table 114, and Table 116 is the
predefined `CMap`s — whose four headings (Chinese Simplified and Traditional, Japanese, Korean) are what ties a
collection to a language. Found: `fonts-noto-cjk` here ships Noto Serif CJK (ten faces); it never won because the
whole-`cmap` count ranked first and the serif family maps 33 fewer.

**Region (ADR 3101).** `Style` carries `Option<Collection>` (from `/CIDSystemInfo`, else a Japanese or Korean
`/Lang`), crossing the port as one byte (description version 3). `ranked_covering` ranks breadth, then region, then
style, then family. Breadth is the collection's own `-UCS2` characters a face maps; region is the Windows language
of the face's `name`-table family name (`Collection::region_rank`). Test
`a_region_ranks_before_the_style_and_the_family_after_it`; planted (region out of the key), it failed.

**Mincho (ADR 3102).** Over each collection's table Sans and Serif CJK tie (Japan1 15 109 each); `names_a_cjk_serif`
makes Mincho, Song, Ming, Batang, Myeongjo, KozMin, Ryumin, HeiseiMin and SimSun serif requests. Adobe-KR's table
names 47 of the sans family's extra ideographs, so an Adobe-KR Myeongjo still draws sans. The glyph-class fixture
was renamed `HiraginoJP-Regular`: its premise is a name with no family word.

**No face for the script (ADR 3103).** `load_composite` falls back to the family face (`drawing_face`); borrowing
and `uncovered_character` do the rest. Test `a_font_whose_script_no_face_draws_still_draws_what_a_face_does`;
planted (no fallback), `NoSubstitute`.

**Pages, looked at.** Rendered with lane 4's `render_at` (b60382af) against this tree at 1.5: `issue19182` draws
胡駿碟 in Ming, `90ms_rksj_h_sample` 日本語テスト in Mincho. Moved: `90ms_rksj_h_sample`, `issue11555`, `issue13343`,
`issue15053`, `issue6286`, `noembed-identity`, `noembed-jis7` (serif), `issue19182` (TC serif), `issue2128r` (SC),
`issue3521` (SC serif: 黑体 states `/Flags` Serif and its GBK name reads as nothing). Held: `issue8372` (目录 alike
in JP and SC), `mixedfonts`, `noembed-sjis`, `noembed-eucjp`, `noembed-identity-2`. Golden not edited.

**Left, in order.** Read a `/BaseFont` in a CJK encoding through its collection (`issue3521`'s 黑体 would then be
sans); an Adobe-KR Myeongjo's serif face; the golden restated for the pages above by ADR 3051's method.

**Gates.** Over `-p pdf-font -p pdf-model -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 101 (four
findings) then 0; `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 0 (4548 passed, difference empty);
rustdoc `-D warnings --document-private-items` 0; `fuzz` `cargo check --bins` 0; `cargo test -p conformance` 0.
