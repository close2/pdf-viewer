# 1874 — The checker reads a shown CID 0 as the CIDFont's `.notdef`, and fails a page tree Table 30 rules out

Slot 4 of batch 128, 2026-10-29, an "archive" round (Opus 5.5), two sessions; ADRs 2582 and 2583 used, Q750 unused.
No conformance ledger row moved: §7.7.3.2, §9.7.4.2 and §9.7.6.3 stay `implemented`; the archive's rows widened.

**The two gaps.** `fonts/no-notdef-glyph-shown` judged a composite font by glyph index, so a code the font draws as CID
0 passed whatever glyph a `/CIDToGIDMap` sent it to, where §9.7.4.2 makes CID 0 the CIDFont's `.notdef` (ADR 2582).
`LoadedFont::drawn_cid` reads the CID after §9.7.6.3's fallbacks from the function the drawing route uses, and the
converter's copy and its proof refuse the same codes. The page tree walk passed a root stating `/Parent` and a `/Kids`
entry naming no dictionary; two section 5.1 rows now fail each, one finding per node (ADR 2583), and the converter
refuses by name the direct-node case its walk does not follow.

**The premise that did not hold.** "The census should not move": it moved one document. pdf.js's `issue7544.pdf`
(OnlyOffice, `Identity-H`, CID 0 drawn as a `T`) was converted and is now refused; veraPDF fails HEAD's output of it on
6.2.11.8-1 at 2b and 6.2.10.9-1 at part 4, so the move is a correction, not a regression.

**Its sentence.** The refusal read the fence's "a glyph its own embedded font program does not define" and, under ADR
2590, the copy's list of conditions: neither fits a program that holds the glyph. `cid_notdef_glyph::CID_0_SHOWN` now says that §9.7.4.2 makes CID 0 the `.notdef` character and that the remedy is
an embedded `CMap`, checked by `shows_cid_0` before the copy is tried. The census line whose sentence changes is the
pdf.js `fonts/no-notdef-glyph-shown` line at 2b and at part 4 that counted `issue7544.pdf`: that document leaves the
fence's line and stands alone, `not-built-yet`, under the new sentence; no count changes.

**The plants.** The checker with its CID 0 condition off: the fixture test failed (left 0, right 1). The converter with
`given` and its proof allowed a shown CID 0 and the identity map restated: `4387ba48` converted to the output ADR 2557
withdrew, and the last check named `fonts/no-notdef-glyph-shown` still failed. The pre-check off: its test failed.

**Gates.** `cargo test -p pdf-archive` exit 0 (248 lib tests and every integration target); `cargo test -p pdf-transform
--test archive --test profiles` exit 0 (502 and 13); clippy `--no-deps -D warnings -p pdf-archive -p pdf-font -p
pdf-transform --all-targets` exit 0, `answer_of` no longer over its 100 lines; `rustfmt --edition 2024 --check` on my
files exit 0; `--bin cited` exit 0, `--bin quotations` exit 0 (3049 quotations, 3 diverging, none mine). On an export:
the census no-pack exit 0 (195 s, 1.67 GiB peak) and with the pack exit 0 (266 s, 2.55 GiB peak), each pdf.js line
one document from converted to refused against HEAD's and every corpora line unchanged; `cargo test --profile gates -p
pdf-archive --test corpus --test cross_check --test witnesses -- --ignored` exit 0 (93 s, 2.26 GiB peak; cross-check
12555 places, 0 disagreements; witnesses 1 of 1). Memory: a `/Kids` of 2 000 000 nulls, `getrusage` high-water
80 748 kB at HEAD and 80 748 kB with both rows. `cargo test -p conformance --no-fail-fast` exit 101 first, its two reds this record's length and a gate it had not yet
stated, then exit 0 (470 tests passed over 69 targets).

**Found, not fixed.** `crates/pdf-archive/src/table/fonts.rs:485` `is_a_font` recognises a font by `/Type /Font`, so a
font a page's `/Font` resource names without `/Type` (Table 109 requires it) escapes the font walk; veraPDF fails
HEAD's `issue7544` output on 6.2.11.2-1 and 6.2.10.2-1 (two checks, `BAAAAA+Arial`) where the checker passed it.
