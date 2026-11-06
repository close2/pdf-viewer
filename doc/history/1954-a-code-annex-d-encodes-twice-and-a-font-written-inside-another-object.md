# 1954 — A code Annex D encodes twice, and a font written inside another object

Session 1954, a single archive round on R10. ADRs 2722 and 2723.

**The premise, checked per document.** `486832`: only `ArialMT`, substituted by Liberation Sans,
shows codes 45 and 173 at 333 and 0; R10 its only stop. `499039`: the same pair in `ArialMT`
(Liberation), in `TimesNewRomanPSMT` (Foxit Serif, CFF) and in the embedded subset
`NMGBFB+TimesNewRomanPS-BoldMT`; three constructions, so it stays. `hesr13613`: four `Type0`
fonts direct in form resources, their `CIDFontType2` and descriptors direct, programs objects;
under `as-if-printed` R10 was its only stop. `SimFang-variant` and `XiaoBiaoSong` need the
substitution route's lift and still stop at R03 and R11 under both profiles, so it buys neither.

**Built.** ADR 2722: a program behind a direct descriptor is restated where it stands (§7.3.10),
and `/CIDToGIDMap /Identity` is written into a `CIDFont` in any holder by the walk `typed_fonts`
now shares (`direct_fonts_edited`). ADR 2723: `WinAnsiEncoding` 173/160 and `MacRomanEncoding`
202, shown at a width other than their twin's, are named `softhyphen` or `nonbreakingspace` where
ISO 32000-1 describes the file (its Annex D notes 5 and 6); a `glyf` face that gives the character
its twin's glyph gains a byte-for-byte copy at the code's width, every Unicode `cmap` subtable
sending the character there (`with_glyph_copied`). A stated interpretation: code 173 extracts as
U+00AD, the character ISO 32000-1 assigns it; reported on the font's line and in the history.

**The defect.** Round 1951's refusal that named no font goes with the route that reaches the
program. `hesr13613` now fails the output's `graphics/only-operators-the-base-standard-defines`
after page 8 is photographed; no stream, form, appearance or `/DA` holds an undefined operator.

**Proof.** `486832` under `as-if-printed`, 22 pages at 72 dpi on the CPU: at most 86 levels apart,
the same with `/OutputIntents` renamed; page 1 at 200 dpi identical to the eye (the substituted
faces' anti-aliasing). veraPDF 1.30.3 passes all six new outputs. Tests: 4 unit, 4 in
`tests/archive/twin_codes.rs`; three plants (`cmap`, direct descriptor, holder) each fail theirs.

**Census.** Shipped against round 1945: 14 NEW (`486832` at 2b and 4, and round 1951's 12), 5105 of
5105 the same; stating against round 1943: 14 NEW, 4561 of 4561 the same. `rank.py --table`: R10
frees 7 / 7 and stops 7 / 7; `hesr13613` is D-defect; R09 stops 121 / 121. Section 9 regenerated.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-transform`, `pdf-archive`): 0. nextest
both: 1430 passed, 0. `cargo test -p conformance` after `git add -N`: 0. `state.sh docs`: 101, two
rustdoc links of this round's; fixed, rustdoc `-D warnings` over both crates 0. Archive tier: 0, 0.
Clocks (owed since 1949), on this tree: `launch_path` 101, out of band: bring-up 60.8 ms (27..40),
WTPDF first page 110.0 / cold 107.7 ms (67..100.3), four peak-anon figures under their low edge;
`turn_path` 0, in band.
