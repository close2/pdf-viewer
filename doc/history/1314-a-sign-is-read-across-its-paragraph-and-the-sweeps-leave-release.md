# 1314 — A sign is read across its paragraph, and the sweeps leave the release profile

Instruments slot of batch forty-eight. ADRs 1463, 1464. No ledger row moved; no question written.

**The scanner (ADR 1464).** `citation.rs` reads who a `§` or a `Table` belongs to across the
paragraph — Markdown paragraph, run of one comment kind — never across a list item, table row,
heading, fence or code line; findings stay on the sign's line. Both `scan` and `scan_prose`, so the
Rust sources, instruction documents, ledger notes and `spec-errata` all take it. Old and new
scanner dumped over the same tree: 19 findings changed, none in the ledger — one new foreign `§`
(`doc/pdf-a-conversion-limits.md` 1418, ISO 19005-2, fixed with its neighbour on the line), twelve
signs counted as clauses that are ADR/RFC/document sections, two `§3d`s now named, three
ISO/TS 32002 tables. `main-checkout` now names Q169:18 twice. Two planted tests; one test rewritten
(a wrapped `ISO/TS` / `32002 Table 3` is now that standard's table).

**Overtaken notes.** 14 by the time they were read (ADRs 1431-1445): every one about `render-cpu`
or a reference while the ADR is raster's, a perf witness, or a mention — none false. 14 `READ`
entries moved; `DIFFERS_AT_THE_EDGES` and `DIFFERS_IN_SHAPE` left `READ` (no longer page lists);
the ArialNarrow heading in `AMBIGUOUS_SUBSTITUTED_FACE` made true. Count 14 → 0.

**Profiles (ADR 1463).** In the persistent directory no release/gates build compiled a dependency;
each pays the merge's workspace crates. Release conformance bins 127-130 s wall, 183-188 s CPU per
edit, dev 11 s; their runs differ by tenths of a second, so `state.sh` and `main-checkout` run them
under `dev`. `--profile gates --workspace --all-targets`: stopped at 25 min, 25 875 s CPU, 415 deps,
load 150, `raster` recompiled mid-run by a sibling — `open` keeps warming `dev` only. `sccache`
serves across directories (12/12) but sits at its 50 GiB ceiling.

**Owner's list.** `main-checkout` prints `doc/patches/` owed while the manifest pins a patch's
`Base:`; the JBIG2 patch gained `Repository:`/`Base:` lines (1312's file, preamble only, still
applies at `64efcaca`). *After a merge* item 6 corrected, item 7 the fork route; HANDOVER row.

**Documents.** state-of-play: ADRs 1443, 1444, 1447 (filter), 1450. crate-map: raster-gpu
(1443-1445), pdf-font (1441, 1449), pdf-sandbox (1447), conformance (1464). Navigation before/after:
13 → 14 absent (Q209, 1312's in-flight file), 26 undefined, history 0, overtaken 14 → 0.

**Gates.** `cargo test -p conformance` 0 (one interim failure: 1312's `t88_conformance.rs` `Table
K.1`, theirs); clippy conformance, spec-errata 0; rustfmt 0; `bash -n` 0; spec-errata tests 0.

Proposed habit: a figure about a shared cache starts from its size against its ceiling.
