# 3071 — `chrono`'s unmaintained advisory, reached through `rxing` alone, is ignored by identifier

Session 2092, a dependency round. Status: accepted. Rows: none (a supply-chain decision; no clause is touched).
Context: ADRs 2877 and 2900 (`rxing` as the barcode decoder), 2038 (the form of an ignore in the shipped graph).

## Context

RustSec published RUSTSEC-2026-0543 on 2026-11-18: "The chrono crate is unmaintained; all versions are
affected", `informational = "unmaintained"`, no patched version. CI's `deny` job failed on it.
`cargo tree -i chrono --target all -e normal,build,dev` shows one route: `rxing` 0.9.4, the dev-dependency of
`xfa-barcode` that reads every symbol the crate encodes back as its data (ADR 2877) — evidence of readability,
never the definition. No crate a user runs contains `chrono`.

## The three routes, in the order taken

1. **A feature that drops it.** None: `rxing` 0.9.4 declares `chrono` (features `clock`, `std`, `oldtime`) as
   a required dependency; it stamps a decode result with `chrono::Utc::now()` in `rxing_result.rs`. The crates.io
   index, read on 2026-11-18, shows every release from 0.8.3 to 0.9.4 the same, and 0.9.4 is the newest.
2. **Another decoder for the tests that need one.** Not without losing an oracle. `datamatrix` 0.3.3 decodes
   Data Matrix and `zedbar` 0.7.0 reads Code 39, but no pure-Rust crate besides `rxing` decodes PDF417 (`pdf417`
   0.2.1 only encodes). Two new crates would replace one and still leave PDF417 on `rxing`; ADR 2900's argument
   (the decoder shares no code with the encoder the Data Matrix tables were taken from) would need restating too.
3. **Ignore the advisory by identifier**, written as `deny.toml`'s other unmaintained ignores are.

## Decision

`deny.toml` ignores RUSTSEC-2026-0543 with its route, its cost and its trigger beside it. The ignore names one
advisory, not a crate: a vulnerability advisory against `chrono` has its own identifier and still fails the job.

**The cost.** A fault later found in `chrono` gets no upstream fix, and `xfa-barcode`'s test binaries carry it.
They decode only symbols this tree's encoder wrote, on a developer's or CI's machine, never a document's bytes.

**The trigger.** An `rxing` release that drops `chrono` or makes it optional: `cargo deny` reports the line as
`advisory-not-detected`, and it is deleted when `rxing` is bumped. Also reopened if `chrono` enters the shipped
graph (`cargo tree -i chrono -e normal` finds a route), which this decision does not cover.
