# 2601 — A `.notdef` row's reason names the format of the program it refused

Status: implemented — session 1883, the third slot of batch 129 (an archive round). Context: ADR 2543 (the `.notdef`
rows join the restatement only where every font they name has a program it extends), ADR 2578 (simple CFF), round 1872's finding that the reason called every program TrueType. Code:
`crates/pdf-transform/src/archive/fonts/notdef_format.rs`, `fonts/truetype_cmap.rs` (`NOTDEF_NOT_GIVEN`'s opening,
`notdef_fonts_have_truetype_programs`), `prepare.rs` (`Owed::notdef_not_given`).

## Decision

ISO 32000-2 §9.9.1's Table 124 names a program by its key and `FontFile3`'s `/Subtype`. Where a font the rows name
holds a program of a format the restatement never extends — a Type 1 program (`FontFile`), an OpenType program
(`FontFile3`/`OpenType`) — or no program, the row's reason is a sentence naming that format. Where every program the
rows name is TrueType or CFF the reason stays `NOTDEF_NOT_GIVEN`, which states the restatement's conditions, and its
opening now says "the program a font embeds" rather than "an embedded TrueType program". The first font in object
order decides. A sentence naming a TrueType or CFF program refused under another dictionary was rejected: the
restatement does extend those formats, and a descendant written in place is refused by its conditions, not by
its format.

## Figures

Against HEAD's census, with and without the CJK pack: of the pdf.js documents `fonts/embedded-programs-define-every-glyph-shown`
gave the TrueType sentence, three now read "an embedded Type 1 program (a FontFile stream)" (`issue11477_reduced`,
`issue6721_reduced`, `issue6901`) and two "the program a font embeds" (`issue14821`, `issue17333`); the rest moved to
the CID 0 sentence or converted (ADR 2598, ADR 2602). In the corpora `504076.pdf` takes the Type 1 sentence, and under
`fonts/no-notdef-glyph-shown` so does `issue6721_reduced`. No document of either corpus reaches the OpenType or the
absent-program sentence; the unit tests in `notdef_format.rs` do.
