# 867 — The transform seam, and `render`, `images`, `attachments` on it

2026-09-01. Argued in [ADR 0800](../adr/0800-the-transform-seam-and-its-first-three-verbs.md).
The first implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`, started by the owner's word of the same day: "Please start the
command line features."

Touched: `crates/pdf-transform/` (new — `Cargo.toml`, `src/lib.rs`, `src/range.rs`,
`src/pattern.rs`, `src/json.rs`, `src/render.rs`, `src/images.rs`, `src/attachments.rs`,
`src/bin/pdf-transform.rs`, `tests/verbs.rs`), `Cargo.toml` (the workspace dependency line),
`doc/rfc/0002-the-transform-suite.md` (status), `doc/rfc/README.md`, `doc/crate-map.md`,
`doc/state-of-play.md`, `doc/todo/README.md`, `doc/todo/57-the-transform-suite.md` (new),
`doc/adr/0800-…` (new), this file.

## 1. What landed

RFC 0002 §14's first landing, complete: the `pdf-transform` library crate with the seam's public
types (plan, source, sinks, policy, budget, report, typed refusals), §4.2's range grammar with
unit tests whose expected values are the grammar's table, §4.3's output-name patterns with §8's
sanitisation, the binary with §4.1's subcommand shape, §4.4's exit statuses and §4.5's JSON
report, and three verbs end to end — `render` (PNG and PPM, `--dpi` / `--scale-to`, rayon across
pages), `images` (inventory and decoded-PNG extraction, each object once, forms descended) and
`attachments` (list, save all, save one). An integration test runs the verbs through the seam and
through the program against the committed documents under `doc/` and one corpus document, and
holds a rendered page byte for byte to the oracle backend's raster produced independently in the
test.

No dependency was added. No existing crate was changed. `CLAUDE.md` was not touched: no writer
landed, so RFC §11's amendment is not yet needed.

## 2. The defaults assumed, because §13 was not answered question by question

ADR 0800 §6 has the table. In one line each: no writer and no `CLAUDE.md` amendment yet; no
DCT encoder; confinement tranche one in-process, **which means the CLI parses and interprets
untrusted documents in its own process and only the three codecs the tree already confines go
through the worker** — the tranche's known cost, said plainly; restrictions default `off` with
`--restrictions=on|warn`; `pdf-transform` for crate and binary with `pdf-retrieve` separate;
deterministic output with no clock reachable. Every one of these is the owner's to overrule.

## 3. The throughput baseline

Measured once, per `doc/habits.md` *Measuring*, with the `gates`-profile binary
(`cargo build --profile gates -p pdf-transform --bin pdf-transform`) on this machine's 24
threads, rendering pages 1–200 of `doc/ISO_32000-2_sponsored_EC3.pdf` at the default 150 dpi to
PNG, a quiet machine, the second of two runs:

| arrangement | wall | user CPU |
|---|---|---|
| 24 threads (rayon's default pool) | 1.02 s | 18.9 s |
| `RAYON_NUM_THREADS=1` | 8.09 s | 7.9 s |

So about **200 pages/s** parallel and **25 pages/s** on one thread, an 8× wall-clock gain from
24 threads at 2.4× the CPU. The gap is the first thing the next transform round should look at
before choosing anything: one `FontCache` per worker thread re-parses the fonts one cache would
parse once, and `interpret` already bands §8.9.5's colour conversion across the global pool, so
the outer parallelism is contending with the inner. `images` over the whole standard — 224
image `XObject`s, every one decoded and written — took 0.38 s wall.

**Not gated.** No transform gate exists to carry a floor, and the tree's perf gates are the
viewer's launch-path ones (`doc/performance.md`); RFC §12's rule that transform gates carry perf
floors from their first landing is therefore owed by the round that creates the gate, and
`doc/todo/57` names it. The number above is the baseline it starts from.

## 4. Gates

The six core lines and `cargo test -p conformance` were run in this worktree at `-j 12` (a
parallel round was gating in the main checkout); their results are in the round's report and not
here, per `doc/todo/02` §2's rule that a number is current only for the round that watched it
print. No first-row crate was changed, so the corpus gates were not owed.

## 5. What the next transform round does first

`doc/todo/57`'s order: the CPU-time gap above; the transform gate with a perf floor; then the
serializer round (RFC §10) — which needs the owner's answer to §13 question 1 before it can
start — and `split` on it.
