# 2636 — A cut font program takes the unembedded route, and one this reader draws whole is refused by name

Status: implemented. Session 1901, the first archive slot of batch 131.

## Context

ADR 2619 decision 2 made a font program whose decode stops short (`Damage::stops_short`) not
embedded: ISO 32000-2 §9.9 embeds a program "as data contained in a PDF stream object", and the
front of one is not the program. `fonts/font-programs-embedded` fails such a font as it fails one
that names no program, and five documents the base had converted were refused on it: govdocs1
`435321`, `178360` and `268346` (at 4), pdf.js `issue11651` and PDFBox's `F001u_3_7j`. The converter
already answers a font with a descriptor and no program: a face supplied by the host, or a shipped
face under ADR 2162's substitution and the losses the profiles name.

## Decision

1. **A cut program is answered as a missing one.** Where `pdf_archive::program_stopping_short` —
   the checker's own reading, now public so the two cannot disagree — finds a rendered font's
   program cut and `LoadedFont::load` cannot load the font either, the converter drops the
   `/FontFile*` entry and hands the font to the substitution: a simple font is read through a copy
   of its descriptor without §9.9's three keys, so every step of the route (the shipped or supplied
   face, the metric route, the restatement's proof) sees the font a file naming no program states;
   a Type 0 font goes through `composite::embed` as before. The descriptor's names and the font's
   widths are kept.
2. **No new loss word.** The front draws nothing in this reader, so the face that replaces it
   changes no shape the window showed beyond what the substitution's own losses (`machine-face`,
   `substitute-stretch`, `layout-from-this-viewer`, `text-as-image`) already state, and each is
   authorised or refused at its site as for an unembedded font. `as-shown` keeps its stop through
   those words. The cut program is named in the report's substitution line and JSON field
   `cut_program {key, damage}` and in the output's `xmpMM:History` clause; the statement page
   lists losses only and so does not name it.
3. **Not where this reader draws the front whole.** A decode that stops short after every byte of
   the program — `F001u_3_7j`'s `FontFile2` decodes all 28976 bytes its `/Length1` states and the
   stream's `/Length` ends two bytes into RFC 1950's Adler-32 — loads, and dropping it would change
   the shapes the window drew. Writing that stream whole again is a remedy of its own and not this
   one; the site is refused by its own sentence (`CUT_PROGRAM_DRAWN_WHOLE`) instead of the
   misleading "no font ... lacks an embedded program".
4. **A decode refused outright, and one its checksum disowns, are not embedded either.** The
   checker passed over a `FlateDecode` this reader refuses from its first byte (`Corrupt`) as
   though the filter were unknown, and an Adler-32 that disagrees (ADR 2619's last sentence); the
   first hands over no program and the second bytes the stream's own checksum disowns, and veraPDF
   counts both as no program. Both now fail `fonts/font-programs-embedded`, and since this reader
   loads neither, the route of decision 1 takes them. A filter this reader does not decode is
   still passed over.
5. **Bytes that decode whole and are not a program in the format they are filed under are not
   embedded either.** `178360`'s `ICLJGJ+Glypha` is filed under `/FontFile3 /Subtype /Type1C`, so
   §9.9's Tables 124 and 125 make it a CFF program as Adobe Technical Note #5176 describes one, and
   it is not: it reads as though every zero byte were written `0x20` (none of its 1009 is `0x00`; its sibling
   `ICLJGI+Glypha-Black` begins `01 00 04 02 00 01 01 01`, it `01 20 04 02 20 01 01 01`), so its
   header states minor version 0x20 and the Name INDEX after it a count of 8193 names whose
   offsets alone overrun the program. The header's minor version is no fault — TN 5176 section 6
   makes a minor version a compatible change — and this reader's signature now admits any minor
   version (`program::is_bare_cff`, `standard::program_format`), where it sent the bytes to the
   sfnt reader, whose refusal named a container the bytes never claimed. The program is then the
   CFF reader's to refuse, and it does. `pdf_archive::program_unparsed` is that reading —
   `LoadedFont::load`'s `Malformed`, the variant whose subject is a program read and rejected —
   and `fonts/font-programs-embedded` fails it under its own sentence; this is a program "under a
   key or name its bytes contradict", in ADR 2307's words, read by the grammar the name selects.
   Since this reader draws none of it, the route of decision 1 takes it; the report's and
   `xmpMM:History`'s clause says the data decodes whole and is not a program in its format, and
   the JSON `cut_program.damage` is `null`. veraPDF and this reading agree: it is no disagreement.

## Consequences

The census against HEAD's, no pack: corpora `as-if-printed` converts 422 → 424 at 2b and 430 → 433
at 4 (either part 428/12/44 → 430/13/41); pdf.js `as-if-printed` 889 → 890 at 2b and 900 → 901 at
4. With the CJK pack: corpora the same 422 → 424 and 430 → 433 (either part 428/12/44 → 430/13/41), pdf.js 890 → 891 and 901 → 902. Every mover is one of the five; `as-shown` totals are unchanged, its refusals of
the five moving to their next reason (`435321`'s an `Adobe-Identity` font with no usable
`/ToUnicode` character). veraPDF 1.30.3 passes `issue11651` and `435321` at 2b and 4 and `268346`
at 4: those three are freed. `178360` converted under decisions 1–3 and veraPDF failed 6.2.11.4.1 on 20
programs this tree kept: of 27 `Type1C` streams, 10 this reader's `FlateDecode` refuses outright
and 9 whose Adler-32 disagrees. Under decision 4 it converts at 2b and 4 with 39 fonts substituted
and veraPDF fails 6.2.11.4.1 on one, `ICLJGJ+Glypha`; under decision 5 that program is substituted
too, and veraPDF 1.30.3 passes `178360` at 2b and 4 (0 failed rules each): it is freed, and the
output that kept the program, held to 4, fails 6.2.10.4.1 as it should. The census after decision 5,
against HEAD's, moves exactly the lines above and no other in either run, with and without the
pack: the wider CFF signature moved no other document. `F001u_3_7j` stays refused under decision 3. The planted failure — the
fixture's cut source, held to 2b — fails veraPDF on 6.2.11.4.1, as it should.

Tests: `program::font_file_signature` reads a CFF header of minor version 0x20 as CFF;
`table::fonts` fails Glypha's first 40 bytes under `/Type1C` by decision 5's sentence; `tests/archive/cut_program.rs` — a program cut mid-stored-block is dropped, the shipped face
embedded and drawn, the report naming the cut; the plant, the whole stream and one byte after its
checksum, is kept and not routed; a stream cut inside its checksum is refused by decision 3's
sentence; the face with every zero byte written `0x20`, whole, is substituted under decision 5.
