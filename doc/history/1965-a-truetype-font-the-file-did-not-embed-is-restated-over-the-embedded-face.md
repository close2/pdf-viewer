# 1965 — A TrueType font the file did not embed is restated over the face the conversion embeds

Session 1965, a single round, PDF/A lane's worktree. R11, ADR 2744.

**The premise.** All ten of R11's documents failed the restatement at one line, `loaded_as_stated`,
and for two causes, not one: eight at a font the file never embedded (its glyphs a substitute, which
ADR 2520 passed over), two (`507676`, `issue13316_reduced`) at a `/FontFile2` whose RFC 1950 check
value disagrees, which this reader does not draw from. R11 was the only refusal under
`as-if-printed` for six of them; `SimFang-variant`, `XiaoBiaoSong` and `issue20489` stop first at
the UTF-8 name fence, and `507676` at the fence, 6.1.13 and the `.notdef` rows.

**Built (ADR 2744).** The restatement reads a font as the conversion writes it (the substitution's
entries, embedding and program). A font the substitution makes `Type1` (a CFF face, `/FontFile3`) is
passed over; one it embeds a shipped face into is restated from that face's own code table
(`pdf_font::standard::shipped_face`), each glyph's outline held equal in the embedded program. A
first version read the embedded program back; on `issue7580` that sent 241 (`ae`) to another glyph
and failed 6.2.11.5. The rewrite is ADR 2520's `truetype-cmap-restated`, already in the history;
the face is the substitution's own stated loss.

**Freed**, under `as-if-printed` at 2b and 4: `issue6127`, `issue7580`, `issue17492`,
`PDFBOX-3067-negativeTf` (also under `as-shown`). Each output drawn at 72 dpi beside its source:
`issue7580`, `issue17492` delta 1, `PDFBOX-3067` 0; `issue6127` 146, the same with `/OutputIntents`
renamed, its Foxit and Liberation faces in place of the machine's (the substitution's losses; the
layout, the Wingdings boxes and numerals unchanged by eye). **Left**: the two damaged programs;
`sample_fonts_solidconvertor`, whose `TimesNewRoman` and `CourierNew` are photographed around, so
no program is embedded to restate; the three behind the fence. R11's rank row: 7, frees 5 / 5.

**Tests** (`archive/truetype_cmap.rs`): an unembedded `Arial` showing 0o361 is made symbolic with
(1, 0) and (3, 0) subtables and the shipped table's glyph, and conforms; an unembedded
`TimesNewRoman` is written `Type1` over `/FontFile3` and the row is stated. Plants: the change
reverted fails the first; the `Type1` pass removed fails the second; reading the embedded program
back fails the first on 6.2.11.5.

**Census** (section 1's command, both profiles; outputs compared with round 1954's): shipped 10 NEW,
stating 8 NEW (`no-data-loss` and `state-every-loss`), 0 GONE, 0 CHANGED; veraPDF passes all 18.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-transform; nextest of pdf-transform
and pdf-archive 0 (1432 passed); archive tier 0; both censuses 0; `cargo test -p conformance` 0 after
`git add -N`; `tools/state.sh --round 1965 docs` 0.
