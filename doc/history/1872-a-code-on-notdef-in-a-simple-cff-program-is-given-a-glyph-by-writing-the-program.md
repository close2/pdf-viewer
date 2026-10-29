# 1872 — a code on .notdef in a simple font's CFF program is given a glyph by writing the program

Session 1872, Slot 2 of batch 128, an archive round in two sessions; ADR 2578 used, ADR 2579 and Q748 unused. I worked
in the shared worktree and built, tested and counted in an export of HEAD plus my own hunks.

**Built.** R03a for a simple font over a bare CFF (`/FontFile3 /Type1C`, ADR 2578): `pdf_font::cff::with_glyphs_given`
grows the `CharStrings` INDEX by a copy of `.notdef`'s charstring per shown width (or an empty glyph where the code
reached no glyph), restates the charset in format 0, the String INDEX and every Top DICT offset, and reads the program
back by `read-fonts` before returning it; `archive/fonts/cff_notdef_glyph.rs` asks for the glyphs from the survey's
shown codes, restates the encoding where it named none, and proves the reloaded font on all 256 codes. A Top DICT
stating an offset twice or with other than Table 9's operands is refused (two fuzz findings). A new fuzz target,
`cff_write`, with its seed recipe and its `doc/verify.md` line.

**Premise.** "The writer re-emits the whole CFF from the parsed INDEXes and `read-fonts` parses what it writes": held,
once the writer reads the Top DICT as its reader does. Not held: that the count of codes is unbounded — the checker
keeps 32 places, and 33 shown codes on glyph 0 are refused before the writer, at HEAD and here alike.

**Plants.** The `CharStrings` INDEX's last offset one byte short: the outline proof passed it, the readback refuses
it and no file is written. veraPDF: the three sources fail it, `issue5256.pdf`'s on 6.2.11.8.

**Bounds.** At most 256 glyphs per call; 32 MiB of copied program per conversion. `getrusage` beside HEAD's binary,
32 codes on `.notdef` at 32 widths, HEAD then here: 3 glyphs 12 712 and 13 388 KiB; 1 980 glyphs at offset size 2 (last
offset 0xFF2B) 13 348 and 15 096 KiB; 63 549 glyphs, 17.5 MB at offset size 3 (0xFFFDF0) 46 344 and 189 540 KiB (HEAD refuses each; this tree writes each, and veraPDF passes all three outputs).

**Gates.** In the export: census no-pack and with the pack under `bounded.sh` 0 and 0 (308 s, 1.95 GiB peak with the
pack); `rank.py` 0 on both: R03a documents/alone 23/14 to 14/11 under `as-if-printed` at both targets, 24/14 to 21/11
and 24/13 to 21/10 under `as-shown`; converted +2 pdf.js and +1 corpora on every line against HEAD's, no refused count
up. veraPDF 1.30.3 on the twelve outputs of `issue5256`, `issue7439` and `10531_2004_Article_5096977` (two profiles,
2b and 4): 12 compliant, exit 0 each. `cargo nextest run -p pdf-font` 0 (317 passed); `cargo test -p pdf-transform
--test archive` 101 (494 passed, 2 failed: the two JPX tests round 1879 owns, failing alone too), `--test profiles` 0
(12 passed); clippy `--no-deps -D warnings -p pdf-font -p pdf-transform --all-targets` 0; `cff_write` fuzz, own target, `-s none`: two
findings fixed (77 at 99 s and 81 s), then 0 after 601 s (3 677 349 runs, 1 286 edges).
In the worktree: `cargo check --tests -p pdf-font -p pdf-transform` 0; `--bin cited` 0; `--bin quotations` 0;
`cargo test -p conformance --no-fail-fast` 0 (69 binaries ok).
