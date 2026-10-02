# 1332 — A record states its gates, and the disk is printed

Instruments slot of batch fifty-one. ADRs 1499, 1500. No ledger row moved; no question written.

**Records (ADR 1499).** `tests/records.rs` now also holds every record from 1327 on to a paragraph
opening `**Gates.**` with a figure (an exit status or pass count) and no "see the report"; planted
records of each verdict calibrate it. Of 1284–1326 the shape counted: 16 such paragraphs (3 of them
"see the report": 1316, 1319, 1322), 5 `## Gates`, 3 other spellings, 19 none (1284, 1287–1289,
1293–1296, 1299, 1301, 1302, 1305, 1307, 1311, 1313, 1318, 1324–1326). Printed by `state.sh
records`, never failed: records are not rewritten. `todo/02` sections 6 and 8 name the rule.

**Disk (ADR 1500).** `state.sh disk` prints, read-only: this tree's directory (batch, 164 G), the
main checkout's (`/home/AI/cargo-target/pdf-viewer`, 637 G, `debug` 582 G — read from the cargo
config, no cargo run there), each directory under the root (857 G) with profiles and last-written
date (`quorra` 45 G, 2026-09-01; `r1285` 12 G, 2026-09-29: nothing names either), sccache 51 G
against its 50 G ceiling (off the disk: `--show-stats` starts a server when none runs), 622 G
free on `/`, scratchpad. `environment.md` gives the pruning commands (the batch's `debug` at a
boundary past 100 GB; the main checkout's `debug`/`gates` past 100 GB; an unnamed directory) and
no longer names `quorra` as the main checkout's directory; `todo/02` sections 5 and 5a follow.

**Batches.** One line per batch: sum, figures, rounds named, entries with no `<n> s`, and a
`<n> s + <m>` with no unit — 8566f66e prints "not summed, no unit: 264 s + 5". `todo/02` section 8
item 4 gives the exact shape.

**Curves.** `cx448` 0.1.1 (RFC 8032 verify with ctx and Ed448ph, but unreviewed by its README, a
self-declared temporary port, previous RustCrypto generation) and `tiny_ed448_goldilocks` 0.2.0
(no signature scheme, unaudited): neither a candidate; `todo/65` and Q192 (appended) say so.
`todo/65` re-derived after 1331's bucket: 15 rows, map gate passes; intro and README name its shapes.

**Documents.** PLAN: banded SSIM (1481), `WHOSE_DEPARTURE` (1483), records, disk, batches.
crate-map: `raster-compare` (1481), `records.rs`. state-of-play: the standard 14 answered before
the descriptor ranking (1483). HANDOVER: the robustness row reads the whose-departure ranking
first; the fuzz row names `fuzz/seeds.sh` and `INITED` beside `DONE`; the host row's verdicts are
read off the window, one golden (1478). Navigation before and after: 14 absent, 28 undefined,
19 overtaken, history 0.

**Gates.** `cargo test -p conformance` exit 0 at the end (`fuzz_workspace` failed mid-batch on
1330's targets in flight); `records` 3/3, `read_only` 2/2, `the_frontier_map` 1/1. clippy `-D
warnings` conformance 0; rustfmt `records.rs` 0; `bash -n tools/state.sh tools/batch.sh` 0;
`batch.sh check` exit 1, only `cargo fmt --all --check` over other rounds' files.
