# 3153 — An embedded `glyf` program's twin code reaches a copy of its twin's glyph

Session 2113, an archive round in the `lane4` worktree, on `doc/pdfa-blocking-issues.md`'s R10
entry for `499039`. Status: accepted and built. In `pdf-transform`: `archive/fonts.rs`
(`restate_metrics` takes a target and the twin route, `embedded_twins`, `EmbeddedTwins`,
`disagreeing`, `shown_widths` sets codes aside, `Metrics::encodings`, `RestatedFont::twin_codes`),
`archive/rewrite.rs` (`name_the_twin_codes`), `archive/report.rs` and
`archive/prepare/font_rewrites.rs` (reported, and recorded in `xmpMM:History`). Tests:
`crates/pdf-transform/tests/archive/twin_codes.rs`
`an_embedded_soft_hyphen_narrower_than_the_hyphen_reaches_a_copy_of_its_glyph`,
`an_embedded_soft_hyphen_of_a_width_of_its_own_is_given_that_advance`. Context: ADR 2723 (the
construction for a substituted face), ADR 2640 (a shared program restated in a copy).

## The case

`499039`'s `NMGBFB+TimesNewRomanPS-BoldMT` is a simple `TrueType` font under `WinAnsiEncoding`
whose embedded subset program carries a (1, 0) and a (3, 1) `cmap` subtable. The dictionary states
333 for code 45 and 0 for code 173. Both codes name `hyphen` (ISO 32000-2 Annex D, Table D.2), so
§9.6.5.4's route — glyph name, Adobe Glyph List, (3, 1) — reaches glyph 16 for both, and this
reader draws the hyphen's outline for 173 and advances by 0. ISO 19005-2 section 6.2.11.5 then asks
glyph 16's one advance to equal two numbers, and `restate_metrics`, which restates an embedded
program's advances, refused with the fence `TWO_WIDTHS_FOR_ONE_GLYPH`.

## The decision

ADR 2723's construction, taken over the file's own program: the encoding is restated with the
name of the character ISO 32000-1's Annex D note 5 says the code means (`/Differences [173
/softhyphen]` on the stated base encoding), and the program gains a byte-for-byte copy of the
twin's glyph to which every Unicode `cmap` subtable sends U+00AD (`twin_codes::split`). The copy is
written at advance 0 and restated with the program's other disagreeing glyphs where the dictionary
states another width. The font dictionary must be an object of its own, since its `/Encoding` is
rewritten; the program is restated in place or, where another font shares it, in the copy ADR 2640
makes. The same edition conditions hold as for ADR 2723 (part 2 always, part 4 where the label is
below 2.0), and a code the producer's `/Differences` already names is not renamed.

What a reader draws does not change: the same outline, at the width the dictionary states. What
it costs is the one ADR 2723 states: text extracted from code 173 is U+00AD rather than U+002D,
which is the character ISO 32000-1 assigns the code. The report names the renamed codes on the
font's line, and the file's history says the same.

## Not done

A program with CFF outlines under `/FontFile3`, a Type 1 program, and a font dictionary written
inside another object keep the fence; none is in the census.
