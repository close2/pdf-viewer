# 1968 — A kept `/ModDate` is compared as an instant, and the packet's other instant is restated from it

Session 1968, a single round, PDF/A lane's worktree. R18 (ADR 2748); R05's last nine decided (ADR 2749).

**The premise.** Two of three. R18's sentence is `MODIFICATION_DATES_DISAGREE`, each document's only
refusal under `as-if-printed` at 4 (each also stops at R09's font losses under `as-shown`).
`225188` states one instant twice (`D:20060922094100-07'00'`, `2006-09-22T09:41-07:00`): the comparison
was of spellings. `20789_ftp` (`+05'30'`, two days after the packet and the PieceInfo's
`LastModified`) and `UMTRI-2008-80` (Quartz's 2023 append) do disagree.

**Built (ADR 2748).** The kept `/ModDate` and `xmp:ModifyDate` are compared as instants (an XMP Date
read into §7.9.4's fields with its defaults; a zone on one side only, or a non-zero fraction, is not
equivalence). Where they differ, §14.5 compares `LastModified` with the dictionary's entry alone, so
it stays byte for byte and the packet's property is cut and restated from it; the former value goes
into `xmpMM:History` (`modification-date`), a condition of the restatement
(`NO_PLACE_TO_RECORD_A_DATE`). New row key `disagreeing`: `discard` in `as-shown`, `as-if-printed`,
`only-metadata-loss`; `stop` elsewhere (`state-every-loss` would have to state a packet property on its
page).

**Freed** at 4: `as-if-printed` all three; `state-every-loss` `225188`. **Pixels**: no page object
changes (the Info dictionary and the packet only).

**R05 (ADR 2749, not built).** Every `/OPM 1` of the nine written 0 by hand: 773 pages at 72 dpi on the
CPU, every one with difference 0 (controls: twice-drawn 0; a changed `k` 202). Still ADR 1978's
`overprint-mode` loss, a press's instruction, which both shipped profiles already accept; the
construction is written down for the next round.

**Tests** (`archive/kept_modification_date.rs`, four): one instant in four spellings converts under
all seven profiles unchanged; a one-sided zone and a fraction refuse; another instant is restated under
the three with the history entry; the four others refuse. Plants: a spelling comparison fails the
first; no zone check fails the second; no cut fails the third; `disagreeing` defaulting to `discard`
fails the fourth.

**Census** (against round 1967): shipped 3 NEW (the three, `as-if-printed/4`), 0 GONE, 0 CHANGED;
stating 1 NEW (`225188`, `state-every-loss/4`), 0 GONE, 0 CHANGED. R18 leaves the rank. veraPDF
1.30.3 passes the four NEW census outputs and the four CLI outputs on the route.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-transform; nextest pdf-transform
and pdf-archive 0 (1443 passed); archive tier 0; both censuses 0; `cargo test -p conformance` 0
after `git add -N`, the 14.3.4 and 14.5 rows naming the new tests; `tools/state.sh --round 1968 docs` 0.
