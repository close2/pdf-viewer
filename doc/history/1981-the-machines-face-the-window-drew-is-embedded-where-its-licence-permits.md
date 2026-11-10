# 1981 — The machine's face the window drew is embedded where its licence permits

Session 1981, a single round, PDF/A lane's worktree. R09 (a); ADRs 2773 and 2774.

**The premise.** Half true. Round 1979's shipped census refuses 76 documents under `as-shown` with
`machine-face`, alone at both parts in 53. A probe of each rendered font's `substitute_program` on
this VM names the faces: URW's Nimbus Roman, Sans, Sans Narrow and Mono PS (OpenType carrying a
name-keyed `CFF `, `fsType` 0x0004, preview-and-print) for simple fonts; in 16 documents a composite
font is drawn from `DroidSansFallback` (`glyf`, 0x0008) or a Nimbus face. No collection, and no face
forbidding embedding, is drawn.

**Built.** ADR 2773: where the profile withholds `machine-face` and the face's `fsType` permits, the
descriptor route embeds the program the window loaded (`Face::Machine`, `FaceAuthority::Machine`),
its code table by the window's route. Table 124 puts a `CFF ` OpenType face under a `Type1`
dictionary, so a `TrueType` one is restated as ADR 2147 does for a shipped CFF face; the font is held
code by code to the window's outline (`machine_face_proved`). Preview-and-print is admitted because
§9.9.1 names it and gives it to every embedded program by default. Whole, not a subset: a name-keyed
CFF subset needs a compacting writer. ADR 2774: `as-if-printed` and `state-every-loss` keep the
shipped face.

**Freed**: under `as-shown`, 25 documents at 2b and 24 at 4; under `no-data-loss`, 19 at each part,
among them `Deficient inflammasome…`, refused before on no shipped face covering a code. R09's row
goes from 80 to 58; six documents move to it from R08 and one more joins it. **Kept by name**: a Nimbus face whose shown
glyph states its width in a subroutine (9 to 12 of 855 glyphs, `pdf_font::restate`; 25 documents), a
fitted machine CFF face (`substitute-stretch`, 7), the name-alone and composite routes. A different
machine converts differently; that is what `as-shown` means.

**Pixels** (CPU, 72 dpi, pages 1–4, `/OutputIntents` renamed): every freed `as-shown` output at 4 is
within one level of its source on every page; the shipped face's pages differ by up to 255.

**Tests.** `machine_face.rs` (two): `embed_machine_face: false` fails the first and the updated
`the_unrestated_route_over_a_machines_face_is_the_loss_machine_face`; `true` fails the second and it.

**Gates.** fmt and clippy `-D warnings --all-targets` 0 on pdf-transform (fmt re-wraps two lines of
`truetype_cmap.rs` that fail `--check` at HEAD); nextest of pdf-transform and pdf-archive 0 (1459);
`cargo test -p conformance` 0 (471); archive tier 0; censuses against 1979: shipped 49 NEW, stating 38
NEW, 0 GONE, 0 CHANGED; veraPDF 1.30.3 passes 46 of 49 and 35 of 38, the rest (`029016`,
`AIAA-2002-4016-606`) failing as HEAD's `as-if-printed` outputs do; `state.sh docs` and a second `cargo test -p conformance` still running at hand-off.
