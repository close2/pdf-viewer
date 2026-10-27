# 1812 — A code a substituted font's own encoding sends to .notdef is given an empty glyph

Slot 7 of batch one hundred and twenty-one, 2026-10-27, the archive slot; ADR 2458; ADR 2459 and
Q678 unused. I worked in `scratchpad/r1812/`. No ledger status moves (ISO 19005 has no ledger rows).
**Read** (HEAD's census, the sixteen under "no face this program ships has a glyph", code by code):
no script. Fifteen show a code their own encoding sends to `.notdef` in any program — twelve a code
it names nothing for (a line feed in seven govdocs files and `embedded_zip`, 0x00 in `issue18059`,
control codes in `ControlCharacters` and `standard_fonts`, 0200–0215 in `ZapfDingbats`, which
Annex D.6's table encodes for no glyph), three a `/Differences` naming `.notdef` — and one shows
`Delta`, which the shipped serif lacks. **Premise** held (sixteen, one sentence); **hypothesis**
(one or two scripts, half convert) did not: no script, two convert. **Priced first** (habit 164):
planting the codes out on HEAD freed none of the seven govdocs files, which also stop at the
content-stream fence, so the route beside a file's own descriptor was not built.
**Built (ADR 2458, A48).** `pdf_font::cff::with_empty_glyphs` appends one width-and-`endchar`
charstring per distinct stated width, named `notdef`, `notdef.1`, … (no AGL name, no character),
every held glyph byte for byte; the described route (a standard font by name alone, CFF face) gives
each such code one, restates the encoding to name it, writes the program as the font's own, holds an
outline-less glyph equal to none in its readback, and names the codes in the report and in
`xmpMM:History` with the clause. A `glyf` face refuses on the fence's side (a non-symbolic TrueType
reaches a glyph only by an AGL name, which states a character); Levels U and A refuse as the
target's (no character to map); a font with its own descriptor refuses by a sentence of its own.
None tells the operator `--font` resolves it. **Census** (converted/conforming/refused/unreadable),
pdf.js, HEAD → this: `as-if-printed` 2b 789/2/162/11 → 791/2/160/11, 4 792/0/161/11 →
794/0/159/11; `as-shown` 2b 764/2/187/11 → 766/2/185/11, 4 768/0/185/11 → 770/0/183/11;
`doc/corpora` unchanged. Freed `ZapfDingbats.pdf`, `issue4304.pdf`; other lines moved only their
sentence, and `issue20489.pdf` (a line feed beside its own descriptor) left the machine-face one.
**veraPDF 1.30.3**: 8 outputs (2 documents × 2 profiles × 2b, 4), 8 PASS; a planted absent glyph
name also PASSes, so veraPDF is no witness here — this tree's second verdict failed the planted-out
construction at `fonts/no-notdef-glyph-shown`. **Drawn** at scale 2 against the source: `issue4304`
identical, `ZapfDingbats` 12 and 10 pixels one level apart in the red header (the output intent's);
looked at: 0200–0215 draw nothing in source and output alike. **Proof.** `tests/archive/
empty_glyphs.rs`, five tests, all red with the construction planted out; `cff::empty_glyphs` holds
every old glyph's outline and advance in four CFF faces; the port is my hunks' multiset (445 lines).
**Gates.** In my export: `cargo fmt --check` pdf-font, pdf-transform exit 0; `RUSTFLAGS="-D
warnings" cargo clippy --all-targets` both exit 0; rustdoc `-D warnings --document-private-items`
both exit 0; nextest both 1020 passed, 0 failed; census `--tree 6` exit 0 before and after;
`archive_corpus --ignored` exit 0, 7 passed; `cross_check --ignored` exit 0, 13 560 compared, 0
disagreements; six arms 5 799 pages, 0 digests differ against HEAD's six arms; `tools/state.sh
--round 1812 archive` exit 0; `cargo test -p conformance --no-fail-fast` exit 0.
