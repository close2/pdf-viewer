# 594 — The second filter the window learned

ADR 0427 finished the first of `doc/todo/14`'s two residues — the four content streams §7.8.2 names
beside a page's `/Contents` — and left the second: a pump for the §7.4 filters that are not
`FlateDecode`. This round takes it, and it takes the one that matters: **`LZWDecode`, the sharper
bomb** (NOTE 2's 1365:1 against Flate's measured 1032:1). An LZW bomb in a content stream now costs
the window rather than the gibibyte.

Date: 2026-08-19.
ADR: [0429](../adr/0429-the-second-filter-the-window-learned.md).

## The residue taken, and the one left

Two residues. Residue 2 — the filter pump — is done. Residue 1 — **§8.7.3.1's tiling cell** — is
not, and is *not* what this round is about: the exclusion could not come off, because it is a
`pdf-render` change (`pdf_render::Repeats`, the cell drawn once and repeated) rather than a filter
one, and taking it would have been half-doing two things. It stays road D's last owed item.

## What now pumps, and what does not

A single `FlateDecode` or `LZWDecode`, no predictor. The three §7.4 filters left out are left out on
their expansion ratio, which is what a bomb needs: `ASCIIHexDecode` shrinks its input, `ASCII85Decode`
reaches 4:1 from a stream of nothing but `z`, `RunLengthDecode` 64:1 — none can name a bomb, so a
window saves them nothing.

## The construction

§7.4.4.2's algorithm moved into a resumable `filter::Lzw` — table, bit accumulator, input cursor —
whose `step` reads one code. `filter::lzw` (whole decode) is now a loop over `step`; `Lzw::pump`
(window) hands the code's sequence over in pieces across as many turns as the window has room for.
One decoder, two loops — trap 6's rule, the way `FlateDecode` already shared `turn`. `filter::Pump`
gained a `Pumping` route enum and became a two-variant `Engine`; the route is carried
(`Pump::pumping`) so a stream read more than once builds the same decoder each read.
`Document::is_pumpable` became `Document::pumping`, the one place a chain's route is decided.

## Numbers this round printed

`lzw_bomb.py` (rebuildable, in a scratchpad — not committed, like `doc/todo/10` §2's flate bombs):
a 2.37 MB file naming a 1.5 GiB LZW `/Contents`, `n\n` operators so the decode is a program rather
than one token, 681:1. A/B in one sitting with this round's own patch (`Document::pumping` reverted
to Flate-only), release, `RAYON_NUM_THREADS=1`, `VmHWM` from `/proc`:

| the LZW bomb in `/Contents` | before (whole) | after (pumped) |
|---|---|---|
| peak resident | 1035 MB | **10.2 MB** |
| interpret wall clock | 2.12 s | **0.11 s** |
| report | `TooLarge { part: Some(0), limit: 1 GiB }` | `MAX_OPERATIONS` |

Both directions improve — the whole route allocates a gibibyte it then refuses; the window allocates
none.

## Correctness

`examples/display_list_digest` over every pdf.js corpus document's page one, same
`pdf-sandbox-worker` on disk both arms: **byte-identical**, `sha256 04a07587…`, 974 documents, HEAD
against the working tree. No committed document states a single-`LZWDecode` content stream large
enough to route through the window (trap 8), so the routing change is invisible to the corpus by
construction — the unit tests carry it: `an_lzw_pump_and_the_whole_decode_agree` (the clause's
example and a table-filling bomb through windows of 1, 3, 7, 64, 4096 bytes),
`the_lzw_pump_reports_the_damage_the_whole_decode_does`, `an_lzw_bomb_costs_the_window_rather_than_its_decode`,
and `nested_content_window.rs`'s new LZW form arm.

## Fuzzing

`page`, seeded with `seed_nested_content.py`'s 26 documents plus a small LZW `/Contents` page and an
LZW form whose decode outgrows the memo, `-fork=6 -rss_limit_mb=4096 -timeout=60`, about ten minutes.
**No crash, OOM or new hang.** One `timeout-` artifact minimised during the run is a `FlateDecode`
form driving inline-image `EI`-scan lookahead — 59 s in release, single-thread, and it *terminates*
— a pre-existing §8.9.7 pathology naming no LZW, on a path the digest proved byte-identical.

## The spec-driven half — the twenty struck passages

Session 591's repair of the errata matcher made 27 struck passages newly visible; 3 (Annex A) and
what turned out to be 5 (clause 13, not 4) are out of scope, leaving **nineteen** unread — the
"twenty" was nineteen, because eight of them straddle a page from the clause the sentence is in and
two clauses carry two lines each. Read, with `spec-errata check` rebuilt against its pre-repair body
to name the new lines with nothing inferred. Three are findings:

- **§9.8.3.3 (#5)** — the clause's self-contradiction the ledger recorded (a CIDFont `/FD` "subset of
  Table 120" that also states metrics) is gone in the amended text; the prohibition moved to Table
  122. No behaviour owed, the reason corrected.
- **§12.3.2.4 (#162, #288)** — a named destination's `/SD` (structure destination) became reachable
  when the erratum repointed it from Table 201 to Tables 202–204. `Destination::of_go_to` applies
  the precedence; `Destination::read_within`, which reads named destinations, never looks for `/SD`.
  **Behaviour owed**, priced below.
- **§9.6.2.2 (#384)** — the fourteenth standard font is `Courier-BoldOblique`; `doc/md/` loses the
  hyphen because the standard sets it broken across a line, and `pdf_font::standard`'s comment reads
  the missing hyphen as "the standard's own typography". A `doc/todo/48` step-4 witness.

The rest cite or are untouched. The subagent corrected seven ledger notes (§7.5.8.2, §9.6.2.2,
§9.8.3.3, §12.3.2.3, §12.3.2.4, §12.5.6.3, §12.11.1), recorded the nineteen in `doc/errata-read.md`,
and rewrote `doc/todo/48` step 3b.

**The one behaviour owed, priced:** `Destination::read_within`'s dictionary arm needs `of_go_to`'s
`/SD`-then-`/D` order, a test only that arm answers, and a corpus count of named destinations stating
`/SD`; §12.3.2.4's row moves to `partial`. About two lines and half a session. Not done this round —
it is a `pdf-model` feature unrelated to the parser work, and it wants its own test and count.

## Gates

`doc/todo/02` §2's full sequence (this round touches `pdf-syntax`/`pdf-model`). Binaries rebuilt and
installed before measuring, per §5. Numbers came off the runs; see the ADR.

## Files

- `crates/pdf-syntax/src/filter.rs` — `Lzw`/`Step`/`Lzw::step`/`Lzw::pump`; `lzw` a loop; `Pump`
  gains `Pumping`/`Engine`/`Inflate`; four new tests.
- `crates/pdf-syntax/src/document.rs` — `pumping` (was `is_pumpable`).
- `crates/pdf-syntax/src/lib.rs` — exports `Pumping`.
- `crates/pdf-model/src/content/reader.rs`, `crates/pdf-model/tests/nested_content_window.rs`.
- `doc/conformance/ledger.toml` — §7.4.4.2, §7.8.2 (this round); §7.5.8.2, §9.6.2.2, §9.8.3.3,
  §12.3.2.3, §12.3.2.4, §12.5.6.3, §12.11.1 (the errata half).
- `doc/adr/0429`, `doc/todo/14`, `doc/errata-read.md`, `doc/todo/48`.
