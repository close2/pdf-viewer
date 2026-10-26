# 1746 — A CIDFontType2 over a name-keyed CFF is refiled, and a font collection is in no row

Slot 2 of batch one hundred and fourteen, 2026-10-25, an archive round; ADRs 2326, 2327; no question.
I worked in `scratchpad/r1746/`. No ledger status moves; the §9.7.4.2 and §9.9 rows' notes and tests grow.
**Refile (ADR 2326).** An `OTTO` program under a `CIDFontType2` with an `Identity` map and a name-keyed
CFF is retyped `CIDFontType0`, its map removed, its program moved to `/FontFile3` `/OpenType`: Table
115's `Identity` and §9.7.4.2's "The CIDs shall be used directly as GID values" select the same glyph
for every CID. A stream map, no map or a CID-keyed program is refused by a sentence of its own.
**Collections (ADR 2327).** Not held here; I read Apple's TrueType Reference Manual, chapter 6, and
Microsoft's OpenType font-file chapter from their sites: a TrueType font file is one font opening
with a font directory whose scaler type names its outlines, and no `ttcf`; a collection holds several
fonts' directories. `table_124_rows` gives it no row, the validator names it, the conversion refuses
it by name. `key_for` now takes the first of `table_124_rows`' rows: 320 face-and-subtype pairings
compared against the old body before it was deleted, all equal; a planted `last()` failed them.
**Sentences.** `transcode.rs`: the decode's four reasons (budget or decoder; no PDF space, PIMA 7667
included; a contradicting codestream; anything else) one each. `normal-appearance-shape`: no `/AS`; an
`/AS` not held; no `N` (`/N 0 0 R` is none, §7.3.10, §7.3.7); structural failures and no spare number apart.
**Premises.** "Wins back `bug1186827`": did not hold — its map is a stream sending CID *c* to glyph
*c* − 31, so it stays refused, now by the new sentence. "`sakula`'s line is no `N`": held in the
validator, not in the site, which read `/N 0 0 R` as an object until the null rule above. "The `ttcf`
pair stays refused": they were not refused at HEAD, they converted into outputs veraPDF failed.
**Drawn** (looked at): `issue5084` page 1, source against its outputs, byte-identical PNGs at 100 dpi
and 360; veraPDF 1.30.3 PASS at 2b and 4 under both profiles.
**Census** (converted / conforming / refused / unreadable), `doc/corpora` unchanged in all four:
| pdf.js | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| HEAD → this | 773/2/181/11 → 772/2/182/11 | 767/0/189/11 → 766/0/190/11 | 751/2/203/11 → 750/2/204/11 | 746/0/210/11 → 745/0/211/11 |
By document, every column: `issue5084` converts, the `ttcf` pair is refused, two sentences move.
**Gates.** In an export of HEAD and my own hunks (ledger as a two-hunk patch, habit 162): `cargo fmt
--check`, `-D warnings` clippy `--all-targets` and rustdoc `-D warnings --document-private-items` on
pdf-archive and pdf-transform exit 0; nextest 984 passed, 0 failed, 17 skipped. Plants: the identity
check, the collection check and the null-`N` filter each taken out fail their tests (trap 13).
`tools/state.sh --round 1746 archive` exit 0: validator over the veraPDF corpus missed 0 and over 0 at
all six targets; converter over it 2b 526/85, 4 271/88, every conforming input still conforming;
cross-check 13 560 compared, 0 disagreements. Census `--tree 6` exit 0 before (the worktree, my crates
at HEAD) and after (the export). Arms not compared: neither crate is in `render-raster`'s graph
(`cargo tree -i`, habit 119). `cargo test -p conformance` in the worktree 463 passed, 1 failed (1748's
record). `tools/state.sh docs` stopped in the queue after 4515 s, before its grant: rustdoc as above.
**Found, not fixed.** `bug1186827` wants its encoding and widths re-keyed to glyph indices; a
collection's face (ADR 0141's choice) written out as its own program would answer the `ttcf` pair. 7663 s.
