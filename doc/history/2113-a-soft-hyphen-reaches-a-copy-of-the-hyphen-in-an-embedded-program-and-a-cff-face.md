# 2113 — A soft hyphen reaches a copy of the hyphen in an embedded program and in a CFF face

Session 2113, an archive round in the `lane4` worktree, on `doc/pdfa-blocking-issues.md`'s R10
documents as round 2110 traced them. ADRs 3153, 3154 (built) and 3155 (a construction); ledger row
D.2 (`implemented`, before and after; it gains the twin route and two tests); no question.

**What the window draws.** `499039` states code 173 at 0 beside the hyphen's 333 in three fonts.
§9.6.5.4 sends 173 (`hyphen` in Table D.2) through the Adobe Glyph List and the (3, 1) subtable to
the hyphen's glyph in the embedded subset `NMGBFB+TimesNewRomanPS-BoldMT` (its (3, 1) maps no
U+00AD), and through the charset name to the hyphen in Foxit Serif for the unembedded
`TimesNewRomanPSMT`: the hyphen's outline, advancing by 0, in both (`ArialMT` is ADR 2723's).

**Built (a), ADR 3153.** `restate_metrics` gains ADR 2723's route: the code renamed `softhyphen`
in the font's `/Encoding`, the program given a copy of the twin's glyph that U+00AD reaches, the
copy restated where the dictionary states a non-zero width; the dictionary's encoding is written by
the rewrite (`Metrics::encodings`), reported on the font's line and recorded in the history.

**Built (b), ADR 3154.** A shipped bare CFF face under a dictionary kept or restated as `Type1`:
the code is held to its twin's glyph under the stated encoding, and `pdf_font::cff` appends a glyph
named `softhyphen` drawing the twin's charstring (`GivenOutline::GlyphsOwn`, refused in a Type 1
program by name); the restatement's proof reads the font back with the renamed encoding.

**Result.** `499039` converts under `as-if-printed` and `as-shown` at 2b and 4; veraPDF 1.30.3
passes all four outputs. Drawn at 150 dpi beside the source on the five pages that show code 173
(62, 85, 95, 115, 416), `as-shown` 4 differs by a channel delta of at most 1; `as-if-printed` 2b
differs where it removes four faces' `substitute-stretch` (Courier and Arial faces), not at 173.

**Construction (c), ADR 3155.** `issue11740_reduced`'s Type 1 program under a `CIDFontType0`
needs a Type 1 to Type 2 charstring converter writing a CID-keyed CFF for `/FontFile3
/CIDFontType0C`, proved outline by outline; no converter exists in the tree, so it is written down.

**Census.** Re-taken with the pack and without: every corpora line converts one more document
(`499039`), pdf.js unchanged; without the pack `issue15053` stops at R08b as in session 2106. R10
falls from 6 documents to 5. Sections 1, 2 and 9 restated; `rank.py`'s R10 status.

**Gates.** Tests in `tests/archive/twin_codes.rs` (an embedded soft hyphen at 0 and at 250, one
in the CFF face). Over `-p pdf-font -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after
1.100's `too_many_lines` and `format_push_string` in the new code); `pdf-sandbox` build 0; nextest
`--lib --bins --tests` 0, 1695 passed, none expected; rustdoc 0; `fuzz` `cargo check --bins` 0
(`RestatedFont` and `GivenOutline` changed); conformance 0.
