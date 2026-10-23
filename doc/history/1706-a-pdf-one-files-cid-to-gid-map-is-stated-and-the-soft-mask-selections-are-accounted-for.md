# 1706 — A PDF 1.x file's absent CIDToGIDMap is stated at part 4, and the soft-mask selections are accounted for

Slot 3 of batch one hundred and nine, 2026-10-23, an archive round. ADRs 2246, 2247; Q569.
**Census** (converted / conforming / refused / unreadable), HEAD `ee365e4f` → this round's export:

| corpus | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| pdf.js (967) | 756/2/198/11 | 703/0/253/11 → 722/0/234/11 | 752/2/202/11 | 699/0/257/11 → 718/0/238/11 |
| `doc/corpora` (491) | 340/6/129/16 | 309/0/166/16 → 330/0/145/16 | 319/6/150/16 | 295/0/180/16 → 309/0/166/16 |

Top refusals: a composite font nobody named a program for (pdf.js, 37 in every column); the
content-stream fence (`doc/corpora`, 33); `to-unicode-values-are-usable` (pdf.js 4, 32);
`machine-face` (`doc/corpora` `as-shown`, 31); `cid-to-gid-map-present` (part 4, 25 + 30, the top
one over both corpora). Second-verdict refusals (`output-fails`, habit 147) unchanged.
**Built.** (1) ADR 2246: at part 4, an embedded Type 2 CIDFont with no `/CIDToGIDMap` in a file
labelled below PDF 2.0 is given `Identity`, a statement under both profiles: ISO 32000-2 §I.2
makes the label the edition describing the file, and ISO 32000-1 defaults the entry to `Identity`.
All 55 documents the row refused are labelled 1.3–1.7; it refuses none of them now, 40 / 33
convert, none is newly refused; a 2.0 label still refuses (control test). (2) ADR 2247: the
thirteen soft-mask selections, judged by A61 — each is under a luminosity mask (twelve veraPDF
`6-2-9`/`6-2-10` shadings, one Isartor `ExtGState`) set from a place both machines compared, and
each file fails the blending-space row its fixture names. The cross-check now counts that, 13 of
13, and asserts the survey's own selections under a soft mask are 0.
**Hypotheses.** "A colour or image rule": did not hold, the top two are font rows; the composite
one stays refused — half `Adobe-Identity` (§9.7.4.2), half wanting a shipped CID-keyed face (Q569,
recommending not now). "The thirteen are judgeable under A61 and most pass": held, all thirteen.
**Witnesses** (100 dpi, page 1): byte-identical for seven (`ArabicCIDTrueType`, `vertical`, `eu-001`,
`wp592`, …), both profiles; `PDFBOX-5809` 16 393 px at 1 level (its added output intent, looked
at); `008677`, `316267` differ by `machine-face` under `as-if-printed`, refused under `as-shown`.
**Gates.** In export `mine` (habit 138): `cargo fmt --check`, `-D warnings` clippy `--all-targets`
on pdf-transform and pdf-archive exit 0; nextest 946 passed, 0 failed, 16 skipped; scoped rustdoc
`-D warnings` exit 0. Census two holds under `--tree 12`, exit 0, 223 s and 218 s. Cross-check exit
0, 0 disagreements; a planted setter moved 13 compared to 0 and named all 13 (trap 13).
`tools/state.sh --round 1706 archive` exit 0: validator missed 0, over 0; `archive_corpus` 2b
525/86, 4 267/92. `t3-foreign_corpus` `--clock --tree 12` exit 0, 200 s held. veraPDF 1.30.3
`-f 4`, 18 outputs: 18 PASS. `cargo test -p conformance`: export all ok but `bounded`, which reads
git's population, empty there; in the worktree `bounded` 24 passed. Arms not compared:
`cargo tree -p render-raster` holds neither crate (habit 119).
**Found, not fixed.** Twelve of the masks paint `DeviceRGB` with no output intent, which the survey
does not walk (ADR 2247). 4 900 s.
