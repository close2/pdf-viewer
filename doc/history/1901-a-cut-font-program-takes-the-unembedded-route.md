# 1901 — A cut font program takes the unembedded route

Batch 131, slot 1 (archive), three sessions on Opus 5.5. ADR 2636.

**What changed.** A rendered font whose embedded program's decode stops short (ADR 2619) and that
`LoadedFont::load` cannot load is answered as a font naming no program: `/FontFile*` is dropped and
the font takes ADR 2162's substitution under its descriptor's names and widths and the losses the
profiles already name. `pdf_archive::program_stopping_short` is public so the converter reads the
bytes as the checker does. The report's substitution line, its JSON `cut_program {key, damage}` and
the output's `xmpMM:History` name the cut program. A cut program this reader loads whole —
`F001u_3_7j`, whose `/Length` ends two bytes into the Adler-32 after all 28976 bytes `/Length1`
states — is refused by its own sentence instead of "no font ... lacks an embedded program".

**The five documents.** `issue11651` and `435321` at 2b and 4 and `268346` at 4 convert and pass
veraPDF 1.30.3: freed. `178360` converted with veraPDF failing 20 kept programs (10 refused outright,
9 disowned by their Adler-32); the checker now fails both (decision 4). Its last, `ICLJGJ+Glypha`, is
a whole `/Type1C` program with no `0x00` byte — read as though every zero were written `0x20`, its
Name INDEX counts 8193 names — which the signature sent to the sfnt reader for its minor version
0x20 (TN 5176 section 6 admits any); the CFF reader now refuses it, the checker fails it as not a
program in its format (decision 5) and the route substitutes it: `178360` passes veraPDF at 2b and 4.
`F001u_3_7j`
stays refused by name. The planted failure, the fixture's cut source, fails veraPDF on 6.2.11.4.1.

**The plant.** The whole zlib stream plus one byte after its checksum is kept and not routed; a
stream cut mid-block is substituted and drawn in Liberation Sans (looked at); one cut inside its
checksum is refused by the new sentence (`tests/archive/cut_program.rs`).

**Gates.** Sessions 1–2: `--test archive --test profiles` exit 0 (527 and 13 passed, 6 ignored);
`cargo test -p pdf-archive` exit 0; clippy exit 101 on two `+` in the test's Adler-32, then 0;
`--bin cited`, `--bin quotations` exit 0 (3049 quotations, 2851 verbatim, 3 diverging, none this
round's); `cargo test -p conformance --no-fail-fast` exit 0 (473 passed, 70 binaries); census
456 s and 728 s; `rank.py` exit 0. Session 3: `cargo nextest run -p pdf-font` exit 0 (355 passed);
`cargo test -p pdf-archive` exit 0; `--test archive --test profiles` exit 0 (529 and 13 passed);
clippy on pdf-font, pdf-archive, pdf-transform exit 0; `cited`, `quotations` exit 0; conformance
exit 101 in the export, its 8 reds the git-index tests (no index there; 33 passed in the worktree).
Census against HEAD's (no-pack 321 s, pack 252 s): corpora `as-if-printed` 422 → 424 converted at
2b and 430 → 433 at 4, pdf.js 889 → 890 and 900 → 901 (with the pack 890 → 891 and 901 → 902);
`as-shown` totals unchanged; every moved line one of the five. veraPDF passes `178360` at 2b and 4.
