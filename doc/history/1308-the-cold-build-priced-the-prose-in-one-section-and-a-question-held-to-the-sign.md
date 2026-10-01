# 1308 — The cold build priced, the prose in one section, and a question held to the sign

Instruments slot of batch forty-seven. ADRs 1451, 1452. No ledger row moved; no question written.

**The cold build (ADR 1451).** First build in `/home/AI/cargo-target/pdf-viewer-batch`:
`cargo build --workspace --all-targets` 354 s wall, 1 643 + 158 s CPU, load 0.3 rising to 46 as
siblings' `release` and `gates` builds ran beside it. After siblings edited `pdf-font` and
`raster-gpu`: 553 s (one lock wait), 25 packages, 1 380 s CPU; `cargo nextest run --workspace
--no-run` 180 s, 24 packages; an immediate rebuild 7 s. A sibling's `dev` build held no `rustc`
until mine ended (one lock per profile directory, no duplicated work); `release` and `gates` had
24 children each (their own locks, the same dependencies compiled again). `sccache`'s shared server
counted 208 Rust hits / 977 misses machine-wide over the cold build. `batch.sh open` now starts
the `dev` build detached into `scratchpad/open/build.log` while the briefs are written
(`BATCH_WARM=0` skips it; no-op without a `Cargo.toml`), driven against a fake `cargo`.

**`tools/state.sh prose`** — seven lines, one per sweep (comments, superlatives, overtaken, unread,
names, variables, the ledger gate for program names), each with its listing command; in `all` and
`quick`, 4 s warm. `tests/state_sections.rs` now holds `all`/`quick`/`composed` to the case arms
both ways; its first parse dropped `jpeg2000` for the digit.

**The `§` rule (ADR 1452).** The population stays: a Q file is the project's writing ("Source: this
round"), read to do work; the A files are already out. `--bin section_signs <root> [files]` runs
the gate's scan elsewhere; `main-checkout` now prints Q169:18 and Q170:12. Q169:18's other `§`
(ISO 19005-2, name wrapped onto line 17) is missed by the scanner and the gate alike — not closed.
`doc/environment.md` *After a merge* item 6 names both files and the fix.

**Documents.** state-of-play: ADRs 1430, 1431, 1433, 1435 (twice), 1437. crate-map: `quorra`
for `pdf-viewer`, `machine_faces`' words, raster-gpu's two stroke paths, `tile_stroke`, the clip
`min`, `area_averaged`, the conformance tests and bin. PLAN: `prose`. HANDOVER: latency row names
the 2026-09-30 rows and trap 78. Navigation 13 absent / 26 undefined, history grep 0 — the same as
1302's reading; `overtaken` now 8 (ADRs since 1302), not read here.

**todo 65.** 14 rows, unchanged (§9.9.x all `implemented`). `cargo search`: no `bp512`;
`aegis-crypto` 0.1.5 has brainpoolP512r1 under PolyForm-Noncommercial; `krypteia-arcana` 0.2.0
states neither curve; `ed448-goldilocks` still `0.14.0-pre.15`. `tests/todo_citers.rs`: all eight
`Cited by:` lines resolve; planted crate fails.

**Records 1290–1302**: every one ≤ 40 lines; the numbers they carry are each round's own runs.
**Gates.** conformance: all pass but `the_ledger_agrees…` (1307's §9.9.1 test name, in flight);
clippy conformance 0; rustfmt 0; `bash -n` 0; `batch.rs` 6/6.
