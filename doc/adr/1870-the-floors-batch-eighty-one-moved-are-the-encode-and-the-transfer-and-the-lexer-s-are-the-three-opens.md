# 1870 — The floors batch eighty-one moved are the encode and the transfer, and the lexer's are the three opens

Session 1516, an instruments round of batch eighty-two. Status: **accepted**. It records the A/B that
batch eighty-one's merge did not take. That merge moved 22 floors in `doc/checks/turn-path.toml`
and 5 in `doc/checks/launch-path.toml`. It named ADRs 1857, 1858 and 1861 and the day's machine
together, and it did not separate them. This ADR changes no band; it changes the reasons beside
them. Context: ADRs 1513, 1577, 1607, 1861, 1863; trap 98 (A/B the change compiled out before a
band is moved or a change is blamed).

## 1. The A/B

Two exports of HEAD `c6fb18fb`, each with its own target directory and its corpora linked (traps 50
and 150). One is HEAD. The other has `crates/pdf-syntax/src/lexer.rs` and
`crates/pdf-model/src/content/reader.rs` as they were at `4c0c04f9`. Every hunk between those two
commits in both files is ADR 1861's (`Lexer::token_at_cursor` and `token_here`;
`with_operand`, `string_end` and `content_token_at_cursor` in place of `may_open_a_string`,
`step_over_a_string` and `ahead`). `run.rs`'s one hunk is ADR 1855's, and it is kept. The two test
binaries differ (`md5sum`). One `--clock` hold of 384 s ran `turn_path` HEAD, without, then
`launch_path` HEAD, without, then `turn_path` without, HEAD, at load averages of 2.5 to 3.6. Every
`turn_path` run judged 33 of 33 figures inside. Each `launch_path` run measured all six documents.
Both failed only on the device's `bring_up_ms`, which is slot 3's of this batch, and on one
`first_page_ms` each, on different documents.

## 2. What moved what

- **The three opens' `open_kinstructions` are ADR 1861's, all of the move.** Compiled out, they
  count 4 031.383, 13 041.702 and 70 297.058 thousand. HEAD counts 3 972.634, 12 752.380 and
  68 847.136, so the change takes off 1.46 %, 2.22 % and 2.06 %. These are counts, and they repeat
  the merge's own figures to the thousandth.
- **The two launch turns are not the lexer's.** The lexer-free build reads 3.685 and 3.737 ms where
  HEAD reads 3.683 and 3.839. The gate prints no stage of a turn.
- **No `turn_path` figure is the lexer's.** No row is quicker with the lexer by more than the
  spread of two runs an arm. Where the arms differ by more than a few tenths, HEAD is the slower one,
  and the difference is in a stage the lexer does not reach. The stroked Type 3 page's turn reads
  9.74–9.88 ms on HEAD and 8.54–10.29 without, all of the gap in the encode. `images`' seventh frame
  reads 31.40–31.56 on HEAD and 30.08–30.98 without, a page of two commands whose interpretation is
  a photograph's decode.
  The one row where the lexer shows is `bug1721218_reduced.pdf`'s interpretation, 52.54 → 50.69 ms.
  That is 1.85 ms of the 9.19 ms the row's interpretation moved since the table, and it stays inside
  the turn's spread (128.24 against 128.13).
- **The stage that moved each turn-path floor**, against `doc/performance.md`'s table of 2026-10-06
  and 2026-10-07, as `turn_path` prints its stages:
  - **the encode** on the text page, the stroke pages and the Type 3 page — ISO 32000-2 p101
    4.83 → 3.07 ms, `issue19802` 4.30 → 3.43, `issue14415` 11.06 → 9.70, the Type 3 page's seventh
    8.61 → 7.20, `bug1743245` 33.25 → 31.49 (seventh 35.12 → 31.32).
  - **the transfer** on the photograph pages — `images` 6.13 → 3.09, `issue13931` 3.94 → 2.70 (step
    4.98 → 3.65), `22060` step 6.61 → 5.86.
  - **the interpretation** where it is a decode — `issue12841`'s photograph 29.20 → 25.15, and
    `bug1721218`'s 59.88 → 50.69 with the scene beside it.
  - **what the gate prints no stage for** on the mesh page — `personwithdog`'s step 5.09 → 3.13,
    scene and elsewhere together.

  Each band's comment now says its own stage, these figures and the A/B.

## 3. What is not separated

Between the table and HEAD lie the device lane's changes (ADRs 1835, 1857, 1858, 1859, 1860) and a
machine that the merge's own A/B found turning HEAD's page under the old floor. This hold compiled
out ADR 1861 only, so the encode's and the transfer's moves are not divided between those changes
and the day. Dividing them is an A/B of the device-lane changes. No band hangs on it now, because
the floors already sit at this file's rule under the figures the gate reads.
