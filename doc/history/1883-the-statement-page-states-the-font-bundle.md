# 1883 — the statement page states the font bundle, and a .notdef row's reason names the program's format

Session 1883, Slot 3 of batch 129, an archive round; ADRs 2600 and 2601 used, Q759 unused. I worked in the shared
worktree, the census in an export of it; three sessions, the second ended by the account's weekly limit.

**Built.** `append-statement` is admitted at `fonts/font-programs-embedded` (`config/steps.rs`: `STATED_FONTS`,
`statement_hands_on`, used where `STATED_GRAPHICS` alone was). `archive/statement/fonts.rs` writes one entry per
photographed page (number, resolution, loss words, fonts not drawn, content-stream requirements, structure leaves,
optional content groups with the state drawn) and one per face embedded with a loss (`/BaseFont`, resource, loss
words), each loss word with a fixed line. `prepare.rs` prepares the fonts before the preserved pages and an output
intent where the font row may be stated (ADR 2600). A page that cannot be composed hands on to `discard` and the
report says why. R03a's reason names a Type 1, OpenType or absent program by Table 124's key
(`fonts/notdef_format.rs`), and `NOTDEF_NOT_GIVEN` opens "the program a font embeds" (ADR 2601).

**Premise.** "One new section reading the per-page records already in the report; the bounds hold as they are":
held, with two additions the hypothesis missed — the fonts were prepared after the preserved pages, and a statement
page in a file with no output intent handed on for Table 51's DeviceGray (the narrow fixture showed it).

**Plant.** The entry's page number written one higher: the read-back fails with "the statement names page 2, which
the report does not"; reverted.

**Census.** Against HEAD's, with and without the CJK pack, alike: the census converts under `as-if-printed` and
`as-shown` and prints no `state-every-loss` line (0 and 0), so the slot's "unchanged in count" holds and witnesses
nothing — `--test profiles` and the read-back tests witness the admission. Mine: the R03a sentence, three pdf.js
documents and `504076.pdf` now naming a Type 1 program, two "the program a font embeds". Every other move is a
sibling's: five corpora sources freed as `page-not-held` (1881), the CID 0 sentence (1882; `issue12963` with 1884's
whole population), seven pdf.js documents off the TrueType rows (1884), the `/Type /Font` row's four refusals (1887);
1888's dictionary rule moves no line. `as-shown` pdf.js 813 → 820 converted at 2b, 818 → 825 at 4.

**Gates.** `cargo test -p pdf-transform --test archive --test profiles` 0 (513 and 13 passed); `--lib
notdef_format` 0; `rustfmt --edition 2024 --check` on my files 0; clippy `--no-deps -D warnings -p pdf-transform
--all-targets` 101, seven `arithmetic_side_effects` in `sites.rs` 1031–1113, 1887's `typed_fonts`, none in my files;
`--bin cited` 0, `--bin quotations` 0; `cargo test -p conformance --no-fail-fast` 101, one red, `--test evidence`
on 1887's record naming its handover path. Census no-pack 0 and pack 0, 706 s for both; `rank.py` 0 on each.
veraPDF: the two-page output PASS at 2b, the four-page and layered PASS at 4, each with bytes after `%%EOF` FAIL
one rule (6.1.3 test 3).

**Found, not fixed.** `settle.rs:81` folds the second pass's `photographed` into the report and not into the first
pass's statement page (ADR 2600, not decided there). A statement that cannot be composed hands every record to
`discard`, all or nothing, where listing what fits and handing on the rest would keep more.
