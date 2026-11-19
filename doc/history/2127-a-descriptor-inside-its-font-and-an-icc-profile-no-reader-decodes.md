# 2127 — A descriptor inside its font, and an ICC profile no reader decodes

Session 2127, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s long
tail: R10's descriptor written inside a font with no program, R07's ICC profile, R03a's invalid
code. ADRs 3209, 3210, 3211; ledger rows 9.6.2.1, 9.6.3 and 8.6.5.5 (`implemented`, before and
after; each gains its code and a test); no question.

**Built (R10), ADRs 3209 and 3210.** `SimFang-variant` and `XiaoBiaoSong` write a direct
`/FontDescriptor` in a symbolic `TrueType` font with no program. The refusal's premise was wrong:
the substitution writes a copy of the descriptor as a new object, so a direct one is copied too.
A print at each refusal found the two stops behind it: section 6.1.8 (each `/BaseFont` is GBK
bytes) and R11, whose `cmap` restatement `refused_anyway` declined to prepare while the name row
stood. The `/BaseFont` and `/FontName` are written as the embedded face's PostScript name where the
producer's is no UTF-8; a restated name no longer blocks R11; and ADR 3127's photograph passes over
a simple font that embeds no program, so `as-if-printed` keeps the text instead of photographing.

**Built (R07), ADR 3211.** `UnknownFilter-ICC`'s `/XXXDecode` profile (`/N 4`, no `/Alternate`) is
written as the profile of the device space Table 65 draws it in: the assumed inks for four
components, sRGB for three; one component or another alternate stays refused by name.

**Not built (R03a).** `issue11768_reduced`'s first code, `e0 e0 e0`, matches no codespace range of
its embedded `UniJIS-UTF8-H` and falls back to CID 0. The construction is written into R03a's entry
(a one-code range and a `cidchar` to a fresh CID `Adobe-Japan1-UCS2` maps to nothing); not started,
for the wall budget.

**Result.** Both R10 documents convert at 2b and 4 under both shipped profiles, veraPDF 1.30.3
passing all eight; page 1 at 150 dpi differs from the source's under `as-shown` by antialiasing (at
most 4 of 255) and under `as-if-printed` by the shipped face (`machine-face`). `UnknownFilter-ICC`
converts at 4 under both, veraPDF passing, pixel-identical; at 2b it stops at its header (R06b).
Every test fixture's output (16) passes veraPDF.

**Census.** With the pack, pdf.js converts two more documents on every line and corpora one more at
part 4; nothing moved the other way. Without the pack `issue15053` stops at R08b as before. Sections
1, 2 and 9, the R10, R11, R07 and R03a entries, and `rank.py`'s R07 and R10 statuses restated; the
mitigations' filter, name and photograph entries.

**Gates.** Over `-p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after a
`map_unwrap_or` in the new test); `pdf-sandbox` build 0; nextest `--lib --bins --tests` 0, 1272
passed, none expected; rustdoc 0; `fuzz` `cargo check --bins` 0 (`UndecodableRole` gained
`Profile`); conformance 0.
