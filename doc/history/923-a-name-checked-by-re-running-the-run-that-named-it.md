# 923 — A name checked by re-running the run that named it

Date: 2026-09-04.
ADRs: [0886](../adr/0886-a-name-validated-by-re-running-the-run-that-named-it.md),
[0887](../adr/0887-the-cache-is-right-to-refuse-and-the-instruments-bound-comes-off.md).
Touched: `crates/pdf-vfs/src/cache.rs`, `crates/pdf-vfs/src/lib.rs`,
`crates/pdf-vfs/tests/a_face.rs`, `crates/pdf-vfs/tests/read_corpus.rs`,
`crates/pdf-vfs/examples/vfs_cost.rs`, `doc/todo/58`, `doc/questions/Q14`,
`doc/traps/instruments-and-reports.md`, `doc/HANDOVER.md`, two ADRs, this file.
**No pixel moves**: no crate that draws changed.

A cost round, on the one document that has held two corpus walks for twenty-five minutes each. The
round was asked to separate three tangled things before writing code, and separating them is most
of what it did: the measurement disagreed with the diagnosis three documents had recorded, and two
of the three "defects" turned out not to be defects at all.

## What was recorded, and what was true

`doc/todo/58` §5, ADR 0878 and `read_corpus.rs`'s own constant all said the same thing about
`corpus-cache/tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf` — 10 084 images on one page: a
`stat` generates, a read puts a whole extraction run in the cache at once, and **a run too large
for the cache's budget is put nowhere at all**, so each of twenty thousand questions re-ran an
extraction of ten thousand images.

Measured, in process, before touching anything: the outputs are **352 bytes each**, the whole run
is 3.4 MiB against a 64 MiB budget, and `Vfs::generated` — the tree's own count of what it produced
— sat at **1** for the entire run while every `stat` and every `open` took **176 ms**, which is
what one `pdf_transform::images` run over that page costs.

The cache was doing its job perfectly. What was costing the time was `locate_in`:

```rust
if !spells(current, path, page) || !images(current, page)?.contains_key(&name) {
```

— `images()` being the extraction. Every `stat`, `open` and `list` under `images/NNNN/` validated
the name by re-running the run that produced it. ADR 0886 has the fix: the cache gains a third kind
of entry, a **directory's own names**, kept beside the sizes and outside the byte budget, and the
one function that runs the extraction now puts the bytes *and* notes the names.

| on that document, `/images/0001` (10 081 entries) | before | after |
|---|---|---|
| the listing | 0.33 s | 0.25 s |
| the same listing again | 0.20 s | 0.31 ms |
| one `stat`, one `open` | 176 ms each | 0.010 ms each |
| 512 entries `stat`ed and read | **274 s**, measured | — |
| all 10 081 `stat`ed and read | ≈ 90 min, extrapolated | **0.25 s**, measured |

On an ordinary document — the tagged-PDF guide's page 60, two images — 53 ms and then 17 ms a
question became 4 µs and 2 µs, because the listing that finds the names now warms the bytes.

## The three questions, separated

1. **The cache's admission rule is right and is kept** (ADR 0887 §1). Refusing an entry larger than
   the whole budget is not "refusing what is most expensive to recompute"; admitting it would evict
   everything else for one item a reader already holds through its `Arc`, ADR 0865 §3's size note
   already makes the `stat` free, and the four alternatives were priced. What the cache was missing
   was a *third kind of note*, not a fourth policy.
2. **The layout is not at fault, and the fix does not touch it.** Session 899's departure —
   `images/` a directory per page — is exactly what kept a listing to one extraction of one page.
   `doc/questions/Q14` gets the measurement as an addition; the question stays open, unchanged.
3. **The instrument's bound came off.** `ENTRIES_SAMPLED = 4` was a bound on the walk rather than a
   fix, and ADR 0878 said so. `read_corpus.rs` now reads every entry of every `images/NNNN/` and
   `attachments/` it lists on a widened document, and the "entries listed and not read" column is
   gone because it is always zero. `PAGES_SAMPLED` stays: it bounds a different cost. The walk is
   **1132 documents in 162.9 s against session 919's 315.5 s** while `stat`ing 31 435 entries
   against 20 976 and reading 24 733 files against 14 274 — half the clock for half again as much
   work, every disagreement column still zero, 0 killed, 0 did not recover. (After merging
   `main` — round 920's resource port among it — the same walk is **107.5 s**, which is that
   round's number as much as this one's; 162.9 s is the figure this change is answerable for,
   because it was taken on session 919's own tree with only this change on it.)

## The lesson, which is trap 33

(Trap **32** at the merge: round 920 took the number on `main` first, and this one renumbered.)

`Vfs::generated` read 1 throughout. Two corpus walks, a `regenerated` report and a whole gate
sequence looked straight at it and saw nothing, because it counts *productions* and the cost was
paid in *validation* — no allocation, no output, no error, and a hundredfold. The new test counts
`Query::ExtractImages` questions with a counting `Workers` decorator instead, and (trap 13) fails
**6 against 1** with the two lines restored.

The other half of that is in `doc/todo/58` §5 now: this crate still has **no perf floor in
`doc/todo/02` §2**, and this defect is the argument for one.
